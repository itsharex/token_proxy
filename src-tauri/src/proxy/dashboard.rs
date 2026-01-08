use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;

use super::sqlite;

const RECENT_PAGE_SIZE: u32 = 50;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardRange {
    pub(crate) from_ts_ms: Option<u64>,
    pub(crate) to_ts_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardSummary {
    pub(crate) total_requests: u64,
    pub(crate) success_requests: u64,
    pub(crate) error_requests: u64,
    pub(crate) total_tokens: u64,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) cached_tokens: u64,
    pub(crate) avg_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardProviderStat {
    pub(crate) provider: String,
    pub(crate) requests: u64,
    pub(crate) total_tokens: u64,
    pub(crate) cached_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardRequestItem {
    pub(crate) id: u64,
    pub(crate) ts_ms: u64,
    pub(crate) path: String,
    pub(crate) provider: String,
    pub(crate) upstream_id: String,
    pub(crate) model: Option<String>,
    pub(crate) stream: bool,
    pub(crate) status: u16,
    pub(crate) total_tokens: Option<u64>,
    pub(crate) cached_tokens: Option<u64>,
    pub(crate) latency_ms: u64,
    pub(crate) upstream_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardSnapshot {
    pub(crate) summary: DashboardSummary,
    pub(crate) providers: Vec<DashboardProviderStat>,
    pub(crate) recent: Vec<DashboardRequestItem>,
    /// 是否只基于日志文件末尾片段做统计（Step1：true；Step2 SQLite 后应为 false）。
    pub(crate) truncated: bool,
}

pub(crate) async fn read_snapshot(
    app: AppHandle,
    range: DashboardRange,
    offset: Option<u32>,
) -> Result<DashboardSnapshot, String> {
    let offset = offset.unwrap_or(0);

    let pool = sqlite::open_pool(&app).await?;
    let from_ts_ms = range.from_ts_ms.map(|value| value as i64);
    let to_ts_ms = range.to_ts_ms.map(|value| value as i64);

    let row = sqlx::query(
        r#"
SELECT
  COUNT(*) AS total_requests,
  COALESCE(SUM(CASE WHEN status BETWEEN 200 AND 299 THEN 1 ELSE 0 END), 0) AS success_requests,
  COALESCE(SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END), 0) AS error_requests,
  COALESCE(SUM(CASE
    WHEN total_tokens IS NOT NULL THEN total_tokens
    WHEN input_tokens IS NOT NULL OR output_tokens IS NOT NULL THEN COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0)
    ELSE 0
  END), 0) AS total_tokens,
  COALESCE(SUM(COALESCE(input_tokens, 0)), 0) AS input_tokens,
  COALESCE(SUM(COALESCE(output_tokens, 0)), 0) AS output_tokens,
  COALESCE(SUM(COALESCE(cached_tokens, 0)), 0) AS cached_tokens,
  COALESCE(SUM(latency_ms), 0) AS latency_sum_ms
FROM request_logs
WHERE (?1 IS NULL OR ts_ms >= ?1) AND (?2 IS NULL OR ts_ms <= ?2);
"#,
    )
    .bind(from_ts_ms)
    .bind(to_ts_ms)
    .fetch_one(&pool)
    .await
    .map_err(|err| format!("Failed to query dashboard summary: {err}"))?;

    let total_requests = i64_to_u64(row.try_get("total_requests").unwrap_or(0));
    let success_requests = i64_to_u64(row.try_get("success_requests").unwrap_or(0));
    let error_requests = i64_to_u64(row.try_get("error_requests").unwrap_or(0));
    let total_tokens = i64_to_u64(row.try_get("total_tokens").unwrap_or(0));
    let input_tokens = i64_to_u64(row.try_get("input_tokens").unwrap_or(0));
    let output_tokens = i64_to_u64(row.try_get("output_tokens").unwrap_or(0));
    let cached_tokens = i64_to_u64(row.try_get("cached_tokens").unwrap_or(0));
    let latency_sum_ms = i64_to_u64(row.try_get("latency_sum_ms").unwrap_or(0));

    let avg_latency_ms = if total_requests == 0 {
        0
    } else {
        latency_sum_ms / total_requests
    };

    let providers = sqlx::query(
        r#"
SELECT
  provider,
  COUNT(*) AS requests,
  COALESCE(SUM(CASE
    WHEN total_tokens IS NOT NULL THEN total_tokens
    WHEN input_tokens IS NOT NULL OR output_tokens IS NOT NULL THEN COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0)
    ELSE 0
  END), 0) AS total_tokens,
  COALESCE(SUM(COALESCE(cached_tokens, 0)), 0) AS cached_tokens
FROM request_logs
WHERE (?1 IS NULL OR ts_ms >= ?1) AND (?2 IS NULL OR ts_ms <= ?2)
GROUP BY provider
ORDER BY total_tokens DESC;
"#,
    )
    .bind(from_ts_ms)
    .bind(to_ts_ms)
    .fetch_all(&pool)
    .await
    .map_err(|err| format!("Failed to query provider stats: {err}"))?
    .into_iter()
    .filter_map(|row| {
        let provider: String = row.try_get("provider").ok()?;
        let requests: i64 = row.try_get("requests").ok()?;
        let total_tokens: i64 = row.try_get("total_tokens").ok()?;
        let cached_tokens: i64 = row.try_get("cached_tokens").ok()?;
        Some(DashboardProviderStat {
            provider,
            requests: i64_to_u64(requests),
            total_tokens: i64_to_u64(total_tokens),
            cached_tokens: i64_to_u64(cached_tokens),
        })
    })
    .collect::<Vec<_>>();

    let recent = sqlx::query(
        r#"
SELECT
  id,
  ts_ms,
  path,
  provider,
  upstream_id,
  model,
  stream,
  status,
  CASE
    WHEN total_tokens IS NOT NULL THEN total_tokens
    WHEN input_tokens IS NOT NULL OR output_tokens IS NOT NULL THEN COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0)
    ELSE NULL
  END AS total_tokens,
  cached_tokens,
  latency_ms,
  upstream_request_id
FROM request_logs
WHERE (?1 IS NULL OR ts_ms >= ?1) AND (?2 IS NULL OR ts_ms <= ?2)
ORDER BY ts_ms DESC
LIMIT ?3 OFFSET ?4;
"#,
    )
    .bind(from_ts_ms)
    .bind(to_ts_ms)
    .bind(i64::from(RECENT_PAGE_SIZE))
    .bind(i64::from(offset))
    .fetch_all(&pool)
    .await
    .map_err(|err| format!("Failed to query recent requests: {err}"))?
    .into_iter()
    .filter_map(|row| {
        let id: i64 = row.try_get("id").ok()?;
        let ts_ms: i64 = row.try_get("ts_ms").ok()?;
        let path: String = row.try_get("path").ok()?;
        let provider: String = row.try_get("provider").ok()?;
        let upstream_id: String = row.try_get("upstream_id").ok()?;
        let model: Option<String> = row.try_get("model").ok()?;
        let stream: bool = row.try_get("stream").unwrap_or(false);
        let status: i64 = row.try_get("status").unwrap_or(0);
        let total_tokens: Option<i64> = row.try_get("total_tokens").ok()?;
        let cached_tokens: Option<i64> = row.try_get("cached_tokens").ok()?;
        let latency_ms: i64 = row.try_get("latency_ms").unwrap_or(0);
        let upstream_request_id: Option<String> = row.try_get("upstream_request_id").ok()?;
        Some(DashboardRequestItem {
            id: i64_to_u64(id),
            ts_ms: i64_to_u64(ts_ms),
            path,
            provider,
            upstream_id,
            model,
            stream,
            status: i64_to_u16(status),
            total_tokens: total_tokens.map(i64_to_u64),
            cached_tokens: cached_tokens.map(i64_to_u64),
            latency_ms: i64_to_u64(latency_ms),
            upstream_request_id,
        })
    })
    .collect::<Vec<_>>();

    Ok(DashboardSnapshot {
        summary: DashboardSummary {
            total_requests,
            success_requests,
            error_requests,
            total_tokens,
            input_tokens,
            output_tokens,
            cached_tokens,
            avg_latency_ms,
        },
        providers,
        recent,
        truncated: false,
    })
}

fn i64_to_u64(value: i64) -> u64 {
    value.max(0) as u64
}

fn i64_to_u16(value: i64) -> u16 {
    value.clamp(0, u16::MAX as i64) as u16
}
