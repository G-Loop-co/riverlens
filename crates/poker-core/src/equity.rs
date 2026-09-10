use crate::{model::*, money};
use anyhow::{Context, Result};
use rs_poker::core::{Card, SevenCardAccum};
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, Ordering},
};

fn exclude(h: &mut Hand, reason: &str) {
    h.ev_status = "excluded".into();
    h.ev_reason = Some(reason.into());
    h.equity_input = None;
}

pub fn classify(h: &mut Hand) {
    if h.status != "valid" {
        exclude(h, "invalid_ledger");
        return;
    }
    if !h.actions.iter().any(|a| a.all_in) {
        h.ev_status = "not_applicable".into();
        return;
    }
    if h.actions.iter().any(|a| a.kind.starts_with("cashout")) {
        exclude(h, "cashout");
        return;
    }
    if h.boards.len() > 1 {
        exclude(h, "multiple_runouts");
        return;
    }
    if h.pot_type == "special" {
        exclude(h, "special_betting");
        return;
    }
    if h.fees.iter().any(|(k, v)| {
        !matches!(k.as_str(), "Rake" | "Jackpot" | "Bingo" | "Fortune" | "Tax")
            || (!matches!(k.as_str(), "Rake" | "Jackpot") && money::parse(v).unwrap_or(1) != 0)
    }) {
        exclude(h, "special_settlement");
        return;
    }
    let mut active: HashSet<u8> = h.players.iter().map(|p| p.seat).collect();
    let mut allin = HashSet::new();
    let mut spent = HashMap::<u8, i64>::new();
    let mut board = vec![];
    let mut lock = None;
    for a in &h.actions {
        if a.kind == "board" {
            board = a.cards.clone();
            continue;
        }
        let Some(s) = a.actor else {
            continue;
        };
        if a.kind == "fold" {
            active.remove(&s);
        }
        if matches!(
            a.kind.as_str(),
            "small_blind"
                | "big_blind"
                | "dead_blind"
                | "ante"
                | "straddle"
                | "call"
                | "bet"
                | "raise"
        ) {
            *spent.entry(s).or_default() += a.amount;
            if a.all_in
                || h.players
                    .iter()
                    .any(|p| p.seat == s && spent[&s] >= p.stack)
            {
                allin.insert(s);
            }
        }
        if a.kind == "return" {
            *spent.entry(s).or_default() -= a.amount;
        }
        if active.len() == 2
            && active.contains(&h.hero_seat)
            && active.iter().any(|s| allin.contains(s))
            && (a.kind == "call" || a.kind == "return")
        {
            let later_betting = h
                .actions
                .iter()
                .skip(a.seq + 1)
                .any(|x| matches!(x.kind.as_str(), "call" | "bet" | "raise" | "fold"));
            if !later_betting {
                lock = Some((active.clone(), board.clone()));
                break;
            }
        }
    }
    let Some((active, board)) = lock else {
        exclude(h, "multiway_or_no_lock");
        return;
    };
    if h.pots.len() != 1 || h.pots[0].eligible.len() != 2 {
        exclude(h, "side_pots");
        return;
    }
    let hero = h.players.iter().find(|p| p.hero).unwrap();
    let villain = h
        .players
        .iter()
        .find(|p| active.contains(&p.seat) && !p.hero)
        .unwrap();
    if hero.cards.len() != 2 || villain.cards.len() != 2 {
        exclude(h, "unknown_hole_cards");
        return;
    }
    if ![0, 3, 4, 5].contains(&board.len()) {
        exclude(h, "incomplete_board");
        return;
    }
    let fees: i64 = h.fees.values().map(|v| money::parse(v).unwrap_or(0)).sum();
    h.equity_input = Some(EquityInput {
        hero: hero.cards.clone(),
        villain: villain.cards.clone(),
        board,
        net_pot: h.total_pot - fees,
        contribution: h.invested - h.returned,
    });
    h.ev_status = "pending".into();
    h.ev_reason = None;
}

/// Enumerate every legal board. Cancellation is checked at bounded intervals.
pub fn exact(input: &EquityInput, cancel: &AtomicBool) -> Result<Option<(f64, String)>> {
    let known: Vec<String> = input
        .hero
        .iter()
        .chain(&input.villain)
        .chain(&input.board)
        .cloned()
        .collect();
    let known_set: HashSet<_> = known.iter().collect();
    anyhow::ensure!(known_set.len() == known.len(), "duplicate known cards");
    let mut deck = vec![];
    for r in "23456789TJQKA".chars() {
        for s in "cdhs".chars() {
            let c = format!("{r}{s}");
            if !known.contains(&c) {
                deck.push(Card::try_from(c.as_str()).context("invalid card")?);
            }
        }
    }
    let hero: Vec<Card> = input
        .hero
        .iter()
        .chain(&input.board)
        .map(|s| Card::try_from(s.as_str()))
        .collect::<std::result::Result<_, _>>()?;
    let villain: Vec<Card> = input
        .villain
        .iter()
        .chain(&input.board)
        .map(|s| Card::try_from(s.as_str()))
        .collect::<std::result::Result<_, _>>()?;
    let needed = 5 - input.board.len();
    let mut tally = (0u64, 0u64, 0u64);
    let mut drawn = vec![];
    enumerate(
        &deck, needed, 0, &mut drawn, &hero, &villain, &mut tally, cancel,
    );
    if cancel.load(Ordering::Relaxed) {
        return Ok(None);
    }
    let (wins, ties, total) = tally;
    anyhow::ensure!(total > 0, "no legal runouts");
    let numerator = (wins * 2 + ties) as i128;
    let denominator = (total * 2) as i128;
    let expected = (numerator * input.net_pot as i128 + denominator / 2) / denominator
        - input.contribution as i128;
    Ok(Some((
        numerator as f64 / denominator as f64,
        money::format(expected as i64),
    )))
}
#[allow(clippy::too_many_arguments)] // Explicit recursion state avoids per-runout allocation.
fn enumerate(
    deck: &[Card],
    needed: usize,
    start: usize,
    drawn: &mut Vec<Card>,
    hero: &[Card],
    villain: &[Card],
    tally: &mut (u64, u64, u64),
    cancel: &AtomicBool,
) {
    if tally.2.is_multiple_of(4096) && cancel.load(Ordering::Relaxed) {
        return;
    }
    if drawn.len() == needed {
        let mut a = SevenCardAccum::new();
        let mut b = SevenCardAccum::new();
        for c in hero.iter().chain(drawn.iter()) {
            a.add(*c);
        }
        for c in villain.iter().chain(drawn.iter()) {
            b.add(*c);
        }
        let ra = a.rank();
        let rb = b.rank();
        if ra > rb {
            tally.0 += 1;
        } else if ra == rb {
            tally.1 += 1;
        }
        tally.2 += 1;
        return;
    }
    for i in start..=deck.len() - (needed - drawn.len()) {
        drawn.push(deck[i]);
        enumerate(deck, needed, i + 1, drawn, hero, villain, tally, cancel);
        drawn.pop();
        if cancel.load(Ordering::Relaxed) {
            break;
        }
    }
}
