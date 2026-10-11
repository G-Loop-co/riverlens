use poker_core::{
    fixtures,
    mda::{
        facts,
        storage::{self, DatasetInfo, Incoming, Job},
        tools, Manager, Request,
    },
    parser,
};
use serde_json::json;
use std::io::Write;
use std::sync::atomic::AtomicBool;
fn info() -> DatasetInfo {
    DatasetInfo {
        id: "mda-test".into(),
        name: "test".into(),
        network: "GG".into(),
        pool: "synthetic".into(),
        format: "GG native NLH cash".into(),
        timezone: "UTC".into(),
        currency: "USD".into(),
        parser_version: poker_core::model::PARSER_VERSION.into(),
        stats_version: facts::VERSION.into(),
        schema: storage::SCHEMA,
        cloud_enabled: false,
        mcp_enabled: false,
        created_at: 0,
    }
}
fn row(index: u64) -> Incoming {
    let mut hand = parser::parse_observed(&fixtures::population_hand(index), "UTC", "GG").unwrap();
    assert_eq!(hand.status, "valid", "{:?}", hand.issues);
    let decisions = facts::derive(&mut hand).unwrap();
    Incoming { hand, decisions }
}
#[test]
fn population_all_players_no_hero_and_before_decision() {
    let r = row(2);
    assert!(serde_json::to_value(&r.hand)
        .unwrap()
        .get("hero_seat")
        .is_none());
    assert!(r.hand.players.iter().all(|p| !p.hero && p.cards.is_empty()));
    let flop: Vec<_> = r.decisions.iter().filter(|d| d.street == "flop").collect();
    assert!(flop.iter().all(|d| d.board.split_whitespace().count() == 3));
    assert!(r
        .decisions
        .iter()
        .filter(|d| d.street == "preflop")
        .all(|d| d.board.is_empty()));
    let rfi = facts::bit("rfi").unwrap();
    assert_eq!(
        r.decisions
            .iter()
            .filter(|d| d.opportunities & rfi != 0)
            .count(),
        1
    );
    let response = flop
        .iter()
        .find(|d| d.opportunities & facts::bit("call_cbet").unwrap() != 0)
        .unwrap();
    assert!((response.facing_bet_pct.unwrap() - 50.).abs() < 1e-6);
    assert!(response.effective_bb < 200.);
    assert_eq!(response.players, 2);
    assert!(r.decisions.iter().any(|d| d.street == "river"
        && d.opportunities & facts::bit("bet_checked_through").unwrap() != 0));
    assert!(!row(0)
        .decisions
        .iter()
        .any(|d| d.position == "BB" && d.opportunities & rfi != 0));
    for index in 0..256 {
        row(index);
    }
    let partial = fixtures::population_hand(2).replace(
        "*** HOLE CARDS ***",
        &format!(
            "*** HOLE CARDS ***\nDealt to {} [Ah Ad]",
            r.hand.players[0].name
        ),
    );
    let observed = parser::parse_observed(&partial, "UTC", "GG").unwrap();
    assert_eq!(observed.players.len(), 6);
    let replay = observed.replay();
    assert_eq!(replay.flop_players, 2);
    assert!(replay.saw_flop);
    let r = row(7);
    assert!(r
        .decisions
        .iter()
        .filter(|d| r
            .hand
            .actions
            .iter()
            .any(|a| a.actor == Some(d.seat) && a.kind == "fold"))
        .all(|d| !d.showdown));
    let mut h = row(6).hand;
    let call = h
        .actions
        .iter()
        .find(|a| a.street == "flop" && a.actor == Some(1) && a.kind == "call")
        .unwrap()
        .seq;
    h.actions[call].kind = "fold".into();
    h.actions[call].amount = 0;
    h.actions.retain(|a| a.seq <= call || a.actor != Some(1));
    let d = facts::derive(&mut h).unwrap();
    assert!(d
        .iter()
        .filter(|d| d.street == "flop")
        .all(|d| d.players == 3));
    assert!(d
        .iter()
        .filter(|d| d.street == "turn")
        .all(|d| d.players == 2));
}

#[test]
fn short_allin_does_not_reopen_and_illegal_actions_are_excluded() {
    let raw="Poker Hand #SHORT: Hold'em No Limit ($0.01/$0.02) - 2026/09/01 00:00:00\nTable 'Regular' 6-max Seat #6 is the button\nSeat 1: A ($2 in chips)\nSeat 2: B ($0.08 in chips)\nSeat 3: C ($2 in chips)\nSeat 4: D ($2 in chips)\nSeat 5: E ($2 in chips)\nSeat 6: F ($2 in chips)\nA: posts small blind $0.01\nB: posts big blind $0.02\n*** HOLE CARDS ***\nC: raises $0.04 to $0.06\nD: folds\nE: folds\nF: folds\nA: folds\nB: raises $0.02 to $0.08 and is all-in\nC: calls $0.02\n*** FLOP *** [Ks 7d 2c]\n*** TURN *** [Ks 7d 2c] [3h]\n*** RIVER *** [Ks 7d 2c 3h] [9c]\n*** SHOWDOWN ***\nC collected $0.17 from pot\n*** SUMMARY ***\nTotal pot $0.17 | Rake $0\n";
    let mut h = parser::parse_observed(raw, "UTC", "GG").unwrap();
    assert_eq!(h.status, "valid");
    let d = facts::derive(&mut h).unwrap();
    let last = d.last().unwrap();
    assert_eq!(last.action, "call");
    assert!(last.opportunities & facts::bit("call_three_bet").unwrap() != 0);
    assert!(d.iter().all(|d| d.street == "preflop"));
    let illegal = raw
        .replace("C: calls $0.02", "C: raises $0.08 to $0.16")
        .replace("Total pot $0.17", "Total pot $0.25")
        .replace("collected $0.17", "collected $0.25");
    let mut h = parser::parse_observed(&illegal, "UTC", "GG").unwrap();
    assert!(facts::derive(&mut h)
        .unwrap_err()
        .to_string()
        .contains("not reopened"));
}
#[test]
fn atomic_dedup_conflict_incremental_and_queries() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = storage::open(&dir.path().join("mda.duckdb")).unwrap();
    storage::init(&c, &info()).unwrap();
    let mut j = Job {
        id: "job".into(),
        ..Job::default()
    };
    storage::commit_batch(
        &mut c,
        vec![row(1), row(2), row(1)],
        vec![],
        &mut j,
        3,
        "one",
    )
    .unwrap();
    assert_eq!(j.inserted, 2);
    assert_eq!(j.duplicates, 1);
    let mut changed = row(1);
    changed.hand.raw.push_str("\nchanged");
    storage::commit_batch(&mut c, vec![changed, row(3)], vec![], &mut j, 5, "two").unwrap();
    assert_eq!(j.inserted, 3);
    assert_eq!(j.conflicts, 1);
    assert_eq!(j.quarantined, 1);
    assert_eq!(storage::jobs(&c).unwrap()[0].committed_hands, 5);
    let a=tools::validate("query_mda_stats",&json!({"dataset_id":"mda-test","version":storage::version(&c).unwrap(),"filter":{"metric":"rfi"},"group":"position"})).unwrap();
    let before = tools::stats(&c, &a, false).unwrap();
    storage::compact_rollups(&mut c).unwrap();
    assert_eq!(before, tools::stats(&c, &a, false).unwrap());
    assert_eq!(before["rows"][0]["opportunities"], 3);
    assert_eq!(before["rows"][0]["numerator"], 3);
    let key = storage::hex_key(storage::key("GG", &row(2).hand.id));
    let a=tools::validate("get_mda_hand",&json!({"dataset_id":"mda-test","version":storage::version(&c).unwrap(),"hand_key":key,"seq":row(2).decisions.iter().find(|d|d.street=="flop").unwrap().seq})).unwrap();
    let d = tools::query(&c, "get_mda_hand", &a).unwrap();
    assert_eq!(d["hand"]["boards"][0].as_array().unwrap().len(), 3);
    assert_eq!(d["hand"]["total_pot"], "0.00");
    assert!(d.to_string().find("Player").is_none());
    assert!(d["hand"]["players"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["cards"].as_array().unwrap().is_empty()));
}
#[test]
fn streaming_zip_resume_backup_and_sharing_version() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::new(dir.path().join("datasets")).unwrap();
    let catalog = manager
        .handle(Request::Create {
            name: "pool".into(),
            timezone: "UTC".into(),
            source: "test".into(),
        })
        .unwrap();
    let id = catalog["dataset"]["id"].as_str().unwrap().to_string();
    let ds = manager.dataset(&id).unwrap();
    let mut c = ds.connection().unwrap();
    let path = dir.path().join("data.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (member, count) in [("01-population.txt", 20), ("02-duplicates.txt", 5)] {
        zip.start_file(member, options).unwrap();
        for index in 0..count {
            zip.write_all(fixtures::population_hand(index).as_bytes())
                .unwrap();
        }
    }
    zip.finish().unwrap();
    let m = path.metadata().unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let sources = (0..zip.len())
        .map(|index| {
            let member = zip.by_index(index).unwrap();
            storage::Source {
                path: path.to_string_lossy().into_owned(),
                member: Some(member.name().into()),
                size: m.len(),
                modified: m
                    .modified()
                    .unwrap()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                crc: Some(member.crc32()),
            }
        })
        .collect();
    drop(zip);
    let mut job = Job {
        id: "testjob".into(),
        sources,
        state: "running".into(),
        ..Job::default()
    };
    // Already committed prefix simulates process interruption mid-file, not only file completion.
    storage::commit_batch(
        &mut c,
        (0..7).map(row).collect(),
        vec![],
        &mut job,
        7,
        "prefix",
    )
    .unwrap();
    poker_core::mda::engine::import(&mut c, &AtomicBool::new(false), &mut job, |_| {}).unwrap();
    assert_eq!(job.inserted, 20);
    assert_eq!(job.scanned, 25);
    assert_eq!(job.duplicates, 5);
    assert_eq!(job.state, "completed");
    let old = manager.version(&id).unwrap();
    storage::commit_batch(&mut c, vec![row(21)], vec![], &mut job, 21, "increment").unwrap();
    assert!(manager
        .query("query_mda_stats", json!({"dataset_id":id,"version":old}))
        .unwrap_err()
        .to_string()
        .contains("dataset_changed"));
    assert!(manager.permitted(&id, true).is_err());
    manager
        .handle(Request::Share {
            dataset_id: id.clone(),
            cloud_enabled: true,
            mcp_enabled: false,
        })
        .unwrap();
    assert!(manager.permitted(&id, true).is_ok());
    assert!(manager.permitted(&id, false).is_err());
    let backup = dir.path().join("backup.duckdb");
    manager
        .handle(Request::Backup {
            dataset_id: id.clone(),
            path: backup.to_string_lossy().into_owned(),
        })
        .unwrap();
    let restored = manager
        .handle(Request::Restore {
            path: backup.to_string_lossy().into_owned(),
        })
        .unwrap();
    assert_eq!(restored["hands"], 21);
    assert_eq!(restored["dataset"]["cloud_enabled"], false);
    assert_ne!(restored["version"], manager.version(&id).unwrap());
}
