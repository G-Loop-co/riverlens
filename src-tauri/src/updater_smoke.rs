//! CI-only driver for an isolated NSIS installation on a runner without browser IPC.
use poker_core::service::Service;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{io, sync::Arc, time::Duration};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Spec {
    before: String,
    after: String,
    token: String,
    hand_path: String,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn verify(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message).into())
    }
}

fn rpc(service: &Arc<Service>, request: Value) -> Result<Value> {
    Ok(service.handle(serde_json::from_value(request)?)?)
}

fn record(service: &Arc<Service>, token: &str, step: &str, detail: &str) -> Result<()> {
    rpc(
        service,
        json!({"op":"save_filter", "name":token,
            "filter":{"search":json!({"step":step,"detail":detail}).to_string()}}),
    )?;
    Ok(())
}

pub fn start(app: AppHandle, service: Arc<Service>) -> Result<()> {
    verify(
        app.config()
            .identifier
            .starts_with("app.riverlens.updaterqa."),
        "Updater smoke requires an isolated QA application identifier",
    )?;
    let spec: Spec = serde_json::from_slice(&std::fs::read(
        service.path.with_file_name("updater-smoke.json"),
    )?)?;
    tauri::async_runtime::spawn(async move {
        let status = match run(&app, &service, &spec).await {
            Ok(()) => 0,
            Err(error) => {
                let _ = record(&service, &spec.token, "failed", &error.to_string());
                1
            }
        };
        app.exit(status);
    });
    Ok(())
}

async fn run(app: &AppHandle, service: &Arc<Service>, spec: &Spec) -> Result<()> {
    let version = app.package_info().version.to_string();
    record(service, &spec.token, "launched", &version)?;
    let hands = || {
        rpc(
            service,
            json!({"op":"hands","filter":{"profile":"qa"},"limit":5}),
        )
    };
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    if version == spec.before {
        let profile = json!({"id":"qa","name":"Updater test","hero":"Hero",
            "brand":"Natural8","timezone":"Asia/Hong_Kong"});
        rpc(service, json!({"op":"save_profile","profile":profile}))?;
        rpc(
            service,
            json!({"op":"start_import","paths":[spec.hand_path],"profile":profile}),
        )?;
        let mut report = hands()?;
        for _ in 0..60 {
            if let Some([_]) = report["rows"].as_array().map(Vec::as_slice) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
            report = hands()?;
        }
        verify(
            report["rows"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1),
            "Fixture hand import failed",
        )?;
        rpc(
            service,
            json!({"op":"save_annotation","id":report["rows"][0]["id"],
            "annotation":{"note":spec.token,"tags":["qa"],"reviewed":true}}),
        )?;
        rpc(
            service,
            json!({"op":"mda","request":{"op":"create","name":spec.token,
            "source":"Synthetic","timezone":"UTC"}}),
        )?;
        let update = updater
            .check()
            .await?
            .ok_or_else(|| io::Error::other("No update offered"))?;
        verify(update.version == spec.after, "Unexpected update version")?;
        let bytes = update.download(|_, _| {}, || {}).await?;
        record(service, &spec.token, "downloaded", &update.version)?;
        update.install(bytes)?;
        app.restart();
    }

    verify(version == spec.after, "Unexpected installed version")?;
    let report = hands()?;
    verify(
        report["rows"]
            .as_array()
            .is_some_and(|rows| rows.len() == 1),
        "Hand lost after update",
    )?;
    let hand = rpc(service, json!({"op":"hand","id":report["rows"][0]["id"]}))?;
    let annotation = &hand["annotation"];
    verify(
        annotation["note"] == spec.token
            && annotation["reviewed"] == true
            && annotation["tags"] == json!(["qa"]),
        "Annotation lost after update",
    )?;
    let catalog = rpc(service, json!({"op":"mda","request":{"op":"list"}}))?;
    verify(
        catalog["datasets"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["dataset"]["name"] == spec.token)),
        "Research dataset lost after update",
    )?;
    verify(
        updater.check().await?.is_none(),
        "Current version still offered an update",
    )?;
    record(service, &spec.token, "passed", &version)
}
