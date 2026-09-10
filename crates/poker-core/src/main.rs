use anyhow::{Context, Result};
use poker_core::{
    fixtures,
    model::*,
    parser,
    service::{Request, Service},
    store,
};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let command = a.get(1).map(String::as_str).unwrap_or("help");
    let path = PathBuf::from(
        a.get(2)
            .map(String::as_str)
            .unwrap_or(".local/riverlens.db"),
    );
    match command {
        "serve"=>{
            let service=Service::new(path)?;service.start_equity();
            let port=a.get(3).map(String::as_str).unwrap_or("4789");
            let server=tiny_http::Server::http(format!("127.0.0.1:{port}")).map_err(|e|anyhow::anyhow!(e.to_string()))?;
            println!("RiverLens local core at http://127.0.0.1:{port}");
            for mut req in server.incoming_requests(){let service=service.clone();std::thread::spawn(move||{
                let origin=req.headers().iter().find(|h|h.field.equiv("Origin")).map(|h|h.value.as_str());
                let allowed=origin.is_none_or(|s|matches!(s,"http://127.0.0.1:1420"|"http://localhost:1420"));
                let json_type=req.headers().iter().any(|h|h.field.equiv("Content-Type")&&h.value.as_str().starts_with("application/json"));
                if !allowed||req.method()!=&tiny_http::Method::Post||req.url()!="/api"||!json_type{let _=req.respond(tiny_http::Response::from_string("Forbidden").with_status_code(403));return;}
                let result=(||->Result<serde_json::Value>{use std::io::Read;let mut bytes=vec![];req.as_reader().take(1_048_577).read_to_end(&mut bytes)?;anyhow::ensure!(bytes.len()<=1_048_576,"request too large");let request:Request=serde_json::from_slice(&bytes)?;service.handle(request)})();
                let (status,value)=match result{Ok(v)=>(200,json!({"result":v})),Err(e)=>(400,json!({"error":format!("{e:#}")}))};
                let response=tiny_http::Response::from_string(value.to_string()).with_status_code(status).with_header(tiny_http::Header::from_bytes("Content-Type","application/json").unwrap());let _=req.respond(response);
            });}
        },
        "import"=>{
            let service=Service::new(path)?;service.handle(Request::EquityControl{paused:true})?;
            let paths=a.get(3..).context("usage: poker-core import DATABASE PATH...")?.to_vec();
            service.handle(Request::StartImport{paths,profile:Profile::default()})?;
            loop{std::thread::sleep(Duration::from_millis(100));let health=service.handle(Request::Health)?;if health["import_active"]==false{break;}}
            println!("{}",serde_json::to_string_pretty(&service.handle(Request::Jobs)?)?);
            println!("{}",serde_json::to_string_pretty(&service.handle(Request::Overview{filter:Filter::default(),group:Some("stakes".into())})?)?);
        },
        "report"=>{let service=Service::new(path)?;println!("{}",serde_json::to_string_pretty(&service.handle(Request::Overview{filter:Filter::default(),group:Some("stakes".into())})?)?);},
        "equity"=>{let service=Service::new(path)?;service.start_equity();loop{std::thread::sleep(Duration::from_millis(250));if service.handle(Request::Health)?["equity_running"]==false{break;}}println!("{}",service.handle(Request::Overview{filter:Filter::default(),group:None})?);},
        "bench"=>{let count:u64=a.get(3).context("usage: poker-core bench NEW_DATABASE HAND_COUNT")?.parse()?;benchmark(&path,count)?;},
        "bench-queries"=>{benchmark_queries(&path)?;},
        "generate"=>{let count:u64=a.get(3).context("usage: poker-core generate NEW_TXT HAND_COUNT")?.parse()?;use std::io::Write;let f=std::fs::OpenOptions::new().create_new(true).write(true).open(path)?;let mut out=std::io::BufWriter::new(f);for i in 0..count{writeln!(out,"{}\n",fixtures::cash_hand(i))?;}},
        _=>println!("RiverLens core\n  import DB PATH...\n  report DB\n  equity DB\n  serve DB [PORT]\n  bench NEW_DB COUNT\n  bench-queries DB\n  generate NEW_TXT COUNT"),
    }
    Ok(())
}
fn benchmark(path: &Path, count: u64) -> Result<()> {
    anyhow::ensure!(
        !path.exists() && count > 0,
        "benchmark needs new database and positive hand count"
    );
    store::init(path)?;
    let mut c = store::open(path)?;
    let profile = Profile {
        id: "synthetic-benchmark".into(),
        name: "Synthetic benchmark".into(),
        ..Profile::default()
    };
    store::save_profile(&c, &profile)?;
    let start = Instant::now();
    let mut batch = vec![];
    let mut written = 0u64;
    for i in 0..count {
        let hand = parser::parse(&fixtures::cash_hand(i), &profile)?;
        anyhow::ensure!(
            hand.status == "valid",
            "invalid generated hand {i}: {:?}",
            hand.issues
        );
        batch.push(hand);
        if batch.len() == 1000 || i + 1 == count {
            store::insert_batch(&mut c, "benchmark", "synthetic.txt", &batch)?;
            written += batch.len() as u64;
            batch.clear();
        }
        if written > 0 && written.is_multiple_of(100_000) && (i + 1) % 1000 == 0 {
            eprintln!(
                "benchmark: {written}/{count} hands; {:.1}s",
                start.elapsed().as_secs_f64()
            );
        }
    }
    let import_seconds = start.elapsed().as_secs_f64();
    let mut timings = vec![];
    for _ in 0..20 {
        let t = Instant::now();
        let r = store::report(&c, &Filter::default(), "date")?;
        anyhow::ensure!(r["hands"].as_u64() == Some(count), "count mismatch");
        timings.push(t.elapsed().as_secs_f64() * 1000.);
    }
    timings.sort_by(f64::total_cmp);
    let mut replay = vec![];
    for i in 1..=100 {
        let id = 1 + ((i * 7919) % count);
        let t = Instant::now();
        store::get_hand(&c, id as i64)?;
        replay.push(t.elapsed().as_secs_f64() * 1000.);
    }
    replay.sort_by(f64::total_cmp);
    c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"hands":count,"import_seconds":import_seconds,"hands_per_second":count as f64/import_seconds,"report_p95_ms":timings[18],"replayer_p95_ms":replay[94],"db_bytes":std::fs::metadata(path)?.len(),"peak_rss_bytes":peak_rss(),"report_samples_ms":timings,"workload":"unique synthetic six-seat cash/Rush hands, six Hero positions, exact decimals; excludes equity work","parser":PARSER_VERSION})
        )?
    );
    Ok(())
}

fn benchmark_queries(path: &Path) -> Result<()> {
    anyhow::ensure!(
        path.exists(),
        "query benchmark requires an existing database"
    );
    store::init(path)?;
    let c = store::open(path)?;
    c.execute_batch("PRAGMA optimize=0x10002")?;
    let mut results = vec![];
    let mut workloads = vec![];
    for group in [
        "date",
        "position",
        "stakes",
        "game",
        "hand_class",
        "session",
    ] {
        workloads.push((format!("all/{group}"), Filter::default(), group));
    }
    workloads.push((
        "position/BTN".into(),
        Filter {
            position: Some("BTN".into()),
            ..Filter::default()
        },
        "date",
    ));
    workloads.push((
        "stat/VPIP-opportunities".into(),
        Filter {
            stat: Some("vpip".into()),
            stat_mode: Some("opportunities".into()),
            ..Filter::default()
        },
        "date",
    ));
    let date: String = c.query_row(
        "SELECT MIN(value) FROM rollups WHERE dimension='date'",
        [],
        |r| r.get(0),
    )?;
    workloads.push((
        "date/source-timezone".into(),
        Filter {
            date_from: Some(date.clone()),
            ..Filter::default()
        },
        "date",
    ));
    workloads.push((
        "combined/date-BTN-Cash".into(),
        Filter {
            date_from: Some(date),
            position: Some("BTN".into()),
            game: Some("Cash".into()),
            ..Filter::default()
        },
        "stakes",
    ));
    for (name, filter, group) in workloads {
        let mut samples = vec![];
        let mut hands = 0;
        for _ in 0..20 {
            let start = Instant::now();
            let report = store::report(&c, &filter, group)?;
            samples.push(start.elapsed().as_secs_f64() * 1000.);
            hands = report["hands"].as_u64().unwrap_or(0);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!("{name}: p95 {:.3}ms", samples[18]);
        results.push(json!({"query":name,"hands":hands,"p95_ms":samples[18],"samples_ms":samples}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"results":results,"peak_rss_bytes":peak_rss(),"scope":"Rust SQLite report queries; 20 repetitions each, process-reopened, OS file cache not flushed; excludes IPC and UI paint"})
        )?
    );
    Ok(())
}

fn peak_rss() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // getrusage initializes the supplied struct on success; Darwin reports bytes, Linux KiB.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } == 0 {
            let rss = unsafe { usage.assume_init() }.ru_maxrss as u64;
            return Some(if cfg!(target_os = "macos") {
                rss
            } else {
                rss * 1024
            });
        }
    }
    None
}
