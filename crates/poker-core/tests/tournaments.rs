use poker_core::{
    hand_formats,
    model::*,
    money, parser,
    service::{Request, Service},
    store,
};

const STARS: &str = include_str!("fixtures/tournaments/pokerstars.txt");
const SOURCES: &[(&str, &str)] = &[
    ("pokerstars", STARS),
    ("gg", include_str!("fixtures/tournaments/gg.txt")),
    ("winning", include_str!("fixtures/tournaments/winning.txt")),
    ("winamax", include_str!("fixtures/tournaments/winamax.txt")),
    (
        "fulltilt",
        include_str!("fixtures/tournaments/fulltilt.txt"),
    ),
    ("888", include_str!("fixtures/tournaments/888.txt")),
    ("ipoker", include_str!("fixtures/tournaments/ipoker.xml")),
];

#[test]
fn seven_tournament_grammars_conserve_chips_and_keep_cash_buyins_outside_ledger() {
    for (format, raw) in SOURCES {
        let h =
            parser::parse(raw, &Profile::default()).unwrap_or_else(|e| panic!("{format}: {e:#}"));
        assert_eq!(h.status, "valid", "{format}: {:?}", h.issues);
        assert_eq!(h.game, "Tournament");
        assert_eq!(h.currency, "CHIPS");
        assert_eq!(h.raw, raw.trim());
        assert!(h.parser_version.ends_with(format));
        assert_eq!(h.net, money::parse("1010").unwrap());
        assert_eq!(h.total_pot, money::parse("2010").unwrap());
        assert_eq!(h.invested, money::parse("1000").unwrap());
        assert_eq!(h.bb, money::parse("100").unwrap());
        assert_eq!(h.stats["vpip"].numerator, 1);
        assert_eq!(h.boards[0].len(), 5);
        assert_eq!(h.player_count, 3);
        assert_eq!(h.ev_status, "pending", "{format}: {:?}", h.ev_reason);
        assert!(h.tournament.as_ref().unwrap().id.is_some());
        assert!(h.actions.iter().filter(|a| a.kind == "ante").count() == 3);
    }
    let stars = parser::parse(STARS, &Profile::default()).unwrap();
    assert_eq!(
        stars.tournament.unwrap().buy_in.as_deref(),
        Some("$10+$1 USD")
    );
    assert_eq!(stars.timezone, "America/New_York");
}

#[test]
fn returns_big_blind_ante_and_bounty_notifications_are_not_live_bets_or_chip_prizes() {
    let raw = STARS.replace("Hero: posts the ante 10\nVillain2: posts the ante 10\nFolder3: posts the ante 10", "Villain2: posts big blind ante 30")
        .replace("Hero: raises 940 to 990 and is all-in", "Hero: raises 450 to 500")
        .replace("Villain2: calls 890", "Villain2: folds\nUncalled bet (400) returned to Hero")
        .replace("Hero collected 2,010", "Hero collected 230")
        .replace("Total pot 2,010", "Total pot 230")
        .replace("*** FLOP *** [2c 3d 7s]\n*** TURN *** [2c 3d 7s] [9h]\n*** RIVER *** [2c 3d 7s 9h] [Jc]\n*** SHOW DOWN ***\nHero: shows [Ah Ad]\nVillain2: shows [Kh Kd]\n", "")
        .replace("*** SUMMARY ***", "Hero wins $5 for eliminating Villain2 and their own bounty increases by $5 to $20\nVillain2 finished the tournament in 10th place\n*** SUMMARY ***");
    let h = parser::parse(&raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.net, money::parse("130").unwrap());
    assert_eq!(h.returned, money::parse("400").unwrap());
    assert_eq!(h.actions.iter().filter(|a| a.kind == "ante").count(), 1);
    assert_eq!(h.tournament.unwrap().id.as_deref(), Some("7001"));
    let ftp = raw.replacen(raw.lines().next().unwrap(), "Full Tilt Poker Game #T1001: Study Cup (7001), Table 2 - 50/100 - No Limit Hold'em - 12:00:00 ET - 2026/10/11", 1)
        .replace("Table '7001 2' 9-max Seat #3 is the button", "The button is in seat #3")
        .replace(" in chips", "")
        .replace("Uncalled bet (400)", "Uncalled bet of 400")
        .replace("Hero: raises 450 to 500", "Hero raises to 500")
        .replace("Villain2: folds", "Villain2 has timed out\nVillain2 is sitting out\nVillain2 folds\nVillain2 has returned");
    let h = parser::parse(&ftp, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.net, money::parse("130").unwrap());
    assert_eq!(h.returned, money::parse("400").unwrap());
}

#[test]
fn unsafe_variants_and_incomplete_settlements_are_rejected_or_quarantined() {
    for raw in [
        STARS.replace("Hold'em No Limit", "Omaha Pot Limit"),
        STARS.replace(" | Rake 0", " | Rake 5"),
        STARS.replace("Hero collected 2,010 from pot\n", ""),
        STARS.replace("Hero collected 2,010", "Hero collected 2,010.50"),
        STARS.replace("(50/100)", "(50K/100K)"),
        STARS.replace("Villain2: calls 890", "Villain2: calls -890"),
        STARS.replace("Total pot 2,010", "Total pot -2,010"),
        STARS.replace("Villain2: calls 890", "Villain2: calls 890K"),
        SOURCES[6].1.replace("chips=\"1500\"", "chips=\"110\""),
        SOURCES[6].1.replace("type=\"15\"", "type=\"99\""),
    ] {
        assert!(parser::parse(&raw, &Profile::default()).is_err());
    }
    let broken = STARS.replace("Total pot 2,010", "Total pot 2,011");
    assert_eq!(
        parser::parse(&broken, &Profile::default()).unwrap().status,
        "quarantined"
    );
}

#[test]
fn multiway_allins_keep_side_pots_and_do_not_claim_heads_up_chip_ev() {
    let raw = STARS.replace("Hero (1,000 in chips)", "Hero (500 in chips)")
        .replace("Folder3: folds\n", "")
        .replace("raises 940 to 990", "raises 440 to 490")
        .replace("Villain2: calls 890", "Villain2: calls 390\nFolder3: calls 490")
        .replace("*** FLOP *** [2c 3d 7s]", "*** FLOP *** [2c 3d 7s]\nVillain2: bets 1000 and is all-in\nFolder3: calls 1000")
        .replace("Hero collected 2,010 from pot", "Folder3: shows [Qh Qd]\nHero collected 1,500 from main pot\nFolder3 collected 2,000 from side pot")
        .replace("Total pot 2,010", "Total pot 3,500");
    let h = parser::parse(&raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.net, money::parse("1000").unwrap());
    assert_eq!(h.pots.len(), 2);
    assert_eq!(h.pots[0].amount, money::parse("1500").unwrap());
    assert_eq!(h.pots[1].amount, money::parse("2000").unwrap());
    assert_eq!(h.ev_status, "excluded");
    assert!(h.equity_input.is_none());
}

#[test]
fn indexed_tournament_filters_and_reports_do_not_mix_cash_and_chips() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("hands.db");
    store::init(&db).unwrap();
    let mut c = store::open(&db).unwrap();
    let tour = parser::parse(STARS, &Profile::default()).unwrap();
    let cash = parser::parse(
        include_str!("fixtures/hand-formats/pokerstars.txt"),
        &Profile::default(),
    )
    .unwrap();
    store::insert_batch(&mut c, "test", "mixed.txt", &[tour.clone(), cash]).unwrap();
    assert!(store::report(&c, &Filter::default(), "date").is_err());
    assert!(store::report(
        &c,
        &Filter {
            stack_min: Some(0.),
            ..Filter::default()
        },
        "date"
    )
    .is_err());
    let filter = Filter {
        tournament_id: Some("7001".into()),
        ..Filter::default()
    };
    let report = store::report(&c, &filter, "date").unwrap();
    assert_eq!(report["currency"], "CHIPS");
    assert_eq!(report["hands"], 1);
    assert_eq!(report["net"], "1010.00");
    let catalog =
        poker_core::agent::call(&mut c, "get_data_catalog", serde_json::json!({}), false).unwrap();
    assert_eq!(
        catalog["data"]["values"]["tournament_id"]["values"],
        serde_json::json!(["7001"])
    );
    assert!(catalog["data"]["money"].as_str().unwrap().contains("CHIPS"));
    let csv = dir.path().join("chips.csv");
    store::export(&c, &filter, &csv, "csv", &[]).unwrap();
    assert!(std::fs::read_to_string(csv).unwrap().contains("CHIPS"));
    // Old stored payloads remain readable after adding optional metadata.
    let mut old = serde_json::to_value(&tour).unwrap();
    old.as_object_mut().unwrap().remove("tournament");
    assert!(serde_json::from_value::<Hand>(old)
        .unwrap()
        .tournament
        .is_none());
    // Exercise migration from a populated pre-tournament schema, including backup.
    c.execute_batch("DROP INDEX hand_tournament; ALTER TABLE hands DROP COLUMN tournament_id;")
        .unwrap();
    drop(c);
    store::init(&db).unwrap();
    let c = store::open(&db).unwrap();
    let count: i64 = c
        .query_row("SELECT COUNT(*) FROM hands", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
    assert!(std::fs::read_dir(dir.path()).unwrap().any(|p| p
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("before-study-")));
}

#[test]
fn tournament_text_and_xml_imports_resume_rebuild_and_preserve_source() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input");
    std::fs::create_dir(&input).unwrap();
    for (format, raw) in SOURCES {
        let ext = if *format == "ipoker" { "xml" } else { "txt" };
        std::fs::write(input.join(format!("{format}.{ext}")), raw).unwrap();
    }
    assert!(hand_formats::is_header(
        SOURCES[2].1.lines().next().unwrap()
    ));
    let db = dir.path().join("hands.db");
    let s = Service::new(db.clone()).unwrap();
    for duplicate in [false, true] {
        let job = s
            .handle(Request::StartImport {
                profile: Profile::default(),
                paths: vec![input.to_string_lossy().into()],
            })
            .unwrap();
        for _ in 0..200 {
            let jobs = s.handle(Request::Jobs).unwrap();
            let item = jobs
                .as_array()
                .unwrap()
                .iter()
                .find(|j| j["id"] == job["id"])
                .unwrap();
            if item["state"] == "complete" {
                assert_eq!(item["quarantined"], 0, "{item}");
                assert_eq!(item[if duplicate { "duplicates" } else { "inserted" }], 7);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let jobs = s.handle(Request::Jobs).unwrap();
        let item = jobs
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["id"] == job["id"])
            .unwrap();
        assert_eq!(item["state"], "complete", "{item}");
    }
    s.handle(Request::Rebuild).unwrap();
    let c = store::open(&db).unwrap();
    assert_eq!(
        store::report(
            &c,
            &Filter {
                currency: Some("CHIPS".into()),
                ..Filter::default()
            },
            "game"
        )
        .unwrap()["hands"],
        7
    );
    let export = dir.path().join("originals.txt");
    store::export(&c, &Filter::default(), &export, "hh", &[]).unwrap();
    let raw = std::fs::read_to_string(export).unwrap();
    for (format, source) in SOURCES {
        if *format == "ipoker" {
            // XML imports store a per-hand envelope, not the entire session.
            assert!(raw.contains("<tournamentname>Study Cup</tournamentname>"));
            assert!(raw.contains("<game gamecode=\"1007\">"));
            assert_eq!(
                std::fs::read_to_string(input.join("ipoker.xml")).unwrap(),
                *source
            );
        } else {
            assert!(raw.contains(source.trim()), "{format}");
        }
    }
}
