// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod proxy;

use tracing_subscriber::{fmt, EnvFilter};

#[tauri::command]
async fn read_proxy_config(app: tauri::AppHandle) -> Result<proxy::config::ConfigResponse, String> {
    proxy::config::read_config(app).await
}

#[tauri::command]
async fn write_proxy_config(
    app: tauri::AppHandle,
    config: proxy::config::ProxyConfigFile,
) -> Result<(), String> {
    proxy::config::write_config(app, config).await
}

#[tauri::command]
async fn read_dashboard_snapshot(
    app: tauri::AppHandle,
    range: proxy::dashboard::DashboardRange,
    limit: Option<u32>,
) -> Result<proxy::dashboard::DashboardSnapshot, String> {
    proxy::dashboard::read_snapshot(app, range, limit).await
}

/// 初始化 tracing 日志系统
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("token_proxy_lib=debug,tower_http=debug"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    tracing::info!("starting token_proxy application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            proxy::spawn(_app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_proxy_config,
            write_proxy_config,
            read_dashboard_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
