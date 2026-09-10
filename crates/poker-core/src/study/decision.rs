use crate::{model::Hand, money};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const INDEX_VERSION: &str = "hero-index/3";
pub const VERSION: &str = "hero-decisions/2";
pub const ACTIONS: &[&str] = &["fold", "check", "call", "bet", "raise"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineAction {
    pub street: String,
    pub actor: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRef {
    pub profile: String,
    pub hand_id: String,
    pub seq: usize,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub reference: DecisionRef,
    #[serde(default)]
    pub scenario: Option<String>,
    pub street: String,
    pub position: String,
    pub opponent: Option<String>,
    pub role: Option<String>,
    pub facing: String,
    pub pot_type: String,
    pub effective_bb: Option<f64>,
    pub action: String,
    pub size_bb: Option<f64>,
    pub bet_pct: Option<f64>,
    pub facing_pct: Option<f64>,
    pub preflop_path: String,
    pub line: Vec<LineAction>,
    pub legal: Vec<String>,
    pub board: Vec<String>,
    pub hand_class: String,
    pub cards: Vec<String>,
    pub player_count: usize,
    pub stacks_bb: BTreeMap<String, f64>,
    pub sb_bb: f64,
    pub bb: String,
    pub currency: String,
    pub game: String,
    pub special: bool,
    pub pot_before: String,
    pub to_call: String,
}

pub fn line_code(line: &[LineAction], street: &str) -> String {
    let order = |s: &str| match s {
        "preflop" => 0,
        "flop" => 1,
        "turn" => 2,
        _ => 3,
    };
    line.iter()
        .filter(|a| order(&a.street) >= order(street))
        .map(|a| format!("{}:{}:{}", a.street, a.actor, a.action))
        .collect::<Vec<_>>()
        .join("|")
}

/// Snapshot only information available BEFORE each Hero decision. No future board,
/// final pot type, eventual opponent or showdown cards enter the decision features.
pub fn derive(h: &Hand) -> Vec<Decision> {
    if h.status != "valid" || h.bb <= 0 {
        return vec![];
    }
    let mut result = vec![];
    let mut spent = BTreeMap::<u8, i64>::new();
    let mut live = BTreeMap::<u8, i64>::new();
    let mut acted = BTreeMap::<u8, i64>::new();
    let mut folded = BTreeSet::new();
    let mut max_bet = 0;
    let mut min_raise = h.bb;
    let mut raises = 0;
    let mut street_raises = 0;
    let mut special = false;
    let mut aggressor = None;
    let mut preflop_aggressor = None;
    let mut hero_opened = false;
    let mut last_pct = None;
    let mut preflop = vec![];
    let mut line = vec![];
    let mut board = vec![];
    let mut flop_count = None;
    let hero = h.players.iter().find(|p| p.hero);
    let stacks: BTreeMap<_, _> = h
        .players
        .iter()
        .map(|p| (p.position.clone(), p.stack as f64 / h.bb as f64))
        .collect();
    for (index, a) in h.actions.iter().enumerate() {
        if a.kind == "board" {
            if a.runout != 0 {
                break;
            }
            if a.street == "flop" {
                flop_count = Some(
                    h.players
                        .iter()
                        .filter(|p| !folded.contains(&p.seat))
                        .count(),
                );
            }
            board = a.cards.clone();
            live.clear();
            acted.clear();
            max_bet = 0;
            min_raise = h.bb;
            street_raises = 0;
            aggressor = None;
            last_pct = None;
            continue;
        }
        let Some(seat) = a.actor else { continue };
        let Some(player) = h.players.iter().find(|p| p.seat == seat) else {
            continue;
        };
        let previous = *live.get(&seat).unwrap_or(&0);
        let remaining = player.stack - spent.get(&seat).copied().unwrap_or(0);
        let to_call = (max_bet - previous).max(0);
        let before = a.pot_after - a.amount;
        let pct = (before > 0).then(|| a.amount as f64 / before as f64 * 100.);
        let is_decision = ACTIONS.contains(&a.kind.as_str()) && remaining > 0;
        let others: Vec<_> = h
            .players
            .iter()
            .filter(|p| p.seat != seat && !folded.contains(&p.seat))
            .collect();
        let responsive = others
            .iter()
            .any(|p| p.stack > spent.get(&p.seat).copied().unwrap_or(0));
        let can_raise = remaining > to_call
            && responsive
            && acted.get(&seat).is_none_or(|v| max_bet - *v >= min_raise);
        if is_decision && seat == h.hero_seat && (a.street == "preflop" || flop_count == Some(2)) {
            let villain = if others.len() == 1 {
                Some(others[0])
            } else {
                aggressor.and_then(|s| h.players.iter().find(|p| p.seat == s && s != seat))
            };
            let role = if a.street != "preflop" && others.len() == 1 {
                // Postflop action order is clockwise after button, independent of who bet.
                let mut ordered: Vec<_> = h
                    .players
                    .iter()
                    .filter(|p| !folded.contains(&p.seat))
                    .collect();
                ordered.sort_by_key(|p| (p.seat + h.max_seats - h.button - 1) % h.max_seats);
                Some(
                    if ordered.last().is_some_and(|p| p.seat == seat) {
                        "IP"
                    } else {
                        "OOP"
                    }
                    .into(),
                )
            } else {
                None
            };
            let mut legal = if to_call > 0 {
                vec!["fold".into(), "call".into()]
            } else {
                vec!["check".into()]
            };
            if can_raise {
                legal.push(if max_bet == 0 { "bet" } else { "raise" }.into());
            }
            let facing = if a.street == "preflop" {
                match raises {
                    0 => {
                        if preflop.iter().any(|x: &String| x == "C") {
                            "limp"
                        } else {
                            "unopened"
                        }
                    }
                    1 => "open",
                    2 => "three_bet",
                    _ => "four_bet_plus",
                }
            } else if to_call == 0 {
                "checked_to"
            } else if street_raises <= 1 {
                "bet"
            } else {
                "raise"
            };
            let path = line_code(&line, "flop");
            let scenario = if a.street == "preflop" && facing == "three_bet" && hero_opened {
                Some("open_vs_3bet")
            } else if a.street == "flop" && preflop_aggressor == Some(h.hero_seat)
                && matches!(path.as_str(), "flop:hero:bet|flop:villain:raise" | "flop:villain:check|flop:hero:bet|flop:villain:raise") {
                Some("cbet_vs_raise")
            } else if a.street == "turn" && preflop_aggressor.is_some_and(|s| s != h.hero_seat)
                && matches!(path.as_str(), "flop:hero:check|flop:villain:bet|flop:hero:call|turn:hero:check|turn:villain:bet" | "flop:villain:bet|flop:hero:call|turn:villain:bet") {
                Some("facing_second_barrel")
            } else { None };
            result.push(Decision {
                scenario: scenario.map(str::to_owned),
                reference: DecisionRef {
                    profile: h.profile.clone(),
                    hand_id: h.id.clone(),
                    seq: index,
                    version: VERSION.into(),
                },
                street: a.street.clone(),
                position: h.position.clone(),
                opponent: villain.map(|p| p.position.clone()),
                role,
                facing: facing.into(),
                pot_type: (if special {
                    "special"
                } else {
                    match raises {
                        0 => "limped",
                        1 => "SRP",
                        2 => "3-bet",
                        _ => "4-bet+",
                    }
                })
                .into(),
                effective_bb: villain.map(|p| p.stack.min(player.stack) as f64 / h.bb as f64),
                action: a.kind.clone(),
                size_bb: match a.kind.as_str() {
                    "raise" => Some(a.to as f64 / h.bb as f64),
                    "bet" => Some(a.amount as f64 / h.bb as f64),
                    _ => None,
                },
                bet_pct: if a.kind == "raise" || a.kind == "bet" {
                    pct
                } else {
                    None
                },
                facing_pct: if to_call > 0 { last_pct } else { None },
                preflop_path: preflop.join("-"),
                line: line.clone(),
                legal,
                board: board.clone(),
                hand_class: h.hand_class.clone(),
                cards: hero.map(|p| p.cards.clone()).unwrap_or_default(),
                player_count: h.player_count,
                stacks_bb: stacks.clone(),
                sb_bb: h.sb as f64 / h.bb as f64,
                bb: money::format(h.bb),
                currency: h.currency.clone(),
                game: h.game.clone(),
                special,
                pot_before: money::format(before),
                to_call: money::format(to_call),
            });
        }
        if is_decision {
            if a.street == "preflop" {
                preflop.push(match a.kind.as_str() {
                    "fold" => "F".into(),
                    "call" => "C".into(),
                    "check" => "X".into(),
                    _ if a.all_in => "RAI".into(),
                    _ => format!("R{}", a.to as f64 / h.bb as f64),
                });
            } else {
                line.push(LineAction {
                    street: a.street.clone(),
                    actor: if seat == h.hero_seat {
                        "hero"
                    } else {
                        "villain"
                    }
                    .into(),
                    action: a.kind.clone(),
                });
            }
        }
        match a.kind.as_str() {
            "small_blind" | "big_blind" | "straddle" | "call" | "bet" | "raise" => {
                *spent.entry(seat).or_default() += a.amount;
                *live.entry(seat).or_default() += a.amount;
                let level = live[&seat];
                if a.kind == "raise" || a.kind == "bet" {
                    let increase = level - max_bet;
                    if increase >= min_raise {
                        min_raise = increase;
                    } else if a.all_in {
                        special = true;
                    }
                    max_bet = max_bet.max(level);
                    aggressor = Some(seat);
                    last_pct = pct;
                    street_raises += 1;
                    if a.street == "preflop" {
                        if raises == 0 && seat == h.hero_seat {
                            hero_opened = true;
                        }
                        preflop_aggressor = Some(seat);
                        raises += 1;
                    }
                } else if a.kind == "straddle" {
                    max_bet = max_bet.max(level);
                    min_raise = max_bet;
                    special = true;
                } else if a.kind.ends_with("blind") {
                    max_bet = max_bet.max(level);
                }
            }
            "ante" | "dead_blind" => {
                *spent.entry(seat).or_default() += a.amount;
                special = true;
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
        if is_decision {
            acted.insert(seat, max_bet);
        }
    }
    result
}
