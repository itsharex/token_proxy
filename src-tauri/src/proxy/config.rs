use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{path::BaseDirectory, AppHandle, Manager};

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct OpenAiConfig {
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ProxyConfigFile {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) local_api_key: Option<String>,
    pub(crate) log_path: String,
    pub(crate) openai: OpenAiConfig,
}

impl Default for ProxyConfigFile {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9208,
            local_api_key: None,
            log_path: "proxy.log".to_string(),
            openai: OpenAiConfig {
                base_url: "https://api.openai.com".to_string(),
                api_key: None,
            },
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProxyConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) local_api_key: Option<String>,
    pub(crate) upstream_api_key: Option<String>,
    pub(crate) upstream_base_url: String,
    pub(crate) log_path: PathBuf,
}

impl ProxyConfig {
    pub(crate) fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub(crate) fn upstream_url(&self, path: &str) -> String {
        let base = self.upstream_base_url.trim_end_matches('/');
        format!("{base}{path}")
    }

    pub(crate) async fn load(app: &AppHandle) -> Result<Self, String> {
        let config = load_config_file(app).await?;
        build_runtime_config(app, config)
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

pub(crate) async fn write_config(app: AppHandle, config: ProxyConfigFile) -> Result<(), String> {
    save_config_file(&app, &config).await
}

fn build_runtime_config(app: &AppHandle, config: ProxyConfigFile) -> Result<ProxyConfig, String> {
    let log_path = resolve_log_path(app, &config.log_path)?;
    Ok(ProxyConfig {
        host: config.host,
        port: config.port,
        local_api_key: config.local_api_key,
        upstream_api_key: config.openai.api_key,
        upstream_base_url: config.openai.base_url,
        log_path,
    })
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
    toml::from_str(contents)
        .map_err(|err| format!("Failed to parse config file {}: {err}", path.display()))
}

async fn save_config_file(app: &AppHandle, config: &ProxyConfigFile) -> Result<(), String> {
    let path = config_file_path(app)?;
    ensure_parent_dir(&path).await?;
    let data = toml::to_string_pretty(config)
        .map_err(|err| format!("Failed to serialize config: {err}"))?;
    tokio::fs::write(&path, data)
        .await
        .map_err(|err| format!("Failed to write config file: {err}"))?;
    Ok(())
}

async fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|err| format!("Failed to create config directory: {err}"))
}

/// 配置文件路径：使用 Tauri BaseDirectory::Config
fn config_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve(CONFIG_FILE_NAME, BaseDirectory::Config)
        .map_err(|err| format!("Failed to resolve config path: {err}"))
}

/// 日志路径：相对路径基于配置目录
fn resolve_log_path(app: &AppHandle, log_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(log_path);
    if path.is_absolute() {
        return Ok(path);
    }
    app.path()
        .resolve(log_path, BaseDirectory::Config)
        .map_err(|err| format!("Failed to resolve log path: {err}"))
}
