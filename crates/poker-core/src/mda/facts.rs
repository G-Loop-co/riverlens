//! Decision opportunities and visible-at-decision dimensions.
use crate::{model::ObservedHand, stats::may_raise};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const VERSION: &str = "population-opportunities/1.0.0";
pub const METRICS: &[(&str, &str)] = &[
    (
        "rfi",
        "無人 limp／raise 的 first-in 合法加注機會；BB walk 除外",
    ),
    (
        "three_bet",
        "面對一次 open、有重新開放及籌碼的合法加注機會，包含 squeeze",
    ),
    (
        "blind_fold",
        "盲位面對一次 open、沒有 cold caller 的 fold／call／raise 決策",
    ),
    ("blind_call", "相同盲位防守機會，call 命中"),
    (
        "blind_raise",
        "相同盲位防守機會；raise 命中，另列合法加注率",
    ),
    (
        "fold_three_bet",
        "原 open 者面對一次 3-bet 的 fold／call／raise 決策",
    ),
    ("call_three_bet", "相同面對 3-bet 機會，call 命中"),
    ("raise_three_bet", "相同面對 3-bet 機會，raise 命中"),
    (
        "cbet",
        "前街最後進攻者、前街曾下注／加注，本街未有人 bet 的首次 check／bet 機會",
    ),
    (
        "fold_cbet",
        "面對本街 C-bet、未有中間 raise 的 fold／call／raise 決策",
    ),
    ("call_cbet", "相同 C-bet 回應機會，call 命中"),
    ("raise_cbet", "相同 C-bet 回應機會，raise 命中"),
    (
        "check_raise",
        "本街曾 check 後面對下注，而且可以合法 raise 的機會",
    ),
    (
        "bet_checked_through",
        "前一翻後街所有仍可決策玩家 check、本街未有 bet 的首次 check／bet 機會",
    ),
    (
        "fold_bet",
        "面對本街首次 bet、未有中間 raise 的 fold／call／raise 決策",
    ),
    ("call_bet", "相同 bet 回應機會，call 命中"),
    ("raise_bet", "相同 bet 回應機會，raise 命中"),
];
pub fn bit(name: &str) -> Option<u64> {
    METRICS.iter().position(|(n, _)| *n == name).map(|i| 1 << i)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub seq: u32,
    pub seat: u8,
    pub day: i32,
    pub bb: i64,
    pub position: String,
    pub opponent: String,
    pub street: String,
    pub pot_type: String,
    pub players: u8,
    pub effective_bb: f64,
    pub facing_bet_pct: Option<f64>,
    pub board: String,
    pub texture: String,
    pub paired: bool,
    pub high_card: String,
    pub pot_units: i64,
    pub to_call_units: i64,
    pub remaining_units: i64,
    pub action: String,
    pub opportunities: u64,
    pub hits: u64,
    pub known_cards: bool,
    pub showdown: bool,
}
fn mark(d: &mut Decision, name: &str, allowed: bool, hit: bool) {
    if allowed {
        let b = bit(name).unwrap();
        d.opportunities |= b;
        if hit {
            d.hits |= b;
        }
    }
}
fn texture(board: &[String]) -> (String, bool, String) {
    if board.is_empty() {
        return (String::new(), false, String::new());
    }
    let suits: HashSet<_> = board.iter().filter_map(|c| c.chars().nth(1)).collect();
    let ranks: HashSet<_> = board.iter().filter_map(|c| c.chars().next()).collect();
    let t = if suits.len() == 1 {
        "monotone"
    } else if suits.len() == 2 {
        "two-tone"
    } else {
        "rainbow"
    };
    (
        t.into(),
        ranks.len() < board.len(),
        ranks
            .iter()
            .max_by_key(|r| "23456789TJQKA".find(**r))
            .map(|r| r.to_string())
            .unwrap_or_default(),
    )
}
/// Uses incremental ledger state, never completed-board dimensions at earlier decisions.
pub fn derive(h: &mut ObservedHand) -> Result<Vec<Decision>> {
    ensure!(
        h.max_seats == 6 && h.game == "Cash" && h.currency == "USD",
        "scope: GG Regular USD NLH 6-max only"
    );
    ensure!(
        h.boards.len() == 1 && !h.actions.iter().any(|a| a.kind == "straddle"),
        "scope: straddles and multiple runouts excluded"
    );
    let mut spent = HashMap::<u8, i64>::new();
    let mut live = HashMap::<u8, i64>::new();
    let mut acted = HashMap::<u8, i64>::new();
    let mut folded = HashSet::<u8>::new();
    let final_folded: HashSet<u8> = h
        .actions
        .iter()
        .filter(|a| a.kind == "fold")
        .filter_map(|a| a.actor)
        .collect();
    let mut checked = HashSet::<u8>::new();
    let mut emitted = HashSet::<(u8, String)>::new();
    let mut street = "preflop".to_string();
    let mut board = Vec::new();
    let mut max_bet = 0;
    let mut minimum = h.bb;
    let mut raises = 0;
    let mut limpers = 0;
    let mut callers = 0;
    let mut opener = None;
    let mut aggressor = None;
    let mut previous_aggressor = None;
    let mut any_bet = false;
    let mut previous_checked = false;
    let mut street_decisions = 0;
    let mut street_players = h.players.len() as u8;
    let mut cbetter = None;
    let mut bettor = None;
    let mut bet_raised = false;
    let mut wager_pct = None;
    let mut pot = 0;
    let mut out = Vec::new();
    let mut final_pot_type = "limped".to_string();
    for a in &h.actions {
        if a.kind == "board" {
            if a.runout != 0 {
                continue;
            }
            previous_checked = street != "preflop"
                && !any_bet
                && street_decisions > 0
                && h.players
                    .iter()
                    .filter(|p| {
                        !folded.contains(&p.seat)
                            && p.stack > spent.get(&p.seat).copied().unwrap_or(0)
                    })
                    .all(|p| checked.contains(&p.seat));
            previous_aggressor = aggressor;
            aggressor = None;
            checked.clear();
            emitted.clear();
            live.clear();
            acted.clear();
            max_bet = 0;
            minimum = h.bb;
            any_bet = false;
            street_decisions = 0;
            cbetter = None;
            bettor = None;
            bet_raised = false;
            wager_pct = None;
            street = a.street.clone();
            street_players = h
                .players
                .iter()
                .filter(|p| !folded.contains(&p.seat))
                .count() as u8;
            board = a.cards.clone();
            continue;
        }
        let Some(seat) = a.actor else {
            continue;
        };
        let player = h
            .players
            .iter()
            .find(|p| p.seat == seat)
            .ok_or_else(|| anyhow::anyhow!("actor not dealt in"))?;
        let remaining = player.stack - spent.get(&seat).copied().unwrap_or(0);
        let prior = live.get(&seat).copied().unwrap_or(0);
        let to_call = (max_bet - prior).max(0);
        let responsive = h.players.iter().any(|p| {
            p.seat != seat
                && !folded.contains(&p.seat)
                && p.stack > spent.get(&p.seat).copied().unwrap_or(0)
        });
        let can_raise = may_raise(
            remaining,
            to_call,
            acted.get(&seat).copied(),
            max_bet,
            minimum,
            responsive,
        );
        let decision = matches!(a.kind.as_str(), "fold" | "check" | "call" | "bet" | "raise");
        if decision {
            ensure!(
                remaining > 0 && !folded.contains(&seat),
                "illegal decision after fold/all-in"
            );
            ensure!(
                h.players
                    .iter()
                    .filter(|p| !folded.contains(&p.seat))
                    .count()
                    > 1,
                "decision after hand ended"
            );
            match a.kind.as_str() {
                "check" => ensure!(to_call == 0, "illegal check facing a bet"),
                "call" => ensure!(
                    to_call > 0 && a.amount == to_call.min(remaining),
                    "invalid call amount"
                ),
                "bet" => ensure!(
                    max_bet == 0 && can_raise && a.amount > 0 && a.amount <= remaining,
                    "illegal bet"
                ),
                "raise" => ensure!(
                    to_call > 0
                        && can_raise
                        && a.amount > to_call
                        && a.amount <= remaining
                        && (prior + a.amount - max_bet >= minimum || a.amount == remaining),
                    "illegal raise / betting not reopened"
                ),
                _ => {}
            }
            let count = h
                .players
                .iter()
                .filter(|p| !folded.contains(&p.seat))
                .count() as u8;
            let other_stack = h
                .players
                .iter()
                .filter(|p| p.seat != seat && !folded.contains(&p.seat))
                .map(|p| p.stack - spent.get(&p.seat).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);
            let opposed = if street == "preflop" {
                aggressor
            } else {
                bettor.or(previous_aggressor)
            };
            let (texture, paired, high_card) = texture(&board);
            let mut d = Decision {
                seq: a.seq as u32,
                seat,
                day: h.local_time[..10].replace('-', "").parse()?,
                bb: h.bb,
                position: player.position.clone(),
                opponent: h
                    .players
                    .iter()
                    .find(|p| Some(p.seat) == opposed)
                    .map(|p| p.position.clone())
                    .unwrap_or_default(),
                street: street.clone(),
                pot_type: if street == "preflop" {
                    match raises {
                        0 => "unopened",
                        1 => "SRP",
                        2 => "3-bet",
                        _ => "4-bet+",
                    }
                    .into()
                } else {
                    final_pot_type.clone()
                },
                players: if street == "preflop" {
                    count
                } else {
                    street_players
                },
                effective_bb: remaining.min(other_stack) as f64 / h.bb as f64,
                facing_bet_pct: if to_call > 0 { wager_pct } else { None },
                board: board.join(" "),
                texture,
                paired,
                high_card,
                pot_units: pot,
                to_call_units: to_call,
                remaining_units: remaining,
                action: a.kind.clone(),
                opportunities: 0,
                hits: 0,
                known_cards: player.cards.len() == 2,
                showdown: h.showdown && !final_folded.contains(&seat),
            };
            let first = emitted.insert((seat, street.clone()));
            if street == "preflop" {
                mark(
                    &mut d,
                    "rfi",
                    raises == 0
                        && limpers == 0
                        && can_raise
                        && !(player.position == "BB" && to_call == 0),
                    a.kind == "raise",
                );
                mark(
                    &mut d,
                    "three_bet",
                    raises == 1 && can_raise,
                    a.kind == "raise",
                );
                let blind = raises == 1
                    && callers == 0
                    && matches!(player.position.as_str(), "SB" | "BB")
                    && opener != Some(seat)
                    && to_call > 0;
                let facing_three = raises == 2 && opener == Some(seat) && to_call > 0;
                for (prefix, allowed) in [("blind", blind), ("three_bet", facing_three)] {
                    for kind in ["fold", "call", "raise"] {
                        let name = if prefix == "blind" {
                            format!("blind_{kind}")
                        } else {
                            format!("{kind}_three_bet")
                        };
                        mark(&mut d, &name, allowed, a.kind == kind);
                    }
                }
            } else {
                mark(
                    &mut d,
                    "cbet",
                    first && previous_aggressor == Some(seat) && !any_bet && can_raise,
                    a.kind == "bet",
                );
                let facing_cbet = cbetter.is_some_and(|s| s != seat) && !bet_raised && to_call > 0;
                let facing_bet = bettor.is_some_and(|s| s != seat) && !bet_raised && to_call > 0;
                for kind in ["fold", "call", "raise"] {
                    mark(&mut d, &format!("{kind}_cbet"), facing_cbet, a.kind == kind);
                    mark(&mut d, &format!("{kind}_bet"), facing_bet, a.kind == kind);
                }
                mark(
                    &mut d,
                    "check_raise",
                    checked.contains(&seat) && to_call > 0 && can_raise,
                    a.kind == "raise",
                );
                mark(
                    &mut d,
                    "bet_checked_through",
                    first && previous_checked && !any_bet && can_raise,
                    a.kind == "bet",
                );
                if a.kind == "bet" && !any_bet {
                    bettor = Some(seat);
                    if previous_aggressor == Some(seat) {
                        cbetter = Some(seat);
                    }
                }
                if a.kind == "raise" {
                    bet_raised = true;
                }
            }
            out.push(d);
            street_decisions += 1;
        }
        match a.kind.as_str() {
            "small_blind" | "big_blind" | "call" | "bet" | "raise" => {
                *spent.entry(seat).or_default() += a.amount;
                *live.entry(seat).or_default() += a.amount;
                if matches!(a.kind.as_str(), "bet" | "raise") {
                    let increase = live[&seat] - max_bet;
                    wager_pct = Some(if a.kind == "bet" {
                        a.amount as f64 / pot.max(1) as f64 * 100.
                    } else {
                        (a.amount - to_call) as f64 / (pot + to_call).max(1) as f64 * 100.
                    });
                    if increase >= minimum {
                        minimum = increase;
                    }
                    max_bet = live[&seat];
                    aggressor = Some(seat);
                    any_bet = true;
                    if street == "preflop" {
                        raises += 1;
                        if raises == 1 {
                            opener = Some(seat);
                        }
                        final_pot_type = match raises {
                            1 => "SRP",
                            2 => "3-bet",
                            _ => "4-bet+",
                        }
                        .into();
                    }
                } else if matches!(a.kind.as_str(), "small_blind" | "big_blind") {
                    max_bet = max_bet.max(live[&seat]);
                }
                if street == "preflop" && a.kind == "call" {
                    if raises == 0 {
                        limpers += 1;
                    } else if raises == 1 {
                        callers += 1;
                    }
                }
                pot += a.amount;
            }
            "ante" | "dead_blind" => {
                *spent.entry(seat).or_default() += a.amount;
                pot += a.amount;
            }
            "return" => {
                *spent.entry(seat).or_default() -= a.amount;
                *live.entry(seat).or_default() -= a.amount;
                pot -= a.amount;
            }
            "fold" => {
                folded.insert(seat);
            }
            "check" => {
                checked.insert(seat);
            }
            _ => {}
        }
        if decision {
            acted.insert(seat, max_bet);
            if a.kind != "check" {
                checked.remove(&seat);
            }
        }
    }
    h.pot_type = final_pot_type;
    h.stats_version = VERSION.into();
    Ok(out)
}
