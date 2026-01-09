mod io;
mod model_mapping;
mod normalize;
mod types;

use tauri::AppHandle;

pub(crate) use io::config_dir_path;
pub(crate) use types::{
    ConfigResponse,
    ProxyConfig,
    ProxyConfigFile,
    ProviderUpstreams,
    TrayTokenRateConfig,
    TrayTokenRateFormat,
    UpstreamConfig,
    UpstreamGroup,
    UpstreamRuntime,
    UpstreamStrategy,
};

pub(crate) async fn read_config(app: AppHandle) -> Result<ConfigResponse, String> {
    let config = io::load_config_file(&app).await?;
    let path = io::config_file_path(&app)?;
    Ok(ConfigResponse {
        path: path.to_string_lossy().to_string(),
        config,
    })
}

pub(crate) async fn write_config(
    app: AppHandle,
    mut config: ProxyConfigFile,
) -> Result<(), String> {
    normalize::fill_missing_upstream_indices(&mut config.upstreams)?;
    build_runtime_config(&app, config.clone())?;
    io::save_config_file(&app, &config).await
}

impl ProxyConfig {
    pub(crate) fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub(crate) async fn load(app: &AppHandle) -> Result<Self, String> {
        let config = io::load_config_file(app).await?;
        build_runtime_config(app, config)
    }

    pub(crate) fn provider_upstreams(&self, provider: &str) -> Option<&ProviderUpstreams> {
        self.upstreams.get(provider)
    }
}

fn build_runtime_config(app: &AppHandle, config: ProxyConfigFile) -> Result<ProxyConfig, String> {
    let log_path = io::resolve_log_path(app, &config.log_path)?;
    let normalized_upstreams = normalize::normalize_upstreams(&config.upstreams)?;
    let upstreams = normalize::build_provider_upstreams(normalized_upstreams)?;
    Ok(ProxyConfig {
        host: config.host,
        port: config.port,
        local_api_key: config.local_api_key,
        log_path,
        enable_api_format_conversion: config.enable_api_format_conversion,
        upstream_strategy: config.upstream_strategy,
        upstreams,
    })
}
