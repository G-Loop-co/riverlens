use crate::model::*;
use std::collections::{BTreeMap, HashMap, HashSet};

fn mark(stats: &mut BTreeMap<String, StatValue>, key: &str, opportunity: bool, hit: bool) {
    let s = stats.entry(key.into()).or_default();
    if opportunity {
        s.opportunities = 1;
        if hit {
            s.numerator = 1;
        }
    }
}

/// Versioned per-hand opportunities. No denominators are inferred from report totals.
pub fn derive(h: &mut Hand, hero_won: bool) {
    let hero = h.hero_seat;
    let mut stats = BTreeMap::new();
    for (id, _, _) in STAT_DEFINITIONS {
        stats.insert(id.to_string(), StatValue::default());
    }
    let mut street = "preflop".to_string();
    let mut live = HashMap::<u8, i64>::new();
    let mut spent = HashMap::<u8, i64>::new();
    let mut acted = HashMap::<u8, i64>::new();
    let mut max_bet = 0i64;
    let mut min_raise = h.bb;
    let mut raises = 0u8;
    let mut limpers = 0usize;
    let mut callers_after_open = 0usize;
    let mut hero_raised = false;
    let mut aggressor = None;
    let mut cbetter = None;
    let mut flop_bet = false;
    let mut cbet_raised = false;
    let mut special = false;
    let mut folded = HashSet::<u8>::new();
    let mut saw_hero_flop_bet = false;
    let mut cues = HashSet::<String>::new();
    for a in &h.actions {
        if a.kind == "board" {
            street = a.street.clone();
            live.clear();
            acted.clear();
            max_bet = 0;
            min_raise = h.bb;
            continue;
        }
        let Some(seat) = a.actor else {
            continue;
        };
        let previous = live.get(&seat).copied().unwrap_or(0);
        let stack = h
            .players
            .iter()
            .find(|p| p.seat == seat)
            .map(|p| p.stack)
            .unwrap_or(0);
        let remaining = stack - spent.get(&seat).copied().unwrap_or(0);
        let to_call = (max_bet - previous).max(0);
        let reopened = acted
            .get(&seat)
            .is_none_or(|last| max_bet - last >= min_raise);
        let responsive_opponent = h.players.iter().any(|p| {
            p.seat != seat
                && !folded.contains(&p.seat)
                && p.stack > spent.get(&p.seat).copied().unwrap_or(0)
        });
        let can_raise = remaining > to_call && reopened && responsive_opponent;
        let decision =
            matches!(a.kind.as_str(), "fold" | "check" | "call" | "bet" | "raise") && remaining > 0;
        if seat == hero && decision && street == "preflop" {
            mark(
                &mut stats,
                "vpip",
                true,
                matches!(a.kind.as_str(), "call" | "raise"),
            );
            mark(&mut stats, "pfr", can_raise, a.kind == "raise");
            mark(
                &mut stats,
                "rfi",
                raises == 0 && limpers == 0 && can_raise,
                a.kind == "raise",
            );
            mark(
                &mut stats,
                "three_bet",
                raises == 1 && can_raise,
                a.kind == "raise",
            );
            mark(
                &mut stats,
                "fold_three_bet",
                raises == 2 && hero_raised,
                a.kind == "fold",
            );
            if raises == 1
                && callers_after_open == 0
                && matches!(h.position.as_str(), "SB" | "BB" | "BTN/SB")
                && !hero_raised
            {
                mark(&mut stats, "blind_fold", true, a.kind == "fold");
                mark(&mut stats, "blind_call", true, a.kind == "call");
                mark(&mut stats, "blind_raise", true, a.kind == "raise");
                cues.insert("blind_defence".into());
            }
            if raises >= 2 && a.kind == "call" {
                cues.insert("call_three_bet".into());
            }
            if raises == 2 && hero_raised {
                cues.insert("open_vs_three_bet".into());
            }
        }
        if street == "flop" && decision {
            if seat == hero && aggressor == Some(hero) && !flop_bet {
                mark(&mut stats, "cbet", true, a.kind == "bet");
            }
            if seat == hero && cbetter.is_some_and(|s| s != hero) && !cbet_raised && to_call > 0 {
                mark(&mut stats, "fold_cbet", true, a.kind == "fold");
            }
            if a.kind == "bet" && !flop_bet && aggressor == Some(seat) {
                cbetter = Some(seat);
            }
            if matches!(a.kind.as_str(), "bet" | "raise") {
                flop_bet = true;
            }
            if a.kind == "raise" && cbetter.is_some() {
                cbet_raised = true;
            }
            if seat == hero && cbetter == Some(hero) && cbet_raised && to_call > 0 {
                cues.insert("cbet_vs_raise".into());
            }
            if seat == hero && a.kind == "bet" && cbetter == Some(hero) {
                saw_hero_flop_bet = true;
            }
        }
        if seat == hero && street == "turn" && a.kind == "check" && saw_hero_flop_bet {
            cues.insert("cbet_turn_check".into());
        }
        if seat == hero && street == "river" && a.kind == "call" {
            cues.insert("river_call".into());
        }
        match a.kind.as_str() {
            "small_blind" | "big_blind" | "straddle" | "call" | "bet" | "raise" => {
                *spent.entry(seat).or_default() += a.amount;
                *live.entry(seat).or_default() += a.amount;
                let new_level = live[&seat];
                if a.kind == "raise" || a.kind == "bet" {
                    let increase = new_level - max_bet;
                    if increase >= min_raise {
                        min_raise = increase;
                    } else if a.all_in {
                        special = true;
                    }
                    max_bet = max_bet.max(new_level);
                    if street == "preflop" {
                        raises += 1;
                        aggressor = Some(seat);
                        if seat == hero {
                            hero_raised = true;
                        }
                    }
                } else if a.kind == "straddle" {
                    max_bet = max_bet.max(new_level);
                    min_raise = max_bet;
                    special = true;
                } else if matches!(a.kind.as_str(), "small_blind" | "big_blind") {
                    max_bet = max_bet.max(new_level);
                }
                if street == "preflop" && a.kind == "call" {
                    if raises == 0 {
                        limpers += 1;
                    } else if raises == 1 {
                        callers_after_open += 1;
                    }
                }
            }
            "ante" | "dead_blind" => {
                *spent.entry(seat).or_default() += a.amount;
            }
            "return" => {
                *spent.entry(seat).or_default() -= a.amount;
                *live.entry(seat).or_default() -= a.amount;
            }
            "fold" => {
                folded.insert(seat);
            }
            _ => {}
        }
        if decision {
            acted.insert(seat, max_bet);
        }
    }
    h.pot_type = if special {
        "special"
    } else {
        match raises {
            0 => "limped",
            1 => "SRP",
            2 => "3-bet",
            _ => "4-bet+",
        }
    }
    .into();
    mark(&mut stats, "wtsd", h.saw_flop, h.showdown);
    mark(&mut stats, "wsd", h.showdown, h.showdown && hero_won);
    mark(&mut stats, "wwsf", h.saw_flop, h.saw_flop && hero_won);
    h.stats = stats;
    h.cues = cues.into_iter().collect();
    h.cues.sort();
}
