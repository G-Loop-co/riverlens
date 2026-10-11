use super::{
    facts,
    storage::{self, DatasetInfo, Incoming, Job, Rejected, Source},
};
use crate::{model::PARSER_VERSION, parser};
use anyhow::{ensure, Context, Result};
use duckdb::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    List,
    Create {
        name: String,
        timezone: String,
        #[serde(default = "default_source")]
        source: String,
    },
    Share {
        dataset_id: String,
        cloud_enabled: bool,
        mcp_enabled: bool,
    },
    StartImport {
        dataset_id: String,
        paths: Vec<String>,
    },
    Pause {
        dataset_id: String,
    },
    Resume {
        dataset_id: String,
        job_id: String,
    },
    Jobs {
        dataset_id: String,
    },
    Query {
        dataset_id: String,
        name: String,
        arguments: Value,
    },
    Hand {
        dataset_id: String,
        hand_key: String,
        version: String,
    },
    Issues {
        dataset_id: String,
        before: Option<i64>,
    },
    IssueRaw {
        dataset_id: String,
        id: i64,
    },
    Backup {
        dataset_id: String,
        path: String,
    },
    Restore {
        path: String,
    },
}
fn default_source() -> String {
    "使用者匯入".into()
}
pub struct Dataset {
    pub connection: Mutex<Connection>,
    pub writer: Mutex<()>,
    pub working: AtomicBool,
    pub stop: AtomicBool,
    pub path: PathBuf,
}
impl Dataset {
    pub fn connection(&self) -> Result<Connection> {
        Ok(self.connection.lock().unwrap().try_clone()?)
    }
}
pub struct Manager {
    root: PathBuf,
    datasets: Mutex<HashMap<String, Arc<Dataset>>>,
}
fn new_id() -> String {
    format!(
        "mda-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
impl Manager {
    pub fn new(root: PathBuf) -> Result<Arc<Self>> {
        fs::create_dir_all(&root)?;
        Ok(Arc::new(Self {
            root,
            datasets: Mutex::new(HashMap::new()),
        }))
    }
    pub fn dataset(&self, id: &str) -> Result<Arc<Dataset>> {
        ensure!(
            id.starts_with("mda-")
                && id.len() <= 80
                && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "invalid dataset id"
        );
        let mut map = self.datasets.lock().unwrap();
        if let Some(d) = map.get(id) {
            return Ok(d.clone());
        }
        let path = self.root.join(format!("{id}.duckdb"));
        ensure!(path.is_file(), "dataset not found");
        let c = storage::open(&path)?;
        storage::validate(&c)?;
        for mut j in storage::jobs(&c)? {
            if j.state == "running" {
                j.state = "interrupted".into();
                j.message = Some("程式中斷；從同交易 checkpoint 續傳。".into());
                storage::save_job(&c, &j)?;
            }
        }
        let d = Arc::new(Dataset {
            connection: Mutex::new(c),
            writer: Mutex::new(()),
            working: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            path,
        });
        map.insert(id.into(), d.clone());
        Ok(d)
    }
    pub fn version(&self, id: &str) -> Result<String> {
        storage::version(&self.dataset(id)?.connection()?)
    }
    pub fn permitted(&self, id: &str, cloud: bool) -> Result<()> {
        let i = storage::info(&self.dataset(id)?.connection()?)?;
        ensure!(
            if cloud {
                i.cloud_enabled
            } else {
                i.mcp_enabled
            },
            "MDA 資料集尚未啟用此分享權限"
        );
        Ok(())
    }
    pub fn list(&self) -> Result<Value> {
        let mut rows = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path.extension().is_none_or(|s| s != "duckdb") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match self
                .dataset(id)
                .and_then(|d| super::tools::catalog(&d.connection()?))
            {
                Ok(v) => rows.push(v),
                Err(e) => rows.push(json!({"dataset":{"id":id},"error":e.to_string()})),
            }
        }
        rows.sort_by_key(|v| v["dataset"]["created_at"].as_i64().unwrap_or(0));
        Ok(
            json!({"datasets":rows,"memory_limit":"2GiB","threads":4,"scope":"GG Regular USD NLH 6-max","vendor_compatibility":"HHDealer 未有真實樣本驗收"}),
        )
    }
    pub fn handle(self: &Arc<Self>, request: Request) -> Result<Value> {
        match request {
            Request::List => self.list(),
            Request::Create {
                name,
                timezone,
                source,
            } => {
                ensure!(
                    !name.trim().is_empty() && name.len() <= 200 && source.len() <= 500,
                    "invalid dataset metadata"
                );
                let _: chrono_tz::Tz = timezone.parse().context("invalid source timezone")?;
                let id = new_id();
                let path = self.root.join(format!("{id}.duckdb"));
                let c = storage::open(&path)?;
                let info = DatasetInfo {
                    id: id.clone(),
                    name,
                    network: "GG".into(),
                    pool: source,
                    format: "GG native NLH cash".into(),
                    timezone,
                    currency: "USD".into(),
                    parser_version: PARSER_VERSION.into(),
                    stats_version: facts::VERSION.into(),
                    schema: storage::SCHEMA,
                    cloud_enabled: false,
                    mcp_enabled: false,
                    created_at: chrono::Utc::now().timestamp(),
                };
                storage::init(&c, &info)?;
                drop(c);
                super::tools::catalog(&self.dataset(&id)?.connection()?)
            }
            Request::Share {
                dataset_id,
                cloud_enabled,
                mcp_enabled,
            } => {
                let d = self.dataset(&dataset_id)?;
                let c = d.connection()?;
                let mut i = storage::info(&c)?;
                i.cloud_enabled = cloud_enabled;
                i.mcp_enabled = mcp_enabled;
                storage::save_info(&c, &i)?;
                Ok(json!(i))
            }
            Request::StartImport { dataset_id, paths } => {
                let d = self.dataset(&dataset_id)?;
                let src = sources(&paths)?;
                ensure!(!src.is_empty(), "沒有 TXT／ZIP 手牌來源");
                let j = Job {
                    id: new_id(),
                    sources: src,
                    state: "running".into(),
                    started_at: chrono::Utc::now().timestamp(),
                    ..Job::default()
                };
                self.launch(d, j)
            }
            Request::Resume { dataset_id, job_id } => {
                let d = self.dataset(&dataset_id)?;
                let mut j = storage::jobs(&d.connection()?)?
                    .into_iter()
                    .find(|j| j.id == job_id)
                    .context("job not found")?;
                ensure!(
                    matches!(j.state.as_str(), "paused" | "interrupted" | "failed"),
                    "job cannot resume"
                );
                j.state = "running".into();
                j.message = None;
                self.launch(d, j)
            }
            Request::Pause { dataset_id } => {
                self.dataset(&dataset_id)?
                    .stop
                    .store(true, Ordering::SeqCst);
                Ok(json!({"pause_requested":true}))
            }
            Request::Jobs { dataset_id } => {
                let d = self.dataset(&dataset_id)?;
                Ok(
                    json!({"jobs":storage::jobs(&d.connection()?)?,"running":d.working.load(Ordering::SeqCst)}),
                )
            }
            Request::Query {
                dataset_id,
                name,
                mut arguments,
            } => {
                arguments["dataset_id"] = json!(dataset_id);
                self.query(&name, arguments)
            }
            Request::Hand {
                dataset_id,
                hand_key,
                version,
            } => {
                let d = self.dataset(&dataset_id)?;
                let mut c = d.connection()?;
                let tx = c.transaction()?;
                ensure!(
                    storage::version(&tx)? == version,
                    "dataset_changed: 重新查詢資料集"
                );
                let h = storage::anonymize(
                    storage::hand(&tx, storage::parse_key(&hand_key)?)?,
                    storage::parse_key(&hand_key)?,
                );
                Ok(json!({"hand":h,"annotation":{"note":"","tags":[],"reviewed":false},"mda":true}))
            }
            Request::Issues { dataset_id, before } => {
                let c = self.dataset(&dataset_id)?.connection()?;
                let rows=c.prepare("SELECT id,original_id,code,source FROM quarantine WHERE id<? ORDER BY id DESC LIMIT 100")?.query_map([before.unwrap_or(i64::MAX)],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"hand_id":r.get::<_,String>(1)?,"code":r.get::<_,String>(2)?,"source":r.get::<_,String>(3)?})))?.collect::<duckdb::Result<Vec<_>>>()?;
                Ok(json!({"rows":rows,"next_cursor":rows.last().map(|r|r["id"].clone())}))
            }
            Request::IssueRaw { dataset_id, id } => {
                let c = self.dataset(&dataset_id)?.connection()?;
                let b: Vec<u8> =
                    c.query_row("SELECT raw FROM quarantine WHERE id=?", [id], |r| r.get(0))?;
                Ok(json!({"raw":String::from_utf8(storage::unpack(&b)?)?}))
            }
            Request::Backup { dataset_id, path } => {
                let d = self.dataset(&dataset_id)?;
                ensure!(!d.working.load(Ordering::SeqCst), "請先暫停匯入再備份");
                let _g = d.writer.lock().unwrap();
                let c = d.connection()?;
                c.execute_batch("CHECKPOINT;")?;
                let dest = Path::new(&path);
                ensure!(dest != d.path, "backup cannot replace current dataset");
                backup_copy(&c, &d.path, dest)?;
                Ok(json!({"saved":true,"version":storage::version(&c)?}))
            }
            Request::Restore { path } => {
                let src = Path::new(&path);
                ensure!(src.is_file(), "backup not found");
                let id = new_id();
                let target = self.root.join(format!("{id}.duckdb"));
                let staging = self.root.join(format!("{id}.restore"));
                fs::copy(src, &staging)?;
                let result = (|| -> Result<()> {
                    let mut c = storage::open(&staging)?;
                    storage::validate(&c)?;
                    let tx = c.transaction()?;
                    let mut i = storage::info(&tx)?;
                    i.id = id.clone();
                    i.name = format!("{}（還原）", i.name);
                    i.cloud_enabled = false;
                    i.mcp_enabled = false;
                    storage::save_info(&tx, &i)?;
                    tx.execute_batch("UPDATE meta SET value=uuid()::VARCHAR WHERE key='epoch';")?;
                    tx.commit()?;
                    c.execute_batch("CHECKPOINT;")?;
                    drop(c);
                    fs::rename(&staging, &target)?;
                    Ok(())
                })();
                if let Err(e) = result {
                    let _ = fs::remove_file(staging);
                    return Err(e);
                }
                super::tools::catalog(&self.dataset(&id)?.connection()?)
            }
        }
    }
    pub fn query(&self, name: &str, arguments: Value) -> Result<Value> {
        let a = super::tools::validate(name, &arguments)?;
        let d = self.dataset(&a.dataset_id)?;
        let mut c = d.connection()?;
        let tx = c.transaction()?;
        let version = storage::version(&tx)?;
        if let Some(v) = &a.version {
            ensure!(v == &version, "dataset_changed: 重新查詢資料集與證據");
        }
        let data = super::tools::query(&tx, name, &a)?;
        Ok(json!({"version":version,"data":data}))
    }
    fn launch(self: &Arc<Self>, d: Arc<Dataset>, job: Job) -> Result<Value> {
        ensure!(
            !d.working.swap(true, Ordering::SeqCst),
            "資料集已有匯入工作"
        );
        d.stop.store(false, Ordering::SeqCst);
        let saved = d.connection().and_then(|c| storage::save_job(&c, &job));
        if let Err(e) = saved {
            d.working.store(false, Ordering::SeqCst);
            return Err(e);
        }
        let result = json!(job);
        std::thread::spawn(move || {
            let _g = d.writer.lock().unwrap();
            let mut job = job;
            let started = Instant::now();
            let prior_ms = job.elapsed_ms;
            let result = d.connection().and_then(|mut c| {
                import(&mut c, &d.stop, &mut job, |j| {
                    j.elapsed_ms = prior_ms + started.elapsed().as_millis() as u64;
                })
            });
            if let Err(e) = result {
                job.state = "failed".into();
                job.message = Some(format!("{e:#}"));
                if let Ok(c) = d.connection() {
                    let _ = storage::save_job(&c, &job);
                }
            }
            d.working.store(false, Ordering::SeqCst);
        });
        Ok(result)
    }
    pub fn shutdown(&self) {
        for d in self.datasets.lock().unwrap().values() {
            d.stop.store(true, Ordering::SeqCst);
        }
    }
}
fn backup_copy(c: &Connection, source: &Path, dest: &Path) -> Result<()> {
    let staging = storage::backup_path(source);
    let files = ["", ".wal", ".wal.checkpoint", ".wal.recovery"]
        .map(|suffix| PathBuf::from(format!("{}{suffix}", staging.to_string_lossy())));
    for path in &files {
        if path.exists() {
            fs::remove_file(path).context("clean previous backup staging")?;
        }
    }
    let catalog: String = c.query_row("SELECT current_database()", [], |r| r.get(0))?;
    let catalog = catalog.replace('"', "\"\"");
    let path = staging.to_string_lossy().replace('\'', "''");
    c.execute_batch(&format!("ATTACH '{path}' AS mda_backup;"))?;
    let result = c.execute_batch(&format!(
        "COPY FROM DATABASE \"{catalog}\" TO mda_backup;CHECKPOINT mda_backup;"
    ));
    let detached = c.execute_batch("DETACH mda_backup;");
    let result = result
        .context("copy MDA backup snapshot")
        .and_then(|_| detached.context("close MDA backup"))
        .and_then(|_| atomic_copy(&staging, dest));
    for path in &files {
        let _ = fs::remove_file(path);
    }
    result
}
fn atomic_copy(source: &Path, dest: &Path) -> Result<()> {
    let parent = dest.parent().context("invalid backup destination")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    let mut f = File::open(source)?;
    std::io::copy(&mut f, &mut tmp)?;
    tmp.as_file().sync_all()?;
    tmp.persist(dest).map_err(|e| e.error)?;
    Ok(())
}
fn signature(path: &Path) -> Result<(u64, u64)> {
    let m = path.metadata()?;
    Ok((m.len(), m.modified()?.duration_since(UNIX_EPOCH)?.as_secs()))
}
fn sources(paths: &[String]) -> Result<Vec<Source>> {
    ensure!(
        !paths.is_empty() && paths.len() <= 1000,
        "invalid source count"
    );
    let mut files = Vec::new();
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            for e in walkdir::WalkDir::new(path).follow_links(false) {
                let e = e?;
                if e.file_type().is_file() {
                    files.push(e.path().to_path_buf());
                    ensure!(files.len() <= 100_000, "too many source files");
                }
            }
        } else {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files.dedup();
    let mut out = Vec::new();
    for path in files {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["txt", "zip"].contains(&ext.as_str()) {
            continue;
        }
        let (size, modified) = signature(&path)?;
        let path_text = path.to_string_lossy().into_owned();
        if ext == "txt" {
            out.push(Source {
                path: path_text,
                member: None,
                size,
                modified,
                crc: None,
            });
        } else {
            let mut archive = zip::ZipArchive::new(File::open(&path)?)?;
            for i in 0..archive.len() {
                let f = archive.by_index(i)?;
                if !f.is_dir() && f.name().to_ascii_lowercase().ends_with(".txt") {
                    out.push(Source {
                        path: path_text.clone(),
                        member: Some(f.name().into()),
                        size,
                        modified,
                        crc: Some(f.crc32()),
                    });
                }
            }
        }
        ensure!(out.len() <= 100_000, "too many ZIP members");
    }
    Ok(out)
}
fn stream<R: Read>(reader: R, mut on_hand: impl FnMut(String, u64) -> Result<bool>) -> Result<()> {
    let mut reader = BufReader::with_capacity(64 * 1024, reader);
    let mut raw = String::new();
    let mut count = 0;
    loop {
        let mut line = String::new();
        let n = (&mut reader).take(1_048_577).read_line(&mut line)?;
        if n == 0 {
            break;
        }
        ensure!(line.len() <= 1_048_576, "line exceeds 1MiB");
        if line
            .trim_start_matches('\u{feff}')
            .starts_with("Poker Hand #")
            && !raw.is_empty()
        {
            count += 1;
            if !on_hand(std::mem::take(&mut raw), count)? {
                return Ok(());
            }
        }
        if !raw.is_empty()
            || line
                .trim_start_matches('\u{feff}')
                .starts_with("Poker Hand #")
        {
            raw.push_str(&line);
            ensure!(raw.len() <= 1_048_576, "hand exceeds 1MiB");
        }
    }
    if !raw.is_empty() {
        on_hand(raw, count + 1)?;
    } else if count == 0 {
        anyhow::bail!("沒有標準 GG Poker Hand header；請核對來源格式");
    }
    Ok(())
}
pub fn import(
    c: &mut Connection,
    stop: &AtomicBool,
    job: &mut Job,
    mut progress: impl FnMut(&mut Job),
) -> Result<()> {
    let info = storage::info(c)?;
    while job.source_index < job.sources.len() {
        let src = job.sources[job.source_index].clone();
        ensure!(
            signature(Path::new(&src.path))? == (src.size, src.modified),
            "source_changed: 來源檔案已改動，請另開增量匯入"
        );
        let mut batch = Vec::new();
        let mut rejected = Vec::new();
        let mut last = job.committed_hands;
        let resume_after = job.committed_hands;
        let mut on_hand = |raw: String, number: u64| -> Result<bool> {
            if number <= resume_after {
                return Ok(!stop.load(Ordering::SeqCst));
            }
            if stop.load(Ordering::SeqCst) {
                return Ok(false);
            }
            last = number;
            let parsed =
                parser::parse_observed(&raw, &info.timezone, &info.network).and_then(|mut h| {
                    ensure!(
                        h.status == "valid",
                        "parser quarantine: {}",
                        h.issues
                            .iter()
                            .map(|i| i.code.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                    let decisions = facts::derive(&mut h)?;
                    h.raw = raw.clone();
                    Ok(Incoming { hand: h, decisions })
                });
            match parsed {
                Ok(row) => batch.push(row),
                Err(e) => rejected.push(Rejected {
                    original_id: raw
                        .lines()
                        .next()
                        .and_then(|l| l.strip_prefix("Poker Hand #"))
                        .and_then(|l| l.split(':').next())
                        .unwrap_or("")
                        .into(),
                    code: format!("{e:#}"),
                    raw,
                }),
            }
            if batch.len() + rejected.len() >= storage::BATCH_SIZE {
                progress(job);
                storage::commit_batch(
                    c,
                    std::mem::take(&mut batch),
                    std::mem::take(&mut rejected),
                    job,
                    last,
                    &src.path,
                )?;
            }
            Ok(true)
        };
        if let Some(member) = &src.member {
            let mut zip = zip::ZipArchive::new(File::open(&src.path)?)?;
            let f = zip.by_name(member)?;
            ensure!(src.crc == Some(f.crc32()), "ZIP member changed");
            stream(f, &mut on_hand)?;
        } else {
            stream(File::open(&src.path)?, &mut on_hand)?;
        }
        progress(job);
        if !batch.is_empty() || !rejected.is_empty() {
            storage::commit_batch(c, batch, rejected, job, last, &src.path)?;
        }
        if stop.load(Ordering::SeqCst) {
            job.state = "paused".into();
            storage::save_job(c, job)?;
            return Ok(());
        }
        job.source_index += 1;
        job.committed_hands = 0;
        storage::save_job(c, job)?;
    }
    storage::compact_rollups(c)?;
    c.execute_batch("CHECKPOINT;")?;
    job.state = "completed".into();
    progress(job);
    storage::save_job(c, job)?;
    Ok(())
}
