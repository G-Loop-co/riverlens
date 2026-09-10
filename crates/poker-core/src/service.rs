use crate::{equity, model::*, parser, store};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Overview {
        filter: Filter,
        group: Option<String>,
    },
    Hands {
        filter: Filter,
        sort: Option<String>,
        cursor: Option<Value>,
        limit: Option<u32>,
    },
    Hand {
        id: i64,
    },
    Matrix {
        filter: Filter,
        stat: String,
    },
    SaveAnnotation {
        id: i64,
        annotation: Annotation,
    },
    Profiles,
    SaveProfile {
        profile: Profile,
    },
    StartImport {
        paths: Vec<String>,
        profile: Profile,
    },
    Jobs,
    CancelImport {
        id: String,
    },
    ResumeImport {
        id: String,
    },
    Issues {
        before: Option<i64>,
    },
    IssueRaw {
        id: i64,
    },
    SavedFilters,
    SaveFilter {
        name: String,
        filter: Filter,
    },
    DeleteFilter {
        id: i64,
    },
    Backup {
        path: String,
    },
    Restore {
        path: String,
    },
    Export {
        path: String,
        format: String,
        filter: Filter,
        selected: Vec<i64>,
    },
    EquityControl {
        paused: bool,
    },
    Rebuild,
    Health,
}

pub struct Service {
    pub path: PathBuf,
    writer: Mutex<()>,
    active: AtomicBool,
    cancel: AtomicBool,
    equity_pause: AtomicBool,
    equity_running: AtomicBool,
    equity_generation: AtomicU64,
    equity_error: Mutex<Option<String>>,
    current: Mutex<Option<Job>>,
}
impl Service {
    pub fn new(path: PathBuf) -> Result<Arc<Self>> {
        store::init(&path)?;
        let c = store::open(&path)?;
        for mut job in store::jobs(&c)? {
            if job.state == "running" {
                job.state = "interrupted".into();
                job.message = Some("程式曾中斷；可從已提交資料續匯。".into());
                store::save_job(&c, &job)?;
            }
        }
        Ok(Arc::new(Self {
            path,
            writer: Mutex::new(()),
            active: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            equity_pause: AtomicBool::new(false),
            equity_running: AtomicBool::new(false),
            equity_generation: AtomicU64::new(0),
            equity_error: Mutex::new(None),
            current: Mutex::new(None),
        }))
    }
    pub fn handle(self: &Arc<Self>, request: Request) -> Result<Value> {
        let mut c = store::open(&self.path)?;
        match request {
            Request::Overview { filter, group } => {
                let tx = c.transaction()?;
                let report = store::report(&tx, &filter, group.as_deref().unwrap_or("date"))?;
                tx.commit()?;
                Ok(report)
            }
            Request::Hands {
                filter,
                sort,
                cursor,
                limit,
            } => store::list_hands(
                &c,
                &filter,
                sort.as_deref().unwrap_or("recent"),
                cursor.as_ref(),
                limit.unwrap_or(60),
            ),
            Request::Hand { id } => Ok(
                json!({"id":id,"hand":store::get_hand(&c,id)?,"annotation":store::annotation(&c,id)?}),
            ),
            Request::Matrix { filter, stat } => store::matrix(&c, &filter, &stat),
            Request::SaveAnnotation { id, annotation } => {
                let _l = self.writer.lock().unwrap();
                store::save_annotation(&mut c, id, &annotation)?;
                Ok(json!({"saved":true}))
            }
            Request::Profiles => Ok(json!(store::profiles(&c)?)),
            Request::SaveProfile { profile } => {
                let _l = self.writer.lock().unwrap();
                store::save_profile(&c, &profile)?;
                Ok(json!({"saved":true}))
            }
            Request::StartImport { paths, profile } => {
                anyhow::ensure!(!paths.is_empty(), "選擇 TXT、ZIP 或資料夾");
                let job = Job {
                    id: format!("import-{}", chrono::Utc::now().timestamp_micros()),
                    profile,
                    paths,
                    state: "running".into(),
                    scanned: 0,
                    inserted: 0,
                    duplicates: 0,
                    quarantined: 0,
                    conflicts: 0,
                    files_done: 0,
                    files_total: 0,
                    current_file: String::new(),
                    message: None,
                    started_at: chrono::Utc::now().timestamp(),
                    elapsed_ms: 0,
                };
                self.start(job.clone())?;
                Ok(json!(job))
            }
            Request::Jobs => {
                let mut jobs = store::jobs(&c)?;
                if let Some(j) = self.current.lock().unwrap().clone() {
                    if let Some(row) = jobs.iter_mut().find(|r| r.id == j.id) {
                        *row = j;
                    } else {
                        jobs.insert(0, j);
                    }
                }
                Ok(json!(jobs))
            }
            Request::CancelImport { id } => {
                if self
                    .current
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|j| j.id == id)
                {
                    self.cancel.store(true, Ordering::Relaxed);
                }
                Ok(json!({"requested":true}))
            }
            Request::ResumeImport { id } => {
                let mut job = store::jobs(&c)?
                    .into_iter()
                    .find(|j| j.id == id)
                    .context("import job not found")?;
                anyhow::ensure!(
                    matches!(job.state.as_str(), "cancelled" | "interrupted" | "failed"),
                    "job cannot be resumed"
                );
                job.state = "running".into();
                job.message = None;
                self.start(job.clone())?;
                Ok(json!(job))
            }
            Request::Issues { before } => store::issues_page(&c, before),
            Request::IssueRaw { id } => {
                let blob: Option<Vec<u8>> =
                    c.query_row("SELECT raw FROM import_issues WHERE id=?1", [id], |r| {
                        r.get(0)
                    })?;
                let raw = if let Some(b) = blob {
                    use std::io::Read;
                    let mut d = flate2::read::ZlibDecoder::new(b.as_slice());
                    let mut s = String::new();
                    d.read_to_string(&mut s)?;
                    Some(s)
                } else {
                    None
                };
                Ok(json!({"raw":raw}))
            }
            Request::SavedFilters => store::saved_filters(&c),
            Request::SaveFilter { name, filter } => {
                let _l = self.writer.lock().unwrap();
                store::save_filter(&c, &name, &filter)?;
                Ok(json!({"saved":true}))
            }
            Request::DeleteFilter { id } => {
                let _l = self.writer.lock().unwrap();
                c.execute("DELETE FROM saved_filters WHERE id=?1", [id])?;
                Ok(json!({"deleted":true}))
            }
            Request::Backup { path } => {
                let _l = self.writer.lock().unwrap();
                store::backup(&c, Path::new(&path))?;
                Ok(json!({"path":path}))
            }
            Request::Restore { path } => {
                anyhow::ensure!(
                    !self.active.swap(true, Ordering::SeqCst),
                    "請先完成或取消匯入"
                );
                self.equity_pause.store(true, Ordering::Relaxed);
                let result = (|| -> Result<Value> {
                    let _l = self.writer.lock().unwrap();
                    self.equity_generation.fetch_add(1, Ordering::SeqCst);
                    let safety = self.path.with_file_name(format!(
                        "before-restore-{}.db",
                        chrono::Utc::now().timestamp_micros()
                    ));
                    store::backup(&c, &safety)?;
                    store::restore(&mut c, Path::new(&path))?;
                    for mut job in store::jobs(&c)? {
                        if job.state == "running" {
                            job.state = "interrupted".into();
                            job.message = Some("備份包含未完成匯入；可續匯。".into());
                            store::save_job(&c, &job)?;
                        }
                    }
                    *self.current.lock().unwrap() = None;
                    *self.equity_error.lock().unwrap() = None;
                    Ok(json!({"restored":true,"safety_backup":safety.to_string_lossy()}))
                })();
                self.active.store(false, Ordering::SeqCst);
                result
            }
            Request::Export {
                path,
                format,
                filter,
                selected,
            } => Ok(
                json!({"hands":store::export(&c,&filter,Path::new(&path),&format,&selected)?,"path":path}),
            ),
            Request::EquityControl { paused } => {
                self.equity_pause.store(paused, Ordering::Relaxed);
                if !paused {
                    self.start_equity();
                }
                Ok(json!({"paused":paused}))
            }
            Request::Rebuild => {
                self.rebuild()?;
                Ok(json!({"complete":true}))
            }
            Request::Health => Ok(
                json!({"version":"0.1.0","parser":PARSER_VERSION,"stats":STATS_VERSION,"database":self.path.to_string_lossy(),"import_active":self.active.load(Ordering::Relaxed),"equity_running":self.equity_running.load(Ordering::Relaxed),"equity_paused":self.equity_pause.load(Ordering::Relaxed),"equity_error":self.equity_error.lock().unwrap().clone(),"schema":2,"offline":true}),
            ),
        }
    }
    fn start(self: &Arc<Self>, job: Job) -> Result<()> {
        if self.active.swap(true, Ordering::SeqCst) {
            return Err(anyhow!("已有匯入工作正在執行"));
        }
        self.cancel.store(false, Ordering::Relaxed);
        let setup = (|| -> Result<()> {
            let _l = self.writer.lock().unwrap();
            let c = store::open(&self.path)?;
            store::save_profile(&c, &job.profile)?;
            store::save_job(&c, &job)?;
            Ok(())
        })();
        if let Err(e) = setup {
            self.active.store(false, Ordering::SeqCst);
            return Err(e);
        }
        *self.current.lock().unwrap() = Some(job.clone());
        let service = self.clone();
        std::thread::spawn(move || {
            let mut job = job;
            let start = Instant::now();
            let previous_elapsed = job.elapsed_ms;
            let result = service.run_import(&mut job);
            if let Ok(c) = store::open(&service.path) {
                let _l = service.writer.lock().unwrap();
                let _ = c.execute_batch("PRAGMA optimize=0x10002");
            }
            job.elapsed_ms = previous_elapsed + start.elapsed().as_millis() as u64;
            job.state = if let Err(e) = result {
                job.message = Some(format!("{e:#}"));
                "failed"
            } else if service.cancel.load(Ordering::Relaxed) {
                "cancelled"
            } else {
                "complete"
            }
            .into();
            if let Ok(c) = store::open(&service.path) {
                let _l = service.writer.lock().unwrap();
                let _ = store::save_job(&c, &job);
            }
            *service.current.lock().unwrap() = Some(job);
            service.active.store(false, Ordering::SeqCst);
            service.start_equity();
        });
        Ok(())
    }
    fn run_import(&self, job: &mut Job) -> Result<()> {
        let files = sources(&job.paths)?;
        job.files_total = files.len() as u64;
        let mut c = store::open(&self.path)?;
        let mut batch = Vec::with_capacity(250);
        for source in files.into_iter().skip(job.files_done as usize) {
            if self.cancel.load(Ordering::Relaxed) {
                break;
            }
            job.current_file = source.label();
            *self.current.lock().unwrap() = Some(job.clone());
            let mut consume = |raw: &str| -> Result<bool> {
                if self.cancel.load(Ordering::Relaxed) {
                    return Ok(false);
                }
                job.scanned += 1;
                match parser::parse(raw, &job.profile) {
                    Ok(h) => batch.push(h),
                    Err(e) => {
                        let _l = self.writer.lock().unwrap();
                        store::reject(&c, &job.id, &job.current_file, raw, &format!("{e:#}"))?;
                        job.quarantined += 1;
                    }
                }
                if batch.len() >= 250 {
                    self.flush(&mut c, job, &mut batch)?;
                }
                Ok(true)
            };
            match source {
                Source::Text(path) => stream(BufReader::new(File::open(path)?), &mut consume)?,
                Source::Zip(path, index) => {
                    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
                    let entry = archive.by_index(index)?;
                    anyhow::ensure!(
                        entry.size() <= 20_000_000_000,
                        "archive entry exceeds 20 GB limit"
                    );
                    stream(BufReader::new(entry), &mut consume)?;
                }
            }
            self.flush(&mut c, job, &mut batch)?;
            if !self.cancel.load(Ordering::Relaxed) {
                job.files_done += 1;
            }
            {
                let _l = self.writer.lock().unwrap();
                store::save_job(&c, job)?;
            }
            *self.current.lock().unwrap() = Some(job.clone());
        }
        Ok(())
    }
    fn flush(
        &self,
        c: &mut rusqlite::Connection,
        job: &mut Job,
        batch: &mut Vec<Hand>,
    ) -> Result<()> {
        if !batch.is_empty() {
            let _l = self.writer.lock().unwrap();
            let result = store::insert_batch(c, &job.id, &job.current_file, batch)?;
            job.inserted += result.inserted;
            job.duplicates += result.duplicates;
            job.quarantined += result.quarantined;
            job.conflicts += result.conflicts;
            store::save_job(c, job)?;
            batch.clear();
        }
        *self.current.lock().unwrap() = Some(job.clone());
        Ok(())
    }
    pub fn start_equity(self: &Arc<Self>) {
        if self.equity_pause.load(Ordering::Relaxed)
            || self.equity_running.swap(true, Ordering::SeqCst)
        {
            return;
        }
        *self.equity_error.lock().unwrap() = None;
        let service = self.clone();
        std::thread::spawn(move || {
            let result = service.run_equity();
            if let Err(e) = &result {
                *service.equity_error.lock().unwrap() = Some(format!("{e:#}"));
                service.equity_pause.store(true, Ordering::Relaxed);
            }
            service.equity_running.store(false, Ordering::SeqCst);
            // Import/resume may have requested work while this worker was leaving.
            if result.is_ok() && !service.equity_pause.load(Ordering::Relaxed) {
                let pending = store::open(&service.path).and_then(|c| {
                    Ok(c.query_row(
                        "SELECT EXISTS(SELECT 1 FROM hands WHERE ev_status='pending')",
                        [],
                        |r| r.get::<_, bool>(0),
                    )?)
                });
                if matches!(pending, Ok(true)) {
                    service.start_equity();
                }
            }
        });
    }
    fn run_equity(&self) -> Result<()> {
        let c = store::open(&self.path)?;
        loop {
            if self.equity_pause.load(Ordering::Relaxed) {
                break;
            }
            let (id, hand, generation) = {
                let _l = self.writer.lock().unwrap();
                let id = match c.query_row(
                    "SELECT id FROM hands WHERE ev_status='pending' ORDER BY id LIMIT 1",
                    [],
                    |r| r.get::<_, i64>(0),
                ) {
                    Ok(id) => id,
                    Err(rusqlite::Error::QueryReturnedNoRows) => break,
                    Err(e) => return Err(e.into()),
                };
                (
                    id,
                    store::get_hand(&c, id)?,
                    self.equity_generation.load(Ordering::SeqCst),
                )
            };
            let input = hand
                .equity_input
                .context("pending hand has no equity input; rebuild from source")?;
            let key = format!(
                "exact-hu-v1:{}",
                store::fingerprint(&serde_json::to_string(&input)?)
            );
            let cached = c
                .query_row(
                    "SELECT equity,adjusted_net FROM equity_results WHERE key=?1",
                    [&key],
                    |r| Ok((r.get::<_, f64>(0)?, crate::money::format(r.get(1)?))),
                )
                .optional()?;
            let calculated = if cached.is_some() {
                cached
            } else {
                equity::exact(&input, &self.equity_pause)?
            };
            if let Some((value, adjusted)) = calculated {
                let _l = self.writer.lock().unwrap();
                if !self.equity_pause.load(Ordering::Relaxed)
                    && generation == self.equity_generation.load(Ordering::SeqCst)
                {
                    c.execute(
                        "INSERT OR IGNORE INTO equity_results VALUES(?1,?2,?3)",
                        params![key, value, crate::money::parse(&adjusted)?],
                    )?;
                    c.execute("UPDATE hands SET ev_status='complete',equity=?1,adjusted_net=?2 WHERE id=?3 AND ev_status='pending'",params![value,crate::money::parse(&adjusted)?,id])?;
                }
            }
        }
        Ok(())
    }
    fn rebuild(self: &Arc<Self>) -> Result<()> {
        if self.active.swap(true, Ordering::SeqCst) {
            return Err(anyhow!("匯入中"));
        }
        self.equity_pause.store(true, Ordering::Relaxed);
        let result = (|| -> Result<()> {
            let _l = self.writer.lock().unwrap();
            self.equity_generation.fetch_add(1, Ordering::SeqCst);
            let mut c = store::open(&self.path)?;
            let backup = self.path.with_file_name(format!(
                "before-rebuild-{}.db",
                chrono::Utc::now().timestamp_micros()
            ));
            store::backup(&c, &backup)?;
            // Reparse into a separate database; swap only after every stored source is processed.
            let staging = self.path.with_file_name(format!(
                "rebuild-{}.db",
                chrono::Utc::now().timestamp_micros()
            ));
            store::init(&staging)?;
            let mut next = store::open(&staging)?;
            let profiles = store::profiles(&c)?;
            for p in &profiles {
                store::save_profile(&next, p)?;
            }
            let mut last = 0;
            loop {
                let ids = {
                    let mut s =
                        c.prepare("SELECT id FROM hands WHERE id>?1 ORDER BY id LIMIT 250")?;
                    let v = s
                        .query_map([last], |r| r.get::<_, i64>(0))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    v
                };
                if ids.is_empty() {
                    break;
                }
                for id in ids {
                    let old = store::get_hand(&c, id)?;
                    let profile = profiles
                        .iter()
                        .find(|p| p.id == old.profile)
                        .context("missing profile")?;
                    let h = parser::parse(&old.raw, profile)?;
                    let source: String =
                        c.query_row("SELECT source_file FROM hands WHERE id=?1", [id], |r| {
                            r.get(0)
                        })?;
                    store::insert_batch(&mut next, "rebuild", &source, &[h])?;
                    let new_id: i64 = next.query_row(
                        "SELECT id FROM hands WHERE profile=?1 AND hand_id=?2",
                        params![old.profile, old.id],
                        |r| r.get(0),
                    )?;
                    store::save_annotation(&mut next, new_id, &store::annotation(&c, id)?)?;
                    last = id;
                }
            }
            for j in store::jobs(&c)? {
                store::save_job(&next, &j)?;
            }
            // Preserve rejected/conflicting source text that is not represented by a stored hand.
            {
                let mut s=c.prepare("SELECT job,source_file,hand_id,code,message,raw FROM import_issues WHERE raw IS NOT NULL")?;
                let mut rows = s.query([])?;
                while let Some(r) = rows.next()? {
                    next.execute("INSERT INTO import_issues(job,source_file,hand_id,code,message,raw) VALUES(?1,?2,?3,?4,?5,?6)",params![r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,Vec<u8>>(5)?])?;
                }
            }
            let saved = store::saved_filters(&c)?;
            for row in saved.as_array().unwrap() {
                store::save_filter(
                    &next,
                    row["name"].as_str().unwrap(),
                    &serde_json::from_value(row["filter"].clone())?,
                )?;
            }
            next.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
            drop(next);
            store::restore(&mut c, &staging)?;
            std::fs::remove_file(staging)?;
            Ok(())
        })();
        self.active.store(false, Ordering::SeqCst);
        self.equity_pause.store(false, Ordering::Relaxed);
        self.start_equity();
        result
    }
}

#[derive(Clone)]
enum Source {
    Text(PathBuf),
    Zip(PathBuf, usize),
}
impl Source {
    fn label(&self) -> String {
        match self {
            Self::Text(p) => p.to_string_lossy().into(),
            Self::Zip(p, i) => format!("{}::entry:{i}", p.to_string_lossy()),
        }
    }
}
fn sources(paths: &[String]) -> Result<Vec<Source>> {
    let mut files = vec![];
    for path in paths {
        let path = Path::new(path);
        if path.is_dir() {
            for e in WalkDir::new(path).follow_links(false).sort_by_file_name() {
                let e = e?;
                if e.file_type().is_file() {
                    files.push(e.path().to_owned());
                }
            }
        } else {
            files.push(path.to_owned());
        }
    }
    files.sort();
    files.dedup();
    let mut out = vec![];
    for p in files {
        match p
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "txt" => out.push(Source::Text(p)),
            "zip" => {
                let mut z = zip::ZipArchive::new(File::open(&p)?)?;
                anyhow::ensure!(z.len() <= 100_000, "too many archive entries");
                for i in 0..z.len() {
                    let e = z.by_index(i)?;
                    if !e.is_dir() && e.name().to_ascii_lowercase().ends_with(".txt") {
                        out.push(Source::Zip(p.clone(), i));
                    }
                }
            }
            _ => {}
        }
    }
    anyhow::ensure!(!out.is_empty(), "沒有可匯入嘅 TXT／ZIP 牌譜");
    Ok(out)
}
fn stream(reader: impl BufRead, consume: &mut impl FnMut(&str) -> Result<bool>) -> Result<()> {
    let mut hand = String::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim_start_matches('\u{feff}');
        if line.starts_with("Poker Hand #") && !hand.trim().is_empty() {
            if !consume(&hand)? {
                return Ok(());
            }
            hand.clear();
        }
        if hand.len() + line.len() > 1_048_576 {
            return Err(anyhow!("single hand exceeds 1 MiB; source may be invalid"));
        }
        hand.push_str(line);
        hand.push('\n');
    }
    if !hand.trim().is_empty() {
        consume(&hand)?;
    }
    Ok(())
}
