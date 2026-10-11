/// Synthetic, reproducible six-seat cash hands. Contains no private player data.
pub fn cash_hand(index: u64) -> String {
    let hero_seat = (index % 6 + 1) as usize;
    let villain_seat = if hero_seat == 2 { 1 } else { 2 };
    let names: Vec<String> = (1..=6)
        .map(|s| {
            if s == hero_seat {
                "Hero".into()
            } else {
                format!("Synthetic{s}")
            }
        })
        .collect();
    let hero_wins = !index.is_multiple_of(3);
    let (hc, vc) = if hero_wins {
        ("As Kd", "Qs Qd")
    } else {
        ("Qs Qd", "As Kd")
    };
    let time = chrono::DateTime::from_timestamp(1_788_192_000 + (index % 2_592_000) as i64, 0)
        .unwrap()
        .format("%Y/%m/%d %H:%M:%S");
    let rush = if index.is_multiple_of(4) {
        "RushAndCashSynthetic"
    } else {
        "NLHSynthetic"
    };
    let mut s=format!("Poker Hand #SYN{index:012}: Hold'em No Limit ($0.01/$0.02) - {time}\nTable '{rush}' 6-max Seat #6 is the button\n");
    for (seat, name) in names.iter().enumerate() {
        s += &format!("Seat {}: {name} ($2 in chips)\n", seat + 1);
    }
    s += &format!(
        "{}: posts small blind $0.01\n{}: posts big blind $0.02\n*** HOLE CARDS ***\n",
        names[0], names[1]
    );
    for (seat, name) in names.iter().enumerate() {
        s += &format!(
            "Dealt to {name} {}\n",
            if seat + 1 == hero_seat {
                format!("[{hc}]")
            } else {
                String::new()
            }
        );
    }
    let opener = if hero_seat == 2 { 1 } else { hero_seat };
    for seat in [3, 4, 5, 6, 1, 2] {
        if seat == opener {
            s += &format!("{}: raises ${} to $0.06\n", names[seat - 1], "0.04");
        } else if seat == villain_seat || seat == hero_seat {
            s += &format!(
                "{}: calls ${}\n",
                names[seat - 1],
                if seat == 2 { "0.04" } else { "0.05" }
            );
        } else {
            s += &format!("{}: folds\n", names[seat - 1]);
        }
    }
    let (oop, ip) = if hero_seat == 1 {
        (hero_seat, villain_seat)
    } else {
        (villain_seat, hero_seat)
    };
    s+=&format!("*** FLOP *** [Ks 7d 2c]\n{}: checks\n{}: bets $0.06\n{}: calls $0.06\n*** TURN *** [Ks 7d 2c] [3h]\n{}: checks\n{}: checks\n*** RIVER *** [Ks 7d 2c 3h] [9c]\n{}: checks\n{}: checks\n*** SHOWDOWN ***\nHero: shows [{hc}]\n{}: shows [{vc}]\n",names[oop-1],names[ip-1],names[oop-1],names[oop-1],names[ip-1],names[oop-1],names[ip-1],names[villain_seat-1]);
    let pot = if hero_seat > 2 { "0.25" } else { "0.24" };
    let award = if hero_seat > 2 { "0.24" } else { "0.23" };
    let winner = if hero_wins {
        "Hero"
    } else {
        names[villain_seat - 1].as_str()
    };
    s+=&format!("{winner} collected ${award} from pot\n*** SUMMARY ***\nTotal pot ${pot} | Rake $0.01 | Jackpot $0 | Bingo $0 | Fortune $0 | Tax $0\n");
    s
}

pub fn hu_hand(actions: &str, summary: &str) -> String {
    format!("Poker Hand #TEST1: Hold'em No Limit ($0.01/$0.02) - 2026/09/01 12:00:00\nTable 'Synthetic' 2-max Seat #1 is the button\nSeat 1: Hero ($2 in chips)\nSeat 2: Villain ($2 in chips)\nHero: posts small blind $0.01\nVillain: posts big blind $0.02\n*** HOLE CARDS ***\nDealt to Hero [As Ad]\nDealt to Villain\n{actions}\n*** SUMMARY ***\n{summary}\n")
}

/// GG Regular population fixture: distinct identities, dates, stakes, positions,
/// SRP/3-bet and multiway pots, folds/calls/raises, three streets, unknown cards.
pub fn population_hand(index: u64) -> String {
    let bb = [20_000, 250_000, 1_000_000][(index / 32 % 3) as usize];
    let sb = bb / 2;
    let opener = 3 + (index / 8 % 4) as usize;
    let kind = index % 8;
    let date = chrono::DateTime::from_timestamp(
        1_788_192_000 + (index % 90) as i64 * 86400 + (index % 86400) as i64,
        0,
    )
    .unwrap()
    .format("%Y/%m/%d %H:%M:%S");
    let fmt = crate::money::format;
    let mut raw=format!("Poker Hand #MDA{index:012}: Hold'em No Limit (${}/${}) - {date}\nTable 'SyntheticRegular' 6-max Seat #6 is the button\n",fmt(sb),fmt(bb));
    for seat in 1..=6 {
        raw += &format!("Seat {seat}: Player{seat} (${} in chips)\n", fmt(bb * 200));
    }
    raw += &format!(
        "Player1: posts small blind ${}\nPlayer2: posts big blind ${}\n*** HOLE CARDS ***\n",
        fmt(sb),
        fmt(bb)
    );
    let mut spent = [0; 7];
    spent[1] = sb;
    spent[2] = bb;
    let mut pot = sb + bb;
    let multi = kind == 6;
    let three = kind == 3 || kind == 7;
    for seat in [3, 4, 5, 6, 1, 2] {
        if kind == 0 && seat != 2 {
            raw += &format!("Player{seat}: folds\n");
        } else if kind == 0 {
            raw += &format!("Uncalled bet (${}) returned to Player2\n", fmt(sb));
            pot -= sb;
        } else if seat == opener {
            raw += &format!(
                "Player{seat}: raises ${} to ${}\n",
                fmt(bb * 3),
                fmt(bb * 3)
            );
            spent[seat] = bb * 3;
            pot += bb * 3;
        } else if seat == 2 {
            if three {
                let amount = bb * 9 - spent[seat];
                raw += &format!("Player2: raises ${} to ${}\n", fmt(bb * 6), fmt(bb * 9));
                spent[seat] += amount;
                pot += amount;
            } else {
                let amount = bb * 3 - spent[seat];
                raw += &format!("Player2: calls ${}\n", fmt(amount));
                spent[seat] += amount;
                pot += amount;
            }
        } else if seat == 1 && multi {
            let amount = bb * 3 - spent[seat];
            raw += &format!("Player1: calls ${}\n", fmt(amount));
            spent[seat] += amount;
            pot += amount;
        } else {
            raw += &format!("Player{seat}: folds\n");
        }
    }
    if kind == 0 {
        raw += &format!(
            "Player2 collected ${} from pot\n*** SUMMARY ***\nTotal pot ${} | Rake $0\n",
            fmt(pot),
            fmt(pot)
        );
        return raw;
    }
    if three {
        raw += &format!("Player{opener}: calls ${}\n", fmt(bb * 6));
        spent[opener] += bb * 6;
        pot += bb * 6;
    }
    let flop = if index.is_multiple_of(2) {
        "Ks 7d 2c"
    } else {
        "Ts 7s 2d"
    };
    let mut live = vec![2, opener];
    if multi {
        live.insert(0, 1);
    }
    let mut winner = opener;
    let mut finished = false;
    for street in 0..3 {
        let marker = match street {
            0 => format!("*** FLOP *** [{flop}]\n"),
            1 => format!("*** TURN *** [{flop}] [3h]\n"),
            _ => format!("*** RIVER *** [{flop} 3h] [9c]\n"),
        };
        raw += &marker;
        for &seat in &live {
            if seat != opener {
                raw += &format!("Player{seat}: checks\n");
            }
        }
        let check = (kind == 2 && street == 1) || (kind == 5 && street == 0);
        if check {
            raw += &format!("Player{opener}: checks\n");
            continue;
        }
        let bet = pot / if kind == 4 { 3 } else { 2 };
        raw += &format!("Player{opener}: bets ${}\n", fmt(bet));
        pot += bet;
        spent[opener] += bet;
        let responders: Vec<_> = live.iter().copied().filter(|s| *s != opener).collect();
        for seat in responders {
            if (kind == 1 && street == 0)
                || (kind == 2 && street == 2)
                || (kind == 7 && street == 2)
            {
                raw += &format!("Player{seat}: folds\n");
                live.retain(|s| *s != seat);
            } else if kind == 4 && street == 0 {
                raw += &format!(
                    "Player{seat}: raises ${} to ${}\nPlayer{opener}: calls ${}\n",
                    fmt(bet),
                    fmt(bet * 2),
                    fmt(bet)
                );
                pot += bet * 3;
                spent[seat] += bet * 2;
                spent[opener] += bet;
            } else {
                raw += &format!("Player{seat}: calls ${}\n", fmt(bet));
                pot += bet;
                spent[seat] += bet;
            }
        }
        if live.len() == 1 {
            raw += &format!("Uncalled bet (${}) returned to Player{opener}\n", fmt(bet));
            pot -= bet;
            finished = true;
            break;
        }
    }
    if !finished {
        raw += "*** SHOWDOWN ***\n";
        if index.is_multiple_of(5) {
            raw += &format!("Player{opener}: shows [As Ad]\n");
        }
        if index.is_multiple_of(7) {
            raw += "Player2: shows [Qs Qd]\n";
            winner = 2;
        }
    }
    raw += &format!(
        "Player{winner} collected ${} from pot\n*** SUMMARY ***\nTotal pot ${} | Rake $0\n",
        fmt(pot),
        fmt(pot)
    );
    raw
}
