// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod proxy;

use tauri::Manager;
use tracing_subscriber::{fmt, EnvFilter};

type ProxyServiceHandle = proxy::service::ProxyServiceHandle;
type ProxyServiceStatus = proxy::service::ProxyServiceStatus;

#[tauri::command]
async fn read_proxy_config(app: tauri::AppHandle) -> Result<proxy::config::ConfigResponse, String> {
    proxy::config::read_config(app).await
}

#[tauri::command]
async fn write_proxy_config(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
    config: proxy::config::ProxyConfigFile,
) -> Result<ProxyServiceStatus, String> {
    proxy::config::write_config(app.clone(), config).await?;
    proxy_service.reload(app).await
}

#[tauri::command]
async fn read_dashboard_snapshot(
    app: tauri::AppHandle,
    range: proxy::dashboard::DashboardRange,
    offset: Option<u32>,
) -> Result<proxy::dashboard::DashboardSnapshot, String> {
    proxy::dashboard::read_snapshot(app, range, offset).await
}

#[tauri::command]
async fn proxy_status(
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
) -> Result<ProxyServiceStatus, String> {
    Ok(proxy_service.status().await)
}

#[tauri::command]
async fn proxy_start(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
) -> Result<ProxyServiceStatus, String> {
    proxy_service.start(app).await
}

#[tauri::command]
async fn proxy_stop(
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
) -> Result<ProxyServiceStatus, String> {
    proxy_service.stop().await
}

#[tauri::command]
async fn proxy_restart(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
) -> Result<ProxyServiceStatus, String> {
    proxy_service.restart(app).await
}

#[tauri::command]
async fn proxy_reload(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
) -> Result<ProxyServiceStatus, String> {
    proxy_service.reload(app).await
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
        .setup(|app| {
            let proxy_service = ProxyServiceHandle::new();
            app.manage(proxy_service.clone());
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = proxy_service.start(app_handle).await {
                    tracing::error!(error = %err, "proxy start failed");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_proxy_config,
            write_proxy_config,
            read_dashboard_snapshot,
            proxy_status,
            proxy_start,
            proxy_stop,
            proxy_restart,
            proxy_reload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
