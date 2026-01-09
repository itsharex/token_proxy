// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod proxy;
mod tray;

use std::time::Instant;
use tauri::Manager;
#[cfg(debug_assertions)]
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
    tray_state: tauri::State<'_, tray::TrayState>,
    config: proxy::config::ProxyConfigFile,
) -> Result<ProxyServiceStatus, String> {
    tracing::debug!("write_proxy_config start");
    let start = Instant::now();
    tracing::debug!("write_proxy_config apply_config start");
    let apply_start = Instant::now();
    tray_state.apply_config(&config.tray_token_rate);
    tracing::debug!(
        elapsed_ms = apply_start.elapsed().as_millis(),
        "write_proxy_config apply_config done"
    );
    if let Err(err) = proxy::config::write_config(app.clone(), config).await {
        tracing::error!(error = %err, "write_proxy_config save failed");
        tray_state.apply_error("保存失败", &err);
        return Err(err);
    }
    tracing::debug!(elapsed_ms = start.elapsed().as_millis(), "write_proxy_config saved");
    let reload_start = Instant::now();
    match proxy_service.reload(app).await {
        Ok(status) => {
            tracing::debug!(
                elapsed_ms = reload_start.elapsed().as_millis(),
                state = ?status.state,
                "write_proxy_config reloaded"
            );
            tray_state.apply_status(&status);
            Ok(status)
        }
        Err(err) => {
            tracing::error!(
                elapsed_ms = reload_start.elapsed().as_millis(),
                error = %err,
                "write_proxy_config reload failed"
            );
            tray_state.apply_error("重载失败", &err);
            Err(err)
        }
    }
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
    tray_state: tauri::State<'_, tray::TrayState>,
) -> Result<ProxyServiceStatus, String> {
    let status = proxy_service.status().await;
    tray_state.apply_status(&status);
    Ok(status)
}

#[tauri::command]
async fn proxy_start(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
    tray_state: tauri::State<'_, tray::TrayState>,
) -> Result<ProxyServiceStatus, String> {
    match proxy_service.start(app).await {
        Ok(status) => {
            tray_state.apply_status(&status);
            Ok(status)
        }
        Err(err) => {
            tray_state.apply_error("启动失败", &err);
            Err(err)
        }
    }
}

#[tauri::command]
async fn proxy_stop(
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
    tray_state: tauri::State<'_, tray::TrayState>,
) -> Result<ProxyServiceStatus, String> {
    match proxy_service.stop().await {
        Ok(status) => {
            tray_state.apply_status(&status);
            Ok(status)
        }
        Err(err) => {
            tray_state.apply_error("停止失败", &err);
            Err(err)
        }
    }
}

#[tauri::command]
async fn proxy_restart(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
    tray_state: tauri::State<'_, tray::TrayState>,
) -> Result<ProxyServiceStatus, String> {
    match proxy_service.restart(app).await {
        Ok(status) => {
            tray_state.apply_status(&status);
            Ok(status)
        }
        Err(err) => {
            tray_state.apply_error("重启失败", &err);
            Err(err)
        }
    }
}

#[tauri::command]
async fn proxy_reload(
    app: tauri::AppHandle,
    proxy_service: tauri::State<'_, ProxyServiceHandle>,
    tray_state: tauri::State<'_, tray::TrayState>,
) -> Result<ProxyServiceStatus, String> {
    match proxy_service.reload(app).await {
        Ok(status) => {
            tray_state.apply_status(&status);
            Ok(status)
        }
        Err(err) => {
            tray_state.apply_error("重载失败", &err);
            Err(err)
        }
    }
}

/// 初始化 tracing 日志系统
fn init_tracing() {
    // release 不初始化 tracing，避免任何运行时日志输出。
    #[cfg(debug_assertions)]
    {
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    tracing::info!("starting token_proxy application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let token_rate = proxy::token_rate::TokenRateTracker::new();
            app.manage(token_rate.clone());
            let proxy_service = ProxyServiceHandle::new();
            app.manage(proxy_service.clone());
            let app_handle = app.handle().clone();
            let tray_state = tray::init_tray(&app_handle, proxy_service.clone())?;
            app.manage(tray_state.clone());

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let tray_state_for_config = tray_state.clone();
            let app_handle_for_config = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(response) = proxy::config::read_config(app_handle_for_config).await {
                    tray_state_for_config.apply_config(&response.config.tray_token_rate);
                }
            });

            let tray_state_for_start = tray_state.clone();
            let proxy_for_start = proxy_service.clone();
            tauri::async_runtime::spawn(async move {
                match proxy_for_start.start(app_handle).await {
                    Ok(status) => tray_state_for_start.apply_status(&status),
                    Err(err) => {
                        tray_state_for_start.apply_error("启动失败", &err);
                        tracing::error!(error = %err, "proxy start failed");
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let tray_state = window.app_handle().try_state::<tray::TrayState>();
                if tray_state.as_ref().map(|state| state.should_quit()).unwrap_or(false) {
                    return;
                }
                api.prevent_close();
                let _ = window.hide();
            }
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
