pub struct SseEventParser {
    buffer: Vec<u8>,
    current_data: String,
}

const MAX_RESPONSES_JSON_DOCUMENTS: usize = 16;
const MAX_RESPONSES_JSON_BYTES: usize = 16 * 1024 * 1024;

impl Default for SseEventParser {
    fn default() -> Self {
        Self::new()
    }
}

pub fn split_responses_json_documents(payload: &[u8]) -> Option<Vec<Vec<u8>>> {
    let payload = payload.trim_ascii();
    if payload.is_empty() || payload.len() > MAX_RESPONSES_JSON_BYTES {
        return None;
    }
    let mut documents = Vec::with_capacity(2);
    let stream = serde_json::Deserializer::from_slice(payload).into_iter::<serde_json::Value>();
    for value in stream {
        let value = value.ok()?;
        let event_type = value
            .get("type")
            .and_then(serde_json::Value::as_str)?
            .trim();
        if event_type.is_empty() || event_type.contains(['\r', '\n']) {
            return None;
        }
        if documents.len() == MAX_RESPONSES_JSON_DOCUMENTS {
            return None;
        }
        documents.push(serde_json::to_vec(&value).ok()?);
    }
    (documents.len() > 1).then_some(documents)
}

impl SseEventParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_data: String::new(),
        }
    }

    pub fn push_chunk<F: FnMut(String)>(&mut self, chunk: &[u8], mut on_event: F) {
        // 网络分片可能切在 UTF-8 字符中间；只在整行收齐后解码，并复用行缓冲。
        for part in chunk.split_inclusive(|byte| *byte == b'\n') {
            self.buffer.extend_from_slice(part);
            if part.last() == Some(&b'\n') {
                self.buffer.pop();
                self.process_buffered_line(&mut on_event);
            }
        }
    }

    pub fn finish<F: FnMut(String)>(&mut self, mut on_event: F) {
        if !self.buffer.is_empty() {
            self.process_buffered_line(&mut on_event);
        }
        self.flush_event(&mut on_event);
    }

    fn process_buffered_line<F: FnMut(String)>(&mut self, on_event: &mut F) {
        let mut buffer = std::mem::take(&mut self.buffer);
        if buffer.last() == Some(&b'\r') {
            buffer.pop();
        }
        let line = String::from_utf8_lossy(&buffer);
        if matches!(line, std::borrow::Cow::Owned(_)) {
            tracing::debug!(
                line_bytes = buffer.len(),
                "replaced invalid UTF-8 in SSE line"
            );
        }
        self.process_line(&line, on_event);
        buffer.clear();
        self.buffer = buffer;
    }

    fn process_line<F: FnMut(String)>(&mut self, line: &str, on_event: &mut F) {
        if line.is_empty() {
            self.flush_event(on_event);
            return;
        }
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim_start();
            if !self.current_data.is_empty() {
                self.current_data.push('\n');
            }
            self.current_data.push_str(data);
        }
    }

    fn flush_event<F: FnMut(String)>(&mut self, on_event: &mut F) {
        if self.current_data.is_empty() {
            return;
        }
        let data = std::mem::take(&mut self.current_data);
        let data = data.trim();
        if data.is_empty() {
            return;
        }
        if let Some(documents) = split_responses_json_documents(data.as_bytes()) {
            tracing::debug!(
                document_count = documents.len(),
                payload_bytes = data.len(),
                "split concatenated Responses SSE JSON documents"
            );
            for document in documents {
                if let Ok(document) = String::from_utf8(document) {
                    on_event(document);
                }
            }
            return;
        }
        on_event(data.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::{split_responses_json_documents, SseEventParser};

    #[test]
    fn utf8_survives_every_network_split_and_unterminated_final_line() {
        let data = "data: 你好🦀\r\n\r\ndata: 最后";
        for split in 0..=data.len() {
            let mut parser = SseEventParser::new();
            let mut events = Vec::new();
            parser.push_chunk(&data.as_bytes()[..split], |event| events.push(event));
            parser.push_chunk(&data.as_bytes()[split..], |event| events.push(event));
            parser.finish(|event| events.push(event));
            assert_eq!(events, ["你好🦀", "最后"], "split {split}");
        }
    }

    #[test]
    fn splits_concatenated_responses_json_documents() {
        let payload = br#"{"type":"response.in_progress"}{"type":"response.completed"}"#;

        let documents = split_responses_json_documents(payload).expect("split documents");

        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0], br#"{"type":"response.in_progress"}"#);
        assert_eq!(documents[1], br#"{"type":"response.completed"}"#);
    }

    #[test]
    fn parser_emits_each_concatenated_responses_document() {
        let mut parser = SseEventParser::new();
        let mut events = Vec::new();

        parser.push_chunk(
            b"data: {\"type\":\"response.in_progress\"}{\"type\":\"response.completed\"}\n\n",
            |event| events.push(event),
        );

        assert_eq!(events.len(), 2);
        assert_eq!(events[0], r#"{"type":"response.in_progress"}"#);
        assert_eq!(events[1], r#"{"type":"response.completed"}"#);
    }

    #[test]
    fn does_not_split_valid_or_non_responses_payloads() {
        for payload in [
            br#"{"type":"response.completed"}"#.as_slice(),
            br#"{"value":1}{"value":2}"#.as_slice(),
            br#"{"type":"response.completed"} trailing"#.as_slice(),
        ] {
            assert!(split_responses_json_documents(payload).is_none());
        }
    }
}
