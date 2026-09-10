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
