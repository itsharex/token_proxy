use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::proxy::service::{ProxyServiceHandle, ProxyServiceState, ProxyServiceStatus};

type AppMenuItem = MenuItem<tauri::Wry>;

const TRAY_ID: &str = "token-proxy-tray";
const MENU_SHOW: &str = "tray_show_window";
const MENU_START: &str = "tray_start_proxy";
const MENU_STOP: &str = "tray_stop_proxy";
const MENU_RESTART: &str = "tray_restart_proxy";
const MENU_STATUS: &str = "tray_status";
const MENU_QUIT: &str = "tray_quit";

#[derive(Clone)]
pub(crate) struct TrayState {
    inner: Arc<TrayStateInner>,
}

struct TrayStateInner {
    start_item: AppMenuItem,
    stop_item: AppMenuItem,
    restart_item: AppMenuItem,
    status_item: AppMenuItem,
    should_quit: AtomicBool,
}

impl TrayState {
    pub(crate) fn should_quit(&self) -> bool {
        self.inner.should_quit.load(Ordering::SeqCst)
    }

    pub(crate) fn mark_quit(&self) {
        self.inner.should_quit.store(true, Ordering::SeqCst);
    }

    pub(crate) fn apply_status(&self, status: &ProxyServiceStatus) {
        let text = format_status_text(status);
        let _ = self.inner.status_item.set_text(text);
        let _ = self.inner.status_item.set_enabled(false);

        match status.state {
            ProxyServiceState::Running => {
                let _ = self.inner.start_item.set_enabled(false);
                let _ = self.inner.stop_item.set_enabled(true);
                let _ = self.inner.restart_item.set_enabled(true);
            }
            ProxyServiceState::Stopped => {
                let _ = self.inner.start_item.set_enabled(true);
                let _ = self.inner.stop_item.set_enabled(false);
                let _ = self.inner.restart_item.set_enabled(false);
            }
        }
    }

    pub(crate) fn apply_error(&self, title: &str, err: &str) {
        let message = format!("{title} · {}", compact_error(err));
        let _ = self.inner.status_item.set_text(message);
        let _ = self.inner.status_item.set_enabled(false);
    }
}

pub(crate) fn init_tray(
    app: &AppHandle,
    proxy_service: ProxyServiceHandle,
) -> Result<TrayState, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, MENU_SHOW, "显示主窗口", true, None::<&str>)?;
    let start_item = MenuItem::with_id(app, MENU_START, "启动代理", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, MENU_STOP, "停止代理", false, None::<&str>)?;
    let restart_item = MenuItem::with_id(app, MENU_RESTART, "重启代理", false, None::<&str>)?;
    let status_item = MenuItem::with_id(app, MENU_STATUS, "状态：启动中...", false, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;

    let menu = Menu::new(app)?;
    menu.append_items(&[
        &show_item,
        &PredefinedMenuItem::separator(app)?,
        &start_item,
        &stop_item,
        &restart_item,
        &PredefinedMenuItem::separator(app)?,
        &status_item,
        &PredefinedMenuItem::separator(app)?,
        &quit_item,
    ])?;

    let tray_state = TrayState {
        inner: Arc::new(TrayStateInner {
            start_item: start_item.clone(),
            stop_item: stop_item.clone(),
            restart_item: restart_item.clone(),
            status_item: status_item.clone(),
            should_quit: AtomicBool::new(false),
        }),
    };

    let tray_state_for_menu = tray_state.clone();
    let proxy_for_menu = proxy_service.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(load_tray_icon()?)
        .tooltip("Token Proxy")
        .show_menu_on_left_click(true)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                MENU_SHOW => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                MENU_START => {
                    let app = app.clone();
                    let tray_state = tray_state_for_menu.clone();
                    let proxy_service = proxy_for_menu.clone();
                    tauri::async_runtime::spawn(async move {
                        match proxy_service.start(app).await {
                            Ok(status) => tray_state.apply_status(&status),
                            Err(err) => tray_state.apply_error("启动失败", &err),
                        }
                    });
                }
                MENU_STOP => {
                    let tray_state = tray_state_for_menu.clone();
                    let proxy_service = proxy_for_menu.clone();
                    tauri::async_runtime::spawn(async move {
                        match proxy_service.stop().await {
                            Ok(status) => tray_state.apply_status(&status),
                            Err(err) => tray_state.apply_error("停止失败", &err),
                        }
                    });
                }
                MENU_RESTART => {
                    let app = app.clone();
                    let tray_state = tray_state_for_menu.clone();
                    let proxy_service = proxy_for_menu.clone();
                    tauri::async_runtime::spawn(async move {
                        match proxy_service.restart(app).await {
                            Ok(status) => tray_state.apply_status(&status),
                            Err(err) => tray_state.apply_error("重启失败", &err),
                        }
                    });
                }
                MENU_QUIT => {
                    tray_state_for_menu.mark_quit();
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(tray_state)
}

fn format_status_text(status: &ProxyServiceStatus) -> String {
    match status.state {
        ProxyServiceState::Running => {
            let addr = status.addr.clone().unwrap_or_default();
            if let Some(err) = status.last_error.as_ref() {
                format!("运行中 · {addr} · 上次错误：{}", compact_error(err))
            } else {
                format!("运行中 · {addr}")
            }
        }
        ProxyServiceState::Stopped => match status.last_error.as_ref() {
            Some(err) => format!("启动失败 · {}", compact_error(err)),
            None => "已停止".to_string(),
        },
    }
}

fn compact_error(err: &str) -> String {
    let trimmed = err.trim();
    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    let mut output = String::new();
    for ch in first_line.chars().take(80) {
        output.push(ch);
    }
    if first_line.chars().count() > 80 {
        output.push_str("...");
    }
    output
}

fn load_tray_icon() -> Result<Image<'static>, Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../icons/icon-state.png");
    Ok(Image::from_bytes(bytes)?)
}
