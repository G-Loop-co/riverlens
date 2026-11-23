//! Six-handed cash review. Cohorts retain currency, stake and game throughout.
use crate::{model::Filter, money, store, study::decision};
use anyhow::{ensure, Result};
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::{json, Value};

const METRICS: &[&str] = &[
    "vpip",
    "pfr",
    "three_bet",
    "fold_three_bet",
    "cbet",
    "fold_cbet",
];

pub fn dashboard(c: &Connection, filter: &Filter) -> Result<Value> {
    let (pred, args) = store::predicate(filter, true)?;
    let sums = METRICS
        .iter()
        .map(|id| {
            let denom = store::benchmark_denominator_stat(id);
            format!("SUM(h.{id}_n), SUM(h.{denom}_d)")
        })
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT h.currency,h.game,h.stakes,h.position,COUNT(*),{sums}
        FROM hands h WHERE {pred} AND h.status='valid' AND h.player_count=6
        AND h.game IN ('Cash','Rush') GROUP BY h.currency,h.game,h.stakes,h.position
        ORDER BY h.currency,h.game,h.stakes,h.position"
    );
    let mut stmt = c.prepare(&sql)?;
    let positions = stmt
        .query_map(params_from_iter(&args), |row| {
            let mut metrics = serde_json::Map::new();
            for (i, id) in METRICS.iter().enumerate() {
                metrics.insert(
                    id.to_string(),
                    json!([row.get::<_, i64>(5 + 2 * i)?, row.get::<_, i64>(6 + 2 * i)?]),
                );
            }
            Ok(
                json!({"currency":row.get::<_,String>(0)?,"game":row.get::<_,String>(1)?,
          "stakes":row.get::<_,String>(2)?,"position":row.get::<_,String>(3)?,
          "hands":row.get::<_,i64>(4)?,"metrics":metrics}),
            )
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    // Session identifiers are file-based. Order by actual play time, not filename.
    let latest = c.query_row(&format!("WITH selected AS (
          SELECT h.session,h.currency,h.played_at,h.id,h.net,h.net_bb FROM hands h WHERE {pred} AND h.status='valid'
        ), last AS (SELECT session,currency FROM selected ORDER BY played_at DESC,id DESC LIMIT 1)
        SELECT s.session,s.currency,MAX(s.played_at),COUNT(*),SUM(s.net),SUM(s.net_bb)
        FROM selected s JOIN last l USING(session,currency) GROUP BY s.session,s.currency"),params_from_iter(&args),|r|
        Ok(json!({"session":r.get::<_,String>(0)?,"currency":r.get::<_,String>(1)?,
        "time":r.get::<_,i64>(2)?,"hands":r.get::<_,i64>(3)?,"net":money::format(r.get(4)?),"net_bb":r.get::<_,f64>(5)?}))).optional()?;
    let default_currency: Option<String> = c.query_row("SELECT currency FROM hands WHERE status='valid' ORDER BY played_at DESC,id DESC LIMIT 1",[],|r|r.get(0)).optional()?;
    let has_hands: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM hands)", [], |r| r.get(0))?;
    Ok(
        json!({"positions":positions,"latest":latest,"has_hands":has_hands,"default_currency":default_currency}),
    )
}

// Exact six-handed action path: UTG/HJ/CO fold, BTN opens, SB folds.
// Aggregate all matching decisions; retain a bounded example list for replay.
pub fn blind_spot(c: &Connection, filter: &Filter, open_bb: &str) -> Result<Value> {
    ensure!(
        matches!(filter.game.as_deref(), Some("Cash" | "Rush"))
            && filter.currency.as_deref().is_some_and(|v| !v.is_empty())
            && filter.stakes.as_deref().is_some_and(|v| !v.is_empty()),
        "Select a cash game, currency and stake for the positional comparison"
    );
    ensure!(
        matches!(open_bb, "2" | "2.5" | "3"),
        "Unsupported open size"
    );
    let (pred, mut args) = store::predicate(filter, true)?;
    args.push(decision::INDEX_VERSION.to_owned().into());
    args.push(format!("F-F-F-R{open_bb}-F").into());
    let from = format!(
        "FROM study_decisions d JOIN hands h ON h.id=d.hand JOIN study_indexed i ON i.hand=h.id
        WHERE {pred} AND h.status='valid' AND h.player_count=6 AND h.game IN ('Cash','Rush')
        AND i.version=? AND d.preflop_path=? AND d.street='preflop' AND d.position='BB'
        AND d.opponent='BTN' AND d.pot_type='SRP' AND d.effective_bb=100"
    );
    let (n,raises,calls):(u64,u64,u64)=c.query_row(&format!("SELECT COUNT(*),COALESCE(SUM(d.action='raise'),0),COALESCE(SUM(d.action='call'),0) {from}"),params_from_iter(&args),|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
    let mut stmt=c.prepare(&format!("SELECT h.id,d.seq,h.hand_class,d.action,h.hand_id {from} ORDER BY h.played_at DESC,h.id DESC LIMIT 50"))?;
    let rows=stmt.query_map(params_from_iter(&args),|r|Ok(json!({"id":r.get::<_,i64>(0)?,"seq":r.get::<_,u64>(1)?,"cards":r.get::<_,String>(2)?,"action":r.get::<_,String>(3)?,"hand_id":r.get::<_,String>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(json!({"n":n,"raises":raises,"calls":calls,"rows":rows}))
}
