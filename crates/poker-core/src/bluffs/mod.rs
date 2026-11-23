mod cards;
mod report;

use crate::{
    model::Hand,
    money,
    study::decision::{Snapshot, ACTIONS},
};
use anyhow::{ensure, Result};
use cards::{classify, Holding};
pub use report::{query, Query};
use rusqlite::{params, Connection};
use std::collections::BTreeSet;

pub(crate) fn missing_cards_only(h: &Hand) -> bool {
    !h.issues.is_empty() && h.issues.iter().all(|i| i.code == "missing_hero_cards")
}

pub(crate) fn init(c: &Connection) -> Result<()> {
    c.execute_batch(
        "
      CREATE TABLE IF NOT EXISTS bluff_samples(
        hand INTEGER REFERENCES hands(id) ON DELETE CASCADE, view TEXT, category TEXT,
        seq INTEGER, start_seq INTEGER, street TEXT, holding TEXT, outs INTEGER, gutshot INTEGER,
        missed INTEGER, action TEXT, response TEXT, players INTEGER, opponent TEXT, depth REAL,
        texture TEXT, size_pct REAL, brand TEXT, bb INTEGER, cashflow INTEGER, exclusion TEXT,
        signature TEXT, PRIMARY KEY(hand,view,category,seq));
      CREATE INDEX IF NOT EXISTS bluff_view ON bluff_samples(view,category,hand,seq);
      CREATE TABLE IF NOT EXISTS bluff_marks(hand INTEGER REFERENCES hands(id) ON DELETE CASCADE,
        signature TEXT, PRIMARY KEY(hand,signature));",
    )?;
    Ok(())
}

struct Spot<'a> {
    snap: &'a Snapshot,
    holding: Holding,
    action: &'static str,
    missed: bool,
    size: Option<f64>,
    texture: String,
    response: &'static str,
    signature: String,
}

impl Spot<'_> {
    fn seq(&self) -> usize {
        self.snap.decision.reference.seq
    }
    fn street(&self) -> &str {
        &self.snap.decision.street
    }
    fn aggressive(&self) -> bool {
        matches!(self.action, "bet" | "raise" | "check_raise")
    }
    fn eligible(&self) -> bool {
        self.snap
            .decision
            .legal
            .iter()
            .any(|s| s == "bet" || s == "raise")
    }
}

fn response(h: &Hand, seq: usize) -> &'static str {
    let street = &h.actions[seq].street;
    let mut live: BTreeSet<_> = h.players.iter().map(|p| p.seat).collect();
    live.remove(&h.hero_seat);
    for a in &h.actions[..seq] {
        if a.kind == "fold" {
            if let Some(s) = a.actor {
                live.remove(&s);
            }
        }
    }
    let mut called = false;
    for a in h.actions[seq + 1..]
        .iter()
        .take_while(|a| &a.street == street)
    {
        if a.actor == Some(h.hero_seat) && ACTIONS.contains(&a.kind.as_str()) {
            break;
        }
        match a.kind.as_str() {
            "raise" => return "raised",
            "call" => called = true,
            "fold" => {
                if let Some(s) = a.actor {
                    live.remove(&s);
                }
            }
            _ => {}
        }
    }
    if called {
        "called"
    } else if live.is_empty() {
        "folded"
    } else {
        "contested"
    }
}

// Decisions can stop before the final board is dealt (all-in runout).
fn cashflow(h: &Hand, first: usize, last: usize) -> (Option<i64>, Option<&'static str>) {
    let street = &h.actions[last].street;
    let end = h.actions[last + 1..]
        .iter()
        .position(|a| a.street != *street && ACTIONS.contains(&a.kind.as_str()))
        .map_or(h.actions.len(), |i| last + 1 + i);
    let terminal = end == h.actions.len();
    if terminal {
        let reason = if h.actions.iter().any(|a| a.kind.starts_with("cashout")) {
            Some("cashout")
        } else if h.boards.len() > 1 {
            Some("runouts")
        } else if h
            .actions
            .iter()
            .filter(|a| a.kind == "collect" && a.amount > 0)
            .filter_map(|a| a.actor)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
        {
            Some(if h.pots.len() > 1 {
                "side_pot"
            } else {
                "split"
            })
        } else {
            None
        };
        if reason.is_some() {
            return (None, reason);
        }
    }
    let mut net = 0;
    for a in &h.actions[first..end] {
        if a.actor != Some(h.hero_seat) {
            continue;
        }
        match a.kind.as_str() {
            "bet" | "call" | "raise" => net -= a.amount,
            "return" => net += a.amount,
            "collect" if terminal => net += a.amount,
            _ => {}
        }
    }
    (Some(net), None)
}

fn texture(board: &[String]) -> String {
    let mut suits = [0; 256];
    let mut ranks = BTreeSet::new();
    for card in board {
        if let [r, s] = *card.as_bytes() {
            suits[s as usize] += 1;
            ranks.insert(r);
        }
    }
    let suit = match suits.into_iter().max().unwrap_or(0) {
        0 | 1 => "rainbow",
        2 => "two_suit",
        3 => "three_suit",
        _ => "four_plus_suit",
    };
    format!(
        "{}_{}",
        if ranks.len() < board.len() {
            "paired"
        } else {
            "unpaired"
        },
        suit
    )
}

fn spots<'a>(h: &Hand, snaps: &'a [Snapshot]) -> Result<Vec<Spot<'a>>> {
    let mut spots = Vec::new();
    let mut last_street = "";
    let mut draw_seen = false;
    let mut current: Option<Holding> = None;
    let mut ordinal = 0;
    for snap in snaps.iter().filter(|s| s.decision.street != "preflop") {
        let d = &snap.decision;
        if d.street != last_street {
            if let Some(old) = current {
                draw_seen |= old.outs > 0;
            }
            current = Some(classify(&d.cards, &d.board));
            last_street = &d.street;
            ordinal = 0;
        }
        let holding = current.unwrap();
        let a = &h.actions[d.reference.seq];
        let checked = d
            .line
            .iter()
            .rev()
            .find(|a| a.street == d.street && a.actor == "hero")
            .is_some_and(|a| a.action == "check");
        let action = match d.action.as_str() {
            "bet" => "bet",
            "raise" if checked => "check_raise",
            "raise" => "raise",
            "call" => "call",
            "fold" => "fold",
            _ => "check",
        };
        let aggressive = matches!(action, "bet" | "raise" | "check_raise");
        let call = money::parse(&d.to_call)?;
        let pot = money::parse(&d.pot_before)?;
        let size = (aggressive && pot + call > 0)
            .then(|| (a.amount - call) as f64 * 100. / (pot + call) as f64);
        let signature = if aggressive {
            ordinal += 1;
            format!(
                "{}:{}:{}:{ordinal}:{}:{}",
                d.street,
                d.board.join(""),
                a.kind,
                a.amount,
                a.to
            )
        } else {
            String::new()
        };
        spots.push(Spot {
            snap,
            holding,
            action,
            missed: d.street == "river" && holding.class == "air" && draw_seen,
            size,
            texture: texture(&d.board),
            response: if aggressive {
                response(h, d.reference.seq)
            } else {
                "none"
            },
            signature,
        });
    }
    Ok(spots)
}

fn checked_through(h: &Hand, street: &str) -> bool {
    let Some(start) = h
        .actions
        .iter()
        .position(|a| a.kind == "board" && a.street == street)
    else {
        return false;
    };
    let mut available: std::collections::BTreeMap<_, _> =
        h.players.iter().map(|p| (p.seat, p.stack)).collect();
    for a in &h.actions[..start] {
        let Some(seat) = a.actor else { continue };
        if a.kind == "fold" {
            available.remove(&seat);
            continue;
        }
        if let Some(stack) = available.get_mut(&seat) {
            match a.kind.as_str() {
                "small_blind" | "big_blind" | "dead_blind" | "ante" | "straddle" | "bet"
                | "raise" | "call" => *stack -= a.amount,
                "return" => *stack += a.amount,
                _ => {}
            }
        }
    }
    let expected: BTreeSet<_> = available
        .into_iter()
        .filter_map(|(s, stack)| (stack > 0).then_some(s))
        .collect();
    let mut checked = BTreeSet::new();
    for a in h.actions[start + 1..]
        .iter()
        .take_while(|a| a.street == street)
    {
        if !ACTIONS.contains(&a.kind.as_str()) {
            continue;
        }
        if a.kind != "check" {
            return false;
        }
        checked.extend(a.actor);
    }
    expected.len() >= 2 && checked == expected
}

fn route(h: &Hand, path: &[&Spot<'_>]) -> String {
    let mut key = path
        .iter()
        .map(|s| s.street())
        .collect::<Vec<_>>()
        .join("_");
    if key == "flop_river" && !checked_through(h, "turn") {
        key.push_str("_interrupted");
    }
    if path.len() > 1 && path[0].action == "check_raise" && path[1].action == "bet" {
        key.push_str("_xr_lead");
    } else if path.iter().any(|s| s.action != "bet") {
        key.push_str("_raise");
    }
    key
}

pub(crate) fn index_hand(c: &Connection, id: i64, h: &Hand, snaps: &[Snapshot]) -> Result<()> {
    c.execute("DELETE FROM bluff_samples WHERE hand=?1", [id])?;
    if (h.status != "valid" && !missing_cards_only(h))
        || !matches!(h.game.as_str(), "Cash" | "Rush")
        || h.bb <= 0
    {
        return Ok(());
    }
    let spots = spots(h, snaps)?;
    let mut insert = c.prepare_cached("INSERT INTO bluff_samples(hand,view,category,seq,start_seq,street,holding,outs,gutshot,missed,action,response,players,opponent,depth,texture,size_pct,brand,bb,cashflow,exclusion,signature) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")?;
    let mut record = |view: &str, category: &str, first: &Spot, last: &Spot| -> Result<()> {
        let (cash, reason) = if h.status == "valid" {
            cashflow(h, first.seq(), last.seq())
        } else {
            (None, Some("unknown"))
        };
        // Continuation denominators are conditioned on the prior shot, before the next action.
        let context = if view == "next" { first } else { last };
        let d = &context.snap.decision;
        insert.execute(params![
            id,
            view,
            category,
            last.seq(),
            first.seq(),
            d.street,
            context.holding.class,
            context.holding.outs,
            context.holding.gutshot,
            context.missed,
            context.action,
            context.response,
            context.snap.street_players,
            d.opponent
                .as_deref()
                .filter(|_| context.snap.street_players == 2),
            context.snap.remaining_bb,
            context.texture,
            context.size,
            h.brand,
            h.bb,
            cash,
            reason,
            last.signature
        ])?;
        Ok(())
    };
    let mut path = vec![];
    for street in ["flop", "turn", "river"] {
        let street_spots: Vec<_> = spots.iter().filter(|s| s.street() == street).collect();
        let first = street_spots.iter().copied().find(|s| s.aggressive());
        for s in street_spots.iter().copied().filter(|s| s.aggressive()) {
            let view = if first.is_some_and(|f| f.seq() == s.seq()) {
                "single"
            } else {
                "decision"
            };
            record(view, &format!("{}_{}", street, s.holding.class), s, s)?;
        }
        if let Some(previous) = path.last().copied() {
            let prior: &Spot = previous;
            let adjacent = matches!(
                (prior.street(), street),
                ("flop", "turn") | ("turn", "river")
            );
            let delayed =
                prior.street() == "flop" && street == "river" && checked_through(h, "turn");
            if adjacent || delayed {
                if let Some(next) =
                    first.or_else(|| street_spots.iter().copied().find(|s| s.eligible()))
                {
                    let outcome = if !next.aggressive() {
                        "stop"
                    } else if next.holding.candidate() {
                        "continue"
                    } else {
                        "made"
                    };
                    record(
                        "next",
                        &format!("{}_{}_{}", prior.street(), street, outcome),
                        prior,
                        next,
                    )?;
                }
            }
        }
        if let Some(s) = first.filter(|s| s.holding.candidate()) {
            path.push(s);
        }
    }
    if let (Some(first), Some(last)) = (path.first(), path.last()) {
        record("line", &route(h, &path), first, last)?;
        if path.len() >= 2
            && !(path[0].street() == "flop"
                && path[1].street() == "river"
                && !checked_through(h, "turn"))
        {
            record("reached", &route(h, &path[..2]), first, path[1])?;
        }
    }
    Ok(())
}

pub(crate) fn marks(c: &Connection, hand: i64) -> Result<Vec<usize>> {
    let mut st = c.prepare(
        "SELECT s.seq FROM bluff_samples s JOIN bluff_marks m USING(hand,signature)
        WHERE s.hand=? AND s.view IN ('single','decision') ORDER BY s.seq",
    )?;
    let rows = st
        .query_map([hand], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub(crate) fn mark(c: &mut Connection, hand: i64, seq: usize, marked: bool) -> Result<()> {
    let tx = c.transaction()?;
    let ready: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM study_indexed WHERE hand=? AND version=?)",
        params![hand, crate::study::decision::INDEX_VERSION],
        |r| r.get(0),
    )?;
    if !ready {
        let h = crate::store::get_hand(&tx, hand)?;
        crate::study::index_hand(&tx, hand, &h)?;
    }
    let signature: String=tx.query_row("SELECT s.signature FROM bluff_samples s JOIN hands h ON h.id=s.hand WHERE s.hand=? AND s.seq=? AND s.view IN ('single','decision') AND h.status='valid'",
        params![hand,seq], |r|r.get(0))?;
    ensure!(!signature.is_empty(), "expected a postflop bet or raise");
    if marked {
        tx.execute(
            "INSERT OR IGNORE INTO bluff_marks VALUES(?,?)",
            params![hand, signature],
        )?;
    } else {
        tx.execute(
            "DELETE FROM bluff_marks WHERE hand=? AND signature=?",
            params![hand, signature],
        )?;
    }
    tx.commit()?;
    Ok(())
}
