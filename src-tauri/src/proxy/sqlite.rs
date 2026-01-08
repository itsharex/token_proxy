use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::path::PathBuf;
use sqlx::Row;
use tauri::AppHandle;
use std::time::Duration;

use super::config;

const DB_FILE_NAME: &str = "data.db";

pub(crate) async fn open_pool(app: &AppHandle) -> Result<SqlitePool, String> {
    let path = usage_db_path(app)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("Failed to create db directory: {err}"))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|err| format!("Failed to connect sqlite: {err}"))?;

    init_schema(&pool).await?;
    Ok(pool)
}

fn usage_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config::config_dir_path(app)?.join(DB_FILE_NAME))
}

async fn init_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS request_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts_ms INTEGER NOT NULL,
  path TEXT NOT NULL,
  provider TEXT NOT NULL,
  upstream_id TEXT NOT NULL,
  model TEXT,
  stream INTEGER NOT NULL,
  status INTEGER NOT NULL,
  input_tokens INTEGER,
  output_tokens INTEGER,
  total_tokens INTEGER,
  cached_tokens INTEGER,
  usage_json TEXT,
  upstream_request_id TEXT,
  latency_ms INTEGER NOT NULL
);
"#,
    )
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to create request_logs table: {err}"))?;

    ensure_request_logs_columns(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_request_logs_ts_ms ON request_logs(ts_ms);")
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to create idx_request_logs_ts_ms: {err}"))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_request_logs_provider_ts_ms ON request_logs(provider, ts_ms);",
    )
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to create idx_request_logs_provider_ts_ms: {err}"))?;

    Ok(())
}

async fn ensure_request_logs_columns(pool: &SqlitePool) -> Result<(), String> {
    let columns = sqlx::query("PRAGMA table_info(request_logs);")
        .fetch_all(pool)
        .await
        .map_err(|err| format!("Failed to read request_logs schema: {err}"))?
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .collect::<std::collections::HashSet<_>>();

    if !columns.contains("cached_tokens") {
        sqlx::query("ALTER TABLE request_logs ADD COLUMN cached_tokens INTEGER;")
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to add cached_tokens column: {err}"))?;
    }

    if !columns.contains("usage_json") {
        sqlx::query("ALTER TABLE request_logs ADD COLUMN usage_json TEXT;")
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to add usage_json column: {err}"))?;
    }

    Ok(())
}
