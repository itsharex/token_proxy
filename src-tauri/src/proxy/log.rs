use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::{
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TokenUsage {
    pub(crate) input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct UsageSnapshot {
    pub(crate) usage: Option<TokenUsage>,
    pub(crate) cached_tokens: Option<u64>,
    pub(crate) usage_json: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LogEntry {
    pub(crate) ts_ms: u128,
    pub(crate) path: String,
    pub(crate) provider: String,
    pub(crate) upstream_id: String,
    pub(crate) model: Option<String>,
    pub(crate) mapped_model: Option<String>,
    pub(crate) stream: bool,
    pub(crate) status: u16,
    pub(crate) usage: Option<TokenUsage>,
    pub(crate) cached_tokens: Option<u64>,
    pub(crate) usage_json: Option<Value>,
    pub(crate) upstream_request_id: Option<String>,
    pub(crate) latency_ms: u128,
}

#[derive(Clone)]
pub(crate) struct LogContext {
    pub(crate) path: String,
    pub(crate) provider: String,
    pub(crate) upstream_id: String,
    pub(crate) model: Option<String>,
    pub(crate) mapped_model: Option<String>,
    pub(crate) stream: bool,
    pub(crate) status: u16,
    pub(crate) upstream_request_id: Option<String>,
    pub(crate) start: Instant,
}

pub(crate) struct LogWriter {
    file: Mutex<tokio::fs::File>,
    sqlite: Option<SqlitePool>,
}

impl LogWriter {
    pub(crate) async fn new(path: &PathBuf, sqlite: Option<SqlitePool>) -> std::io::Result<Self> {
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
            sqlite,
        })
    }

    pub(crate) async fn write(&self, entry: &LogEntry) {
        let line = serde_json::to_string(entry)
            .unwrap_or_else(|_| "{\"error\":\"proxy_log_serialize_failed\"}".to_string());
        {
            let mut file = self.file.lock().await;
            if let Err(err) = file.write_all(line.as_bytes()).await {
                eprintln!("proxy log write failed: {err}");
            } else if let Err(err) = file.write_all(b"\n").await {
                eprintln!("proxy log write failed: {err}");
            }
        }

        let Some(pool) = self.sqlite.as_ref() else {
            return;
        };
        if let Err(err) = insert_log_entry(pool, entry).await {
            eprintln!("proxy sqlite write failed: {err}");
        }
    }
}

pub(crate) fn build_log_entry(context: &LogContext, usage: UsageSnapshot) -> LogEntry {
    LogEntry {
        ts_ms: now_ms(),
        path: context.path.clone(),
        provider: context.provider.clone(),
        upstream_id: context.upstream_id.clone(),
        model: context.model.clone(),
        mapped_model: context.mapped_model.clone(),
        stream: context.stream,
        status: context.status,
        usage: usage.usage,
        cached_tokens: usage.cached_tokens,
        usage_json: usage.usage_json,
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

async fn insert_log_entry(pool: &SqlitePool, entry: &LogEntry) -> Result<(), sqlx::Error> {
    let usage = entry.usage.as_ref();
    let input_tokens = usage.and_then(|usage| usage.input_tokens).map(to_i64_u64);
    let output_tokens = usage.and_then(|usage| usage.output_tokens).map(to_i64_u64);
    let total_tokens = usage.and_then(|usage| usage.total_tokens).map(to_i64_u64);
    let cached_tokens = entry.cached_tokens.map(to_i64_u64);
    let usage_json = entry.usage_json.as_ref().map(Value::to_string);

    sqlx::query(
        r#"
INSERT INTO request_logs (
  ts_ms,
  path,
  provider,
  upstream_id,
  model,
  mapped_model,
  stream,
  status,
  input_tokens,
  output_tokens,
  total_tokens,
  cached_tokens,
  usage_json,
  upstream_request_id,
  latency_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
"#,
    )
    .bind(to_i64_u128(entry.ts_ms))
    .bind(entry.path.as_str())
    .bind(entry.provider.as_str())
    .bind(entry.upstream_id.as_str())
    .bind(entry.model.as_deref())
    .bind(entry.mapped_model.as_deref())
    .bind(entry.stream)
    .bind(i64::from(entry.status))
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(total_tokens)
    .bind(cached_tokens)
    .bind(usage_json.as_deref())
    .bind(entry.upstream_request_id.as_deref())
    .bind(to_i64_u128(entry.latency_ms))
    .execute(pool)
    .await?;

    Ok(())
}

fn to_i64_u128(value: u128) -> i64 {
    value.min(i64::MAX as u128) as i64
}

fn to_i64_u64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}
