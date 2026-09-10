use poker_core::{equity, fixtures, model::*, money, parser, store};
use std::sync::atomic::AtomicBool;

fn hu(actions: &str, total: &str) -> Hand {
    parser::parse(&fixtures::hu_hand(actions, total), &Profile::default()).unwrap()
}
#[test]
fn synthetic_fixture_conservation_all_seats() {
    for i in 0..30 {
        let h = parser::parse(&fixtures::cash_hand(i), &Profile::default()).unwrap();
        assert_eq!(h.status, "valid", "{i}: {:?}", h.issues);
        assert!(h.showdown);
        assert_eq!(h.player_count, 6);
        assert!((h.invested - h.returned) == money::parse("0.12").unwrap());
    }
}
#[test]
fn preflop_fold_marker_is_not_showdown() {
    let h=hu("Hero: folds\nUncalled bet ($0.01) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.02 from pot","Total pot $0.02 | Rake $0 | Jackpot $0");
    assert_eq!(h.status, "valid");
    assert!(!h.showdown);
    assert!(!h.saw_flop);
    assert_eq!(h.net, -10_000);
    assert_eq!(h.stats["vpip"].opportunities, 1);
    assert_eq!(h.stats["vpip"].numerator, 0);
}
#[test]
fn big_blind_walk_has_no_voluntary_opportunity() {
    let raw=fixtures::hu_hand("Hero: folds\nUncalled bet ($0.01) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.02 from pot","Total pot $0.02 | Rake $0 | Jackpot $0");
    let p = Profile {
        hero: "Villain".into(),
        ..Profile::default()
    };
    let h = parser::parse(&raw, &p).unwrap();
    assert_eq!(h.stats["vpip"].opportunities, 0);
    assert_eq!(h.stats["pfr"].opportunities, 0);
    assert!(!h.showdown);
}
#[test]
fn sb_complete_and_bb_check() {
    let h=hu("Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2c 3d 4h]\nVillain: bets $0.02\nHero: folds\nUncalled bet ($0.02) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.04 from pot","Total pot $0.04 | Rake $0 | Jackpot $0");
    assert_eq!(h.stats["vpip"].numerator, 1);
    assert_eq!(h.stats["pfr"].numerator, 0);
    assert!(!h.showdown);
    assert!(h.saw_flop);
}
#[test]
fn return_is_not_profit_and_summary_not_double_counted() {
    let h=hu("Hero: raises $0.04 to $0.06\nVillain: folds\nUncalled bet ($0.04) returned to Hero\n*** SHOWDOWN ***\nHero collected $0.04 from pot","Total pot $0.04 | Rake $0 | Jackpot $0\nSeat 1: Hero won ($0.04)");
    assert_eq!(h.status, "valid");
    assert_eq!(h.net, 20_000);
    assert_eq!(h.collected, 40_000);
    assert_eq!(h.returned, 40_000);
}
#[test]
fn double_straddle_is_total_to() {
    let h=hu("Hero: straddle $0.04\nHero: straddle $0.08\nVillain: folds\nUncalled bet ($0.06) returned to Hero\n*** SHOWDOWN ***\nHero collected $0.04 from pot","Total pot $0.04 | Rake $0 | Jackpot $0");
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.invested, 80_000);
    assert_eq!(h.net, 20_000);
    assert_eq!(h.pot_type, "special");
}
#[test]
fn inconsistent_source_is_quarantined() {
    let h=hu("Hero: raises $0.04 to $0.06\nVillain: calls $0.04\n*** FLOP *** [2c 3d 4h]\nVillain: bets $0.04\nHero: calls $0.22\n*** SHOWDOWN ***\nHero collected $0.56 from pot","Total pot $0.56 | Rake $0 | Jackpot $0");
    assert_eq!(h.status, "quarantined");
    assert!(h.issues.iter().any(|i| i.code == "pot_mismatch"));
    assert_eq!(h.ev_status, "excluded");
}
#[test]
fn fold_after_open_three_bet() {
    let h=hu("Hero: raises $0.04 to $0.06\nVillain: raises $0.12 to $0.18\nHero: folds\nUncalled bet ($0.12) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.12 from pot","Total pot $0.12 | Rake $0 | Jackpot $0");
    assert_eq!(h.stats["fold_three_bet"].opportunities, 1);
    assert_eq!(h.stats["fold_three_bet"].numerator, 1);
    assert_eq!(h.stats["three_bet"].opportunities, 0);
}
#[test]
fn donk_removes_cbet_opportunity() {
    let h=hu("Hero: raises $0.04 to $0.06\nVillain: calls $0.04\n*** FLOP *** [2c 3d 4h]\nVillain: bets $0.06\nHero: folds\nUncalled bet ($0.06) returned to Villain\n*** SHOWDOWN ***\nVillain collected $0.12 from pot","Total pot $0.12 | Rake $0 | Jackpot $0");
    assert_eq!(h.stats["cbet"].opportunities, 0);
    assert_eq!(h.stats["fold_cbet"].opportunities, 0);
}
#[test]
fn split_pot_counts_showdown_win() {
    let h=hu("Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2c 3d 4h]\nVillain: checks\nHero: checks\n*** TURN *** [2c 3d 4h] [5s]\nVillain: checks\nHero: checks\n*** RIVER *** [2c 3d 4h 5s] [6c]\nVillain: checks\nHero: checks\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Ks Kd]\nHero collected $0.02 from pot\nVillain collected $0.02 from pot","Total pot $0.04 | Rake $0 | Jackpot $0");
    assert!(h.showdown);
    assert_eq!(h.stats["wsd"].numerator, 1);
    assert_eq!(h.net, 0);
}
#[test]
fn cashout_outside_pot_ledger() {
    let h=hu("Hero: raises $1.98 to $2 and is all-in\nVillain: calls $1.98 and is all-in\nHero: Chooses to EV Cashout\n*** FLOP *** [2c 3d 4h]\n*** TURN *** [2c 3d 4h] [5s]\n*** RIVER *** [2c 3d 4h 5s] [Kc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Kh Kd]\nHero collected $3.90 from pot\nHero: Pays Cashout Risk ($0.70)","Total pot $4 | Rake $0.10 | Jackpot $0");
    assert_eq!(h.status, "valid");
    assert_eq!(h.net, 1_200_000);
    assert_eq!(h.ev_reason.as_deref(), Some("cashout"));
}
#[test]
fn all_in_equity_ignores_actual_runout() {
    let actions="Hero: raises $1.98 to $2 and is all-in\nVillain: calls $1.98 and is all-in\n*** FLOP *** [2c 3d 4h]\n*** TURN *** [2c 3d 4h] [5s]\n*** RIVER *** [2c 3d 4h 5s] [Kc]\n*** SHOWDOWN ***\nHero: shows [As Ad]\nVillain: shows [Kh Kd]\nHero collected $3.90 from pot";
    let a = hu(actions, "Total pot $4 | Rake $0.10 | Jackpot $0");
    let b = hu(
        &actions.replace("[Kc]", "[7c]"),
        "Total pot $4 | Rake $0.10 | Jackpot $0",
    );
    assert_eq!(a.ev_status, "pending");
    assert!(a.equity_input.as_ref().unwrap().board.is_empty());
    assert_eq!(
        serde_json::to_string(&a.equity_input).unwrap(),
        serde_json::to_string(&b.equity_input).unwrap()
    );
}
#[test]
fn exact_turn_equity_and_cancel() {
    let input = EquityInput {
        hero: vec!["As".into(), "Ad".into()],
        villain: vec!["Kh".into(), "Kd".into()],
        board: vec!["2c".into(), "3d".into(), "8h".into(), "9s".into()],
        net_pot: 4_000_000,
        contribution: 2_000_000,
    };
    let (eq, _) = equity::exact(&input, &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    assert!((eq - 42. / 44.).abs() < 1e-12);
    assert!(equity::exact(&input, &AtomicBool::new(true))
        .unwrap()
        .is_none());
}
#[test]
fn timezone_is_explicit() {
    let h = parser::parse(&fixtures::cash_hand(1), &Profile::default()).unwrap();
    let p = Profile {
        timezone: "UTC".into(),
        ..Profile::default()
    };
    let utc = parser::parse(&h.raw, &p).unwrap();
    assert_eq!(utc.played_at - h.played_at, 8 * 3600);
}
#[test]
fn database_idempotence_filters_notes_and_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    store::init(&path).unwrap();
    let mut c = store::open(&path).unwrap();
    let hands: Vec<_> = (0..20)
        .map(|i| parser::parse(&fixtures::cash_hand(i), &Profile::default()).unwrap())
        .collect();
    assert_eq!(
        store::insert_batch(&mut c, "test", "synth", &hands)
            .unwrap()
            .inserted,
        20
    );
    let first = store::report(&c, &Filter::default(), "date").unwrap();
    assert_eq!(
        store::insert_batch(&mut c, "test", "synth", &hands)
            .unwrap()
            .duplicates,
        20
    );
    assert_eq!(
        first,
        store::report(&c, &Filter::default(), "date").unwrap()
    );
    let a = Annotation {
        note: "review this".into(),
        tags: vec!["複盤".into()],
        reviewed: true,
    };
    store::save_annotation(&mut c, 1, &a).unwrap();
    assert_eq!(store::annotation(&c, 1).unwrap().tags, a.tags);
    let f = Filter {
        tag: Some("複盤".into()),
        ..Filter::default()
    };
    assert_eq!(store::report(&c, &f, "date").unwrap()["hands"], 1);
    let backup = dir.path().join("backup.db");
    store::backup(&c, &backup).unwrap();
    let image = rusqlite::Connection::open(&backup).unwrap();
    assert_eq!(
        image
            .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
    drop(image);
    assert!(
        store::backup(&c, &backup).is_err(),
        "must not overwrite an existing backup"
    );
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".riverlens-snapshot-")));
    store::save_annotation(&mut c, 1, &Annotation::default()).unwrap();
    store::restore(&mut c, &backup).unwrap();
    assert_eq!(store::annotation(&c, 1).unwrap().note, "review this");
    drop(c);
    let c = store::open(&path).unwrap();
    assert_eq!(
        store::report(&c, &Filter::default(), "date").unwrap()["hands"],
        20
    );
}
