//! DuckDB storage, appender batches and atomic import checkpoints.
use super::facts::Decision;
use crate::model::{Hand, ObservedHand};
use anyhow::{ensure, Context, Result};
use duckdb::{appender_params_from_iter, params, params_from_iter, types::Value, Connection};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    path::Path,
};

pub const SCHEMA: u32 = 1;
pub const BATCH_SIZE: usize = 2048;
pub const DIMENSIONS: &str =
    "day,bb,position,opponent,street,pot_type,players,texture,paired,high_card,opportunities,hits";
pub const DECISION_COLUMNS: &str="hand_key,seq,seat,day,bb,position,opponent,street,pot_type,players,effective_bb,facing_bet_pct,board,texture,paired,high_card,pot_units,to_call_units,remaining_units,action,opportunities,hits,known_cards,showdown";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub id: String,
    pub name: String,
    pub network: String,
    pub pool: String,
    pub format: String,
    pub timezone: String,
    pub currency: String,
    pub parser_version: String,
    pub stats_version: String,
    pub schema: u32,
    pub cloud_enabled: bool,
    pub mcp_enabled: bool,
    pub created_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub path: String,
    pub member: Option<String>,
    pub size: u64,
    pub modified: u64,
    pub crc: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Job {
    pub id: String,
    pub sources: Vec<Source>,
    pub source_index: usize,
    pub committed_hands: u64,
    pub scanned: u64,
    pub inserted: u64,
    pub duplicates: u64,
    pub quarantined: u64,
    pub conflicts: u64,
    pub state: String,
    pub message: Option<String>,
    pub started_at: i64,
    pub elapsed_ms: u64,
}
#[derive(Default, Debug, Serialize)]
pub struct Counts {
    pub scanned: u64,
    pub inserted: u64,
    pub duplicates: u64,
    pub quarantined: u64,
    pub conflicts: u64,
}
pub struct Incoming {
    pub hand: ObservedHand,
    pub decisions: Vec<Decision>,
}
pub struct Rejected {
    pub original_id: String,
    pub code: String,
    pub raw: String,
}
pub fn key(network: &str, id: &str) -> u64 {
    let h = Sha256::digest(format!("{network}\0{id}"));
    u64::from_le_bytes(h[..8].try_into().unwrap())
}
pub fn hex_key(key: u64) -> String {
    format!("{key:016x}")
}
pub fn parse_key(value: &str) -> Result<u64> {
    ensure!(value.len() == 16, "invalid hand key");
    Ok(u64::from_str_radix(value, 16)?)
}
pub fn pack(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::fast());
    e.write_all(bytes)?;
    Ok(e.finish()?)
}
pub fn unpack(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    ZlibDecoder::new(bytes)
        .take(8_388_609)
        .read_to_end(&mut out)?;
    ensure!(out.len() <= 8_388_608, "payload too large");
    Ok(out)
}
pub fn open(path: &Path) -> Result<Connection> {
    let c = Connection::open(path)?;
    c.execute_batch("SET memory_limit='2GiB';SET threads=4;SET preserve_insertion_order=false;SET enable_external_access=false;")?;
    Ok(c)
}
pub fn init(c: &Connection, info: &DatasetInfo) -> Result<()> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS meta(key VARCHAR PRIMARY KEY,value VARCHAR NOT NULL);
    CREATE TABLE IF NOT EXISTS jobs(id VARCHAR PRIMARY KEY,body VARCHAR NOT NULL);
    CREATE TABLE IF NOT EXISTS hands(hand_key UBIGINT PRIMARY KEY,network VARCHAR NOT NULL,original_id VARCHAR NOT NULL,fingerprint VARCHAR NOT NULL,played_at BIGINT,day INTEGER,bb BIGINT,payload BLOB);
    CREATE TABLE IF NOT EXISTS quarantine(id BIGINT PRIMARY KEY,original_id VARCHAR,code VARCHAR,raw BLOB,source VARCHAR,created_at BIGINT);
    CREATE SEQUENCE IF NOT EXISTS issue_ids START 1;
    CREATE TABLE IF NOT EXISTS decisions(hand_key UBIGINT,seq UINTEGER,seat UTINYINT,day INTEGER,bb BIGINT,position VARCHAR,opponent VARCHAR,street VARCHAR,pot_type VARCHAR,players UTINYINT,effective_bb DOUBLE,facing_bet_pct DOUBLE,board VARCHAR,texture VARCHAR,paired BOOLEAN,high_card VARCHAR,pot_units BIGINT,to_call_units BIGINT,remaining_units BIGINT,action VARCHAR,opportunities UBIGINT,hits UBIGINT,known_cards BOOLEAN,showdown BOOLEAN);
    CREATE TABLE IF NOT EXISTS rollups(day INTEGER,bb BIGINT,position VARCHAR,opponent VARCHAR,street VARCHAR,pot_type VARCHAR,players UTINYINT,texture VARCHAR,paired BOOLEAN,high_card VARCHAR,opportunities UBIGINT,hits UBIGINT,n UBIGINT,known UBIGINT,showdowns UBIGINT,shown UBIGINT);
    INSERT INTO meta VALUES('revision','0') ON CONFLICT DO NOTHING;
    INSERT INTO meta VALUES('epoch',uuid()::VARCHAR) ON CONFLICT DO NOTHING;")?;
    c.execute(
        "INSERT INTO meta VALUES('dataset',?) ON CONFLICT DO NOTHING",
        [serde_json::to_string(info)?],
    )?;
    Ok(())
}
pub fn info(c: &Connection) -> Result<DatasetInfo> {
    Ok(serde_json::from_str(&c.query_row(
        "SELECT value FROM meta WHERE key='dataset'",
        [],
        |r| r.get::<_, String>(0),
    )?)?)
}
pub fn save_info(c: &Connection, info: &DatasetInfo) -> Result<()> {
    c.execute(
        "UPDATE meta SET value=? WHERE key='dataset'",
        [serde_json::to_string(info)?],
    )?;
    Ok(())
}
pub fn version(c: &Connection) -> Result<String> {
    let id = info(c)?.id;
    let suffix:String=c.query_row("SELECT (SELECT value FROM meta WHERE key='epoch')||':'||(SELECT value FROM meta WHERE key='revision')",[],|r|r.get(0))?;
    Ok(format!("mda:{id}:{suffix}"))
}
pub fn jobs(c: &Connection) -> Result<Vec<Job>> {
    Ok(
        c.prepare("SELECT body FROM jobs ORDER BY id DESC LIMIT 100")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<duckdb::Result<Vec<_>>>()?
            .into_iter()
            .map(|s| serde_json::from_str(&s))
            .collect::<serde_json::Result<Vec<_>>>()?,
    )
}
pub fn save_job(c: &Connection, job: &Job) -> Result<()> {
    c.execute(
        "INSERT INTO jobs VALUES(?,?) ON CONFLICT(id) DO UPDATE SET body=excluded.body",
        params![job.id, serde_json::to_string(job)?],
    )?;
    Ok(())
}
fn rejection(c: &Connection, r: &Rejected, source: &str) -> Result<()> {
    c.execute(
        "INSERT INTO quarantine VALUES(nextval('issue_ids'),?,?,?, ?,?)",
        params![
            r.original_id,
            r.code,
            pack(r.raw.as_bytes())?,
            source,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}
/// Appenders must flush successfully before commit. Any failure rolls back data AND checkpoint.
pub fn commit_batch(
    c: &mut Connection,
    incoming: Vec<Incoming>,
    mut rejected: Vec<Rejected>,
    job: &mut Job,
    next_hands: u64,
    source: &str,
) -> Result<Counts> {
    let tx = c.transaction()?;
    let network = info(&tx)?.network;
    let mut counts = Counts {
        scanned: (incoming.len() + rejected.len()) as u64,
        ..Counts::default()
    };
    let keys: Vec<Value> = incoming
        .iter()
        .map(|r| Value::UBigInt(key(&network, &r.hand.id)))
        .collect();
    let mut existing = HashMap::<u64, (String, String, String)>::new();
    if !keys.is_empty() {
        let sql = format!(
            "SELECT hand_key,network,original_id,fingerprint FROM hands WHERE hand_key IN ({})",
            vec!["?"; keys.len()].join(",")
        );
        let mut stmt = tx.prepare(&sql)?;
        for row in stmt.query_map(params_from_iter(keys), |r| {
            Ok((
                r.get::<_, u64>(0)?,
                (
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ),
            ))
        })? {
            let (k, v) = row?;
            existing.insert(k, v);
        }
    }
    let mut accepted = Vec::new();
    for row in incoming {
        let k = key(&network, &row.hand.id);
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(row.hand.raw.replace("\r\n", "\n").trim().as_bytes())
        );
        if let Some((old_network, old_id, old_fp)) = existing.get(&k) {
            if old_network == &network && old_id == &row.hand.id && old_fp == &fingerprint {
                counts.duplicates += 1;
            } else {
                counts.conflicts += 1;
                rejected.push(Rejected {
                    original_id: row.hand.id.clone(),
                    code: if old_network != &network || old_id != &row.hand.id {
                        "identity_hash_collision"
                    } else {
                        "content_conflict"
                    }
                    .into(),
                    raw: row.hand.raw.clone(),
                });
            }
        } else {
            existing.insert(
                k,
                (network.clone(), row.hand.id.clone(), fingerprint.clone()),
            );
            accepted.push((k, fingerprint, row));
        }
    }
    tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS batch_decisions AS SELECT * FROM decisions LIMIT 0;DELETE FROM batch_decisions;")?;
    {
        let mut hands = tx.appender("hands")?;
        let mut decisions = tx.appender_to_db("batch_decisions", "main")?;
        for (k, fp, row) in &accepted {
            hands.append_row(params![
                k,
                network,
                row.hand.id,
                fp,
                row.hand.played_at,
                row.decisions.first().map(|d| d.day).unwrap_or(0),
                row.hand.bb,
                pack(&serde_json::to_vec(&row.hand)?)?
            ])?;
            for d in &row.decisions {
                let values = vec![
                    Value::UBigInt(*k),
                    Value::UInt(d.seq),
                    Value::UTinyInt(d.seat),
                    Value::Int(d.day),
                    Value::BigInt(d.bb),
                    Value::Text(d.position.clone()),
                    Value::Text(d.opponent.clone()),
                    Value::Text(d.street.clone()),
                    Value::Text(d.pot_type.clone()),
                    Value::UTinyInt(d.players),
                    Value::Double(d.effective_bb),
                    d.facing_bet_pct.map(Value::Double).unwrap_or(Value::Null),
                    Value::Text(d.board.clone()),
                    Value::Text(d.texture.clone()),
                    Value::Boolean(d.paired),
                    Value::Text(d.high_card.clone()),
                    Value::BigInt(d.pot_units),
                    Value::BigInt(d.to_call_units),
                    Value::BigInt(d.remaining_units),
                    Value::Text(d.action.clone()),
                    Value::UBigInt(d.opportunities),
                    Value::UBigInt(d.hits),
                    Value::Boolean(d.known_cards),
                    Value::Boolean(d.showdown),
                ];
                decisions.append_row(appender_params_from_iter(values))?;
            }
        }
        hands.flush()?;
        decisions.flush()?;
    }
    tx.execute_batch(&format!("INSERT INTO decisions SELECT * FROM batch_decisions;
        INSERT INTO rollups SELECT {DIMENSIONS},count(*),sum(known_cards::INTEGER),sum(showdown::INTEGER),sum((showdown AND known_cards)::INTEGER) FROM batch_decisions WHERE opportunities>0 GROUP BY {DIMENSIONS};"))?;
    for r in &rejected {
        rejection(&tx, r, source)?;
    }
    counts.inserted = accepted.len() as u64;
    counts.quarantined = rejected.len() as u64;
    let mut next = job.clone();
    next.committed_hands = next_hands;
    next.scanned += counts.scanned;
    next.inserted += counts.inserted;
    next.duplicates += counts.duplicates;
    next.quarantined += counts.quarantined;
    next.conflicts += counts.conflicts;
    save_job(&tx, &next)?;
    if counts.inserted + counts.quarantined > 0 {
        tx.execute_batch(
            "UPDATE meta SET value=(value::UBIGINT+1)::VARCHAR WHERE key='revision';",
        )?;
    }
    tx.commit()?;
    *job = next;
    Ok(counts)
}
pub fn compact_rollups(c: &mut Connection) -> Result<()> {
    let tx = c.transaction()?;
    tx.execute_batch(&format!("CREATE TEMP TABLE compacted AS SELECT {DIMENSIONS},sum(n)::UBIGINT AS n,sum(known)::UBIGINT AS known,sum(showdowns)::UBIGINT AS showdowns,sum(shown)::UBIGINT AS shown FROM rollups GROUP BY {DIMENSIONS};DELETE FROM rollups;INSERT INTO rollups SELECT * FROM compacted;DROP TABLE compacted;"))?;
    tx.commit()?;
    Ok(())
}
pub fn hand(c: &Connection, key: u64) -> Result<ObservedHand> {
    let bytes: Vec<u8> = c
        .query_row("SELECT payload FROM hands WHERE hand_key=?", [key], |r| {
            r.get(0)
        })
        .context("MDA hand not found")?;
    Ok(serde_json::from_slice(&unpack(&bytes)?)?)
}
pub fn anonymize(observed: ObservedHand, key: u64) -> Hand {
    let mut h = observed.replay();
    h.id = format!("mda-{}", hex_key(key));
    h.table_name = "匿名一般桌".into();
    h.profile = "mda".into();
    h.raw.clear();
    for p in &mut h.players {
        p.name = format!("Seat {}", p.seat);
        p.hero = false;
    }
    // Full retrospective replay may reveal shown cards; source identifiers never leave this scope.
    h.issues.clear();
    h
}
pub fn validate(c: &Connection) -> Result<()> {
    let i = info(c)?;
    ensure!(
        i.schema == SCHEMA
            && i.currency == "USD"
            && i.parser_version == crate::model::PARSER_VERSION
            && i.stats_version == super::facts::VERSION,
        "incompatible MDA backup / parser version"
    );
    let tables: HashSet<String> = c
        .prepare("SELECT table_name FROM information_schema.tables WHERE table_schema='main'")?
        .query_map([], |r| r.get(0))?
        .collect::<duckdb::Result<_>>()?;
    ensure!(
        ["hands", "decisions", "rollups", "quarantine", "jobs"]
            .iter()
            .all(|t| tables.contains(*t)),
        "incomplete MDA backup"
    );
    Ok(())
}
