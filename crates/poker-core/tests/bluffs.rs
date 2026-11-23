use poker_core::{
    bluffs, fixtures,
    model::{Hand, Profile},
    parser,
    service::{Request, Service},
    store, study,
};
use rusqlite::Connection;
use serde_json::{json, Value};

fn parse(raw: &str) -> Hand {
    let h = parser::parse(raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    h
}
fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("fixture.db");
    store::init(&p).unwrap();
    (dir, store::open(&p).unwrap())
}
fn insert(c: &mut Connection, h: &Hand) {
    store::insert_batch(c, "test", "synthetic", std::slice::from_ref(h)).unwrap();
}
fn query(c: &mut Connection, args: Value) -> Value {
    bluffs::query(c, &serde_json::from_value(args).unwrap()).unwrap()
}
fn group<'a>(result: &'a Value, category: &str) -> &'a Value {
    result["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["category"] == category)
        .unwrap_or_else(|| panic!("missing {category} in {result}"))
}
fn line(mask: u8, turn: &str, river: &str) -> String {
    let mut actions = "Hero: calls $0.01\nVillain: checks\n".to_owned();
    let streets = [
        ("FLOP", "[2h 7h Kc]".to_owned(), 2),
        ("TURN", format!("[2h 7h Kc] [{turn}]"), 4),
        ("RIVER", format!("[2h 7h Kc {turn}] [{river}]"), 8),
    ];
    let mut pot = 4;
    for (index, (name, board, b)) in streets.iter().enumerate() {
        actions += &format!("*** {name} *** {board}\nVillain: checks\n");
        if mask & (1 << index) == 0 {
            actions += "Hero: checks\n";
            continue;
        }
        actions += &format!("Hero: bets $0.{b:02}\n");
        if index == 2 {
            actions += &format!("Villain: folds\nUncalled bet ($0.{b:02}) returned to Hero\n");
        } else {
            actions += &format!("Villain: calls $0.{b:02}\n");
            pot += 2 * b;
        }
    }
    let winner = if mask & 4 != 0 { "Hero" } else { "Villain" };
    actions += &format!(
        "*** SHOWDOWN ***\n{winner} collected $0.{:02} from pot",
        pot - 1
    );
    fixtures::hu_hand(
        &actions,
        &format!("Total pot $0.{pot:02} | Rake $0.01 | Jackpot $0"),
    )
    .replace("[As Ad]", "[Qh Jh]")
}

#[test]
fn three_shots_reconcile_and_future_cards_never_reclassify_previous_streets() {
    let (_dir, mut c) = db();
    let h = parse(&line(7, "4s", "8d"));
    insert(&mut c, &h);
    let singles = query(&mut c, json!({"view":"single"}));
    let flop = group(&singles, "flop_draw");
    assert_eq!(
        (flop["segment_bb"].as_f64(), flop["net_bb"].as_f64()),
        (Some(-1.), Some(3.5))
    );
    let turn = group(&singles, "turn_draw");
    assert_eq!(turn["segment_bb"], -2.);
    let river = group(&singles, "river_air");
    assert_eq!(river["segment_bb"], 7.5);
    assert_eq!(river["folded"], 1);
    let whole = query(&mut c, json!({"view":"line"}));
    assert_eq!(group(&whole, "flop_turn_river")["segment_bb"], 4.5);
    let reached = query(&mut c, json!({"view":"reached"}));
    assert_eq!(group(&reached, "flop_turn")["segment_bb"], -3.);
    let opportunities = query(&mut c, json!({"view":"next","sizing":"medium"}));
    assert_eq!(group(&opportunities, "turn_river_continue")["attempts"], 1);
    let detail = query(
        &mut c,
        json!({"view":"single","draw":"missed","cell":{"category":"river_air","bucket":""}}),
    );
    assert_eq!(detail["rows"][0]["missed"], true);
    let seq = detail["rows"][0]["seq"].as_u64().unwrap() as usize;
    assert_eq!(
        (&h.actions[seq].street, &h.actions[seq].kind),
        (&"river".into(), &"bet".into())
    );
    let h2 = parse(&line(7, "4s", "8h").replace("TEST1", "SECOND"));
    insert(&mut c, &h2);
    let singles = query(&mut c, json!({"view":"single"}));
    assert_eq!(group(&singles, "turn_draw")["attempts"], 2);
    assert_eq!(group(&singles, "river_air")["attempts"], 1);
    let opportunities = query(&mut c, json!({"view":"next"}));
    assert_eq!(group(&opportunities, "turn_river_made")["attempts"], 1);
}

#[test]
fn double_routes_and_checked_through_continuations() {
    for (mask, key) in [(3, "flop_turn"), (5, "flop_river"), (6, "turn_river")] {
        let (_dir, mut c) = db();
        insert(&mut c, &parse(&line(mask, "4s", "8d")));
        let full = query(&mut c, json!({"view":"line"}));
        assert_eq!(full["groups"].as_array().unwrap().len(), 1);
        assert_eq!(group(&full, key)["attempts"], 1);
        if mask == 5 {
            let next = query(&mut c, json!({"view":"next"}));
            assert_eq!(group(&next, "flop_turn_stop")["attempts"], 1);
            assert_eq!(group(&next, "flop_river_continue")["attempts"], 1);
        }
    }
}

#[test]
fn multiway_partial_fold_and_new_heads_up_street() {
    let raw = "Poker Hand #MULTI: Hold'em No Limit ($0.01/$0.02) - 2026/09/01 12:00:00
Table 'Synthetic' 3-max Seat #3 is the button
Seat 1: SB ($2 in chips)
Seat 2: BB ($2 in chips)
Seat 3: Hero ($2 in chips)
SB: posts small blind $0.01
BB: posts big blind $0.02
*** HOLE CARDS ***
Dealt to SB
Dealt to BB
Dealt to Hero [Qh Jh]
Hero: calls $0.02
SB: calls $0.01
BB: checks
*** FLOP *** [2h 7h Kc]
SB: checks
BB: checks
Hero: bets $0.02
SB: folds
BB: calls $0.02
*** TURN *** [2h 7h Kc] [4s]
BB: checks
Hero: bets $0.04
BB: folds
Uncalled bet ($0.04) returned to Hero
Hero collected $0.09 from pot
*** SUMMARY ***
Total pot $0.10 | Rake $0.01 | Jackpot $0";
    let (_dir, mut c) = db();
    let h = parse(raw);
    insert(&mut c, &h);
    let multi = query(
        &mut c,
        json!({"view":"single","players":"multi","breakdown":"response"}),
    );
    assert_eq!(group(&multi, "flop_draw")["bucket"], "called");
    assert_eq!(group(&multi, "flop_draw")["folded"], 0);
    let hu = query(&mut c, json!({"view":"single","players":"hu"}));
    assert_eq!(group(&hu, "turn_draw")["folded"], 1);
    assert_eq!(group(&hu, "turn_draw")["segment_bb"], 4.5);
    assert!(study::decision::derive(&h)
        .iter()
        .all(|d| d.street == "preflop"));
}

#[test]
fn all_in_runout_and_split_only_exclude_the_affected_metric() {
    let a="Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2h 7h Kc]\nVillain: checks\nHero: bets $1.98 and is all-in\nVillain: calls $1.98 and is all-in\n*** TURN *** [2h 7h Kc] [4s]\n*** RIVER *** [2h 7h Kc 4s] [8d]\n*** SHOWDOWN ***\nHero collected $2 from pot\nVillain collected $2 from pot";
    let raw =
        fixtures::hu_hand(a, "Total pot $4 | Rake $0 | Jackpot $0").replace("[As Ad]", "[Qh Jh]");
    let (_dir, mut c) = db();
    insert(&mut c, &parse(&raw));
    let report = query(&mut c, json!({}));
    let f = group(&report, "flop_draw");
    assert!(f["segment_bb"].is_null());
    assert_eq!(f["settled"], 0);
    assert_eq!(f["net_bb"], 0.);
    assert_eq!(report["exclusions"]["split"], 1);
    assert!(query(&mut c, json!({"view":"next"}))["groups"]
        .as_array()
        .unwrap()
        .is_empty());
    let raw = raw.replace("TEST1", "WIN").replace(
        "Hero collected $2 from pot\nVillain collected $2 from pot",
        "Hero collected $4 from pot",
    );
    insert(&mut c, &parse(&raw));
    let report = query(&mut c, json!({}));
    assert_eq!(group(&report, "flop_draw")["segment_bb"], 101.);
}

#[test]
fn repeated_aggression_is_one_street_and_each_action_has_its_own_replay() {
    let (_dir, mut c) = db();
    let a="Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2h 7h Kc]\nVillain: checks\nHero: bets $0.02\nVillain: raises $0.04 to $0.06\nHero: raises $0.08 to $0.14\nVillain: folds\nUncalled bet ($0.08) returned to Hero\nHero collected $0.15 from pot";
    let raw = fixtures::hu_hand(a, "Total pot $0.16 | Rake $0.01 | Jackpot $0")
        .replace("[As Ad]", "[Qh Jh]");
    let h = parse(&raw);
    insert(&mut c, &h);
    let report = query(&mut c, json!({}));
    let f = group(&report, "flop_draw");
    assert_eq!(f["attempts"], 1);
    assert_eq!(f["folded"], 0);
    assert_eq!(f["segment_bb"], 4.5);
    let detail = query(&mut c, json!({"cell":{"category":"flop_draw","bucket":""}}));
    let row = &detail["rows"][0];
    assert_eq!(row["response"], "raised");
    assert_eq!(row["followups"].as_array().unwrap().len(), 1);
    let later = row["followups"][0]["seq"].as_u64().unwrap() as usize;
    assert_eq!(h.actions[later].kind, "raise");
    assert_eq!(row["followups"][0]["size"], 50.);
}

#[test]
fn check_raise_lead_is_its_own_line() {
    let (_dir, mut c) = db();
    let text=fixtures::hu_hand("Villain: calls $0.01\nHero: checks\n*** FLOP *** [2h 7h Kc]\nHero: checks\nVillain: bets $0.02\nHero: raises $0.04 to $0.06\nVillain: calls $0.04\n*** TURN *** [2h 7h Kc] [4s]\nHero: bets $0.08\nVillain: folds\nUncalled bet ($0.08) returned to Hero\nHero collected $0.15 from pot","Total pot $0.16 | Rake $0.01 | Jackpot $0")
        .replace("Seat #1 is the button","Seat #2 is the button").replace("Hero: posts small blind","Villain: posts small blind").replace("Villain: posts big blind","Hero: posts big blind").replace("[As Ad]","[Qh Jh]");
    insert(&mut c, &parse(&text));
    let report = query(&mut c, json!({"view":"line"}));
    assert_eq!(group(&report, "flop_turn_xr_lead")["attempts"], 1);
    let check_raise = query(&mut c, json!({"view":"single","action":"check_raise"}));
    assert_eq!(group(&check_raise, "flop_draw")["attempts"], 1);
}

#[test]
fn missing_cards_are_reported_and_mixed_currency_stays_in_bb() {
    let (_dir, mut c) = db();
    let raw = line(7, "4s", "8d");
    let unknown = parser::parse(
        &raw.replace("Dealt to Hero [Qh Jh]", "Dealt to Hero"),
        &Profile::default(),
    )
    .unwrap();
    assert_ne!(unknown.status, "valid");
    insert(&mut c, &unknown);
    let report = query(&mut c, json!({}));
    assert_eq!(report["unknown"], 3);
    assert!(report["groups"].as_array().unwrap().is_empty());
    for (id, currency) in [("E", "EUR"), ("U", "USD")] {
        let mut h = parse(&raw.replace("TEST1", id));
        h.currency = currency.into();
        insert(&mut c, &h);
    }
    let report = query(&mut c, json!({}));
    let group_all = group(&report, "flop_draw");
    assert!(group_all["net"].is_null());
    assert_eq!(group_all["net_bb"], 7.);
    assert_eq!(group_all["hands"], 2);
    let usd = query(&mut c, json!({"filter":{"currency":"USD"}}));
    assert_eq!(group(&usd, "flop_draw")["net"], "0.07");
}

#[test]
fn other_river_air_omits_early_streets_and_missed_draws() {
    let (_dir, mut c) = db();
    insert(&mut c, &parse(&line(7, "4s", "8d")));
    insert(
        &mut c,
        &parse(
            &line(7, "4s", "8d")
                .replace("TEST1", "OTHER")
                .replace("Qh Jh", "Ac 9d"),
        ),
    );
    let result = query(&mut c, json!({"draw":"other_air"}));
    assert_eq!(result["groups"].as_array().unwrap().len(), 1);
    assert_eq!(group(&result, "river_air")["attempts"], 1);
}

#[test]
fn special_payouts_keep_observed_attempts_and_hand_net() {
    let (_dir, mut c) = db();
    let action="Hero: calls $0.01\nVillain: checks\n*** FLOP *** [2h 7h Kc]\nVillain: checks\nHero: bets $1.98 and is all-in\nVillain: calls $1.98 and is all-in";
    let paid = "*** SHOWDOWN ***\nHero collected $3.90 from pot";
    let board = "*** TURN *** [4s]\n*** RIVER *** [8h]";
    let extra="*** FIRST TURN *** [4s]\n*** FIRST RIVER *** [8h]\n*** SECOND TURN *** [9d]\n*** SECOND RIVER *** [Th]";
    for (id, tail, reason) in [
        (
            "C",
            format!(
                "Hero: Chooses to EV Cashout\n{board}\n{paid}\nHero: Pays Cashout Risk ($0.70)"
            ),
            "cashout",
        ),
        ("R", format!("{extra}\n{paid}"), "runouts"),
    ] {
        let raw = fixtures::hu_hand(
            &format!("{action}\n{tail}"),
            "Total pot $4 | Rake $0.10 | Jackpot $0",
        )
        .replace("TEST1", id)
        .replace("As Ad", "Qh Jh");
        let h = parse(&raw);
        insert(&mut c, &h);
        let result = query(&mut c, json!({}));
        assert_eq!(result["exclusions"][reason], 1);
        assert_eq!(group(&result, "flop_draw")["settled"], 0);
    }
    let result = query(&mut c, json!({}));
    assert_eq!(group(&result, "flop_draw")["attempts"], 2);
    assert_eq!(group(&result, "flop_draw")["net"], "3.10");
}

#[test]
fn side_pot_with_distinct_winners_has_an_explicit_settlement_exclusion() {
    let raw = "Poker Hand #SIDE: Hold'em No Limit ($0.01/$0.02) - 2026/09/01 12:00:00
Table 'Synthetic' 3-max Seat #3 is the button
Seat 1: Small ($0.06 in chips)
Seat 2: Big ($0.04 in chips)
Seat 3: Hero ($0.06 in chips)
Small: posts small blind $0.01
Big: posts big blind $0.02
*** HOLE CARDS ***
Dealt to Small
Dealt to Big
Dealt to Hero [Qh Jh]
Hero: calls $0.02
Small: calls $0.01
Big: checks
*** FLOP *** [2h 7h Kc]
Small: checks
Big: checks
Hero: bets $0.04 and is all-in
Small: calls $0.04 and is all-in
Big: calls $0.02 and is all-in
*** TURN *** [4s]
*** RIVER *** [8d]
*** SHOWDOWN ***
Big collected $0.12 from main pot
Hero collected $0.04 from side pot
*** SUMMARY ***
Total pot $0.16 | Rake $0 | Jackpot $0";
    let (_dir, mut c) = db();
    insert(&mut c, &parse(raw));
    let report = query(&mut c, json!({"players":"3"}));
    let f = group(&report, "flop_draw");
    assert_eq!(report["exclusions"]["side_pot"], 1);
    assert_eq!(f["net_bb"], -1.);
    assert!(f["segment_bb"].is_null());
}

#[test]
fn cell_pagination_matches_filtered_totals() {
    let (_dir, mut c) = db();
    let raw = line(3, "4s", "8d");
    for n in 0..51 {
        insert(&mut c, &parse(&raw.replace("TEST1", &format!("P{n}"))));
    }
    let report = query(&mut c, json!({"breakdown":"size"}));
    let f = group(&report, "flop_draw");
    let mut request =
        json!({"breakdown":"size","cell":{"category":"flop_draw","bucket":f["bucket"]}});
    let first = query(&mut c, request.clone());
    request["before"] = first["next"].clone();
    let second = query(&mut c, request);
    let rows = first["rows"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["rows"].as_array().unwrap());
    assert_eq!(
        rows.map(|r| r["id"].as_i64().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        51
    );
    assert!(second["next"].is_null());
    assert_eq!(f["attempts"], 51);
}

#[test]
fn manual_marks_survive_rebuild_backfill_and_identical_imports() {
    let (dir, mut c) = db();
    let h = parse(&line(7, "4s", "8d"));
    insert(&mut c, &h);
    insert(&mut c, &h);
    let seq = h
        .actions
        .iter()
        .find(|a| a.kind == "bet" && a.street == "river")
        .unwrap()
        .seq;
    let service = Service::new(dir.path().join("fixture.db")).unwrap();
    service
        .handle(Request::StudyControl { paused: true })
        .unwrap();
    service
        .handle(Request::BluffMark {
            id: 1,
            seq,
            marked: true,
        })
        .unwrap();
    assert!(service
        .handle(Request::BluffMark {
            id: 1,
            seq: 0,
            marked: true
        })
        .is_err());
    c.execute("DELETE FROM study_indexed", []).unwrap();
    assert_eq!(query(&mut c, json!({}))["coverage"]["complete"], false);
    assert_eq!(study::backfill(&mut c, 1).unwrap(), 1);
    assert_eq!(study::backfill(&mut c, 1).unwrap(), 0);
    service.handle(Request::Rebuild).unwrap();
    let report = query(&mut c, json!({"view":"manual"}));
    assert_eq!(group(&report, "river_air")["attempts"], 1);
    assert_eq!(
        service.handle(Request::Hand { id: 1 }).unwrap()["bluff_marks"],
        json!([seq])
    );
    service
        .handle(Request::BluffMark {
            id: 1,
            seq,
            marked: false,
        })
        .unwrap();
    assert!(query(&mut c, json!({"view":"manual"}))["groups"]
        .as_array()
        .unwrap()
        .is_empty());
}
