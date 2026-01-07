use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const CONFIG_FILE_NAME: &str = "config.jsonc";
const DEFAULT_CONFIG_HEADER: &str =
    "// Token Proxy config (JSONC). Comments and trailing commas are supported.\n";

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UpstreamStrategy {
    PriorityRoundRobin,
    PriorityFillFirst,
}

impl Default for UpstreamStrategy {
    fn default() -> Self {
        Self::PriorityRoundRobin
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct UpstreamConfig {
    pub(crate) id: String,
    pub(crate) provider: String,
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
    pub(crate) priority: Option<i32>,
    pub(crate) index: Option<i32>,
    #[serde(default = "default_enabled")]
    pub(crate) enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ProxyConfigFile {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) local_api_key: Option<String>,
    pub(crate) log_path: String,
    #[serde(default)]
    pub(crate) upstream_strategy: UpstreamStrategy,
    #[serde(default)]
    pub(crate) upstreams: Vec<UpstreamConfig>,
}

impl Default for ProxyConfigFile {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9208,
            local_api_key: None,
            log_path: "proxy.log".to_string(),
            upstream_strategy: UpstreamStrategy::PriorityRoundRobin,
            upstreams: vec![
                UpstreamConfig {
                    id: "openai-default".to_string(),
                    provider: "openai".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                    api_key: None,
                    priority: Some(0),
                    index: Some(0),
                    enabled: true,
                },
                UpstreamConfig {
                    id: "openai-responses".to_string(),
                    provider: "openai-response".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                    api_key: None,
                    priority: Some(0),
                    index: Some(1),
                    enabled: true,
                },
                UpstreamConfig {
                    id: "claude-default".to_string(),
                    provider: "claude".to_string(),
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key: None,
                    priority: Some(0),
                    index: Some(2),
                    enabled: true,
                },
            ],
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProxyConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) local_api_key: Option<String>,
    pub(crate) log_path: PathBuf,
    pub(crate) upstream_strategy: UpstreamStrategy,
    pub(crate) upstreams: HashMap<String, ProviderUpstreams>,
}

#[derive(Clone)]
pub(crate) struct ProviderUpstreams {
    pub(crate) groups: Vec<UpstreamGroup>,
}

#[derive(Clone)]
pub(crate) struct UpstreamGroup {
    pub(crate) priority: i32,
    pub(crate) items: Vec<UpstreamRuntime>,
}

#[derive(Clone)]
pub(crate) struct UpstreamRuntime {
    pub(crate) id: String,
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
    pub(crate) priority: i32,
    pub(crate) index: i32,
    order: usize,
}

impl ProxyConfig {
    pub(crate) fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub(crate) async fn load(app: &AppHandle) -> Result<Self, String> {
        let config = load_config_file(app).await?;
        build_runtime_config(app, config)
    }

    pub(crate) fn provider_upstreams(&self, provider: &str) -> Option<&ProviderUpstreams> {
        self.upstreams.get(provider)
    }
}

#[derive(Serialize)]
pub(crate) struct ConfigResponse {
    pub(crate) path: String,
    pub(crate) config: ProxyConfigFile,
}

pub(crate) async fn read_config(app: AppHandle) -> Result<ConfigResponse, String> {
    let config = load_config_file(&app).await?;
    let path = config_file_path(&app)?;
    Ok(ConfigResponse {
        path: path.to_string_lossy().to_string(),
        config,
    })
}

pub(crate) async fn write_config(
    app: AppHandle,
    mut config: ProxyConfigFile,
) -> Result<(), String> {
    fill_missing_upstream_indices(&mut config.upstreams)?;
    build_runtime_config(&app, config.clone())?;
    save_config_file(&app, &config).await
}

fn build_runtime_config(app: &AppHandle, config: ProxyConfigFile) -> Result<ProxyConfig, String> {
    let log_path = resolve_log_path(app, &config.log_path)?;
    let normalized_upstreams = normalize_upstreams(&config.upstreams)?;
    let upstreams = build_provider_upstreams(normalized_upstreams)?;
    Ok(ProxyConfig {
        host: config.host,
        port: config.port,
        local_api_key: config.local_api_key,
        log_path,
        upstream_strategy: config.upstream_strategy,
        upstreams,
    })
}

fn fill_missing_upstream_indices(upstreams: &mut [UpstreamConfig]) -> Result<(), String> {
    let mut max_index: Option<i32> = None;
    for upstream in upstreams.iter() {
        if let Some(index) = upstream.index {
            max_index = Some(max_index.map_or(index, |current| current.max(index)));
        }
    }
    let mut next_index = match max_index {
        Some(value) => value
            .checked_add(1)
            .ok_or_else(|| "Upstream index is out of range.".to_string())?,
        None => 0,
    };
    for upstream in upstreams.iter_mut() {
        if upstream.index.is_none() {
            upstream.index = Some(assign_next_index(&mut next_index)?);
        }
    }
    Ok(())
}

#[derive(Clone)]
struct NormalizedUpstream {
    provider: String,
    runtime: UpstreamRuntime,
}

impl UpstreamRuntime {
    /// 构建上游请求 URL，智能处理 base_url 与 path 的路径重叠
    /// 例如：base_url = "https://example.com/openai/v1", path = "/v1/chat/completions"
    /// 结果：https://example.com/openai/v1/chat/completions（去掉重复的 /v1）
    pub(crate) fn upstream_url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let effective_path = strip_overlapping_prefix(base, path);
        format!("{base}{effective_path}")
    }
}

/// 去掉 path 开头与 base_url 路径部分重叠的前缀
/// base_url: "https://example.com/openai/v1" -> base_path: "/openai/v1"
/// 如果 path 以 base_path 的某个后缀开头（如 "/v1"），则去掉该重叠部分
fn strip_overlapping_prefix<'a>(base_url: &str, path: &'a str) -> &'a str {
    let Some(base_path) = url::Url::parse(base_url)
        .ok()
        .map(|url| url.path().to_string())
    else {
        return path;
    };
    // 检查 base_path 的每个后缀是否与 path 的前缀重叠
    // 例如 base_path = "/openai/v1"，依次检查 "/openai/v1", "/v1"
    let base_path = base_path.trim_end_matches('/');
    for (idx, ch) in base_path.char_indices() {
        if ch == '/' {
            let suffix = &base_path[idx..];
            if path.starts_with(suffix) {
                return &path[suffix.len()..];
            }
        }
    }
    // 完整匹配检查（base_path 本身）
    if path.starts_with(base_path) {
        return &path[base_path.len()..];
    }
    path
}

fn normalize_upstreams(upstreams: &[UpstreamConfig]) -> Result<Vec<NormalizedUpstream>, String> {
    let (mut seen_ids, mut max_index) = (HashSet::new(), None::<i32>);
    for upstream in upstreams {
        let id = upstream.id.trim();
        if id.is_empty() {
            return Err("Upstream id cannot be empty.".to_string());
        }
        if !seen_ids.insert(id.to_string()) {
            return Err(format!("Upstream id already exists: {id}."));
        }
        if let Some(index) = upstream.index {
            max_index = Some(max_index.map_or(index, |current| current.max(index)));
        }
    }
    let mut next_index = match max_index {
        Some(value) => value
            .checked_add(1)
            .ok_or_else(|| "Upstream index is out of range.".to_string())?,
        None => 0,
    };
    let mut normalized = Vec::with_capacity(upstreams.len());
    for (order, upstream) in upstreams.iter().enumerate() {
        if !upstream.enabled {
            continue;
        }
        let provider = upstream.provider.trim();
        if provider.is_empty() {
            return Err(format!("Upstream {} provider cannot be empty.", upstream.id));
        }
        let base_url = upstream.base_url.trim();
        if base_url.is_empty() {
            return Err(format!("Upstream {} base_url cannot be empty.", upstream.id));
        }
        // When index is missing, assign sequentially after the global max for stable ordering.
        let index = match upstream.index {
            Some(value) => value,
            None => assign_next_index(&mut next_index)?,
        };
        let api_key = upstream
            .api_key
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
        let runtime = UpstreamRuntime {
            id: upstream.id.trim().to_string(),
            base_url: base_url.to_string(),
            api_key,
            priority: upstream.priority.unwrap_or(0),
            index,
            order,
        };
        normalized.push(NormalizedUpstream {
            provider: provider.to_string(),
            runtime,
        });
    }
    Ok(normalized)
}

fn assign_next_index(next_index: &mut i32) -> Result<i32, String> {
    let current = *next_index;
    *next_index = next_index
        .checked_add(1)
        .ok_or_else(|| "Upstream index is out of range.".to_string())?;
    Ok(current)
}

fn build_provider_upstreams(
    upstreams: Vec<NormalizedUpstream>,
) -> Result<HashMap<String, ProviderUpstreams>, String> {
    let mut grouped: HashMap<String, Vec<UpstreamRuntime>> = HashMap::new();
    for upstream in upstreams {
        grouped
            .entry(upstream.provider)
            .or_default()
            .push(upstream.runtime);
    }
    let mut output = HashMap::new();
    for (provider, upstreams) in grouped {
        let groups = group_upstreams_by_priority(upstreams);
        output.insert(provider, ProviderUpstreams { groups });
    }
    Ok(output)
}

fn group_upstreams_by_priority(mut upstreams: Vec<UpstreamRuntime>) -> Vec<UpstreamGroup> {
    upstreams.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.index.cmp(&right.index))
            .then_with(|| left.order.cmp(&right.order))
    });
    let mut groups: Vec<UpstreamGroup> = Vec::new();
    for upstream in upstreams {
        match groups.last_mut() {
            Some(group) if group.priority == upstream.priority => group.items.push(upstream),
            _ => groups.push(UpstreamGroup {
                priority: upstream.priority,
                items: vec![upstream],
            }),
        }
    }
    groups
}

async fn load_config_file(app: &AppHandle) -> Result<ProxyConfigFile, String> {
    let path = config_file_path(app)?;
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => parse_config_file(&contents, &path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let config = ProxyConfigFile::default();
            save_config_file(app, &config).await?;
            Ok(config)
        }
        Err(err) => Err(format!("Failed to read config file: {err}")),
    }
}

fn parse_config_file(contents: &str, path: &Path) -> Result<ProxyConfigFile, String> {
    let sanitized = sanitize_jsonc(contents);
    serde_json::from_str(&sanitized)
        .map_err(|err| format!("Failed to parse config file {}: {err}", path.display()))
}

async fn save_config_file(app: &AppHandle, config: &ProxyConfigFile) -> Result<(), String> {
    let path = config_file_path(app)?;
    ensure_parent_dir(&path).await?;
    let data = serde_json::to_string_pretty(config)
        .map_err(|err| format!("Failed to serialize config: {err}"))?;
    let header = read_existing_header(&path)
        .await
        .unwrap_or_else(default_config_header);
    let output = merge_header_and_body(header, data);
    tokio::fs::write(&path, output)
        .await
        .map_err(|err| format!("Failed to write config file: {err}"))?;
    Ok(())
}

fn sanitize_jsonc(contents: &str) -> String {
    let without_comments = strip_jsonc_comments(contents);
    strip_trailing_commas(&without_comments)
}

fn strip_jsonc_comments(contents: &str) -> String {
    let bytes = contents.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escape = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            consume_string_byte(byte, &mut in_string, &mut escape, &mut output);
            index += 1;
            continue;
        }

        if byte == b'"' {
            in_string = true;
            output.push(byte);
            index += 1;
            continue;
        }

        if let Some(next_index) = try_skip_comment(bytes, index, &mut output) {
            index = next_index;
            continue;
        }

        output.push(byte);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_default()
}

fn consume_string_byte(byte: u8, in_string: &mut bool, escape: &mut bool, output: &mut Vec<u8>) {
    output.push(byte);
    if *escape {
        *escape = false;
    } else if byte == b'\\' {
        *escape = true;
    } else if byte == b'"' {
        *in_string = false;
    }
}

fn try_skip_comment(bytes: &[u8], index: usize, output: &mut Vec<u8>) -> Option<usize> {
    if bytes[index] != b'/' || index + 1 >= bytes.len() {
        return None;
    }
    match bytes[index + 1] {
        b'/' => Some(skip_line_comment(bytes, index + 2, output)),
        b'*' => Some(skip_block_comment(bytes, index + 2, output)),
        _ => None,
    }
}

fn skip_line_comment(bytes: &[u8], mut index: usize, output: &mut Vec<u8>) -> usize {
    // Line comment: skip until newline, keep the newline for line numbers.
    while index < bytes.len() {
        let current = bytes[index];
        if current == b'\n' {
            output.push(b'\n');
            return index + 1;
        }
        index += 1;
    }
    index
}

fn skip_block_comment(bytes: &[u8], mut index: usize, output: &mut Vec<u8>) -> usize {
    // Block comment: preserve line breaks for better error positions.
    while index + 1 < bytes.len() {
        let current = bytes[index];
        if current == b'\n' {
            output.push(b'\n');
        }
        if current == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    index
}

fn strip_trailing_commas(contents: &str) -> String {
    let bytes = contents.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escape = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            output.push(byte);
            if escape {
                escape = false;
            } else if byte == b'\\' {
                escape = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if byte == b'"' {
            in_string = true;
            output.push(byte);
            index += 1;
            continue;
        }

        if byte == b',' {
            let mut lookahead = index + 1;
            let mut should_skip = false;
            while lookahead < bytes.len() {
                let next = bytes[lookahead];
                if next == b' ' || next == b'\t' || next == b'\r' || next == b'\n' {
                    lookahead += 1;
                    continue;
                }
                if next == b'}' || next == b']' {
                    should_skip = true;
                }
                break;
            }
            if should_skip {
                index += 1;
                continue;
            }
        }

        output.push(byte);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_default()
}

async fn read_existing_header(path: &Path) -> Option<String> {
    let contents = tokio::fs::read_to_string(path).await.ok()?;
    let header = extract_leading_jsonc_comments(&contents);
    if header.trim().is_empty() {
        None
    } else {
        Some(header)
    }
}

fn extract_leading_jsonc_comments(contents: &str) -> String {
    let bytes = contents.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b' ' || byte == b'\t' || byte == b'\r' || byte == b'\n' {
            output.push(byte);
            index += 1;
            continue;
        }

        if byte == b'/' && index + 1 < bytes.len() {
            let next = bytes[index + 1];
            if next == b'/' {
                output.push(byte);
                output.push(next);
                index += 2;
                while index < bytes.len() {
                    let current = bytes[index];
                    output.push(current);
                    index += 1;
                    if current == b'\n' {
                        break;
                    }
                }
                continue;
            }
            if next == b'*' {
                output.push(byte);
                output.push(next);
                index += 2;
                while index < bytes.len() {
                    let current = bytes[index];
                    output.push(current);
                    index += 1;
                    if current == b'*' && index < bytes.len() && bytes[index] == b'/' {
                        output.push(b'/');
                        index += 1;
                        break;
                    }
                }
                continue;
            }
        }

        break;
    }

    String::from_utf8(output).unwrap_or_default()
}

fn default_config_header() -> String {
    DEFAULT_CONFIG_HEADER.to_string()
}

fn merge_header_and_body(header: String, body: String) -> String {
    if header.is_empty() {
        format!("{body}\n")
    } else if header.ends_with('\n') {
        format!("{header}{body}\n")
    } else {
        format!("{header}\n{body}\n")
    }
}

async fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|err| format!("Failed to create config directory: {err}"))
}

/// Config directory: BaseDirectory::AppConfig
pub(crate) fn config_dir_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve config dir: {err}"))
}

/// Config file path: based on the config directory
fn config_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir_path(app)?.join(CONFIG_FILE_NAME))
}

/// Log path: relative paths are based on the config directory
fn resolve_log_path(app: &AppHandle, log_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(log_path);
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(config_dir_path(app)?.join(log_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_overlapping_prefix() {
        // 标准 OpenAI 兼容格式：base_url 包含 /v1
        assert_eq!(
            strip_overlapping_prefix("https://api.example.com/openai/v1", "/v1/chat/completions"),
            "/chat/completions"
        );
        assert_eq!(
            strip_overlapping_prefix("https://api.example.com/v1", "/v1/chat/completions"),
            "/chat/completions"
        );

        // 无重叠情况：base_url 不包含路径
        assert_eq!(
            strip_overlapping_prefix("https://api.openai.com", "/v1/chat/completions"),
            "/v1/chat/completions"
        );

        // 无重叠情况：base_url 路径与请求路径无公共后缀
        assert_eq!(
            strip_overlapping_prefix("https://api.example.com/openai/", "/v1/chat/completions"),
            "/v1/chat/completions"
        );
        assert_eq!(
            strip_overlapping_prefix("https://api.example.com/openai", "/v1/chat/completions"),
            "/v1/chat/completions"
        );

        // 多层路径重叠
        assert_eq!(
            strip_overlapping_prefix("https://example.com/api/openai/v1", "/v1/models"),
            "/models"
        );

        // 完整路径重叠
        assert_eq!(
            strip_overlapping_prefix("https://example.com/openai/v1", "/openai/v1/completions"),
            "/completions"
        );

        // 带尾斜杠的 base_url
        assert_eq!(
            strip_overlapping_prefix("https://example.com/v1/", "/v1/chat/completions"),
            "/chat/completions"
        );

        // 无效 URL 回退
        assert_eq!(
            strip_overlapping_prefix("not-a-valid-url", "/v1/chat/completions"),
            "/v1/chat/completions"
        );
    }

    #[test]
    fn test_upstream_url() {
        // openai provider: /v1/chat/completions
        let upstream = UpstreamRuntime {
            id: "test".to_string(),
            base_url: "https://api.example.com/openai/v1".to_string(),
            api_key: None,
            priority: 0,
            index: 0,
            order: 0,
        };
        assert_eq!(
            upstream.upstream_url("/v1/chat/completions"),
            "https://api.example.com/openai/v1/chat/completions"
        );

        // openai-response provider: /v1/responses
        let upstream_responses = UpstreamRuntime {
            id: "test".to_string(),
            base_url: "https://api.example.com/openai/v1".to_string(),
            api_key: None,
            priority: 0,
            index: 0,
            order: 0,
        };
        assert_eq!(
            upstream_responses.upstream_url("/v1/responses"),
            "https://api.example.com/openai/v1/responses"
        );

        // 无路径前缀的 base_url
        let upstream_no_path = UpstreamRuntime {
            id: "test".to_string(),
            base_url: "https://api.openai.com".to_string(),
            api_key: None,
            priority: 0,
            index: 0,
            order: 0,
        };
        assert_eq!(
            upstream_no_path.upstream_url("/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            upstream_no_path.upstream_url("/v1/responses"),
            "https://api.openai.com/v1/responses"
        );

        // 带尾斜杠的 base_url
        let upstream_trailing_slash = UpstreamRuntime {
            id: "test".to_string(),
            base_url: "https://api.example.com/openai/v1/".to_string(),
            api_key: None,
            priority: 0,
            index: 0,
            order: 0,
        };
        assert_eq!(
            upstream_trailing_slash.upstream_url("/v1/chat/completions"),
            "https://api.example.com/openai/v1/chat/completions"
        );
    }
}
