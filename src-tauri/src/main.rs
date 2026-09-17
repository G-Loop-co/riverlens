#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use poker_core::service::{Request, Service};
use riverlens_ai::{bridge::Bridge, coach};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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

struct AiState {
    bridge: Arc<Bridge>,
    job: Mutex<Option<(String, tauri::async_runtime::JoinHandle<()>)>>,
}
#[tauri::command]
fn ai_status(state: tauri::State<'_, AiState>) -> Result<serde_json::Value, String> {
    let mut status = state.bridge.status().map_err(|e| e.to_string())?;
    status["keys"] = coach::key_status();
    Ok(status)
}
#[tauri::command]
fn ai_connect(
    enabled: bool,
    notes: bool,
    state: tauri::State<'_, AiState>,
) -> Result<serde_json::Value, String> {
    state
        .bridge
        .configure(enabled, notes)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn ai_key(provider: String, key: String) -> Result<(), String> {
    coach::save_key(&provider, &key).map_err(|e| e.to_string())
}
#[tauri::command]
async fn ai_organize(
    provider: String,
    model: String,
    id: String,
    state: tauri::State<'_, Arc<Service>>,
) -> Result<(), String> {
    coach::organize(state.inner().clone(), &provider, &model, &id)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn ai_cancel(id: String, state: tauri::State<'_, AiState>) -> Result<(), String> {
    let mut job = state.job.lock().unwrap();
    if job.as_ref().is_some_and(|j| j.0 == id) {
        if let Some((_, h)) = job.take() {
            h.abort();
        }
    }
    Ok(())
}
#[tauri::command]
fn ai_chat(
    chat: coach::Chat,
    app: tauri::AppHandle,
    state: tauri::State<'_, AiState>,
) -> Result<(), String> {
    let mut job = state.job.lock().unwrap();
    // A new turn cancels the prior turn, including its HTTP stream.
    if let Some((_, h)) = job.take() {
        h.abort();
    }
    let service = state.bridge.service.clone();
    let id = chat.id.clone();
    let event_id = id.clone();
    let emit: Arc<dyn Fn(serde_json::Value) + Send + Sync> = Arc::new(move |mut event| {
        event["id"] = serde_json::json!(event_id);
        let _ = app.emit("riverlens-ai", event);
    });
    let handle = tauri::async_runtime::spawn(async move {
        match coach::run(service, chat, emit.clone()).await {
            Ok(()) => emit(serde_json::json!({"type":"done"})),
            Err(e) => emit(serde_json::json!({"type":"error","message":e.to_string()})),
        }
    });
    *job = Some((id, handle));
    Ok(())
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--mcp") {
        if args.len() != 4 || args[2] != "--socket" {
            std::process::exit(2);
        }
        if let Err(e) = tauri::async_runtime::block_on(riverlens_ai::mcp::run(args[3].clone())) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = std::env::var_os("RIVERLENS_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            let service = Service::new(dir.join("riverlens.db"))?;
            service.start_equity();
            let bridge = tauri::async_runtime::block_on(Bridge::start(service.clone()))?;
            app.manage(AiState {
                bridge,
                job: Mutex::new(None),
            });
            app.manage(service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api,
            ai_status,
            ai_connect,
            ai_key,
            ai_chat,
            ai_cancel,
            ai_organize
        ])
        .build(tauri::generate_context!())
        .expect("RiverLens could not start")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(state) = app.try_state::<AiState>() {
                    let _ = state.bridge.configure(false, false);
                    if let Some((_, job)) = state.job.lock().unwrap().take() {
                        job.abort();
                    }
                }
            }
        });
}
