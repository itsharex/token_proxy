use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager};

use crate::proxy::config::{TrayTokenRateConfig, TrayTokenRateFormat};
use crate::proxy::service::{ProxyServiceHandle, ProxyServiceState, ProxyServiceStatus};
use crate::proxy::token_rate::{TokenRateSnapshot, TokenRateTracker};

type AppMenuItem = MenuItem<tauri::Wry>;
type AppTrayIcon = TrayIcon<tauri::Wry>;

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
    tray: AppTrayIcon,
    start_item: AppMenuItem,
    stop_item: AppMenuItem,
    restart_item: AppMenuItem,
    status_item: AppMenuItem,
    token_rate: Arc<TokenRateTracker>,
    token_rate_config: RwLock<TrayTokenRateConfig>,
    last_title: RwLock<Option<String>>,
    should_quit: AtomicBool,
}

impl TrayState {
    pub(crate) fn should_quit(&self) -> bool {
        self.inner.should_quit.load(Ordering::SeqCst)
    }

    pub(crate) fn mark_quit(&self) {
        self.inner.should_quit.store(true, Ordering::SeqCst);
    }

    pub(crate) fn apply_config(&self, config: &TrayTokenRateConfig) {
        let mut guard = self
            .inner
            .token_rate_config
            .write()
            .expect("tray token rate config lock poisoned");
        *guard = config.clone();
        // 配置变化也唤醒托盘刷新，避免空闲等待时错过更新。
        self.inner.token_rate.notify_activity();
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

    #[cfg(target_os = "macos")]
    fn update_token_rate_title(&self) {
        let config = self
            .inner
            .token_rate_config
            .read()
            .expect("tray token rate config lock poisoned")
            .clone();
        if !config.enabled {
            self.set_title(None);
            return;
        }
        // 启用后始终显示速率，空闲时自然显示 0。
        let snapshot = self.inner.token_rate.snapshot();
        let title = format_rate_title(snapshot, config.format);
        self.set_title(Some(title));
    }

    #[cfg(target_os = "macos")]
    fn set_title(&self, title: Option<String>) {
        let mut last_title = self
            .inner
            .last_title
            .write()
            .expect("tray title lock poisoned");
        if *last_title == title {
            return;
        }
        let _ = self.inner.tray.set_title(title.as_deref());
        *last_title = title;
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

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(load_tray_icon()?)
        .tooltip("Token Proxy")
        .show_menu_on_left_click(true)
        .icon_as_template(true)
        .menu(&menu)
        .build(app)?;

    let token_rate = app
        .state::<Arc<TokenRateTracker>>()
        .inner()
        .clone();
    let tray_state = TrayState {
        inner: Arc::new(TrayStateInner {
            tray,
            start_item: start_item.clone(),
            stop_item: stop_item.clone(),
            restart_item: restart_item.clone(),
            status_item: status_item.clone(),
            token_rate,
            token_rate_config: RwLock::new(TrayTokenRateConfig::default()),
            last_title: RwLock::new(None),
            should_quit: AtomicBool::new(false),
        }),
    };

    let tray_state_for_menu = tray_state.clone();
    let proxy_for_menu = proxy_service.clone();
    tray_state.inner.tray.on_menu_event(move |app, event| {
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
        });

    #[cfg(target_os = "macos")]
    start_token_rate_loop(tray_state.clone());

    Ok(tray_state)
}

#[cfg(target_os = "macos")]
fn start_token_rate_loop(tray_state: TrayState) {
    let token_rate = tray_state.inner.token_rate.clone();
    tauri::async_runtime::spawn(async move {
        let mut activity_rx = token_rate.subscribe_activity();
        loop {
            if tray_state.should_quit() {
                break;
            }
            if token_rate.has_active_requests() {
                let mut interval = tokio::time::interval(Duration::from_millis(300));
                loop {
                    interval.tick().await;
                    if tray_state.should_quit() {
                        return;
                    }
                    tray_state.update_token_rate_title();
                    if !token_rate.has_active_requests() {
                        break;
                    }
                }
                continue;
            }

            tray_state.update_token_rate_title();
            // 空闲时不轮询，等待新请求或配置变化唤醒。
            if activity_rx.changed().await.is_err() {
                break;
            }
        }
    });
}

fn format_rate_title(snapshot: TokenRateSnapshot, format: TrayTokenRateFormat) -> String {
    match format {
        TrayTokenRateFormat::Combined => format!("{}", snapshot.total),
        TrayTokenRateFormat::Split => format!("↑{} ↓{}", snapshot.input, snapshot.output),
        TrayTokenRateFormat::Both => {
            format!("{} | ↑{} ↓{}", snapshot.total, snapshot.input, snapshot.output)
        }
    }
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
