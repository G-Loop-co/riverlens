use poker_core::{
    fixtures,
    model::*,
    parser, store,
    study::{self, decision, strategy, training, StudyQuery},
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

fn database() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("study.db");
    store::init(&path).unwrap();
    (dir, store::open(&path).unwrap())
}
fn hu(actions: &str, total: &str) -> Hand {
    let h = parser::parse(&fixtures::hu_hand(actions, total), &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    h
}
fn reraise() -> Hand {
    hu("Hero: raises $0.04 to $0.06\nVillain: raises $0.12 to $0.18\nHero: folds\nUncalled bet ($0.12) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.12 from pot", "Total pot $0.12 | Rake $0 | Jackpot $0")
}
fn pack_fixture(dir: &Path) -> std::path::PathBuf {
    // Synthetic weights, not proprietary strategy. QQ deliberately exceeds arrival.
    let ranges = [
        (
            "open.txt",
            "UTG",
            "",
            "Raise 2.5",
            "AA:0.5,KK:0.000001,QQ:0.5",
            3,
        ),
        (
            "fourbet.txt",
            "UTG",
            "R2.5-R8-F-F-F-F",
            "Raise 21.5",
            "AA:0.1,KK:0.0000002,QQ:0.4",
            3,
        ),
        (
            "call.txt",
            "UTG",
            "R2.5-R8-F-F-F-F",
            "Call",
            "AA:0.2,KK:0.0000003,QQ:0.1001",
            3,
        ),
    ];
    let files: Vec<_> = ranges.iter().map(|(path,hero,line,action,text,n)| {
        std::fs::write(dir.join(path),text).unwrap();
        json!({"relativePath":path,"hero":hero,"sourcePath":line,"action":action,"hands":n,"sourceUrl":"https://app.gtowizard.com/solutions"})
    }).collect();
    let path = dir.join("manifest.json");
    std::fs::write(&path,json!({"source":{"gametype":"Cash6mGeneral_6mNL10R25","depthBb":100},"files":files,"estimate":{"decisionSpots":2}}).to_string()).unwrap();
    path
}

#[test]
fn decisions_do_not_see_future_raises_board_or_outcome() {
    let mut h = reraise();
    let d = decision::derive(&h);
    assert_eq!(d.len(), 2);
    assert_eq!(d[0].facing, "unopened");
    assert_eq!(d[0].pot_type, "limped");
    assert_eq!(d[1].facing, "three_bet");
    assert_eq!(d[1].pot_type, "3-bet");
    assert!(d[0].board.is_empty());
    assert!(d[0].line.is_empty());
    assert_eq!(d[1].preflop_path, "R3-R9");
    h.net = 999999;
    h.pot_type = "4-bet+".into();
    h.actions.truncate(d[0].reference.seq + 1);
    assert_eq!(
        serde_json::to_value(&decision::derive(&h)[0]).unwrap(),
        serde_json::to_value(&d[0]).unwrap()
    );
}

#[test]
fn ordered_postflop_lines_and_opportunities_are_not_hand_counts() {
    let (_dir, mut c) = database();
    let h=hu("Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2c 3d 4h]\nVillain: checks\nHero: bets $0.02\nVillain: raises $0.04 to $0.06\nHero: calls $0.04\n*** TURN *** [2c 3d 4h] [5s]\nVillain: checks\nHero: checks\n*** SHOWDOWN ***\nHero collected $0.16 from pot","Total pot $0.16 | Rake $0 | Jackpot $0");
    store::insert_batch(&mut c, "test", "test", std::slice::from_ref(&h)).unwrap();
    let d = decision::derive(&h);
    assert_eq!(d.len(), 4);
    assert_eq!(d[1].board, vec!["2c", "3d", "4h"]);
    assert_eq!(d[1].role.as_deref(), Some("IP"));
    assert_eq!(d[1].size_bb, Some(1.0));
    assert_eq!(d[2].facing, "raise");
    assert_eq!(d[2].line.len(), 3);
    let q: StudyQuery =
        serde_json::from_value(json!({"spot":{"street":"flop"},"limit":1})).unwrap();
    let report = study::explore(&c, &q).unwrap();
    assert_eq!(report["opportunities"], 2);
    assert_eq!(report["hands"], 1);
    assert_eq!(report["net_bb"], h.net as f64 / h.bb as f64);
    let second = study::explore(
        &c,
        &StudyQuery {
            before: report["next_cursor"].as_i64(),
            ..q
        },
    )
    .unwrap();
    assert_ne!(report["rows"][0]["id"], second["rows"][0]["id"]);
    assert!(second["next_cursor"].is_null());
    let line = d[2].line.clone();
    let mut q = StudyQuery::default();
    q.spot.street = Some("flop".into());
    q.spot.line = line;
    assert_eq!(study::explore(&c, &q).unwrap()["opportunities"], 1);
    q.spot.line.swap(0, 1);
    assert_eq!(study::explore(&c, &q).unwrap()["opportunities"], 0);
    q.filter.reviewed = Some(true);
    assert!(study::explore(&c, &q).is_err());
}

#[test]
fn all_in_response_cannot_raise_and_multiway_postflop_is_excluded() {
    let h=hu("Hero: raises $0.04 to $0.06\nVillain: raises $1.94 to $2 and is all-in\nHero: folds\nUncalled bet ($1.94) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.12 from pot","Total pot $0.12 | Rake $0 | Jackpot $0");
    assert_eq!(decision::derive(&h)[1].legal, vec!["fold", "call"]);
    let mut h = parser::parse(&fixtures::cash_hand(2), &Profile::default()).unwrap();
    // A third player remains live at flop; later heads-up status must not qualify it.
    let folded = h.actions.iter().position(|a| a.kind == "fold").unwrap();
    h.actions.remove(folded);
    assert!(decision::derive(&h).iter().all(|d| d.street == "preflop"));
}

#[test]
fn conditional_weights_preserve_tiny_mixes_and_exclude_inconsistent_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = pack_fixture(dir.path());
    let pack = strategy::parse_nexus(&path).unwrap();
    let node = pack.nodes.iter().find(|n| !n.path.is_empty()).unwrap();
    assert!((node.frequencies["AA"]["R21.5"] - 0.2).abs() < 1e-12);
    assert!((node.frequencies["AA"]["F"] - 0.4).abs() < 1e-12);
    assert!((node.frequencies["KK"]["C"] - 0.3).abs() < 1e-12);
    assert_eq!(node.issues["QQ"], "source_weight_inconsistent");
    assert!(!node.frequencies.contains_key("QQ"));
    assert_eq!(pack.id, strategy::parse_nexus(&path).unwrap().id);
    std::fs::remove_file(dir.path().join("call.txt")).unwrap();
    assert!(strategy::parse_nexus(&path).is_err());
}

#[test]
fn strategy_pack_rejects_path_escape() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = pack_fixture(dir.path());
    std::fs::write(outside.path().join("range.txt"), "AA").unwrap();
    let mut manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["files"][0]["relativePath"] = json!(outside.path().join("range.txt"));
    std::fs::write(&path, manifest.to_string()).unwrap();
    assert!(strategy::parse_nexus(&path).is_err());
}

#[test]
fn unknown_models_never_enter_gto_denominators() {
    let (dir, mut c) = database();
    let path = pack_fixture(dir.path());
    let pack = strategy::import(&c, path.to_str().unwrap()).unwrap();
    let id = pack["id"].as_str().unwrap();
    let mut h = parser::parse(&fixtures::cash_hand(2), &Profile::default()).unwrap();
    let mut d = decision::derive(&h)[0].clone();
    d.bb = "0.10".into();
    d.sb_bb = 0.5;
    d.stacks_bb.values_mut().for_each(|v| *v = 100.);
    d.hand_class = "AA".into();
    d.action = "raise".into();
    d.size_bb = Some(2.5);
    let p = strategy::load(&c, id).unwrap();
    let node = p.nodes.iter().find(|n| n.path.is_empty()).unwrap();
    assert!(strategy::mismatches(&d, node, &p).contains(&"hand_model_unverified".into()));
    d.stacks_bb.insert("BB".into(), 99.);
    assert!(strategy::mismatches(&d, node, &p).contains(&"stack_depth".into()));
    d.stacks_bb.insert("BB".into(), 100.);
    h.id = "MATCHED".into();
    store::insert_batch(&mut c, "test", "test", &[h]).unwrap();
    // Even a snapshot with identical observable stakes cannot verify a model.
    c.execute(
        "UPDATE study_decisions SET data=?1,strategy_data=?2 WHERE hand=1 AND street='preflop'",
        [
            serde_json::to_string(&d).unwrap(),
            strategy::aggregate_facts(&d).unwrap(),
        ],
    )
    .unwrap();
    let m = strategy::matrix(&c, id, &node.id, &Filter::default()).unwrap();
    let aa = m["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["hand"] == "AA")
        .unwrap();
    assert_eq!(aa["observed"]["matched"], 0);
    let rows = strategy::leaks(&c, &Filter::default(), id).unwrap();
    assert!(rows.is_empty());
    assert_eq!(aa["observed"]["reference_expected"]["R2.5"], 0.5);
    d.size_bb = Some(3.);
    assert!(strategy::action_code(&d, node).is_none());
}

#[test]
fn benchmarks_have_zero_sample_nulls_and_real_wilson_intervals() {
    let (_dir, c) = database();
    assert!(study::wilson(0, 0).is_none());
    let (lo, hi) = study::wilson(50, 100).unwrap();
    assert!((lo - 40.383).abs() < 0.001);
    assert!((hi - 59.617).abs() < 0.001);
    let mut b = json!({"name":"River folds","spot":{"street":"river"},"action":"fold","low":30,"high":50,"min_samples":100,"note":"Custom target"});
    study::save_item(&c, "benchmark", None, b.clone()).unwrap();
    let result = study::leaks(&c, &Filter::default(), None).unwrap();
    assert!(result["rows"][0]["actual"].is_null());
    assert_eq!(result["rows"][0]["priority"], 0.);
    b["low"] = json!(60);
    assert!(study::save_item(&c, "benchmark", None, b).is_err());
}

#[test]
fn trainer_hides_answer_until_submission_and_resumes_idempotently() {
    let (dir, mut c) = database();
    let path = pack_fixture(dir.path());
    let p = strategy::import(&c, path.to_str().unwrap()).unwrap();
    let pack = p["id"].as_str().unwrap();
    assert_eq!(
        training::enqueue(&c, &[], Some(pack), Some("UTG:"), Some("AA")).unwrap()["added"],
        1
    );
    assert_eq!(
        training::enqueue(&c, &[], Some(pack), Some("UTG:"), Some("AA")).unwrap()["duplicates"],
        1
    );
    let q = training::start(&c, 20).unwrap();
    assert!(q["feedback"].is_null());
    assert!(q["question"].get("frequencies").is_none());
    assert!(q["question"].get("observed").is_none());
    let id = q["id"].as_str().unwrap();
    let card = q["card_id"].as_str().unwrap();
    assert_eq!(training::start(&c, 20).unwrap(), q);
    assert!(training::rate(&mut c, id, card, "advance").is_err());
    let invalid = training::TrainingAnswer::Frequency {
        frequencies: BTreeMap::from([("F".into(), 20.), ("R2.5".into(), 20.)]),
        note: String::new(),
    };
    assert!(training::answer(&c, id, card, &invalid).is_err());
    let input = training::TrainingAnswer::Frequency {
        frequencies: BTreeMap::from([("F".into(), 50.), ("R2.5".into(), 50.)]),
        note: "My reason".into(),
    };
    let a = training::answer(&c, id, card, &input).unwrap();
    assert_eq!(a["feedback"]["feedback"]["max_gap"], 0.);
    assert_eq!(a["feedback"]["expected"]["F"], 0.5);
    assert_eq!(training::answer(&c, id, card, &invalid).unwrap(), a);
    assert_eq!(
        training::rate(&mut c, id, card, "advance").unwrap()["complete"],
        true
    );
    training::rate(&mut c, id, card, "advance").unwrap();
    assert_eq!(training::state(&c).unwrap()["completed"], 1);
    assert_eq!(training::state(&c).unwrap()["due"], 0);
    let backup = dir.path().join("backup.db");
    store::backup(&c, &backup).unwrap();
    c.execute("DELETE FROM study_cards", []).unwrap();
    store::restore(&mut c, &backup).unwrap();
    assert_eq!(training::state(&c).unwrap()["total"], 1);
}

#[test]
fn historical_self_review_and_rebuild_keep_stable_references() {
    let (_dir, mut c) = database();
    let h = reraise();
    let reference = decision::derive(&h)[1].reference.clone();
    store::insert_batch(&mut c, "test", "test", std::slice::from_ref(&h)).unwrap();
    store::save_annotation(
        &mut c,
        1,
        &Annotation {
            note: "Saved reasoning".into(),
            ..Annotation::default()
        },
    )
    .unwrap();
    training::enqueue(&c, std::slice::from_ref(&reference), None, None, None).unwrap();
    let q = training::start(&c, 1).unwrap();
    assert_eq!(q["source"], "self");
    assert!(q.to_string().find("Saved reasoning").is_none());
    let id = q["id"].as_str().unwrap();
    let card = q["card_id"].as_str().unwrap();
    let input = training::TrainingAnswer::Action {
        action: "call".into(),
        note: "Review".into(),
    };
    let a = training::answer(&c, id, card, &input).unwrap();
    assert_eq!(a["feedback"]["observed"], "fold");
    assert_eq!(a["feedback"]["source_note"], "Saved reasoning");
    assert!(a["feedback"]["expected"].is_null());
    let (_other, mut rebuilt) = database();
    let mut filler = h.clone();
    filler.id = "FIRST".into();
    store::insert_batch(&mut rebuilt, "test", "test", &[filler, h]).unwrap();
    study::copy_user_data(&c, &rebuilt).unwrap();
    assert_eq!(study::resolve(&rebuilt, &reference).unwrap().0, 2);
    assert_eq!(training::current(&rebuilt, id).unwrap()["stale"], false);
    assert_eq!(
        training::current(&rebuilt, id).unwrap()["feedback"]["replay_hand"],
        2
    );
    rebuilt.execute("DELETE FROM hands WHERE id=2", []).unwrap();
    assert_eq!(training::current(&rebuilt, id).unwrap()["stale"], true);
    assert_eq!(
        training::rate(&mut rebuilt, id, card, "skip").unwrap()["complete"],
        true
    );
}

#[test]
fn backfill_is_bounded_resumable_and_versioned() {
    let (_dir, mut c) = database();
    let hands: Vec<_> = (0..3)
        .map(|i| parser::parse(&fixtures::cash_hand(i), &Profile::default()).unwrap())
        .collect();
    store::insert_batch(&mut c, "test", "test", &hands).unwrap();
    c.execute("DELETE FROM study_indexed", []).unwrap();
    c.execute("DELETE FROM study_decisions", []).unwrap();
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 0);
    assert_eq!(study::backfill(&mut c, 1).unwrap(), 1);
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 1);
    assert_eq!(study::backfill(&mut c, 50).unwrap(), 2);
    assert_eq!(study::backfill(&mut c, 50).unwrap(), 0);
    assert_eq!(study::coverage(&c).unwrap()["complete"], true);
}

#[test]
fn imported_nl10_hand_remains_reference_without_model_verification() {
    let (dir, mut c) = database();
    let path = pack_fixture(dir.path());
    let pack = strategy::import(&c, path.to_str().unwrap()).unwrap();
    let id = pack["id"].as_str().unwrap();
    let raw = include_str!("../../../tests/fixtures/study-nl10.txt");
    let h = parser::parse(raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    let reference = decision::derive(&h)[0].reference.clone();
    store::insert_batch(&mut c, "test", "study-nl10.txt", std::slice::from_ref(&h)).unwrap();
    let result = strategy::compare_decision(&c, id, &reference, None).unwrap();
    assert_eq!(result["status"], "reference", "{result}");
    assert_eq!(result["observed_action"], "R2.5");
    let mut short = h;
    short.id = "SHORT".into();
    short.players[0].stack = 9_900_000;
    store::insert_batch(&mut c, "test", "study-nl10.txt", &[short]).unwrap();
    let matrix = strategy::matrix(&c, id, "UTG:", &Filter::default()).unwrap();
    let aa = &matrix["cells"][0];
    assert_eq!(aa["observed"]["opportunities"], 2);
    assert_eq!(aa["observed"]["matched"], 0);
    assert_eq!(matrix["reasons"]["stack_depth"], 1);
}

#[test]
fn schema_two_upgrade_creates_backup_and_preserves_user_state() {
    let (dir, mut c) = database();
    let h = reraise();
    store::insert_batch(&mut c, "test", "test", &[h]).unwrap();
    c.execute_batch("DROP TABLE study_decisions; DROP TABLE study_indexed; DROP TABLE study_items; DROP TABLE study_packs; DROP TABLE study_cards; DROP TABLE study_sessions; DROP TABLE study_attempts; UPDATE metadata SET value='2' WHERE key='schema_version';").unwrap();
    drop(c);
    store::init(&dir.path().join("study.db")).unwrap();
    let backup = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-study-")
        })
        .unwrap();
    let original = Connection::open(&backup).unwrap();
    assert_eq!(
        original
            .query_row(
                "SELECT value FROM metadata WHERE key='schema_version'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "2"
    );
    drop(original);
    let mut c = store::open(&dir.path().join("study.db")).unwrap();
    assert_eq!(study::coverage(&c).unwrap()["indexed"], 0);
    study::backfill(&mut c, 50).unwrap();
    study::save_item(
        &c,
        "spot",
        Some("saved"),
        json!({"name":"Saved","spot":{"street":"preflop"}}),
    )
    .unwrap();
    let current = dir.path().join("current.db");
    store::backup(&c, &current).unwrap();
    store::restore(&mut c, &backup).unwrap();
    assert_eq!(study::coverage(&c).unwrap()["total"], 1);
    assert_eq!(
        study::items(&c, "spot").unwrap().as_array().unwrap().len(),
        0
    );
    store::restore(&mut c, &current).unwrap();
    assert_eq!(study::items(&c, "spot").unwrap()[0]["id"], "saved");
}
