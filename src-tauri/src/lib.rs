// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod proxy;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            proxy::spawn(_app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_proxy_config,
            write_proxy_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
