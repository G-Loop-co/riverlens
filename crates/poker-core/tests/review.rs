use poker_core::{
    fixtures,
    model::{Filter, Hand, Profile},
    money, parser, review, store,
};

// Six players, BTN opens, SB folds, Hero in BB 3-bets and wins preflop.
fn blind_defence(id: &str, day: u8, table: &str, bb: i64, open: i64) -> Hand {
    let sb = if bb == 50_000 { 20_000 } else { 10_000 };
    let raise_to = 3 * open;
    let pot = 2 * open + sb;
    let raw = format!(
        "Poker Hand #{id}: Hold'em No Limit (${}/${}) - 2026/09/{day:02} 12:00:00\n\
         Table '{table}' 6-max Seat #6 is the button\n\
         Seat 1: Small (${stack} in chips)\nSeat 2: Hero (${stack} in chips)\n\
         Seat 3: UTG (${stack} in chips)\nSeat 4: HJ (${stack} in chips)\n\
         Seat 5: CO (${stack} in chips)\nSeat 6: Button (${stack} in chips)\n\
         Small: posts small blind ${}\nHero: posts big blind ${}\n*** HOLE CARDS ***\n\
         Dealt to Small\nDealt to Hero [Kh Kd]\nDealt to UTG\nDealt to HJ\nDealt to CO\nDealt to Button\n\
         UTG: folds\nHJ: folds\nCO: folds\nButton: raises ${} to ${}\nSmall: folds\n\
         Hero: raises ${} to ${}\nButton: folds\nUncalled bet (${}) returned to Hero\n\
         Hero collected ${} from pot\n*** SUMMARY ***\nTotal pot ${} | Rake $0 | Jackpot $0\n",
        money::format(sb), money::format(bb), money::format(sb), money::format(bb),
        money::format(open - bb), money::format(open), money::format(raise_to - open),
        money::format(raise_to), money::format(raise_to - open), money::format(pot),
        money::format(pot), stack = money::format(100 * bb),
    );
    let hand = parser::parse(&raw, &Profile::default()).unwrap();
    assert_eq!(hand.status, "valid", "{:?}", hand.issues);
    hand
}

#[test]
fn parsed_cash_review_cohorts_sessions_and_positional_replay_match() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("review.db");
    store::init(&db).unwrap();
    let mut c = store::open(&db).unwrap();
    let empty = review::dashboard(&c, &Filter::default()).unwrap();
    assert_eq!(empty["has_hands"], false);
    assert!(empty["latest"].is_null());

    let rush_small = blind_defence("RUSH25", 6, "RushPractice", 20_000, 50_000);
    let rush_large = blind_defence("RUSH3", 4, "RushPractice", 20_000, 60_000);
    let regular = blind_defence("REG25", 3, "RegularPractice", 20_000, 50_000);
    let nl5 = blind_defence("RUSH5", 2, "RushPractice", 50_000, 150_000);
    let short = parser::parse(&fixtures::hu_hand(
        "Hero: folds\nUncalled bet ($0.01) returned to Villain\nVillain collected $0.02 from pot",
        "Total pot $0.02 | Rake $0 | Jackpot $0",
    ), &Profile::default()).unwrap();
    let last_time = rush_small.played_at;
    let hero_seq = rush_small
        .actions
        .iter()
        .position(|a| a.actor == Some(rush_small.hero_seat) && a.kind == "raise")
        .unwrap();
    store::insert_batch(&mut c, "one", "a-latest.txt", &[rush_small, rush_large]).unwrap();
    store::insert_batch(&mut c, "two", "z-earlier.txt", &[regular, nl5, short]).unwrap();
    let all = review::dashboard(&c, &Filter::default()).unwrap();
    assert_eq!(all["has_hands"], true);
    let positions = all["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 3);
    for (game, stakes, count) in [
        ("Cash", "0.01/0.02", 1),
        ("Rush", "0.01/0.02", 2),
        ("Rush", "0.02/0.05", 1),
    ] {
        let row = positions
            .iter()
            .find(|r| r["game"] == game && r["stakes"] == stakes)
            .unwrap();
        assert_eq!(row["currency"], "USD");
        assert_eq!(row["position"], "BB");
        assert_eq!(row["hands"], count);
        assert_eq!(
            row["metrics"]["three_bet"],
            serde_json::json!([count, count])
        );
    }
    assert_eq!(
        all["latest"]["session"],
        format!("{}:a-latest.txt", Profile::default().id)
    );
    assert_eq!(all["latest"]["hands"], 2);
    assert_eq!(all["latest"]["time"], last_time);
    assert_eq!(all["latest"]["net"], "0.13");

    let mut filter = Filter {
        currency: Some("USD".into()),
        stakes: Some("0.01/0.02".into()),
        game: Some("Rush".into()),
        ..Filter::default()
    };
    for (open_bb, expected_id) in [("2.5", "RUSH25"), ("3", "RUSH3")] {
        let spot = review::blind_spot(&c, &filter, open_bb).unwrap();
        assert_eq!(spot["n"], 1);
        assert_eq!(spot["raises"], 1);
        assert_eq!(spot["calls"], 0);
        let spot = &spot["rows"][0];
        assert_eq!(spot["hand_id"], expected_id);
        assert_eq!(spot["seq"], hero_seq);
        let hand = store::get_hand(&c, spot["id"].as_i64().unwrap()).unwrap();
        let at = &hand.actions[hero_seq];
        assert_eq!(
            (at.actor, at.kind.as_str()),
            (Some(hand.hero_seat), "raise")
        );
    }
    assert!(review::blind_spot(&c, &filter, "22").is_err());
    assert!(review::blind_spot(&c, &Filter::default(), "2.5").is_err());
    assert_eq!(review::blind_spot(&c, &filter, "2").unwrap()["n"], 0);
    filter.source_date = Some("2026-09-05".into());
    let filtered = review::dashboard(&c, &filter).unwrap();
    assert_eq!(filtered["has_hands"], true);
    assert!(filtered["latest"].is_null());
    assert!(filtered["positions"].as_array().unwrap().is_empty());
}
