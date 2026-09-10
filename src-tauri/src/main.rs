#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use poker_core::service::{Request, Service};
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
async fn api(
    request: Request,
    state: tauri::State<'_, Arc<Service>>,
) -> Result<serde_json::Value, String> {
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.handle(request).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = std::env::var_os("RIVERLENS_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            let service = Service::new(dir.join("riverlens.db"))?;
            service.start_equity();
            app.manage(service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api])
        .run(tauri::generate_context!())
        .expect("RiverLens could not start");
}
