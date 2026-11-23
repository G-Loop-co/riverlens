use rs_poker::core::{Card, CoreRank, Rankable, SevenCardAccum};

#[derive(Clone, Copy)]
pub(super) struct Holding {
    pub class: &'static str,
    pub outs: u8,
    pub gutshot: bool,
}

impl Holding {
    pub fn candidate(self) -> bool {
        matches!(self.class, "air" | "draw")
    }
}

fn straight_high(ranks: u16) -> u8 {
    for high in (4..13).rev() {
        let mask = 31 << (high - 4);
        if ranks & mask == mask {
            return high + 2;
        }
    }
    if ranks & 0x100f == 0x100f {
        5
    } else {
        0
    }
}

pub(super) fn classify(hole: &[String], board: &[String]) -> Holding {
    let unknown = Holding {
        class: "unknown",
        outs: 0,
        gutshot: false,
    };
    if hole.len() != 2 || !(3..=5).contains(&board.len()) {
        return unknown;
    }
    let parse = |v: &[String]| {
        v.iter()
            .map(|s| Card::try_from(s.as_str()))
            .collect::<Result<Vec<_>, _>>()
    };
    let (Ok(cards), Ok(public)) = (parse(hole), parse(board)) else {
        return unknown;
    };
    let mut known = 0u64;
    let mut board_ranks = 0u16;
    let mut all_ranks = 0u16;
    let mut counts = [0u8; 13];
    let mut suits = [0u8; 4];
    let mut board_acc = SevenCardAccum::default();
    let mut all = SevenCardAccum::default();
    for (i, &c) in public.iter().chain(&cards).enumerate() {
        let bit = 1u64 << u8::from(c);
        if known & bit != 0 {
            return unknown;
        }
        known |= bit;
        let rank_bit = 1 << u8::from(c.value);
        all_ranks |= rank_bit;
        counts[u8::from(c.value) as usize] += 1;
        suits[u8::from(c.suit) as usize] += 1;
        all += c;
        if i < public.len() {
            board_ranks |= rank_bit;
            board_acc += c;
        }
    }
    let rank = all.rank();
    // A pocket pair counterfeited by two higher board pairs supplies no made pair.
    let paired = cards.iter().any(|card| {
        let r = u8::from(card.value) as usize;
        counts[r] >= 2
            && (counts[r] >= 3
                || (rank.category() != CoreRank::ThreeOfAKind
                    && counts[r + 1..].iter().filter(|n| **n >= 2).count() < 2))
    });
    let strong = rank.category() >= CoreRank::Straight;
    let mut outs = 0;
    let mut straight_ranks = 0u16;
    if public.len() < 5 && !strong {
        let mut winning_ranks = 0u16;
        for rank in 0..13 {
            let bit = 1 << rank;
            if straight_high(all_ranks | bit) > straight_high(board_ranks | bit) {
                winning_ranks |= bit;
            }
        }
        for n in 0..52u8 {
            if known & (1u64 << n) != 0 {
                continue;
            }
            let next = Card::from(n);
            let rank_bit = 1 << u8::from(next.value);
            let straight = winning_ranks & rank_bit != 0;
            if straight {
                straight_ranks |= rank_bit;
            }
            let flush = suits[u8::from(next.suit) as usize] == 4
                && cards.iter().any(|h| h.suit == next.suit)
                && (public.len() == 3 || (all + next).rank() > (board_acc + next).rank());
            if straight || flush {
                outs += 1;
            }
        }
    }
    let class = if strong && public.len() == 5 && rank == public.rank() {
        "board"
    } else if strong || paired {
        if paired && rank.category() <= CoreRank::TwoPair && outs >= 8 {
            "pair_draw"
        } else {
            "made"
        }
    } else if outs >= 8 {
        "draw"
    } else {
        "air"
    };
    Holding {
        class,
        outs,
        gutshot: straight_ranks.count_ones() == 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn holding(hole: &str, board: &str) -> Holding {
        let cards = |s: &str| s.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        classify(&cards(hole), &cards(board))
    }
    #[test]
    fn distinct_immediate_completions_and_hole_card_contribution() {
        for (cards, board, class, outs) in [
            ("8h 9h", "6h 7h Kc", "draw", 15), // nine hearts plus six non-heart straight cards
            ("8c 9d", "6h 7s Kc", "draw", 8),
            ("8c Td", "6h 7s Kc", "air", 4),
            ("Ah 3c", "2d 4h Ks", "air", 4),
            ("Ah Jh", "Jd 7h 2h", "pair_draw", 9),
            ("Ac 3d", "Ks Kh 8c", "air", 0),
        ] {
            let h = holding(cards, board);
            assert_eq!((h.class, h.outs), (class, outs), "{cards} / {board}");
        }
        assert_eq!(holding("2c 2d", "Ks Kh 8c 8d As").class, "air");
        assert!(holding("8c Td", "6h 7s Kc").gutshot);
        assert_eq!(holding("Ac 2d", "3h 4h 5h 6h 7h").class, "board");
        assert_eq!(holding("Ac 2d", "3h 4h 5h 6h").outs, 0); // already has the wheel
        assert_eq!(holding("Jc Qd", "2h 4h 6h 8h").outs, 0); // a fifth heart plays the board
        assert_eq!(holding("Jc Qd", "2h 4h 6h 8h 9s").class, "air");
        assert_eq!(holding("", "2h 4h 6h").class, "unknown");
    }
}
