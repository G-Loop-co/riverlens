use poker_core::{fixtures, hand_formats, model::*, money, parser, store};

fn profile(brand: &str) -> Profile {
    Profile {
        brand: brand.into(),
        ..Profile::default()
    }
}

#[test]
fn personal_room_formats_do_not_expand_gg_population_scope() {
    let observed = parser::parse_observed(&fixtures::population_hand(2), "UTC", "GG").unwrap();
    assert_eq!(observed.status, "valid");
    assert_eq!(observed.players.len(), 6);
    assert!(observed.players.iter().all(|p| !p.hero));
    for raw in [
        include_str!("fixtures/hand-formats/pokerstars.txt"),
        include_str!("fixtures/hand-formats/ipoker.xml"),
        include_str!("fixtures/tournaments/gg.txt"),
        include_str!("fixtures/tournaments/pokerstars.txt"),
    ] {
        assert_eq!(
            parser::parse(raw, &Profile::default()).unwrap().status,
            "valid"
        );
        assert!(parser::parse_observed(raw, "UTC", "GG").is_err());
    }
}

#[test]
fn public_mit_room_fixtures_conserve_money_and_preserve_source() {
    for (format, raw) in [
        (
            "pokerstars",
            include_str!("fixtures/hand-formats/pokerstars.txt"),
        ),
        (
            "fulltilt",
            include_str!("fixtures/hand-formats/fulltilt.txt"),
        ),
        (
            "partypoker",
            include_str!("fixtures/hand-formats/partypoker.txt"),
        ),
        ("888", include_str!("fixtures/hand-formats/888.txt")),
        ("winamax", include_str!("fixtures/hand-formats/winamax.txt")),
        (
            "winning-legacy",
            include_str!("fixtures/hand-formats/winning-legacy.txt"),
        ),
        ("ipoker", include_str!("fixtures/hand-formats/ipoker.xml")),
        ("ongame", include_str!("fixtures/hand-formats/ongame.txt")),
        ("merge", include_str!("fixtures/hand-formats/merge.xml")),
        (
            "microgaming",
            include_str!("fixtures/hand-formats/microgaming.xml"),
        ),
    ] {
        let h = parser::parse(raw, &profile(format)).unwrap_or_else(|e| panic!("{format}: {e:#}"));
        assert_eq!(h.status, "valid", "{format}: {:?}", h.issues);
        assert_eq!(h.raw, raw.trim(), "{format}: original source retained");
        assert!(h.parser_version.ends_with(format));
        assert!(
            h.player_count >= 2,
            "{format}: opponents must not disappear"
        );
        assert_eq!(h.boards[0].len(), 5, "{format}");
        assert_eq!(
            h.players.iter().find(|p| p.hero).unwrap().cards.len(),
            2,
            "{format}"
        );
        let expected = match format {
            "pokerstars" => "282",
            "fulltilt" => "2.76",
            "partypoker" => "0.09",
            "888" => "-6",
            "winamax" => "-22.85",
            "winning-legacy" => "109.9",
            "ipoker" => "-3.73",
            "ongame" => "6.75",
            "merge" => "206.6",
            "microgaming" => "0.59",
            _ => unreachable!(),
        };
        assert_eq!(
            h.net,
            money::parse(expected).unwrap(),
            "{format}: independently expected source result"
        );
        assert_eq!(h.net, h.collected + h.returned - h.invested, "{format}");
    }
}

#[test]
fn converted_pokerstars_and_native_coinpoker_retain_currency_and_explicit_timezone() {
    let gg=fixtures::hu_hand("Hero: raises $0.04 to $0.06\nVillain: folds\nUncalled bet ($0.04) returned to Hero\nHero collected $0.04 from pot","Total pot $0.04 | Rake $0");
    let stars = gg
        .replace("Poker Hand #TEST1:", "PokerStars Hand #TEST1:")
        .replace("($0.01/$0.02)", "($0.01/$0.02 USD)")
        .replace("2026/09/01 12:00:00", "2026/09/01 12:00:00 ET")
        .replace("Dealt to Villain\n", "");
    let h = parser::parse(&stars, &profile("PokerStars")).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.player_count, 2);
    assert_eq!(h.net, 20_000);
    assert_eq!(h.timezone, "America/New_York");
    let coin = gg
        .replace(
            "Poker Hand #TEST1: Hold'em No Limit ($0.01/$0.02) -",
            "CoinPoker Hand #TEST1: NLH (₮0.01/₮0.02/₮0) ",
        )
        .replace('$', "₮")
        .replace("2026/09/01 12:00:00", "2026/09/01 12:00:00 MSK");
    let h = parser::parse(&coin, &profile("CoinPoker")).unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.currency, "USDT");
    assert_eq!(h.timezone, "Europe/Moscow");
    assert_eq!(h.net, 20_000);
}

#[test]
fn unsupported_variants_and_broken_ledgers_never_enter_valid_statistics() {
    let raw = include_str!("fixtures/hand-formats/pokerstars.txt");
    for bad in [
        raw.replace("Hold'em No Limit", "Omaha Pot Limit"),
        raw.replace("Hold'em No Limit", "Tournament #1 Hold'em No Limit"),
        raw.replace("Total pot $565", "Total pot $999"),
    ] {
        if let Ok(h) = parser::parse(&bad, &profile("PokerStars")) {
            assert_ne!(h.status, "valid");
        }
    }
    let xml = include_str!("fixtures/hand-formats/ipoker.xml");
    assert!(parser::parse(
        &xml.replace("type=\"23\"", "type=\"99\""),
        &profile("iPoker")
    )
    .is_err());
    assert!(parser::parse(
        &format!("<!DOCTYPE session [<!ENTITY x 'x'>]>{xml}"),
        &profile("iPoker")
    )
    .is_err());
}

#[test]
fn xml_stream_splits_compact_games_and_keeps_metadata_and_stop_boundary() {
    let raw = include_str!("fixtures/hand-formats/ipoker.xml");
    let doc = roxmltree::Document::parse(raw).unwrap();
    let game = doc.descendants().find(|n| n.has_tag_name("game")).unwrap();
    let two = raw
        .replace(
            "</session>",
            &format!(
                "{}</session>",
                raw[game.range()].replace("gamecode=\"", "gamecode=\"copy-")
            ),
        )
        .replace('\n', "");
    let mut seen = vec![];
    hand_formats::stream_xml(two.as_bytes(), &mut |raw| {
        seen.push(parser::parse(raw, &profile("iPoker"))?.id);
        Ok(true)
    })
    .unwrap();
    assert_eq!(seen.len(), 2);
    assert_ne!(seen[0], seen[1]);
    let mut count = 0;
    hand_formats::stream_xml(two.as_bytes(), &mut |_| {
        count += 1;
        Ok(false)
    })
    .unwrap();
    assert_eq!(count, 1);
    assert!(hand_formats::stream_xml(b"<session><game>".as_slice(), &mut |_| Ok(true)).is_err());
}

#[test]
fn modern_text_rooms_account_for_raises_returns_walks_and_anonymous_hero() {
    let gg=fixtures::hu_hand("Hero: raises $0.04 to $0.06\nVillain: folds\nUncalled bet ($0.04) returned to Hero\nHero collected $0.04 from pot","Total pot $0.04 | Rake $0");
    let winning = gg
        .replacen(
            "Poker Hand #TEST1: Hold'em No Limit ($0.01/$0.02) -",
            "Hand #TEST1 - Holdem(No Limit) - $0.01/$0.02 -",
            1,
        )
        .replace("Table 'Test'", "Test");
    let cake="Hand#CAKE1 - Synthetic (2-max) -- $0.01/$0.02 NL Hold'em -- 2026/09/01 - 12:00:00\nSeat 1: Hero ($2 in chips) DEALER\nSeat 2: Villain ($2 in chips)\nHero: posts small blind $0.01\nVillain: posts big blind $0.02\nDealt to Hero [As,Ad]\nHero: raises to $0.06\nVillain: folds\nHero: returns uncalled bet $0.04\n***SHOW DOWN***\nHero wins $0.04";
    let unibet="Game #UNI1: Table €2 NL - 0.01/0.02 - No Limit Hold'Em Banzai - 12:00:00 2026/09/01\n*** Seated players ***\nSeat 1: Hero (€2)\nSeat 2: Villain (€2)\nHero has the button\nHero posts small blind €0.01\nVillain posts big blind €0.02\nDealt to Hero [As Ad]\nHero raises €1.99 to €2, and is all-in\nVillain folds\nHero wins €2.02\n*** Summary ***\nTotal pot €2.02 Rake €0";
    let ignition="Ignition Hand #IGN1 TBL#1 HOLDEM No Limit - 2026-09-01 12:00:00\nSeat 1: Dealer [ME] ($2 in chips)\nSeat 2: Big Blind ($2 in chips)\nDealer [ME] : Set dealer [1]\nDealer  [ME] : Small Blind $0.01\nBig Blind : Big blind $0.02\n*** HOLE CARDS ***\nDealer [ME] : Card dealt to a spot [As Ad]\nBig Blind : Card dealt to a spot [Ks Kd]\nDealer [ME] : Raises $0.04 to $0.06\nBig Blind : Folds\nDealer [ME] : Return uncalled portion of bet $0.04\nDealer [ME] : Does not show [As Ad]\nDealer [ME] : Hand result $0.04\nDealer [ME] : Leave(Auto)\n*** SUMMARY ***";
    for (format, raw) in [
        ("winning", winning.as_str()),
        ("cake", &cake.replace("Hero wins", "Hero: wins")),
        (
            "unibet",
            &unibet.replace("Hero raises", "*** Preflop ***\nHero raises"),
        ),
        ("ignition", ignition),
    ] {
        let h = parser::parse(raw, &profile(format)).unwrap_or_else(|e| panic!("{format}: {e:#}"));
        assert_eq!(h.status, "valid", "{format}: {:?}", h.issues);
        assert_eq!(h.player_count, 2);
        assert_eq!(
            h.button, 1,
            "{format}: dealer seat is an integer, not money"
        );
        assert_eq!(h.position, "BTN/SB", "{format}");
        assert_eq!(h.net, 20_000);
        assert!(!h.showdown);
        assert!(h.parser_version.ends_with(format));
        assert_eq!(h.raw, raw.trim());
    }
    let walk=winning.replace("Hero: raises $0.04 to $0.06\nVillain: folds\nUncalled bet ($0.04) returned to Hero\nHero collected $0.04 from pot","Hero: folds").replace("Total pot $0.04 | Rake $0","Total pot $0.03\nSeat 2: Villain (big blind) did not show and won $0.03");
    let walk = walk.replace("Dealt to Villain\n", "Dealt to Villain [Ks Kd]\n");
    let h = parser::parse(
        &walk,
        &Profile {
            hero: "Villain".into(),
            ..profile("ACR")
        },
    )
    .unwrap();
    assert_eq!(h.status, "valid", "{:?}", h.issues);
    assert_eq!(h.net, 10_000);
    assert!(parser::parse(cake.split("Hero wins").next().unwrap(), &profile("Cake")).is_err());
}

#[test]
fn xml_archives_stream_multiple_games_and_reject_balance_or_action_corruption() {
    for raw in [
        include_str!("fixtures/hand-formats/merge.xml"),
        include_str!("fixtures/hand-formats/microgaming.xml"),
    ] {
        let two = format!("{raw}\n{}", raw.replacen("id=\"", "id=\"copy-", 1));
        let mut hands = vec![];
        hand_formats::stream_xml(two.as_bytes(), &mut |raw| {
            hands.push(parser::parse(raw, &profile("archive"))?);
            Ok(true)
        })
        .unwrap();
        assert_eq!(hands.len(), 2);
        assert!(hands.iter().all(|h| h.status == "valid"));
        assert_ne!(hands[0].id, hands[1].id);
    }
    let mpn = include_str!("fixtures/hand-formats/microgaming.xml");
    assert!(parser::parse(
        &mpn.replace("endbalance=\"1.20\"", "endbalance=\"99\""),
        &profile("MPN")
    )
    .is_err());
    let xml = include_str!("fixtures/hand-formats/ipoker.xml");
    assert!(parser::parse(
        &xml.replace("<mode>real</mode>", "<mode>play</mode>"),
        &profile("iPoker")
    )
    .is_err());
    let coin = fixtures::hu_hand(
        "Hero: folds\nVillain collected $0.03 from pot",
        "Total pot $0.03 | Rake $0",
    )
    .replace(
        "Poker Hand #TEST1: Hold'em No Limit ($0.01/$0.02) -",
        "CoinPoker Hand #TEST1: NLH (₮0.01/₮0.02/₮0.001) ",
    )
    .replace(
        "Villain: posts big blind $0.02",
        "Villain: posts auto big blind ₮0.001",
    )
    .replace('$', "₮");
    assert!(parser::parse(&coin, &profile("CoinPoker")).is_err());
}

#[test]
fn mixed_currencies_cannot_be_summed_in_rollups_reports_or_matrices() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("currency.db");
    store::init(&db).unwrap();
    let mut c = store::open(&db).unwrap();
    let usd = parser::parse(&fixtures::cash_hand(1), &Profile::default()).unwrap();
    let eur = parser::parse(
        include_str!("fixtures/hand-formats/winamax.txt"),
        &profile("Winamax"),
    )
    .unwrap();
    store::insert_batch(&mut c, "currency", "synthetic", &[usd, eur]).unwrap();
    for group in [
        "date",
        "position",
        "stakes",
        "game",
        "hand_class",
        "session",
    ] {
        assert!(store::report(&c, &Filter::default(), group)
            .unwrap_err()
            .to_string()
            .contains("Multiple currencies"));
    }
    let direct = Filter {
        stack_min: Some(0.),
        ..Filter::default()
    };
    assert!(store::report(&c, &direct, "date").is_err());
    assert!(store::matrix(&c, &Filter::default(), "net").is_err());
    let eur = Filter {
        currency: Some("EUR".into()),
        ..Filter::default()
    };
    let r = store::report(&c, &eur, "date").unwrap();
    assert_eq!(r["currency"], "EUR");
    assert_eq!(r["hands"], 1);
    assert_eq!(r["net"], "-22.85");
    assert_eq!(
        store::report(
            &c,
            &Filter {
                stack_min: Some(0.),
                ..eur.clone()
            },
            "date"
        )
        .unwrap()["net"],
        r["net"]
    );
    let csv = d.path().join("eur.csv");
    store::export(&c, &eur, &csv, "report:date", &[]).unwrap();
    let text = std::fs::read_to_string(csv).unwrap();
    assert!(text.lines().next().unwrap().ends_with(",currency"));
    assert!(text.lines().nth(1).unwrap().ends_with(",\"EUR\""));
    assert!(!text.contains("net_usd"));
}

#[test]
fn explicit_source_timezone_does_not_change_requested_date_filter_boundaries() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("date.db");
    store::init(&db).unwrap();
    let mut c = store::open(&db).unwrap();
    let raw = fixtures::hu_hand(
        "Hero: folds\nUncalled bet ($0.01) returned to Villain\nVillain collected $0.02 from pot",
        "Total pot $0.02 | Rake $0",
    )
    .replace("Poker Hand #", "PokerStars Hand #")
    .replace("2026/09/01 12:00:00", "2026/09/01 20:00:00 ET");
    let h = parser::parse(&raw, &Profile::default()).unwrap();
    assert_eq!(h.status, "valid");
    store::insert_batch(&mut c, "date", "synthetic", &[h]).unwrap();
    for (date, count) in [("2026-09-01", 0), ("2026-09-02", 1)] {
        let f = Filter {
            date_from: Some(date.into()),
            date_to: Some(date.into()),
            timezone: Some("Asia/Hong_Kong".into()),
            ..Filter::default()
        };
        assert_eq!(store::report(&c, &f, "date").unwrap()["hands"], count);
    }
}

#[test]
fn import_mixed_text_xml_zip_deduplicates_rebuilds_and_exports_original_hands() {
    use poker_core::service::{Request, Service};
    use std::{
        io::Write,
        time::{Duration, Instant},
    };
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("import.db");
    let source = d.path().join("mixed.zip");
    // Exercise Windows input on every platform, independently of git checkout.
    let stars = include_str!("fixtures/hand-formats/pokerstars.txt")
        .replace("\r\n", "\n")
        .replace('\n', "\r\n");
    let originals = [
        ("stars.txt", stars.as_str()),
        (
            "ipoker.xml",
            include_str!("fixtures/hand-formats/ipoker.xml"),
        ),
        ("merge.xml", include_str!("fixtures/hand-formats/merge.xml")),
        (
            "mpn.xml",
            include_str!("fixtures/hand-formats/microgaming.xml"),
        ),
    ];
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&source).unwrap());
    for (name, raw) in originals {
        let padded;
        let raw = if name == "ipoker.xml" {
            padded = raw.replacen(
                "<duration>N/A</duration>",
                &format!(
                    "<duration>{}€</duration>",
                    "x".repeat(
                        8191 - raw.split("<duration>").next().unwrap().len() - "<duration>".len()
                    )
                ),
                1,
            );
            padded.as_str()
        } else {
            raw
        };
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(raw.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    let s = Service::new(db.clone()).unwrap();
    s.handle(Request::EquityControl { paused: true }).unwrap();
    let wait = || {
        let started = Instant::now();
        while s.handle(Request::Health).unwrap()["import_active"] == true {
            assert!(started.elapsed() < Duration::from_secs(15));
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    for duplicate in [false, true] {
        let j = s
            .handle(Request::StartImport {
                paths: vec![source.to_string_lossy().into()],
                profile: profile("mixed export"),
            })
            .unwrap();
        wait();
        let jobs = s.handle(Request::Jobs).unwrap();
        let job = jobs
            .as_array()
            .unwrap()
            .iter()
            .find(|job| job["id"] == j["id"])
            .unwrap();
        assert_eq!(job["state"], "complete", "{job}");
        assert_eq!(job["quarantined"], 0);
        assert_eq!(job["conflicts"], 0);
        assert_eq!(job[if duplicate { "duplicates" } else { "inserted" }], 4);
    }
    // An LF export of the same hand must dedup against the retained CRLF raw.
    let lf = d.path().join("same-stars-lf.txt");
    std::fs::write(&lf, stars.replace("\r\n", "\n")).unwrap();
    let j = s
        .handle(Request::StartImport {
            paths: vec![lf.to_string_lossy().into()],
            profile: profile("mixed export"),
        })
        .unwrap();
    wait();
    let jobs = s.handle(Request::Jobs).unwrap();
    let job = jobs
        .as_array()
        .unwrap()
        .iter()
        .find(|job| job["id"] == j["id"])
        .unwrap();
    assert_eq!(job["state"], "complete", "{job}");
    assert_eq!(job["inserted"], 0);
    assert_eq!(job["duplicates"], 1);
    assert_eq!(job["conflicts"], 0);
    s.handle(Request::Rebuild).unwrap();
    let c = store::open(&db).unwrap();
    let count: i64 = c
        .query_row("SELECT COUNT(*) FROM hands WHERE status='valid'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 4);
    let mut stmt = c.prepare("SELECT id FROM hands ORDER BY id").unwrap();
    let ids = stmt
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for id in ids {
        let h = s.handle(Request::Hand { id }).unwrap();
        let raw = h["hand"]["raw"].as_str().unwrap();
        let target = d.path().join(format!("{id}.txt"));
        store::export(&c, &Filter::default(), &target, "hh", &[id]).unwrap();
        assert_eq!(std::fs::read_to_string(target).unwrap().trim(), raw.trim());
        if raw.starts_with("PokerStars") {
            assert_eq!(raw.trim(), originals[0].1.trim());
        }
    }
}

#[test]
fn xml_pocket_metadata_cannot_become_a_public_reveal_before_betting() {
    for raw in [
        include_str!("fixtures/hand-formats/ipoker.xml"),
        include_str!("fixtures/hand-formats/merge.xml"),
    ] {
        let h = parser::parse(raw, &profile("XML")).unwrap();
        assert_eq!(h.status, "valid");
        let last_bet = h
            .actions
            .iter()
            .filter(|a| matches!(a.kind.as_str(), "raise" | "call" | "bet"))
            .map(|a| a.seq)
            .max()
            .unwrap();
        for a in h
            .actions
            .iter()
            .filter(|a| a.kind == "show" && a.actor != Some(h.hero_seat))
        {
            assert!(
                a.seq > last_bet,
                "showdown metadata must stay after source betting"
            );
        }
    }
}
