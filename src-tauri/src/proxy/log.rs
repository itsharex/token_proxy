use serde::Serialize;
use std::{
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[derive(Clone, Serialize)]
pub(crate) struct TokenUsage {
    pub(crate) input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct LogEntry {
    pub(crate) ts_ms: u128,
    pub(crate) path: String,
    pub(crate) provider: String,
    pub(crate) upstream_id: String,
    pub(crate) model: Option<String>,
    pub(crate) stream: bool,
    pub(crate) status: u16,
    pub(crate) usage: Option<TokenUsage>,
    pub(crate) upstream_request_id: Option<String>,
    pub(crate) latency_ms: u128,
}

#[derive(Clone)]
pub(crate) struct LogContext {
    pub(crate) path: String,
    pub(crate) provider: String,
    pub(crate) upstream_id: String,
    pub(crate) model: Option<String>,
    pub(crate) stream: bool,
    pub(crate) status: u16,
    pub(crate) upstream_request_id: Option<String>,
    pub(crate) start: Instant,
}

pub(crate) struct LogWriter {
    file: Mutex<tokio::fs::File>,
}

impl LogWriter {
    pub(crate) async fn new(path: &PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub(crate) async fn write(&self, entry: &LogEntry) {
        let line = serde_json::to_string(entry)
            .unwrap_or_else(|_| "{\"error\":\"proxy_log_serialize_failed\"}".to_string());
        let mut file = self.file.lock().await;
        if let Err(err) = file.write_all(line.as_bytes()).await {
            eprintln!("proxy log write failed: {err}");
            return;
        }
        if let Err(err) = file.write_all(b"\n").await {
            eprintln!("proxy log write failed: {err}");
        }
    }
}

pub(crate) fn build_log_entry(context: &LogContext, usage: Option<TokenUsage>) -> LogEntry {
    LogEntry {
        ts_ms: now_ms(),
        path: context.path.clone(),
        provider: context.provider.clone(),
        upstream_id: context.upstream_id.clone(),
        model: context.model.clone(),
        stream: context.stream,
        status: context.status,
        usage,
        upstream_request_id: context.upstream_request_id.clone(),
        latency_ms: context.start.elapsed().as_millis(),
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
