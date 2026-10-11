//! Reproducible end-to-end ZIP import; no real player data or provider traffic.
use anyhow::{ensure, Result};
use poker_core::{
    fixtures,
    mda::{engine, storage, tools, Manager, Request},
};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Instant, UNIX_EPOCH},
};
fn rss() -> u64 {
    #[cfg(unix)]
    {
        let mut r = std::mem::MaybeUninit::<libc::rusage>::uninit();
        unsafe {
            libc::getrusage(libc::RUSAGE_SELF, r.as_mut_ptr());
            let v = r.assume_init().ru_maxrss as u64;
            if cfg!(target_os = "macos") {
                v
            } else {
                v * 1024
            }
        }
    }
    #[cfg(not(unix))]
    {
        0
    }
}
fn timed(c: &duckdb::Connection, a: &tools::Args) -> Result<Value> {
    let mut samples = Vec::new();
    for _ in 0..21 {
        let t = Instant::now();
        let v = tools::stats(c, a, false)?;
        ensure!(
            !v["rows"].as_array().unwrap().is_empty(),
            "benchmark query must match data"
        );
        samples.push(t.elapsed().as_secs_f64() * 1000.);
    }
    samples.sort_by(f64::total_cmp);
    Ok(json!({"runs":samples.len(),"p50_ms":samples[10],"p95_ms":samples[19],"max_ms":samples[20]}))
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let count = args
        .get(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(10_000_000u64);
    let root = PathBuf::from(
        args.get(2)
            .context("usage: mda-bench COUNT ROOT OUTPUT.json")?,
    );
    let output = PathBuf::from(args.get(3).context("output path required")?);
    ensure!(
        !root.exists(),
        "benchmark root must be fresh; preserve prior evidence"
    );
    fs::create_dir_all(&root)?;
    let generation = Instant::now();
    let zip_path = root.join("synthetic-gg-regular.zip");
    let reuse = std::env::var_os("RIVERLENS_MDA_BENCH_INPUT").map(PathBuf::from);
    if let Some(input) = &reuse {
        ensure!(input.is_file(), "benchmark input missing");
        if fs::hard_link(input, &zip_path).is_err() {
            fs::copy(input, &zip_path)?;
        }
    } else {
        let mut zip = zip::ZipWriter::new(BufWriter::new(File::create(&zip_path)?));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1))
            .large_file(true);
        zip.start_file("population.txt", options)?;
        for index in 0..count {
            zip.write_all(fixtures::population_hand(index).as_bytes())?;
        }
        zip.finish()?.flush()?;
    }
    let generation_secs = generation.elapsed().as_secs_f64();
    let manager = Manager::new(root.join("datasets"))?;
    let catalog = manager.handle(Request::Create {
        name: "10m synthetic acceptance".into(),
        timezone: "UTC".into(),
        source: "reproducible synthetic GG Regular; not supplier acceptance".into(),
    })?;
    let id = catalog["dataset"]["id"].as_str().unwrap();
    let ds = manager.dataset(id)?;
    let mut c = ds.connection()?;
    let metadata = zip_path.metadata()?;
    let mut archive = zip::ZipArchive::new(File::open(&zip_path)?)?;
    let crc = archive.by_index(0)?.crc32();
    drop(archive);
    let mut job = storage::Job {
        id: "benchmark".into(),
        sources: vec![storage::Source {
            path: zip_path.to_string_lossy().into_owned(),
            member: Some("population.txt".into()),
            size: metadata.len(),
            modified: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs(),
            crc: Some(crc),
        }],
        state: "running".into(),
        ..storage::Job::default()
    };
    let start = Instant::now();
    let mut last = 0;
    engine::import(&mut c, &AtomicBool::new(false), &mut job, |j| {
        j.elapsed_ms = start.elapsed().as_millis() as u64;
        if j.scanned >= last + 100_000 {
            last = j.scanned;
            eprintln!(
                "{} hands / {:.1}s / peak RSS {:.0} MiB",
                j.scanned,
                start.elapsed().as_secs_f64(),
                rss() as f64 / 1048576.
            );
        }
    })?;
    let import_secs = start.elapsed().as_secs_f64();
    ensure!(
        job.inserted == count && job.quarantined == 0,
        "invalid benchmark import: {} inserted, {} quarantined",
        job.inserted,
        job.quarantined
    );
    let version = storage::version(&c)?;
    let aggregate = tools::validate(
        "query_mda_stats",
        &json!({"dataset_id":id,"version":version,"group":"position","limit":100}),
    )?;
    let contextual = tools::validate(
        "query_mda_stats",
        &json!({"dataset_id":id,"version":version,"group":"position","limit":100,"filter":{"bb_units":250000,"street":"flop","pot_type":"SRP","effective_min":80,"bet_min":30,"bet_max":65,"texture":"rainbow"}}),
    )?;
    let aggregate = timed(&c, &aggregate)?;
    let contextual = timed(&c, &contextual)?;
    c.execute_batch("CHECKPOINT;")?;
    let disk = ds.path.metadata()?.len();
    let mut hardware = json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"logical_threads":std::thread::available_parallelism()?.get()});
    #[cfg(target_os = "macos")]
    {
        for key in ["machdep.cpu.brand_string", "hw.memsize"] {
            let command = std::process::Command::new("sysctl")
                .args(["-n", key])
                .output()?;
            hardware[key] = if command.status.success() {
                json!(String::from_utf8_lossy(&command.stdout).trim())
            } else {
                Value::Null
            };
        }
    }
    for (key, var) in [
        ("machdep.cpu.brand_string", "RIVERLENS_BENCH_CPU"),
        ("hw.memsize", "RIVERLENS_BENCH_RAM_BYTES"),
    ] {
        if let Ok(v) = std::env::var(var) {
            hardware[key] = json!(v);
        }
    }
    use sha2::{Digest, Sha256};
    let source_hash = format!(
        "{:x}",
        Sha256::digest(
            [
                include_str!("../mda/engine.rs"),
                include_str!("../mda/storage.rs"),
                include_str!("../mda/facts.rs"),
                include_str!("../mda/tools.rs"),
                include_str!("../parser.rs"),
                include_str!("../model.rs"),
                include_str!("../fixtures.rs"),
                include_str!("../../Cargo.toml")
            ]
            .join("\0")
        )
    );
    let report = json!({"fixture_version":"gg-population/1","hands":count,"input":"ZIP / GG native / six seats / no Hero / source UTC","hardware":hardware,"duckdb":{"memory_limit":"2GiB","threads":4,"crate":"1.10506.0"},"generation_seconds":generation_secs,"import_seconds":import_secs,"hands_per_second":count as f64/import_secs,"input_zip_bytes":metadata.len(),"dataset_bytes":disk,"peak_process_rss_bytes":rss(),"aggregate":aggregate,"contextual":contextual,"job":job,"version":version,"acceptance":{"ten_million":count==10_000_000,"aggregate_p95_le_2s":aggregate["p95_ms"].as_f64().unwrap()<=2000.,"contextual_p95_le_5s":contextual["p95_ms"].as_f64().unwrap()<=5000.,"peak_rss_le_4gib":rss()>0&&rss()<=4*1024*1024*1024},"limits":"21 in-process queries; warm OS cache; includes parser, ZIP streaming, DuckDB appender, dedup, rollups and checkpoint. Excludes UI/IPC and actual supplier/provider acceptance."});
    let mut report = report;
    report["engine_source_sha256"] = json!(source_hash);
    report["reused_synthetic_input"] = json!(reuse.is_some());
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
use anyhow::Context;
