//! Independent acceptance probes. Synthetic hands only; failures identify unmet contracts.
use poker_core::{
    fixtures,
    model::*,
    parser,
    service::{Request, Service},
    store,
    study::{self, decision, strategy, training, StudyQuery},
};
use rusqlite::Connection;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    store::init(&dir.path().join("audit.db")).unwrap();
    let c = store::open(&dir.path().join("audit.db")).unwrap();
    (dir, c)
}
fn hand() -> Hand {
    parser::parse(
        include_str!("../../../tests/fixtures/study-nl10.txt"),
        &Profile::default(),
    )
    .unwrap()
}
fn pack(dir: &Path) -> strategy::StrategyPack {
    let rows = [
        ("open.txt", "", "Raise 2.5", "AA:0.5,KK:0.000001"),
        (
            "fourbet.txt",
            "R2.5-R8-F-F-F-F",
            "Raise 21.5",
            "AA:0.4,KK:0.0000002",
        ),
        ("call.txt", "R2.5-R8-F-F-F-F", "Call", "AA:0.2,KK:0.0000003"),
        (
            "sixbet.txt",
            "R2.5-R8-F-F-F-F-R21.5-R40",
            "Allin 100",
            "AA:0.1,KK:0.0000001",
        ),
    ];
    let files: Vec<_> = rows.iter().map(|(file,path,action,text)| {
        std::fs::write(dir.join(file), text).unwrap();
        json!({"relativePath":file,"hero":"UTG","sourcePath":path,"action":action,"hands":2,"sourceUrl":"https://app.gtowizard.com/solutions"})
    }).collect();
    let manifest = dir.join("manifest.json");
    std::fs::write(&manifest, json!({"source":{"gametype":"Cash6mGeneral_6mNL10R25","depthBb":100,"treeId":null},"files":files,"estimate":{"decisionSpots":3}}).to_string()).unwrap();
    strategy::parse_nexus(&manifest).unwrap()
}

#[test]
fn acceptance_unknown_source_model_must_remain_reference() {
    let (dir, mut c) = db();
    let p = pack(dir.path());
    strategy::import(&c, dir.path().join("manifest.json").to_str().unwrap()).unwrap();
    let h = hand();
    let r = decision::derive(&h)[0].reference.clone();
    store::insert_batch(&mut c, "audit", "synthetic", &[h]).unwrap();
    let result = strategy::compare_decision(&c, &p.id, &r, None).unwrap();
    assert_eq!(
        result["status"], "reference",
        "Missing tree/rake verification must not become matched; actual reasons={}",
        result["reasons"]
    );
}

#[test]
fn acceptance_inconsistent_parent_must_taint_descendant() {
    let (dir, _) = db();
    let p = pack(dir.path());
    let parent = p
        .nodes
        .iter()
        .find(|n| n.path == "R2.5-R8-F-F-F-F")
        .unwrap();
    assert!(parent.issues.contains_key("AA"));
    let child = p.nodes.iter().find(|n| n.path.ends_with("R40")).unwrap();
    assert!(
        !child.frequencies.contains_key("AA"),
        "Parent AA sums to 0.6 > arrival 0.5, but descendant is treated as verified: {:?}",
        child.frequencies.get("AA")
    );
}

#[test]
fn acceptance_no_target_still_shows_descriptive_data() {
    let (_dir, mut c) = db();
    store::insert_batch(&mut c, "audit", "synthetic", &[hand()]).unwrap();
    let result = study::leaks(&c, &Filter::default(), None).unwrap();
    assert!(
        !result["rows"].as_array().unwrap().is_empty(),
        "No benchmark currently returns an empty page despite indexed decisions"
    );
}

#[test]
fn checked_preflop_and_postflop_answer_fields_and_all_schedule_steps() {
    let (dir, mut c) = db();
    pack(dir.path());
    let p = strategy::import(&c, dir.path().join("manifest.json").to_str().unwrap()).unwrap();
    training::enqueue(&c, &[], p["id"].as_str(), Some("UTG:"), Some("KK")).unwrap();
    for (stage, days) in [(1, 1), (2, 3), (3, 7), (4, 14), (5, 30), (5, 30)] {
        c.execute("UPDATE study_cards SET due=0", []).unwrap();
        let q = training::start(&c, 20).unwrap();
        assert!(q["feedback"].is_null());
        assert!(!q["question"]
            .as_object()
            .unwrap()
            .contains_key("frequencies"));
        let id = q["id"].as_str().unwrap();
        let card = q["card_id"].as_str().unwrap();
        let answer = training::TrainingAnswer::Action {
            action: "R2.5".into(),
            note: "tiny mix".into(),
        };
        assert_eq!(
            training::answer(&c, id, card, &answer).unwrap()["feedback"]["feedback"]["in_strategy"],
            true
        );
        let now = chrono::Utc::now().timestamp();
        training::rate(&mut c, id, card, "advance").unwrap();
        training::rate(&mut c, id, card, "advance").unwrap();
        let (due, actual): (i64, i64) = c
            .query_row("SELECT due,stage FROM study_cards", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(actual, stage);
        assert!((due - now - days * 86400).abs() <= 1);
    }
    c.execute("UPDATE study_cards SET due=0", []).unwrap();
    let q = training::start(&c, 1).unwrap();
    let id = q["id"].as_str().unwrap();
    let card = q["card_id"].as_str().unwrap();
    training::answer(
        &c,
        id,
        card,
        &training::TrainingAnswer::Action {
            action: "F".into(),
            note: "".into(),
        },
    )
    .unwrap();
    training::rate(&mut c, id, card, "repeat").unwrap();
    let stage: i64 = c
        .query_row("SELECT stage FROM study_cards", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stage, 0);
    let h = parser::parse(&fixtures::cash_hand(2), &Profile::default()).unwrap();
    let d = decision::derive(&h)
        .into_iter()
        .find(|d| d.street == "flop")
        .unwrap();
    store::insert_batch(&mut c, "audit", "synthetic", &[h]).unwrap();
    training::enqueue(&c, &[d.reference], None, None, None).unwrap();
    let q = training::start(&c, 1).unwrap();
    assert_eq!(q["question"]["board"].as_array().unwrap().len(), 3);
    for forbidden in [
        "observed",
        "action",
        "net",
        "result",
        "opponent_cards",
        "frequencies",
        "source_note",
    ] {
        assert!(q["question"].get(forbidden).is_none(), "{forbidden}");
    }
    let id = q["id"].as_str().unwrap().to_string();
    drop(c);
    let c = store::open(&dir.path().join("audit.db")).unwrap();
    assert_eq!(training::current(&c, &id).unwrap(), q);
}

#[test]
fn checked_full_service_rebuild_and_restore_preserve_every_user_table() {
    let (dir, mut c) = db();
    pack(dir.path());
    let p = strategy::import(&c, dir.path().join("manifest.json").to_str().unwrap()).unwrap();
    let h = hand();
    let r = decision::derive(&h)[0].reference.clone();
    store::insert_batch(&mut c, "audit", "synthetic", &[h]).unwrap();
    store::save_annotation(
        &mut c,
        1,
        &Annotation {
            note: "Audit note".into(),
            reviewed: true,
            ..Annotation::default()
        },
    )
    .unwrap();
    study::save_item(
        &c,
        "spot",
        Some("audit"),
        json!({"name":"Audit","spot":{"street":"preflop"}}),
    )
    .unwrap();
    study::save_item(&c,"benchmark",Some("audit"),json!({"name":"Audit","spot":{"street":"preflop"},"action":"fold","low":20,"high":40,"min_samples":100,"note":"Synthetic target"})).unwrap();
    training::enqueue(&c, std::slice::from_ref(&r), p["id"].as_str(), None, None).unwrap();
    let q = training::start(&c, 20).unwrap();
    training::answer(
        &c,
        q["id"].as_str().unwrap(),
        q["card_id"].as_str().unwrap(),
        &training::TrainingAnswer::Action {
            action: "raise".into(),
            note: "Audit attempt".into(),
        },
    )
    .unwrap();
    fn snapshot(c: &Connection) -> BTreeMap<String, Vec<String>> {
        [
            "study_items",
            "study_packs",
            "study_cards",
            "study_sessions",
            "study_attempts",
        ]
        .into_iter()
        .map(|t| {
            let mut s = c
                .prepare(&format!("SELECT data FROM {t} ORDER BY data"))
                .unwrap();
            (
                t.into(),
                s.query_map([], |r| r.get(0))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
            )
        })
        .collect()
    }
    let before = snapshot(&c);
    drop(c);
    let service = Service::new(dir.path().join("audit.db")).unwrap();
    service.handle(Request::Rebuild).unwrap();
    let mut c = store::open(&dir.path().join("audit.db")).unwrap();
    assert_eq!(snapshot(&c), before);
    let id = study::resolve(&c, &r).unwrap().0;
    assert_eq!(store::annotation(&c, id).unwrap().note, "Audit note");
    assert!(store::annotation(&c, id).unwrap().reviewed);
    let backup = dir.path().join("complete.db");
    store::backup(&c, &backup).unwrap();
    c.execute("DELETE FROM study_items", []).unwrap();
    store::restore(&mut c, &backup).unwrap();
    assert_eq!(snapshot(&c), before);
    let bad = dir.path().join("invalid.db");
    std::fs::write(&bad, b"not sqlite").unwrap();
    assert!(store::restore(&mut c, &bad).is_err());
    assert_eq!(snapshot(&c), before);
}

#[test]
fn checked_100k_index_resume_incremental_and_cursor_pages() {
    let (_dir, mut c) = db();
    let start = std::time::Instant::now();
    for batch in 0..100 {
        let hands: Vec<_> = (batch * 1000..batch * 1000 + 1000)
            .map(|i| parser::parse(&fixtures::cash_hand(i), &Profile::default()).unwrap())
            .collect();
        store::insert_batch(&mut c, "audit", "synthetic", &hands).unwrap();
    }
    let initial = study::explore(&c, &StudyQuery::default()).unwrap();
    let count = initial["opportunities"].as_u64().unwrap();
    c.execute("DELETE FROM study_indexed", []).unwrap();
    assert_eq!(
        study::explore(&c, &StudyQuery::default()).unwrap()["opportunities"],
        0
    );
    assert_eq!(study::backfill(&mut c, 50).unwrap(), 50);
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 50);
    c.execute_batch("PRAGMA wal_checkpoint(PASSIVE)").unwrap();
    let path = c.path().unwrap().to_string();
    drop(c);
    let mut c = store::open(Path::new(&path)).unwrap();
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 50);
    while study::backfill(&mut c, 1000).unwrap() > 0 {}
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 100000);
    let mut q = StudyQuery {
        limit: Some(100),
        ..StudyQuery::default()
    };
    let mut seen = BTreeSet::new();
    for _ in 0..3 {
        let page = study::explore(&c, &q).unwrap();
        assert_eq!(page["opportunities"], count);
        for row in page["rows"].as_array().unwrap() {
            assert!(seen.insert(row["id"].as_i64().unwrap()));
        }
        q.before = page["next_cursor"].as_i64();
        assert!(q.before.is_some());
    }
    q.before = Some(2);
    assert!(study::explore(&c, &q).unwrap()["next_cursor"].is_null());
    let h = parser::parse(&fixtures::cash_hand(100001), &Profile::default()).unwrap();
    store::insert_batch(&mut c, "audit", "incremental", &[h]).unwrap();
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 100001);
    println!(
        "AUDIT_100K {}",
        json!({"hands":100000,"decisions":count,"seconds":start.elapsed().as_secs_f64(),"resume_after":50,"unique_page_rows":seen.len(),"incremental_indexed":100001})
    );
}

#[test]
fn checked_real_manifest_structure_without_publishing_private_ranges() {
    let Ok(path) = std::env::var("RIVERLENS_AUDIT_MANIFEST") else {
        return;
    };
    let p = strategy::parse_nexus(Path::new(&path)).unwrap();
    assert_eq!(p.files, 191);
    assert_eq!(p.nodes.len(), 81);
    assert!(p.nodes.iter().any(|n| n.hero == "BB"
        && n.path == "F-F-F-F-C"
        && n.actions.iter().any(|a| a.code == "X")));
    let count: usize = p.nodes.iter().map(|n| n.issues.len()).sum();
    let issues = p.nodes.iter().filter(|n| !n.issues.is_empty()).count();
    println!(
        "AUDIT_MANIFEST {}",
        json!({"files":p.files,"nodes":p.nodes.len(),"issue_cells":count,"issue_nodes":issues,"pack_id":p.id})
    );
}

#[test]
fn acceptance_flop_raise_preset_includes_ip_cbet_response() {
    let (_dir, mut c) = db();
    let raw=fixtures::hu_hand("Hero: raises $0.04 to $0.06\nVillain: calls $0.04\n*** FLOP *** [2c 3d 8h]\nVillain: checks\nHero: bets $0.02\nVillain: raises $0.04 to $0.06\nHero: folds\nUncalled bet ($0.04) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.16 from pot","Total pot $0.16 | Rake $0 | Jackpot $0");
    let h = parser::parse(&raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    store::insert_batch(&mut c, "audit", "synthetic", &[h]).unwrap();
    // Use the same preset returned to the UI; exact custom paths stay exact.
    let preset = study::presets()
        .into_iter()
        .find(|p| p["name"] == "study.flopFacingRaise")
        .unwrap();
    let q: StudyQuery = serde_json::from_value(json!({"spot":preset["spot"]})).unwrap();
    assert_eq!(
        study::explore(&c, &q).unwrap()["opportunities"],
        1,
        "The preset omits the preceding Villain check and loses the IP c-bet opportunity"
    );
}

#[test]
fn checked_observed_hand_mix_and_99_100_threshold() {
    let (dir, mut c) = db();
    let p = pack(dir.path());
    strategy::import(&c, dir.path().join("manifest.json").to_str().unwrap()).unwrap();
    let mut hands = vec![];
    for i in 0..100 {
        let raw = hand().raw.replace("STUDYNL10001", &format!("MIX{i}"));
        let raw = if i < 75 {
            raw
        } else {
            raw.replace("[As Ad]", "[Ks Kd]")
        };
        hands.push(parser::parse(&raw, &Profile::default()).unwrap());
    }
    store::insert_batch(&mut c, "audit", "synthetic", &hands[..99]).unwrap();
    study::save_item(&c, "benchmark", Some("threshold"), json!({"name":"Open", "spot":{"street":"preflop","position":"UTG","preflop_path":""},"action":"raise","low":30.,"high":40.,"min_samples":100,"note":"Synthetic target"})).unwrap();
    let leaks = study::leaks(&c, &Filter::default(), Some(&p.id)).unwrap();
    let row = leaks["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "threshold")
        .unwrap();
    assert_eq!(row["enough"], false);
    assert_eq!(row["priority"], 0.);
    store::insert_batch(&mut c, "audit", "synthetic", &hands[99..]).unwrap();
    let leaks = study::leaks(&c, &Filter::default(), Some(&p.id)).unwrap();
    let row = leaks["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "threshold")
        .unwrap();
    assert_eq!(row["enough"], true);
    assert_eq!(row["opportunities"], 100);
    let matrix = strategy::matrix(&c, &p.id, "UTG:", &Filter::default()).unwrap();
    let expected: f64 = matrix["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["observed"]["reference_expected"]["R2.5"].as_f64())
        .sum();
    assert!((expected - 37.500025).abs() < 1e-9);
    assert!(strategy::leaks(&c, &Filter::default(), &p.id)
        .unwrap()
        .is_empty());
    for patch in [
        json!({"result":"loss"}),
        json!({"reviewed":true}),
        json!({"stat":"vpip"}),
    ] {
        let f: Filter = serde_json::from_value(patch).unwrap();
        assert!(strategy::leaks(&c, &f, &p.id).is_err());
    }
}

#[test]
fn checked_short_allin_and_flop_only_board_filter() {
    let (_dir, mut c) = db();
    let raw=fixtures::hu_hand("Hero: calls $0.01\nVillain: raises $0.01 to $0.03 and is all-in\nHero: calls $0.01\n*** FLOP *** [2c 3d 8h]\n*** TURN *** [2c 3d 8h] [9s]\n*** RIVER *** [2c 3d 8h 9s] [Tc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $0.06 from pot","Total pot $0.06 | Rake $0 | Jackpot $0").replace("Villain ($2 in chips)","Villain ($0.03 in chips)");
    let h = parser::parse(&raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid");
    let d = decision::derive(&h);
    assert_eq!(d.len(), 2);
    assert_eq!(d[1].legal, vec!["fold", "call"]);
    let h = parser::parse(&fixtures::cash_hand(2), &Profile::default()).unwrap();
    store::insert_batch(&mut c, "audit", "synthetic", &[h]).unwrap();
    let q: StudyQuery = serde_json::from_value(
        json!({"spot":{"street":"flop","high_card":"K","texture":"rainbow","paired":false}}),
    )
    .unwrap();
    assert_eq!(study::explore(&c, &q).unwrap()["opportunities"], 1);
}

#[test]
fn checked_schema_one_staging_restore_keeps_annotations() {
    let (dir, mut c) = db();
    store::insert_batch(&mut c, "audit", "synthetic", &[hand()]).unwrap();
    store::save_annotation(
        &mut c,
        1,
        &Annotation {
            note: "schema one note".into(),
            tags: vec!["audit".into()],
            reviewed: true,
        },
    )
    .unwrap();
    c.execute_batch("ALTER TABLE hands ADD COLUMN detail BLOB; UPDATE hands SET detail=(SELECT detail FROM hand_payload p WHERE p.hand=hands.id); DROP TABLE hand_payload; DROP TABLE study_decisions; DROP TABLE study_indexed; DROP TABLE study_items; DROP TABLE study_packs; DROP TABLE study_cards; DROP TABLE study_sessions; DROP TABLE study_attempts; UPDATE metadata SET value='1' WHERE key='schema_version';").unwrap();
    let old = dir.path().join("v1.db");
    store::backup(&c, &old).unwrap();
    drop(c);
    let (_dest, mut dest) = db();
    store::restore(&mut dest, &old).unwrap();
    assert_eq!(store::annotation(&dest, 1).unwrap().note, "schema one note");
    assert_eq!(store::get_hand(&dest, 1).unwrap().id, "STUDYNL10001");
    study::backfill(&mut dest, 50).unwrap();
    assert_eq!(study::coverage(&dest).unwrap()["indexed"], 1);
}

#[test]
fn stored_packs_and_old_training_cards_recheck_ancestry() {
    let (dir, mut c) = db();
    let mut p = pack(dir.path());
    let node_id = "UTG:R2.5-R8-F-F-F-F-R21.5-R40";
    // Simulate a pre-fix stored pack/card with an erroneously trusted descendant.
    let child = p.nodes.iter_mut().find(|n| n.id == node_id).unwrap();
    child.issues.remove("AA");
    child.frequencies.insert(
        "AA".into(),
        BTreeMap::from([("RAI".into(), 0.25), ("F".into(), 0.75)]),
    );
    c.execute(
        "INSERT INTO study_packs VALUES(?1,?2)",
        [&p.id, &serde_json::to_string(&p).unwrap()],
    )
    .unwrap();
    let card = training::TrainingCard {
        reference: None,
        question: json!({"position":"UTG","preflop_path":"R2.5-R8-F-F-F-F-R21.5-R40","hand_class":"AA","theory":true}),
        frequencies: Some(BTreeMap::from([("RAI".into(), 0.25)])),
        observed: None,
        source: "gto".into(),
        pack: Some(p.id.clone()),
        source_note: "original".into(),
    };
    c.execute(
        "INSERT INTO study_cards VALUES('old',?1,0,2)",
        [serde_json::to_string(&card).unwrap()],
    )
    .unwrap();
    let loaded = strategy::load(&c, &p.id).unwrap();
    assert!(!loaded
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .unwrap()
        .frequencies
        .contains_key("AA"));
    assert!(training::enqueue(&c, &[], Some(&p.id), Some(node_id), Some("AA")).is_err());
    let q = training::start(&c, 20).unwrap();
    assert_eq!(q["stale"], true);
    assert!(training::answer(
        &c,
        q["id"].as_str().unwrap(),
        "old",
        &training::TrainingAnswer::Action {
            action: "RAI".into(),
            note: "".into()
        }
    )
    .is_err());
    training::rate(&mut c, q["id"].as_str().unwrap(), "old", "skip").unwrap();
    let raw: String = c
        .query_row("SELECT data FROM study_packs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, serde_json::to_string(&p).unwrap());
}

#[test]
fn strategy_trace_is_predecision_paginated_and_reference_isolated() {
    let (dir, mut c) = db();
    let p = pack(dir.path());
    strategy::import(&c, dir.path().join("manifest.json").to_str().unwrap()).unwrap();
    let mut a = hand();
    a.id = "TRACE1".into();
    let mut b = hand();
    b.id = "TRACE2".into();
    store::insert_batch(&mut c, "audit", "synthetic", &[a, b]).unwrap();
    let mut q: StudyQuery = serde_json::from_value(
        json!({"strategy":{"pack":p.id,"node":"UTG:","matched_only":true},"limit":1}),
    )
    .unwrap();
    assert_eq!(study::explore(&c, &q).unwrap()["opportunities"], 0);
    q.strategy.as_mut().unwrap().matched_only = false;
    let first = study::explore(&c, &q).unwrap();
    assert_eq!(first["opportunities"], 2);
    let reference: decision::DecisionRef =
        serde_json::from_value(first["rows"][0]["decision"]["reference"].clone()).unwrap();
    assert_eq!(
        study::resolve(&c, &reference).unwrap().0,
        first["rows"][0]["hand"].as_i64().unwrap()
    );
    q.before = first["next_cursor"].as_i64();
    let next = study::explore(&c, &q).unwrap();
    assert_ne!(first["rows"][0]["id"], next["rows"][0]["id"]);
    assert_eq!(next["opportunities"], 2);
    assert!(next["next_cursor"].is_null());
    q.filter.result = Some("loss".into());
    assert!(study::explore(&c, &q).is_err());
    training::enqueue(&c, &[reference], Some(&p.id), None, None).unwrap();
    assert_eq!(training::start(&c, 20).unwrap()["source"], "self");
}

#[test]
fn replay_training_entry_uses_stable_refs_without_touching_annotations() {
    let (_dir, mut c) = db();
    let h = hand();
    store::insert_batch(&mut c, "audit", "synthetic", &[h]).unwrap();
    let annotation = Annotation {
        tags: vec!["review".into()],
        reviewed: true,
        note: "keep".into(),
    };
    store::save_annotation(&mut c, 1, &annotation).unwrap();
    let refs = study::StudyRequest::HandDecisions { hand: 1 }
        .run(&mut c)
        .unwrap();
    let r: decision::DecisionRef = serde_json::from_value(refs[0].clone()).unwrap();
    training::enqueue(&c, std::slice::from_ref(&r), None, None, None).unwrap();
    assert_eq!(
        training::enqueue(&c, std::slice::from_ref(&r), None, None, None).unwrap()["duplicates"],
        1
    );
    assert_eq!(
        serde_json::to_value(store::annotation(&c, 1).unwrap()).unwrap(),
        serde_json::to_value(annotation).unwrap()
    );
    c.execute("UPDATE study_indexed SET version='old'", [])
        .unwrap();
    assert!(study::resolve(&c, &r).is_err());
    assert_eq!(study::backfill(&mut c, 1).unwrap(), 1);
    assert!(study::resolve(&c, &r).is_ok());
}

#[test]
fn semantic_presets_exclude_donk_cold_threebet_and_extra_actions() {
    let raw=fixtures::hu_hand("Hero: raises $0.04 to $0.06\nVillain: calls $0.04\n*** FLOP *** [2c 3d 8h]\nVillain: checks\nHero: bets $0.02\nVillain: raises $0.04 to $0.06\nHero: folds\nUncalled bet ($0.04) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.16 from pot","Total pot $0.16 | Rake $0 | Jackpot $0");
    let h = parser::parse(&raw, &Profile::default()).unwrap();
    let last = decision::derive(&h).pop().unwrap();
    assert_eq!(last.scenario.as_deref(), Some("cbet_vs_raise"));
    // OOP branch: no preceding villain check. Position order is set explicitly.
    let mut oop = h.clone();
    oop.button = oop.players.iter().find(|p| !p.hero).unwrap().seat;
    oop.actions
        .retain(|a| !(a.street == "flop" && a.kind == "check"));
    let d = decision::derive(&oop).pop().unwrap();
    assert_eq!(d.role.as_deref(), Some("OOP"));
    assert_eq!(d.scenario.as_deref(), Some("cbet_vs_raise"));
    // Same postflop betting path after an opponent open is a donk, not a c-bet.
    let mut donk = oop.clone();
    for a in &mut donk.actions {
        if a.street == "preflop" && matches!(a.kind.as_str(), "raise" | "call") {
            a.actor = Some(if a.actor == Some(donk.hero_seat) {
                donk.players.iter().find(|p| !p.hero).unwrap().seat
            } else {
                donk.hero_seat
            });
        }
    }
    assert!(decision::derive(&donk)
        .iter()
        .all(|d| d.scenario.as_deref() != Some("cbet_vs_raise")));
    let mut cold = hand();
    // Hero's first action faces two raises; Hero never opened.
    let hero_index = cold
        .actions
        .iter()
        .position(|a| a.actor == Some(cold.hero_seat) && a.kind == "raise")
        .unwrap();
    let mut first = cold.actions[hero_index].clone();
    first.actor = Some(cold.players.iter().find(|p| !p.hero).unwrap().seat);
    let mut second = first.clone();
    second.to *= 3;
    second.amount = second.to;
    cold.actions.splice(hero_index..hero_index, [first, second]);
    let d = decision::derive(&cold)[0].clone();
    assert_eq!(d.facing, "three_bet");
    assert_ne!(d.scenario.as_deref(), Some("open_vs_3bet"));
    // Custom paths retain exact action order, independent of semantic presets.
    let mut line = last.line.clone();
    line.swap(0, 1);
    assert_ne!(
        decision::line_code(&line, "flop"),
        decision::line_code(&last.line, "flop")
    );
}
