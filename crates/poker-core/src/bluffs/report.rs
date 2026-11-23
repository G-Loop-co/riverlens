use crate::{model::Filter, money, store, study};
use anyhow::{bail, Result};
use rusqlite::{params_from_iter, types::Value as Sql, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Query {
    pub filter: Filter,
    pub view: String,
    pub breakdown: String,
    pub brand: Option<String>,
    pub players: Option<String>,
    pub action: Option<String>,
    pub sizing: Option<String>,
    pub draw: Option<String>,
    pub opponent: Option<String>,
    pub depth: Option<String>,
    pub texture: Option<String>,
    pub cell: Option<Cell>,
    pub before: Option<(i64, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub category: String,
    pub bucket: String,
}

const SIZE: &str = "CASE WHEN s.size_pct IS NULL THEN 'none' WHEN s.size_pct<=100.0/3 THEN 'small' WHEN s.size_pct<=200.0/3 THEN 'medium' WHEN s.size_pct<=100 THEN 'large' ELSE 'over' END";
const DEPTH: &str = "CASE WHEN s.depth<40 THEN 'short' WHEN s.depth<100 THEN 'mid' ELSE 'deep' END";
const PLAYERS: &str = "CASE WHEN s.players=2 THEN 'hu' ELSE CAST(s.players AS TEXT) END";

fn bucket(name: &str) -> Result<&'static str> {
    Ok(match name {
        "" | "none" => "''",
        "size" => SIZE,
        "action" => "s.action",
        "response" => "s.response",
        "position" => "h.position",
        "opponent" => "COALESCE(s.opponent,'multi')",
        "players" => PLAYERS,
        "pot" => "h.pot_type",
        "depth" => DEPTH,
        "texture" => "s.texture",
        _ => bail!("unknown bluff breakdown"),
    })
}

fn view(name: &str) -> Result<&'static str> {
    let predicate = match name {
        "" | "single" => "s.view='single' AND s.holding IN ('air','draw')",
        "pair_draw" => "s.view='single' AND s.holding='pair_draw'",
        "manual" => {
            "s.view IN ('single','decision') AND EXISTS(
            SELECT 1 FROM bluff_marks m WHERE m.hand=s.hand AND m.signature=s.signature)"
        }
        "line" => "s.view='line'",
        "reached" => "s.view='reached'",
        "next" => "s.view='next'",
        _ => bail!("unknown bluff view"),
    };
    Ok(predicate)
}

fn read_hand(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let bb = row.get::<_, i64>("bb")? as f64;
    let segment: Option<i64> = row.get("cashflow")?;
    let net: i64 = row.get("net")?;
    Ok(json!({
        "id": row.get::<_, i64>("hand")?,
        "hand_id": row.get::<_, String>("hand_id")?,
        "seq": row.get::<_, usize>("seq")?,
        "start": row.get::<_, usize>("start_seq")?,
        "date": row.get::<_, String>("local_date")?,
        "street": row.get::<_, String>("street")?,
        "holding": row.get::<_, String>("holding")?,
        "outs": row.get::<_, u8>("outs")?,
        "action": row.get::<_, String>("action")?,
        "response": row.get::<_, String>("response")?,
        "segment_bb": segment.map(|n| n as f64 / bb),
        "net_bb": net as f64 / bb,
        "segment": segment.map(money::format),
        "net": money::format(net),
        "currency": row.get::<_, String>("currency")?,
        "exclusion": row.get::<_, Option<String>>("exclusion")?,
        "size": row.get::<_, Option<f64>>("size_pct")?,
        "players": row.get::<_, usize>("players")?,
        "gutshot": row.get::<_, bool>("gutshot")?,
        "missed": row.get::<_, bool>("missed")?
    }))
}

fn read_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "category": row.get::<_, String>("category")?,
        "bucket": row.get::<_, String>("bucket")?,
        "attempts": row.get::<_, i64>("attempts")?,
        "hands": row.get::<_, i64>("hands")?,
        "folded": row.get::<_, i64>("folded")?,
        "settled": row.get::<_, i64>("settled")?,
        "segment_bb": row.get::<_, Option<f64>>("segment_bb")?,
        "net_bb": row.get::<_, f64>("net_bb")?,
        "segment": row.get::<_, Option<i64>>("segment")?.map(money::format),
        "net": row.get::<_, Option<i64>>("whole")?.map(money::format),
        "currency": row.get::<_, Option<String>>("currency")?
    }))
}

fn add(pred: &mut String, values: &mut Vec<Sql>, col: &str, value: &Option<String>) {
    if let Some(v) = value.as_deref().filter(|v| !v.is_empty()) {
        pred.push_str(&format!(" AND {col}=?"));
        values.push(v.to_owned().into());
    }
}

pub fn query(c: &mut Connection, q: &Query) -> Result<Value> {
    let tx = c.transaction()?;
    let (mut pred, mut values) = store::predicate(&q.filter, false)?;
    pred.push_str(" AND h.game IN ('Cash','Rush')");
    for (col, value) in [
        ("s.brand", &q.brand),
        ("s.action", &q.action),
        (SIZE, &q.sizing),
        ("s.opponent", &q.opponent),
        (DEPTH, &q.depth),
        ("s.texture", &q.texture),
    ] {
        add(&mut pred, &mut values, col, value);
    }
    match q.players.as_deref().unwrap_or("") {
        "" => {}
        "multi" => pred.push_str(" AND s.players>=3"),
        "hu" => pred.push_str(" AND s.players=2"),
        n => {
            let seats: u8 = n.parse()?;
            anyhow::ensure!(
                (2..=10).contains(&seats),
                "street player count must be between 2 and 10"
            );
            pred.push_str(" AND s.players=?");
            values.push(i64::from(seats).into());
        }
    }
    match q.draw.as_deref().unwrap_or("") {
        "" => {}
        "missed" => pred.push_str(" AND s.missed=1"),
        "gutshot" => pred.push_str(" AND s.gutshot=1"),
        "other_air" => pred.push_str(" AND s.street='river' AND s.holding='air' AND s.missed=0"),
        _ => bail!("unknown draw filter"),
    }
    let from =
        " FROM bluff_samples s JOIN hands h ON h.id=s.hand JOIN study_indexed i ON i.hand=h.id ";
    pred.push_str(" AND i.version=?");
    values.push(study::decision::INDEX_VERSION.to_owned().into());
    let selection = format!(
        "{from} WHERE {pred} AND h.status='valid' AND ({})",
        view(&q.view)?
    );
    let group = bucket(&q.breakdown)?;
    if let Some(cell) = &q.cell {
        let mut detail = format!("{selection} AND s.category=? AND ({group})=?");
        values.extend([cell.category.clone().into(), cell.bucket.clone().into()]);
        if let Some((id, seq)) = q.before {
            detail.push_str(" AND (s.hand,s.seq)<(?,?)");
            values.extend([id.into(), (seq as i64).into()]);
        }
        let mut st=tx.prepare(&format!("SELECT s.hand,h.hand_id,s.seq,s.start_seq,h.local_date,s.street,s.holding,s.outs,
            s.action,s.response,s.cashflow,h.net,h.currency,s.bb,s.exclusion,s.size_pct,s.players,s.gutshot,s.missed
            {detail} ORDER BY s.hand DESC,s.seq DESC LIMIT 51"))?;
        let mut rows = st
            .query_map(params_from_iter(values.iter()), read_hand)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let next = if rows.len() > 50 {
            rows.pop();
            rows.last().map(|v| json!([v["id"], v["seq"]]))
        } else {
            None
        };
        if matches!(q.view.as_str(), "" | "single" | "pair_draw") {
            let mut later = tx.prepare(
                "SELECT seq,action,size_pct FROM bluff_samples
                WHERE hand=? AND view='decision' AND street=? AND seq>? ORDER BY seq",
            )?;
            for row in &mut rows {
                let followups = later
                    .query_map(
                        rusqlite::params![
                            row["id"].as_i64(),
                            row["street"].as_str(),
                            row["seq"].as_i64()
                        ],
                        |r| {
                            Ok(json!({
                                "seq": r.get::<_, usize>(0)?,
                                "action": r.get::<_, String>(1)?,
                                "size": r.get::<_, Option<f64>>(2)?
                            }))
                        },
                    )?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                row["followups"] = json!(followups);
            }
        }
        return Ok(json!({"rows":rows,"next":next}));
    }
    let source=format!("SELECT s.category,({group}) bucket,s.hand,s.seq,s.response,s.bb,s.cashflow,
        h.net,h.currency,ROW_NUMBER() OVER(PARTITION BY s.category,({group}),s.hand ORDER BY s.seq) ordinal {selection}");
    let mut stmt = tx.prepare(&format!("WITH samples AS ({source})
        SELECT category, bucket, COUNT(*) AS attempts, COUNT(DISTINCT hand) AS hands,
            SUM(response='folded') AS folded, COUNT(cashflow) AS settled,
            SUM(cashflow*1.0/bb) AS segment_bb,
            SUM(CASE WHEN ordinal=1 THEN net*1.0/bb END) AS net_bb,
            CASE WHEN MIN(currency)=MAX(currency) THEN SUM(cashflow) END AS segment,
            CASE WHEN MIN(currency)=MAX(currency) THEN SUM(CASE WHEN ordinal=1 THEN net END) END AS whole,
            CASE WHEN MIN(currency)=MAX(currency) THEN MIN(currency) END AS currency
        FROM samples GROUP BY category,bucket ORDER BY category,bucket"))?;
    let groups = stmt
        .query_map(params_from_iter(values.iter()), read_group)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut exclusions = serde_json::Map::new();
    let mut st = tx.prepare(&format!(
        "SELECT s.exclusion,COUNT(*) {selection} AND s.exclusion IS NOT NULL GROUP BY s.exclusion"
    ))?;
    for row in st.query_map(params_from_iter(values.iter()), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (reason, n) = row?;
        exclusions.insert(reason, json!(n));
    }
    let unknown: i64 = tx.query_row(
        &format!("SELECT COUNT(*) {from} WHERE {pred} AND s.view='single' AND s.holding='unknown'"),
        params_from_iter(values.iter()),
        |r| r.get(0),
    )?;
    let coverage = study::coverage(&tx)?;
    let choices = |sql: &str| -> Result<Vec<String>> {
        let mut s = tx.prepare(sql)?;
        let result = s
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(result)
    };
    let stakes=choices("SELECT DISTINCT value FROM rollups WHERE dimension='stakes' AND currency!='CHIPS' ORDER BY CAST(value AS REAL),value")?;
    let currencies=choices("SELECT DISTINCT currency FROM rollups WHERE dimension='stakes' AND currency!='CHIPS' ORDER BY currency")?;
    Ok(
        json!({"groups":groups,"unknown":unknown,"exclusions":exclusions,"coverage":coverage,"stakes":stakes,"currencies":currencies}),
    )
}
