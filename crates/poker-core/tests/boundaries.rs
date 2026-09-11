use poker_core::{
    equity, fixtures,
    model::*,
    money, parser,
    service::{Request, Service},
    store,
};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn parse(raw: &str) -> Hand {
    parser::parse(raw, &Profile::default()).unwrap()
}
fn three(actions: &str, total: &str) -> String {
    format!("Poker Hand #THREE: Hold'em No Limit ($0.01/$0.02) - 2026/09/01 12:00:00\nTable 'Synthetic' 3-max Seat #3 is the button\nSeat 1: Small ($2 in chips)\nSeat 2: Big ($1 in chips)\nSeat 3: Hero ($3 in chips)\nSmall: posts small blind $0.01\nBig: posts big blind $0.02\n*** HOLE CARDS ***\nDealt to Hero [As Ad]\nDealt to Small\nDealt to Big\n{actions}\n*** SUMMARY ***\n{total}\n")
}
#[test]
fn missed_blind_is_dead_not_raise_credit() {
    let raw=fixtures::hu_hand("Hero: posts missed blind $0.01\nHero: raises $0.04 to $0.06\nVillain: folds\nUncalled bet ($0.04) returned to Hero\n*** SHOWDOWN ***\nHero collected $0.05 from pot","Total pot $0.05 | Rake $0 | Jackpot $0");
    let h = parse(&raw);
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.invested, 70_000);
    assert_eq!(h.net, 20_000);
}
#[test]
fn side_pots_inferred_from_contributions() {
    let h=parse(&three("Hero: raises $0.98 to $1\nSmall: raises $1 to $2 and is all-in\nBig: calls $0.98 and is all-in\nHero: calls $1\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nSmall: shows [Ks Kd]\nBig: shows [Qs Qd]\nHero collected $2.90 from main pot\nHero collected $2 from side pot","Total pot $5 | Rake $0.10 | Jackpot $0"));
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.pots.len(), 2);
    assert_eq!(h.pots[0].amount, 3_000_000);
    assert_eq!(h.pots[1].amount, 2_000_000);
    assert_eq!(h.collected, 4_900_000);
    assert_eq!(h.ev_status, "excluded");
}
#[test]
fn shared_flop_multiple_runouts_and_payouts() {
    let h=parse(&fixtures::hu_hand("Hero: raises $0.04 to $0.06\nVillain: calls $0.04\n*** FLOP *** [2c 3d 8h]\nVillain: bets $1.94 and is all-in\nHero: calls $1.94 and is all-in\n*** FIRST TURN *** [9s]\n*** FIRST RIVER *** [Tc]\n*** SECOND TURN *** [Jh]\n*** SECOND RIVER *** [Qd]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $1.95 from pot\nHero collected $1.95 from pot","Total pot $4 | Rake $0.10 | Jackpot $0\nSeat 1: Hero won ($3.90)"));
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.boards[1], vec!["2c", "3d", "8h", "Jh", "Qd"]);
    assert_eq!(h.collected, 3_900_000);
    assert_eq!(h.ev_reason.as_deref(), Some("multiple_runouts"));
}
#[test]
fn hero_folds_before_other_players_showdown() {
    let h=parse(&three("Hero: folds\nSmall: calls $0.01\nBig: checks\n*** FLOP *** [2c 3d 8h]\nSmall: checks\nBig: checks\n*** TURN *** [2c 3d 8h] [9s]\nSmall: checks\nBig: checks\n*** RIVER *** [2c 3d 8h 9s] [Tc]\nSmall: checks\nBig: checks\n*** SHOWDOWN ***\nSmall: shows [Ks Kd]\nBig: shows [Qs Qd]\nSmall collected $0.04 from pot","Total pot $0.04 | Rake $0 | Jackpot $0"));
    assert_eq!(h.status, "valid");
    assert!(!h.showdown);
    assert!(!h.saw_flop);
    assert_eq!(h.stats["wsd"].opportunities, 0);
}
#[test]
fn squeeze_has_one_three_bet_opportunity() {
    let raw=three("Hero: raises $0.04 to $0.06\nSmall: calls $0.05\nBig: raises $0.18 to $0.24\nHero: folds\nSmall: folds\nUncalled bet ($0.18) returned to Big\n*** SHOWDOWN ***\nBig collected $0.18 from pot","Total pot $0.18 | Rake $0 | Jackpot $0");
    let p = Profile {
        hero: "Big".into(),
        ..Profile::default()
    };
    let h = parser::parse(&raw.replace("Dealt to Big\n", "Dealt to Big [Kh Kd]\n"), &p).unwrap();
    assert_eq!(h.status, "valid");
    assert_eq!(h.stats["three_bet"].numerator, 1);
    assert_eq!(h.stats["three_bet"].opportunities, 1);
    assert_eq!(h.stats["rfi"].opportunities, 0);
}
#[test]
fn short_all_in_does_not_reopen_prior_limper() {
    let raw=fixtures::hu_hand("Hero: calls $0.01\nVillain: raises $0.01 to $0.03 and is all-in\nHero: calls $0.01\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $0.06 from pot","Total pot $0.06 | Rake $0 | Jackpot $0").replace("Villain ($2 in chips)","Villain ($0.03 in chips)");
    let h = parse(&raw);
    assert_eq!(h.status, "valid");
    assert_eq!(h.stats["three_bet"].opportunities, 0);
    assert_eq!(h.pot_type, "special");
}
#[test]
fn unknown_holes_excluded_and_tie_equity_exact() {
    let raw=fixtures::hu_hand("Hero: raises $1.98 to $2 and is all-in\nVillain: calls $1.98 and is all-in\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero collected $4 from pot","Total pot $4 | Rake $0 | Jackpot $0");
    assert_eq!(parse(&raw).ev_reason.as_deref(), Some("unknown_hole_cards"));
    let input = EquityInput {
        hero: vec!["As".into(), "Ad".into()],
        villain: vec!["Ks".into(), "Kd".into()],
        board: vec![
            "2c".into(),
            "3d".into(),
            "4h".into(),
            "5s".into(),
            "6c".into(),
        ],
        net_pot: 3_900_000,
        contribution: 2_000_000,
    };
    let (eq, net) = equity::exact(&input, &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    assert_eq!(eq, 0.5);
    assert_eq!(money::parse(&net).unwrap(), -50_000);
}
#[test]
fn conflicting_duplicates_and_sql_filters_are_safe() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    store::init(&p).unwrap();
    let mut c = store::open(&p).unwrap();
    let h = parse(&fixtures::cash_hand(2));
    store::insert_batch(&mut c, "a", "a", std::slice::from_ref(&h)).unwrap();
    let mut changed = h;
    changed.raw.push_str("\nsource edit");
    let r = store::insert_batch(&mut c, "b", "b", &[changed]).unwrap();
    assert_eq!(r.conflicts, 1);
    assert_eq!(store::issues(&c).unwrap()["total"], 1);
    assert!(store::issues(&c).unwrap()["rows"][0]["hand_row"].is_null());
    let f = Filter {
        stat: Some("vpip); DROP TABLE hands;--".into()),
        ..Filter::default()
    };
    assert!(store::report(&c, &f, "date").is_err());
    let f = Filter {
        search: Some("' OR 1=1--".into()),
        ..Filter::default()
    };
    assert_eq!(store::report(&c, &f, "date").unwrap()["hands"], 0);
    let profile = Profile {
        timezone: "UTC".into(),
        ..Profile::default()
    };
    assert!(store::save_profile(&c, &profile).is_err());
}

#[test]
fn issue_links_stay_with_the_importing_profile_and_all_pages_are_reachable() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    store::init(&p).unwrap();
    let mut c = store::open(&p).unwrap();
    let original = parse(&fixtures::cash_hand(2));
    store::insert_batch(&mut c, "first", "a.txt", std::slice::from_ref(&original)).unwrap();
    let mut other = original;
    other.profile = "second-profile".into();
    other.status = "quarantined".into();
    other.issues.push(Issue {
        code: "pot_mismatch".into(),
        message: "synthetic discrepancy".into(),
        line: None,
    });
    c.execute(
        "INSERT INTO jobs(id,data) VALUES('second',?1)",
        [r#"{"profile":{"id":"second-profile"}}"#],
    )
    .unwrap();
    store::insert_batch(&mut c, "second", "b.txt", &[other]).unwrap();
    let issues = store::issues(&c).unwrap();
    let linked = issues["rows"][0]["hand_row"].as_i64().unwrap();
    let profile: String = c
        .query_row("SELECT profile FROM hands WHERE id=?1", [linked], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(profile, "second-profile");
    for _ in 0..302 {
        c.execute("INSERT INTO import_issues(job,source_file,code,message) VALUES('unknown','synthetic','synthetic','synthetic')", []).unwrap();
    }
    let first = store::issues_page(&c, None).unwrap();
    assert_eq!(first["rows"].as_array().unwrap().len(), 300);
    let cursor = first["next_cursor"].as_i64().unwrap();
    let second = store::issues_page(&c, Some(cursor)).unwrap();
    assert_eq!(second["rows"].as_array().unwrap().len(), 3);
    assert!(second["next_cursor"].is_null());
    assert!(second["rows"][0]["id"].as_i64().unwrap() < cursor);
}

#[test]
fn date_report_drill_uses_the_source_date_instead_of_hong_kong_midnight() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    store::init(&p).unwrap();
    let mut c = store::open(&p).unwrap();
    let profile = Profile {
        id: "utc-profile".into(),
        timezone: "UTC".into(),
        ..Profile::default()
    };
    store::save_profile(&c, &profile).unwrap();
    let base = fixtures::cash_hand(2);
    let mut lines = base.lines();
    let header = lines.next().unwrap().split(" - ").next().unwrap();
    let raw = format!(
        "{header} - 2026/09/01 20:00:00\n{}\n",
        lines.collect::<Vec<_>>().join("\n")
    );
    let hand = parser::parse(&raw, &profile).unwrap();
    assert_eq!(hand.status, "valid");
    store::insert_batch(&mut c, "utc", "utc.txt", &[hand]).unwrap();
    let source = Filter {
        source_date: Some("2026-09-01".into()),
        ..Filter::default()
    };
    assert_eq!(store::report(&c, &source, "date").unwrap()["hands"], 1);
    let hkt = Filter {
        date_from: Some("2026-09-01".into()),
        date_to: Some("2026-09-01".into()),
        ..Filter::default()
    };
    assert_eq!(store::report(&c, &hkt, "date").unwrap()["hands"], 0);
}
#[test]
fn saved_filter_names_use_characters_for_all_languages() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    store::init(&p).unwrap();
    let c = store::open(&p).unwrap();
    let name = "篩".repeat(150);
    store::save_filter(&c, &name, &Filter::default()).unwrap();
    assert_eq!(store::saved_filters(&c).unwrap()[0]["name"], name);
    assert!(store::save_filter(&c, &"篩".repeat(151), &Filter::default()).is_err());
    assert!(store::save_filter(&c, "   ", &Filter::default()).is_err());
}

fn wait_done(s: &std::sync::Arc<Service>) {
    let start = Instant::now();
    while s.handle(Request::Health).unwrap()["import_active"] == true {
        // This checks lifecycle correctness, not throughput. Hosted Windows runners
        // vary in debug SQLite/index speed; avoid hot-polling the coverage query.
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "import did not finish within 120s; jobs: {}",
            s.handle(Request::Jobs).unwrap()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn cannot_raise_when_only_opponent_is_all_in() {
    let raw=fixtures::hu_hand("Hero: raises $0.08 to $0.10 and is all-in\nVillain: calls $0.08\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $0.20 from pot","Total pot $0.20 | Rake $0 | Jackpot $0").replace("Hero ($2 in chips)","Hero ($0.10 in chips)");
    let p = Profile {
        hero: "Villain".into(),
        ..Profile::default()
    };
    let h = parser::parse(&raw, &p).unwrap();
    assert_eq!(h.status, "valid");
    assert_eq!(h.stats["three_bet"].opportunities, 0);
    assert_eq!(h.stats["pfr"].opportunities, 0);
    assert_eq!(h.stats["vpip"].numerator, 1);
}

#[test]
fn cashout_receipt_is_added_without_duplicate_summary() {
    let raw=fixtures::hu_hand("Hero: raises $1.98 to $2 and is all-in\nVillain: calls $1.98 and is all-in\nHero: Chooses to EV Cashout\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Kc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nVillain collected $3.90 from pot\nHero: Receives Cashout ($3.20)","Total pot $4 | Rake $0.10 | Jackpot $0\nSeat 1: Hero receives ($3.20)");
    let h = parse(&raw);
    assert_eq!(h.status, "valid");
    assert_eq!(h.cashout, 3_200_000);
    assert_eq!(h.net, 1_200_000);
    assert_eq!(h.ev_reason.as_deref(), Some("cashout"));
}
#[test]
fn import_cancel_resume_restart_rebuild_and_export() {
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("test.txt");
    use std::io::Write;
    let mut f = std::fs::File::create(&source).unwrap();
    for i in 0..3000 {
        writeln!(f, "{}\n", fixtures::cash_hand(i)).unwrap();
    }
    drop(f);
    let path = d.path().join("test.db");
    let s = Service::new(path.clone()).unwrap();
    s.handle(Request::EquityControl { paused: true }).unwrap();
    let j = s
        .handle(Request::StartImport {
            paths: vec![source.to_string_lossy().into()],
            profile: Profile::default(),
        })
        .unwrap();
    s.handle(Request::CancelImport {
        id: j["id"].as_str().unwrap().into(),
    })
    .unwrap();
    wait_done(&s);
    assert_eq!(s.handle(Request::Jobs).unwrap()[0]["state"], "cancelled");
    drop(s);
    // Emulate process restart with an unfinished job after committed batches.
    let c = store::open(&path).unwrap();
    let mut job = store::jobs(&c).unwrap().remove(0);
    job.state = "running".into();
    store::save_job(&c, &job).unwrap();
    drop(c);
    let s = Service::new(path.clone()).unwrap();
    s.handle(Request::EquityControl { paused: true }).unwrap();
    assert_eq!(s.handle(Request::Jobs).unwrap()[0]["state"], "interrupted");
    s.handle(Request::ResumeImport { id: job.id.clone() })
        .unwrap();
    wait_done(&s);
    let report = s
        .handle(Request::Overview {
            filter: Filter::default(),
            group: None,
        })
        .unwrap();
    assert_eq!(report["hands"], 3000);
    s.handle(Request::SaveAnnotation {
        id: 1,
        annotation: Annotation {
            note: "persist".into(),
            tags: vec!["test".into()],
            reviewed: true,
        },
    })
    .unwrap();
    s.handle(Request::Rebuild).unwrap();
    assert_eq!(
        s.handle(Request::Hand { id: 1 }).unwrap()["annotation"]["note"],
        "persist"
    );
    assert_eq!(
        s.handle(Request::Overview {
            filter: Filter::default(),
            group: None
        })
        .unwrap(),
        report
    );
    let exported = d.path().join("one.txt");
    let r = s
        .handle(Request::Export {
            path: exported.to_string_lossy().into(),
            format: "hh".into(),
            filter: Filter::default(),
            selected: vec![1],
        })
        .unwrap();
    assert_eq!(r["hands"], 1);
    assert_eq!(
        parse(&std::fs::read_to_string(exported).unwrap()).id,
        "SYN000000000000"
    );
}
#[test]
fn equity_cache_is_persistent_and_idempotent() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("db");
    let s = Service::new(path.clone()).unwrap();
    s.handle(Request::EquityControl { paused: true }).unwrap();
    let raw=fixtures::hu_hand("Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2c 3d 8h]\nHero: checks\nVillain: checks\n*** TURN *** [2c 3d 8h] [9s]\nHero: bets $1.98 and is all-in\nVillain: calls $1.98 and is all-in\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $3.90 from pot","Total pot $4 | Rake $0.10 | Jackpot $0");
    let mut c = store::open(&path).unwrap();
    store::insert_batch(
        &mut c,
        "a",
        "a",
        &[parse(&raw), parse(&raw.replace("#TEST1:", "#TEST2:"))],
    )
    .unwrap();
    s.handle(Request::EquityControl { paused: false }).unwrap();
    let start = Instant::now();
    while s.handle(Request::Health).unwrap()["equity_running"] == true {
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM equity_results", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        s.handle(Request::Hand { id: 2 }).unwrap()["hand"]["ev_status"],
        "complete"
    );
}

#[test]
fn quarantine_cannot_enter_reports_even_with_explicit_status_filter() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("db");
    store::init(&path).unwrap();
    let mut c = store::open(&path).unwrap();
    let raw = fixtures::cash_hand(2).replace("Total pot $", "Total pot $1");
    let h = parse(&raw);
    assert_eq!(h.status, "quarantined");
    store::insert_batch(&mut c, "a", "a", &[h]).unwrap();
    let f = Filter {
        status: Some("quarantined".into()),
        ..Filter::default()
    };
    assert_eq!(store::report(&c, &f, "date").unwrap()["hands"], 0);
    assert!(store::matrix(&c, &f, "vpip")
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        store::list_hands(&c, &f, "recent", None, 60).unwrap()["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn session_rollups_migrate_and_match_filtered_ledger() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("db");
    store::init(&path).unwrap();
    let mut c = store::open(&path).unwrap();
    let hands: Vec<_> = (0..36).map(|i| parse(&fixtures::cash_hand(i))).collect();
    let expected: i64 = hands.iter().map(|h| h.net).sum();
    store::insert_batch(&mut c, "a", "session-a.txt", &hands[..18]).unwrap();
    store::insert_batch(&mut c, "b", "session-b.txt", &hands[18..]).unwrap();
    let report = store::report(&c, &Filter::default(), "session").unwrap();
    assert_eq!(report["groups"].as_array().unwrap().len(), 2);
    assert_eq!(report["net"], money::format(expected));
    let filtered = Filter {
        position: Some("BTN".into()),
        ..Filter::default()
    };
    let btn: Vec<_> = hands.iter().filter(|h| h.position == "BTN").collect();
    let scanned = store::report(&c, &filtered, "date").unwrap();
    assert_eq!(scanned["hands"], btn.len());
    assert_eq!(
        scanned["net"],
        money::format(btn.iter().map(|h| h.net).sum())
    );
    let date = hands[0].local_time[..10].to_string();
    for timezone in ["Asia/Hong_Kong", "UTC"] {
        let f = Filter {
            date_from: Some(date.clone()),
            position: Some("BTN".into()),
            game: Some("Cash".into()),
            timezone: Some(timezone.into()),
            ..Filter::default()
        };
        let fast = store::report(&c, &f, "date").unwrap();
        let full_scan = Filter {
            player_count: Some(6),
            ..f
        };
        let direct = store::report(&c, &full_scan, "date").unwrap();
        for key in ["hands", "net", "stats", "coverage", "ev"] {
            assert_eq!(fast[key], direct[key], "{timezone}: {key}");
        }
    }
    c.execute_batch("DELETE FROM metadata WHERE key IN ('session_rollup_v1','slice_rollup_v1'); DELETE FROM rollups WHERE dimension IN ('session','slice')").unwrap();
    drop(c);
    store::init(&path).unwrap();
    let c = store::open(&path).unwrap();
    assert_eq!(
        store::report(&c, &Filter::default(), "session").unwrap(),
        report
    );
    assert_eq!(store::report(&c, &filtered, "date").unwrap(), scanned);
}

#[test]
fn restore_invalidates_inflight_equity_and_recovers_after_invalid_backup() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("live.db");
    let backup = d.path().join("backup.db");
    let s = Service::new(path.clone()).unwrap();
    let mut c = store::open(&path).unwrap();
    let raw = fixtures::hu_hand("Hero: raises $1.98 to $2 and is all-in\nVillain: calls $1.98 and is all-in\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $3.90 from pot", "Total pot $4 | Rake $0.10 | Jackpot $0");
    let h = parse(&raw);
    assert_eq!(h.ev_status, "pending");
    store::insert_batch(&mut c, "a", "a", &[h]).unwrap();
    store::init(&backup).unwrap();
    let mut b = store::open(&backup).unwrap();
    store::insert_batch(&mut b, "b", "b", &[parse(&fixtures::cash_hand(2))]).unwrap();
    b.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    drop(b);
    s.start_equity();
    s.handle(Request::Restore {
        path: backup.to_string_lossy().into(),
    })
    .unwrap();
    s.handle(Request::EquityControl { paused: false }).unwrap();
    let started = Instant::now();
    while s.handle(Request::Health).unwrap()["equity_running"] == true {
        assert!(started.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(10));
    }
    let restored = store::get_hand(&c, 1).unwrap();
    assert_eq!(restored.id, "SYN000000000002");
    assert_ne!(restored.ev_status, "complete");
    assert!(restored.adjusted_net.is_none());
    assert!(s
        .handle(Request::Restore {
            path: d.path().join("missing.db").to_string_lossy().into()
        })
        .is_err());
    assert_eq!(s.handle(Request::Health).unwrap()["import_active"], false);
    assert_eq!(store::get_hand(&c, 1).unwrap().id, restored.id);
}
