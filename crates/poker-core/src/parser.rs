use crate::{model::*, money, stats};
use anyhow::{bail, Context, Result};
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::LazyLock,
};

static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Poker Hand #([^:]+): Hold'em No Limit \(\$([\d.]+)/\$([\d.]+)\) - (\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2})").unwrap()
});
static SEAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Seat (\d+): (.+?) \(\$([\d.]+) in chips\)").unwrap());
static TABLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Table '(.+)' (\d+)-max Seat #(\d+) is the button").unwrap());
static BRACKETS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]").unwrap());
static CASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$([\d.]+)").unwrap());
static RETURN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Uncalled bet \(\$([\d.]+)\) returned to (.+)$").unwrap());
static COLLECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?) collected \$([\d.]+) from (.+)$").unwrap());
static FEE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([A-Za-z ]+) \$([\d.]+)").unwrap());

pub fn cards(s: &str) -> Vec<String> {
    BRACKETS
        .captures_iter(s)
        .flat_map(|c| {
            c[1].split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn amount(s: &str) -> Result<i64> {
    money::parse(&CASH.captures(s).context("missing dollar amount")?[1])
}

pub fn classify_cards(cards: &[String]) -> String {
    if cards.len() != 2 {
        return String::new();
    }
    let ranks = "23456789TJQKA";
    let a = cards[0].chars().next().unwrap_or('?');
    let b = cards[1].chars().next().unwrap_or('?');
    if a == b {
        return format!("{a}{b}");
    }
    let (hi, lo) = if ranks.find(a) > ranks.find(b) {
        (a, b)
    } else {
        (b, a)
    };
    format!(
        "{hi}{lo}{}",
        if cards[0].as_bytes().get(1) == cards[1].as_bytes().get(1) {
            's'
        } else {
            'o'
        }
    )
}

fn issue(h: &mut Hand, code: &str, message: impl Into<String>, line: Option<usize>) {
    h.issues.push(Issue {
        code: code.into(),
        message: message.into(),
        line,
    });
}

/// GG export parser. All amounts are incremental ledger entries; source text is immutable.
pub fn parse(raw: &str, profile: &Profile) -> Result<Hand> {
    if raw.len() > 1_048_576 {
        bail!("hand exceeds 1 MiB limit")
    }
    let raw = raw.trim().trim_start_matches('\u{feff}');
    let head = HEADER
        .captures(raw)
        .context("Unsupported header: expected GG NLHE cash in USD")?;
    let tz: Tz = profile
        .timezone
        .parse()
        .context("invalid source timezone")?;
    let naive = NaiveDateTime::parse_from_str(&head[4], "%Y/%m/%d %H:%M:%S")?;
    let played_at = tz
        .from_local_datetime(&naive)
        .single()
        .context("ambiguous/nonexistent local time; choose explicit source timezone")?
        .timestamp();
    let sb = money::parse(&head[2])?;
    let bb = money::parse(&head[3])?;
    if sb <= 0 || bb <= 0 || sb > bb {
        bail!("invalid blinds")
    }
    let mut h = Hand {
        id: head[1].into(),
        profile: profile.id.clone(),
        brand: profile.brand.clone(),
        played_at,
        local_time: naive.format("%Y-%m-%d %H:%M:%S").to_string(),
        timezone: profile.timezone.clone(),
        table_name: String::new(),
        max_seats: 0,
        player_count: 0,
        currency: "USD".into(),
        game: "Cash".into(),
        sb,
        bb,
        button: 0,
        hero_seat: 0,
        position: String::new(),
        hand_class: String::new(),
        players: vec![],
        actions: vec![],
        boards: vec![vec![]],
        pots: vec![],
        invested: 0,
        returned: 0,
        collected: 0,
        cashout: 0,
        cashout_risk: 0,
        net: 0,
        total_pot: 0,
        fees: BTreeMap::new(),
        showdown: false,
        saw_flop: false,
        pot_type: String::new(),
        flop_players: 0,
        hu_effective_bb: None,
        hero_stack_bb: 0.,
        texture: String::new(),
        paired: false,
        high_card: String::new(),
        stats: BTreeMap::new(),
        cues: vec![],
        status: "valid".into(),
        issues: vec![],
        ev_status: "not_applicable".into(),
        ev_reason: None,
        equity_input: None,
        equity: None,
        adjusted_net: None,
        raw: raw.into(),
        parser_version: PARSER_VERSION.into(),
        stats_version: STATS_VERSION.into(),
    };
    let mut names = HashMap::<String, u8>::new();
    let mut live = HashMap::<u8, i64>::new();
    let mut invested = HashMap::<u8, i64>::new();
    let mut returned = HashMap::<u8, i64>::new();
    let mut folded = HashSet::<u8>::new();
    let mut dealt = HashSet::<u8>::new();
    let mut street = "preflop".to_string();
    let mut runout = 0usize;
    let mut pot = 0i64;
    let mut awards = 0i64;
    let mut summary = false;
    let mut total_found = false;
    let mut showdown_marker = false;
    let mut hero_showed = false;
    let mut collected_hero = false;

    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Poker Hand #") {
            continue;
        }
        if line.starts_with("*** SUMMARY") {
            summary = true;
            continue;
        }
        if summary {
            if line.starts_with("Total pot ") {
                h.total_pot = amount(line)?;
                for c in FEE.captures_iter(line) {
                    let name = c[1].trim();
                    if name != "Total pot" {
                        h.fees
                            .insert(name.into(), money::format(money::parse(&c[2])?));
                    }
                }
                total_found = true;
            }
            // Summary repeats awards. Never add its "won" or "collected" amounts.
            continue;
        }
        if let Some(c) = TABLE.captures(line) {
            h.table_name = c[1].into();
            h.max_seats = c[2].parse()?;
            h.button = c[3].parse()?;
            let t = h.table_name.to_lowercase();
            if t.contains("rush") || h.id.starts_with("RC") {
                h.game = "Rush".into();
            }
            continue;
        }
        if let Some(c) = SEAT.captures(line) {
            let seat: u8 = c[1].parse()?;
            let name = c[2].to_string();
            if names.contains_key(&name) || h.players.iter().any(|p| p.seat == seat) {
                bail!("duplicate player/seat")
            }
            let hero = name == profile.hero;
            if hero {
                h.hero_seat = seat;
            }
            names.insert(name.clone(), seat);
            h.players.push(Player {
                seat,
                name,
                hero,
                stack: money::parse(&c[3])?,
                cards: vec![],
                position: String::new(),
            });
            continue;
        }
        if let Some(s) = line.strip_prefix("Dealt to ") {
            let name = s.split(" [").next().unwrap_or(s).trim();
            if let Some(seat) = names.get(name) {
                dealt.insert(*seat);
                let cs = cards(line);
                if !cs.is_empty() {
                    if let Some(p) = h.players.iter_mut().find(|p| p.seat == *seat) {
                        p.cards = cs;
                    }
                }
            } else {
                issue(
                    &mut h,
                    "unknown_player",
                    "Dealt-to player missing from seat list",
                    Some(line_no + 1),
                );
            }
            continue;
        }
        if line.starts_with("*** HOLE CARDS") {
            continue;
        }
        if line.starts_with("***") {
            if line.contains("SHOWDOWN") || line.contains("SHOW DOWN") {
                showdown_marker = true;
                continue;
            }
            let next_street = if line.contains("FLOP") {
                "flop"
            } else if line.contains("TURN") {
                "turn"
            } else if line.contains("RIVER") {
                "river"
            } else {
                issue(&mut h, "unknown_street", line, Some(line_no + 1));
                continue;
            };
            let next_runout = if line.contains("SECOND") {
                1
            } else if line.contains("THIRD") {
                2
            } else {
                0
            };
            if next_street != street || next_runout != runout {
                live.clear();
            }
            street = next_street.into();
            runout = next_runout;
            while h.boards.len() <= runout {
                h.boards.push(vec![]);
            }
            let cs = cards(line);
            let wanted = match next_street {
                "flop" => 3,
                "turn" => 4,
                _ => 5,
            };
            let board = if cs.len() == wanted {
                cs
            } else if cs.len() == 1 && wanted > 3 {
                let mut prefix = if h.boards[runout].len() >= wanted - 1 {
                    h.boards[runout][..wanted - 1].to_vec()
                } else {
                    h.boards[0].iter().take(wanted - 1).cloned().collect()
                };
                prefix.extend(cs);
                prefix
            } else {
                cs
            };
            if board.len() != wanted {
                issue(
                    &mut h,
                    "board_length",
                    format!("{street} board requires {wanted} cards"),
                    Some(line_no + 1),
                );
            }
            h.boards[runout] = board.clone();
            if street == "flop" && runout == 0 {
                h.saw_flop = h.hero_seat != 0 && !folded.contains(&h.hero_seat);
                h.flop_players = h
                    .players
                    .iter()
                    .filter(|p| {
                        !folded.contains(&p.seat) && (dealt.is_empty() || dealt.contains(&p.seat))
                    })
                    .count();
                if h.flop_players == 2 {
                    let v: Vec<_> = h
                        .players
                        .iter()
                        .filter(|p| {
                            !folded.contains(&p.seat)
                                && (dealt.is_empty() || dealt.contains(&p.seat))
                        })
                        .collect();
                    h.hu_effective_bb = v
                        .iter()
                        .map(|p| p.stack as f64 / bb as f64)
                        .reduce(f64::min);
                }
            }
            h.actions.push(Action {
                seq: h.actions.len(),
                street: street.clone(),
                runout,
                actor: None,
                kind: "board".into(),
                amount: 0,
                to: 0,
                all_in: false,
                cards: board,
                pot_after: pot,
            });
            continue;
        }
        if let Some(c) = RETURN.captures(line) {
            let seat = *names.get(c[2].trim()).context("unknown return recipient")?;
            let v = money::parse(&c[1])?;
            *returned.entry(seat).or_default() += v;
            *live.entry(seat).or_default() -= v;
            pot -= v;
            h.actions.push(Action {
                seq: h.actions.len(),
                street: street.clone(),
                runout,
                actor: Some(seat),
                kind: "return".into(),
                amount: v,
                to: 0,
                all_in: false,
                cards: vec![],
                pot_after: pot,
            });
            continue;
        }
        if let Some(c) = COLLECT.captures(line) {
            let seat = *names.get(c[1].trim()).context("unknown payout recipient")?;
            let v = money::parse(&c[2])?;
            awards += v;
            if seat == h.hero_seat {
                h.collected += v;
                collected_hero = true;
            }
            h.actions.push(Action {
                seq: h.actions.len(),
                street: street.clone(),
                runout,
                actor: Some(seat),
                kind: "collect".into(),
                amount: v,
                to: 0,
                all_in: false,
                cards: vec![],
                pot_after: pot,
            });
            continue;
        }
        if let Some((name, text)) = line.split_once(": ") {
            let Some(&seat) = names.get(name) else {
                issue(
                    &mut h,
                    "unknown_actor",
                    format!("Unknown actor on line {}", line_no + 1),
                    Some(line_no + 1),
                );
                continue;
            };
            let (kind, v, to) = if text.starts_with("posts small blind") {
                ("small_blind", amount(text)?, 0)
            } else if text.starts_with("posts big blind") {
                ("big_blind", amount(text)?, 0)
            } else if text.starts_with("posts missed blind") || text.starts_with("posts dead blind")
            {
                ("dead_blind", amount(text)?, 0)
            } else if text.starts_with("posts") && text.contains("ante") {
                ("ante", amount(text)?, 0)
            } else if text.starts_with("straddle") || text.starts_with("posts straddle") {
                let total = amount(text)?;
                (
                    "straddle",
                    total - live.get(&seat).copied().unwrap_or(0),
                    total,
                )
            } else if text.starts_with("folds") {
                folded.insert(seat);
                ("fold", 0, 0)
            } else if text.starts_with("checks") {
                ("check", 0, 0)
            } else if text.starts_with("calls") {
                ("call", amount(text)?, 0)
            } else if text.starts_with("bets") {
                ("bet", amount(text)?, 0)
            } else if text.starts_with("raises") {
                let t = text
                    .split_once(" to ")
                    .context("raise missing total-to amount")?
                    .1;
                let total = amount(t)?;
                (
                    "raise",
                    total - live.get(&seat).copied().unwrap_or(0),
                    total,
                )
            } else if text.starts_with("shows ") {
                if let Some(p) = h.players.iter_mut().find(|p| p.seat == seat) {
                    p.cards = cards(text);
                }
                if seat == h.hero_seat {
                    hero_showed = true;
                }
                ("show", 0, 0)
            } else if text.starts_with("mucks") || text.starts_with("doesn't show") {
                ("muck", 0, 0)
            } else if text.starts_with("Chooses to EV Cashout") {
                ("cashout_choice", 0, 0)
            } else if text.starts_with("Receives Cashout") {
                let v = amount(text)?;
                if seat == h.hero_seat {
                    h.cashout += v;
                }
                ("cashout_receive", v, 0)
            } else if text.starts_with("Pays Cashout Risk") {
                let v = amount(text)?;
                if seat == h.hero_seat {
                    h.cashout_risk += v;
                }
                ("cashout_risk", v, 0)
            } else {
                issue(
                    &mut h,
                    "unknown_action",
                    format!("Unrecognized action: {text}"),
                    Some(line_no + 1),
                );
                continue;
            };
            if v < 0 {
                issue(
                    &mut h,
                    "negative_delta",
                    format!("Negative {kind} contribution"),
                    Some(line_no + 1),
                );
            }
            let own_live = live.get(&seat).copied().unwrap_or(0);
            let level = live.values().copied().max().unwrap_or(0);
            let call_due = (level - own_live).max(0);
            let remaining = h
                .players
                .iter()
                .find(|p| p.seat == seat)
                .map(|p| p.stack)
                .unwrap_or(0)
                - invested.get(&seat).copied().unwrap_or(0)
                + returned.get(&seat).copied().unwrap_or(0);
            if (kind == "call" && (v != call_due.min(remaining) || v <= 0))
                || (kind == "check" && call_due > 0)
                || (kind == "bet" && (level > 0 || v <= 0))
                || (kind == "raise" && to <= level)
            {
                issue(
                    &mut h,
                    "action_mismatch",
                    format!("Seat {seat} {kind} amount does not match current betting level"),
                    Some(line_no + 1),
                );
            }
            if matches!(
                kind,
                "call"
                    | "bet"
                    | "raise"
                    | "small_blind"
                    | "big_blind"
                    | "dead_blind"
                    | "ante"
                    | "straddle"
            ) && v > remaining
            {
                issue(
                    &mut h,
                    "over_stack",
                    format!("Seat {seat} action exceeds remaining stack"),
                    Some(line_no + 1),
                );
            }
            if matches!(
                kind,
                "small_blind"
                    | "big_blind"
                    | "dead_blind"
                    | "ante"
                    | "straddle"
                    | "call"
                    | "bet"
                    | "raise"
            ) {
                *invested.entry(seat).or_default() += v;
                pot += v;
                if !matches!(kind, "dead_blind" | "ante") {
                    *live.entry(seat).or_default() += v;
                }
            }
            let show_cards = if kind == "show" { cards(text) } else { vec![] };
            h.actions.push(Action {
                seq: h.actions.len(),
                street: street.clone(),
                runout,
                actor: Some(seat),
                kind: kind.into(),
                amount: v,
                to,
                all_in: text.contains("all-in"),
                cards: show_cards,
                pot_after: pot,
            });
            continue;
        }
        if line.contains("$") {
            issue(
                &mut h,
                "unknown_money_line",
                format!("Unrecognized settlement on line {}", line_no + 1),
                Some(line_no + 1),
            );
        }
    }
    if h.hero_seat == 0 {
        bail!("Hero '{}' not in this hand", profile.hero);
    }
    if h.max_seats < 2 || h.max_seats > 9 {
        bail!("unsupported or missing table seat count")
    }
    if !total_found || !summary {
        issue(
            &mut h,
            "missing_summary",
            "Missing complete pot summary",
            None,
        );
    }
    h.player_count = if dealt.is_empty() {
        h.players.len()
    } else {
        dealt.len()
    };
    if !dealt.is_empty() {
        h.players.retain(|p| dealt.contains(&p.seat));
    }
    assign_positions(&mut h);
    let hero = h
        .players
        .iter()
        .find(|p| p.hero)
        .context("Hero not dealt in")?;
    h.position = hero.position.clone();
    h.hero_stack_bb = hero.stack as f64 / bb as f64;
    h.hand_class = classify_cards(&hero.cards);
    if hero.cards.len() != 2 {
        issue(
            &mut h,
            "missing_hero_cards",
            "Missing Hero hole cards",
            None,
        );
    }
    h.invested = invested.get(&h.hero_seat).copied().unwrap_or(0);
    h.returned = returned.get(&h.hero_seat).copied().unwrap_or(0);
    h.net = h.collected + h.cashout - h.cashout_risk + h.returned - h.invested;
    // A GG SHOWDOWN marker also appears in hands won without a showdown.
    let remaining = h
        .players
        .iter()
        .filter(|p| !folded.contains(&p.seat))
        .count();
    h.showdown = showdown_marker
        && !folded.contains(&h.hero_seat)
        && remaining >= 2
        && (hero_showed || h.saw_flop);
    if pot != h.total_pot {
        let message = format!(
            "Action ledger {} differs from total pot {} (delta {})",
            money::format(pot),
            money::format(h.total_pot),
            money::format(pot - h.total_pot)
        );
        issue(&mut h, "pot_mismatch", message, None);
    }
    let fee_total: i64 = h.fees.values().map(|s| money::parse(s).unwrap_or(0)).sum();
    if total_found && h.total_pot - fee_total != awards {
        issue(
            &mut h,
            "payout_mismatch",
            "Net pot does not reconcile with collected events",
            None,
        );
    }
    for player in h.players.clone() {
        let v = invested.get(&player.seat).copied().unwrap_or(0)
            - returned.get(&player.seat).copied().unwrap_or(0);
        if v < 0 || v > player.stack {
            issue(
                &mut h,
                "stack_mismatch",
                format!(
                    "Seat {} contribution exceeds stack or is negative",
                    player.seat
                ),
                None,
            );
        }
    }
    let contributions: Vec<(u8, i64)> = h
        .players
        .iter()
        .map(|p| {
            (
                p.seat,
                invested.get(&p.seat).copied().unwrap_or(0)
                    - returned.get(&p.seat).copied().unwrap_or(0),
            )
        })
        .collect();
    let mut levels: Vec<i64> = contributions
        .iter()
        .filter_map(|(_, v)| (*v > 0).then_some(*v))
        .collect();
    levels.sort_unstable();
    levels.dedup();
    let mut previous = 0;
    for level in levels {
        let amount =
            (level - previous) * contributions.iter().filter(|(_, v)| *v >= level).count() as i64;
        let eligible: Vec<u8> = contributions
            .iter()
            .filter_map(|(s, v)| (*v >= level && !folded.contains(s)).then_some(*s))
            .collect();
        if let Some(last) = h.pots.last_mut().filter(|p| p.eligible == eligible) {
            last.amount += amount;
        } else {
            h.pots.push(Pot { amount, eligible });
        }
        previous = level;
    }
    validate_cards(&mut h);
    if let Some(board) = h.boards.first().filter(|b| b.len() >= 3) {
        let b = &board[..3];
        let suits: HashSet<_> = b.iter().filter_map(|c| c.chars().nth(1)).collect();
        h.texture = match suits.len() {
            1 => "monotone",
            2 => "two-tone",
            _ => "rainbow",
        }
        .into();
        let ranks: HashSet<_> = b.iter().filter_map(|c| c.chars().next()).collect();
        h.paired = ranks.len() < 3;
        h.high_card = ranks
            .iter()
            .max_by_key(|c| "23456789TJQKA".find(**c))
            .map(|c| c.to_string())
            .unwrap_or_default();
    }
    if !h.issues.is_empty() {
        h.status = "quarantined".into();
    }
    stats::derive(&mut h, collected_hero);
    crate::equity::classify(&mut h);
    Ok(h)
}

fn assign_positions(h: &mut Hand) {
    let mut seats: Vec<u8> = h.players.iter().map(|p| p.seat).collect();
    seats.sort_unstable();
    // Rotate from the first dealt seat clockwise after the physical button.
    let split = seats.iter().position(|s| *s > h.button).unwrap_or(0);
    seats.rotate_left(split);
    let n = seats.len();
    let labels: Vec<&str> = match n {
        2 => vec!["BB", "BTN/SB"],
        3 => vec!["SB", "BB", "BTN"],
        4 => vec!["SB", "BB", "CO", "BTN"],
        5 => vec!["SB", "BB", "HJ", "CO", "BTN"],
        6 => vec!["SB", "BB", "UTG", "HJ", "CO", "BTN"],
        7 => vec!["SB", "BB", "UTG", "MP", "HJ", "CO", "BTN"],
        8 => vec!["SB", "BB", "UTG", "UTG+1", "MP", "HJ", "CO", "BTN"],
        9 => vec!["SB", "BB", "UTG", "UTG+1", "UTG+2", "MP", "HJ", "CO", "BTN"],
        _ => vec![],
    };
    for (seat, label) in seats.iter().zip(labels) {
        if let Some(p) = h.players.iter_mut().find(|p| p.seat == *seat) {
            p.position = label.into();
        }
    }
}

fn validate_cards(h: &mut Hand) {
    let hole: Vec<_> = h.players.iter().flat_map(|p| p.cards.clone()).collect();
    let valid = |c: &String| {
        c.len() == 2
            && "23456789TJQKA".contains(c.chars().next().unwrap())
            && "cdhs".contains(c.chars().nth(1).unwrap())
    };
    if hole.iter().any(|c| !valid(c)) || h.boards.iter().flatten().any(|c| !valid(c)) {
        issue(h, "invalid_card", "Invalid rank/suit", None);
        return;
    }
    for b in &h.boards {
        let combined: Vec<_> = hole.iter().chain(b.iter()).collect();
        if combined.iter().copied().collect::<HashSet<_>>().len() != combined.len() {
            issue(h, "duplicate_card", "Duplicate card within a runout", None);
            return;
        }
    }
}
