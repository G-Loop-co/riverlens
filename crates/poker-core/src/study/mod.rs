pub mod decision;
pub mod strategy;
pub mod training;

use crate::{
    model::{Filter, Hand},
    store,
};
use anyhow::{bail, Context, Result};
use rusqlite::{params, params_from_iter, types::Value as Sql, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct SpotDefinition {
    pub street: Option<String>,
    pub position: Option<String>,
    pub opponent: Option<String>,
    pub role: Option<String>,
    pub facing: Option<String>,
    pub pot_type: Option<String>,
    pub effective_min: Option<f64>,
    pub effective_max: Option<f64>,
    pub texture: Option<String>,
    pub paired: Option<bool>,
    pub high_card: Option<String>,
    pub facing_min: Option<f64>,
    pub facing_max: Option<f64>,
    pub line: Vec<decision::LineAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct StudyQuery {
    pub filter: Filter,
    pub spot: SpotDefinition,
    pub before: Option<i64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub name: String,
    pub spot: SpotDefinition,
    pub action: String,
    pub low: f64,
    pub high: f64,
    pub min_samples: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum StudyRequest {
    Explore {
        query: Box<StudyQuery>,
    },
    Items {
        kind: String,
    },
    SaveItem {
        kind: String,
        id: Option<String>,
        data: Value,
    },
    DeleteItem {
        kind: String,
        id: String,
    },
    Packs,
    ImportPack {
        path: String,
    },
    Nodes {
        pack: String,
    },
    Matrix {
        pack: String,
        node: String,
        filter: Filter,
    },
    Compare {
        pack: String,
        reference: decision::DecisionRef,
        node: Option<String>,
    },
    Leaks {
        filter: Filter,
        pack: Option<String>,
    },
    Enqueue {
        references: Vec<decision::DecisionRef>,
        pack: Option<String>,
        node: Option<String>,
        hand_class: Option<String>,
    },
    TrainingState,
    TrainingStart {
        limit: u32,
    },
    TrainingCurrent {
        session: String,
    },
    TrainingAnswer {
        session: String,
        card: String,
        answer: training::TrainingAnswer,
    },
    TrainingRate {
        session: String,
        card: String,
        rating: String,
    },
    Resolve {
        reference: decision::DecisionRef,
    },
}

impl StudyRequest {
    pub fn writes(&self) -> bool {
        matches!(
            self,
            Self::SaveItem { .. }
                | Self::DeleteItem { .. }
                | Self::ImportPack { .. }
                | Self::Enqueue { .. }
                | Self::TrainingStart { .. }
                | Self::TrainingAnswer { .. }
                | Self::TrainingRate { .. }
        )
    }
    pub fn run(self, c: &mut Connection) -> Result<Value> {
        match self {
            Self::SaveItem { kind, id, data } => save_item(c, &kind, id.as_deref(), data),
            Self::DeleteItem { kind, id } => {
                anyhow::ensure!(
                    matches!(kind.as_str(), "spot" | "benchmark"),
                    "Invalid study item kind"
                );
                c.execute(
                    "DELETE FROM study_items WHERE kind=?1 AND id=?2",
                    params![kind, id],
                )?;
                Ok(json!({"deleted":true}))
            }
            Self::ImportPack { path } => strategy::import(c, &path),
            Self::Enqueue {
                references,
                pack,
                node,
                hand_class,
            } => training::enqueue(
                c,
                &references,
                pack.as_deref(),
                node.as_deref(),
                hand_class.as_deref(),
            ),
            Self::TrainingStart { limit } => training::start(c, limit),
            Self::TrainingAnswer {
                session,
                card,
                answer,
            } => training::answer(c, &session, &card, &answer),
            Self::TrainingRate {
                session,
                card,
                rating,
            } => training::rate(c, &session, &card, &rating),
            other => {
                let tx = c.transaction()?;
                let value = match other {
                    Self::Explore { query } => explore(&tx, &query),
                    Self::Items { kind } => items(&tx, &kind),
                    Self::Packs => strategy::list(&tx),
                    Self::Nodes { pack } => strategy::nodes(&tx, &pack),
                    Self::Matrix { pack, node, filter } => {
                        strategy::matrix(&tx, &pack, &node, &filter)
                    }
                    Self::Compare {
                        pack,
                        reference,
                        node,
                    } => strategy::compare_decision(&tx, &pack, &reference, node.as_deref()),
                    Self::Leaks { filter, pack } => leaks(&tx, &filter, pack.as_deref()),
                    Self::TrainingState => training::state(&tx),
                    Self::TrainingCurrent { session } => training::current(&tx, &session),
                    Self::Resolve { reference } => {
                        let (id, d) = resolve(&tx, &reference)?;
                        Ok(json!({"hand":id,"seq":d.reference.seq}))
                    }
                    _ => unreachable!(),
                }?;
                tx.commit()?;
                Ok(value)
            }
        }
    }
}

pub fn init(c: &Connection) -> Result<()> {
    c.execute_batch("BEGIN IMMEDIATE;
      CREATE TABLE IF NOT EXISTS study_indexed(hand INTEGER PRIMARY KEY REFERENCES hands(id) ON DELETE CASCADE, version TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS study_decisions(
        id INTEGER PRIMARY KEY, hand INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE, seq INTEGER NOT NULL,
        street TEXT NOT NULL, position TEXT NOT NULL, opponent TEXT, role TEXT, facing TEXT NOT NULL,
        pot_type TEXT NOT NULL, effective_bb REAL, facing_pct REAL, action TEXT NOT NULL,
        line_flop TEXT NOT NULL, line_turn TEXT NOT NULL, line_river TEXT NOT NULL,
        preflop_path TEXT NOT NULL, data TEXT NOT NULL, strategy_data TEXT NOT NULL DEFAULT '', UNIQUE(hand,seq));
      CREATE INDEX IF NOT EXISTS study_spot ON study_decisions(street,position,facing,id);
      CREATE INDEX IF NOT EXISTS study_path ON study_decisions(street,line_flop,id);
      CREATE INDEX IF NOT EXISTS study_turn ON study_decisions(street,line_turn,id);
      CREATE INDEX IF NOT EXISTS study_preflop ON study_decisions(street,preflop_path,position,id);
      CREATE TABLE IF NOT EXISTS study_items(kind TEXT NOT NULL,id TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));
      CREATE TABLE IF NOT EXISTS study_packs(id TEXT PRIMARY KEY, data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS study_cards(id TEXT PRIMARY KEY,data TEXT NOT NULL,due INTEGER NOT NULL,stage INTEGER NOT NULL DEFAULT 0);
      CREATE INDEX IF NOT EXISTS study_due ON study_cards(due,id);
      CREATE TABLE IF NOT EXISTS study_sessions(id TEXT PRIMARY KEY,data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS study_attempts(id TEXT PRIMARY KEY,card TEXT NOT NULL,session TEXT NOT NULL,created INTEGER NOT NULL,data TEXT NOT NULL);
      UPDATE metadata SET value='3' WHERE key='schema_version';
      COMMIT;")?;
    let has_facts:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('study_decisions') WHERE name='strategy_data')",[],|r|r.get(0))?;
    if !has_facts {
        c.execute_batch("BEGIN IMMEDIATE; ALTER TABLE study_decisions ADD COLUMN strategy_data TEXT NOT NULL DEFAULT ''; DELETE FROM study_indexed; COMMIT;")?;
    }
    c.execute_batch("CREATE INDEX IF NOT EXISTS study_hand_actions ON study_decisions(hand,action); CREATE INDEX IF NOT EXISTS study_strategy_group ON study_decisions(street,position,preflop_path,strategy_data,hand);")?;
    Ok(())
}

pub fn index_hand(c: &Connection, id: i64, h: &Hand) -> Result<()> {
    c.execute("DELETE FROM study_decisions WHERE hand=?1", [id])?;
    let mut s = c.prepare_cached("INSERT INTO study_decisions(hand,seq,street,position,opponent,role,facing,pot_type,effective_bb,facing_pct,action,line_flop,line_turn,line_river,preflop_path,data,strategy_data) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)")?;
    for d in decision::derive(h) {
        s.execute(params![
            id,
            d.reference.seq,
            d.street,
            d.position,
            d.opponent,
            d.role,
            d.facing,
            d.pot_type,
            d.effective_bb,
            d.facing_pct,
            d.action,
            decision::line_code(&d.line, "flop"),
            decision::line_code(&d.line, "turn"),
            decision::line_code(&d.line, "river"),
            d.preflop_path,
            serde_json::to_string(&d)?,
            strategy::aggregate_facts(&d)?
        ])?;
    }
    c.execute(
        "INSERT OR REPLACE INTO study_indexed VALUES(?1,?2)",
        params![id, decision::VERSION],
    )?;
    Ok(())
}

/// Short atomic batches. The caller holds the existing Service writer mutex.
pub fn backfill(c: &mut Connection, limit: u32) -> Result<usize> {
    let tx = c.transaction()?;
    let ids = {
        let mut s = tx.prepare("SELECT h.id FROM hands h LEFT JOIN study_indexed i ON i.hand=h.id WHERE i.version IS NULL OR i.version!=?1 ORDER BY h.id LIMIT ?2")?;
        let rows = s
            .query_map(params![decision::VERSION, limit], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows
    };
    for id in &ids {
        index_hand(&tx, *id, &store::get_hand(&tx, *id)?)?;
    }
    tx.commit()?;
    Ok(ids.len())
}

pub fn coverage(c: &Connection) -> Result<Value> {
    let (total, indexed):(u64,u64) = c.query_row("SELECT COUNT(*),COALESCE(SUM(CASE WHEN i.version=?1 THEN 1 ELSE 0 END),0) FROM hands h LEFT JOIN study_indexed i ON i.hand=h.id", [decision::VERSION], |r|Ok((r.get(0)?,r.get(1)?)))?;
    Ok(
        json!({"total":total,"indexed":indexed,"complete":total==indexed,"version":decision::VERSION}),
    )
}

pub fn validate_spot(s: &SpotDefinition) -> Result<()> {
    let check = |v: &Option<String>, allowed: &[&str]| -> Result<()> {
        if let Some(v) = v {
            anyhow::ensure!(allowed.contains(&v.as_str()), "Invalid spot value: {v}");
        }
        Ok(())
    };
    check(&s.street, &["preflop", "flop", "turn", "river"])?;
    check(
        &s.position,
        &["UTG", "HJ", "CO", "BTN", "SB", "BB", "BTN/SB"],
    )?;
    check(
        &s.opponent,
        &["UTG", "HJ", "CO", "BTN", "SB", "BB", "BTN/SB"],
    )?;
    check(&s.role, &["IP", "OOP"])?;
    check(
        &s.facing,
        &[
            "unopened",
            "limp",
            "open",
            "three_bet",
            "four_bet_plus",
            "checked_to",
            "bet",
            "raise",
        ],
    )?;
    check(
        &s.pot_type,
        &["limped", "SRP", "3-bet", "4-bet+", "special"],
    )?;
    check(&s.texture, &["rainbow", "two-tone", "monotone"])?;
    check(
        &s.high_card,
        &[
            "A", "K", "Q", "J", "T", "9", "8", "7", "6", "5", "4", "3", "2",
        ],
    )?;
    for (lo, hi) in [
        (s.effective_min, s.effective_max),
        (s.facing_min, s.facing_max),
    ] {
        for v in [lo, hi].into_iter().flatten() {
            anyhow::ensure!(
                v.is_finite() && (0.0..=100_000.).contains(&v),
                "Invalid spot bound"
            );
        }
        if let (Some(l), Some(h)) = (lo, hi) {
            anyhow::ensure!(l <= h, "Minimum exceeds maximum");
        }
    }
    anyhow::ensure!(s.line.len() <= 24, "Action path is too long");
    let order = |v: &str| match v {
        "flop" => 1,
        "turn" => 2,
        "river" => 3,
        _ => 0,
    };
    let mut last = 0;
    for a in &s.line {
        let street = order(&a.street);
        anyhow::ensure!(
            street > 0
                && street >= last
                && matches!(a.actor.as_str(), "hero" | "villain")
                && decision::ACTIONS.contains(&a.action.as_str()),
            "Invalid ordered action path"
        );
        last = street;
    }
    if let Some(street) = &s.street {
        anyhow::ensure!(
            s.line.is_empty() || order(street) >= last,
            "Path extends past selected street"
        );
    }
    Ok(())
}

pub fn predicate(q: &StudyQuery) -> Result<(String, Vec<Sql>)> {
    validate_spot(&q.spot)?;
    // Explicit pre-deal cohort fields only. Outcome/action/board filters from the
    // tracker cannot silently bias opportunity denominators.
    let f = &q.filter;
    let clean = Filter {
        profile: f.profile.clone(),
        date_from: f.date_from.clone(),
        date_to: f.date_to.clone(),
        timezone: f.timezone.clone(),
        game: f.game.clone(),
        stakes: f.stakes.clone(),
        currency: f.currency.clone(),
        hand_class: f.hand_class.clone(),
        ..Filter::default()
    };
    anyhow::ensure!(
        serde_json::to_value(f)? == serde_json::to_value(&clean)?,
        "Study cohorts accept only source, date, stakes, game, currency and starting hand"
    );
    let (mut clause, mut values) = store::predicate(&clean, true)?;
    clause.push_str(" AND d.hand=h.id AND si.hand=h.id AND si.version=?");
    values.push(Sql::Text(decision::VERSION.into()));
    for (field, value) in [
        ("street", &q.spot.street),
        ("position", &q.spot.position),
        ("opponent", &q.spot.opponent),
        ("role", &q.spot.role),
        ("facing", &q.spot.facing),
        ("pot_type", &q.spot.pot_type),
    ] {
        if let Some(v) = value {
            clause.push_str(&format!(" AND d.{field}=?"));
            values.push(Sql::Text(v.clone()));
        }
    }
    for (field, op, v) in [
        ("effective_bb", ">=", q.spot.effective_min),
        ("effective_bb", "<=", q.spot.effective_max),
        ("facing_pct", ">=", q.spot.facing_min),
        ("facing_pct", "<=", q.spot.facing_max),
    ] {
        if let Some(v) = v {
            clause.push_str(&format!(" AND d.{field}{op}?"));
            values.push(Sql::Real(v));
        }
    }
    for (field, v) in [
        ("texture", &q.spot.texture),
        ("high_card", &q.spot.high_card),
    ] {
        if let Some(v) = v {
            clause.push_str(&format!(" AND d.street!='preflop' AND h.{field}=?"));
            values.push(Sql::Text(v.clone()));
        }
    }
    if let Some(v) = q.spot.paired {
        clause.push_str(" AND d.street!='preflop' AND h.paired=?");
        values.push(Sql::Integer(i64::from(v)));
    }
    if let Some(a) = q.spot.line.first() {
        clause.push_str(&format!(" AND d.line_{}=?", a.street));
        values.push(Sql::Text(decision::line_code(&q.spot.line, &a.street)));
    }
    Ok((clause, values))
}

pub const FROM: &str =
    "study_decisions d JOIN hands h ON h.id=d.hand JOIN study_indexed si ON si.hand=h.id";

pub fn explore(c: &Connection, q: &StudyQuery) -> Result<Value> {
    let (clause, mut vals) = predicate(q)?;
    // Group by hand once: opportunity counts may repeat, hand results may not.
    let (opportunities,hands,net_bb,actions)=c.query_row(
        &format!("SELECT COALESCE(SUM(n),0),COUNT(*),COALESCE(SUM(net_bb),0),COALESCE(SUM(f),0),COALESCE(SUM(x),0),COALESCE(SUM(c),0),COALESCE(SUM(b),0),COALESCE(SUM(r),0) FROM (SELECT h.id,h.net_bb,COUNT(*) n,SUM(d.action='fold') f,SUM(d.action='check') x,SUM(d.action='call') c,SUM(d.action='bet') b,SUM(d.action='raise') r FROM {FROM} WHERE {clause} GROUP BY h.id)"),
        params_from_iter(&vals),|row| {
            let mut actions=BTreeMap::<String,u64>::new();
            for (index,name) in decision::ACTIONS.iter().enumerate(){actions.insert((*name).into(),row.get(index+3)?);}
            Ok((row.get::<_,u64>(0)?,row.get::<_,u64>(1)?,row.get::<_,f64>(2)?,actions))
        })?;
    let mut page_clause = clause;
    if let Some(before) = q.before {
        page_clause.push_str(" AND d.id<?");
        vals.push(Sql::Integer(before));
    }
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    vals.push(Sql::Integer((limit + 1).into()));
    let mut s=c.prepare(&format!("SELECT d.id,d.hand,d.data,h.played_at,h.net_bb FROM {FROM} WHERE {page_clause} ORDER BY d.id DESC LIMIT ?"))?;
    let mut rows=s.query_map(params_from_iter(vals),|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,f64>(4)?)))?
        .map(|r| {let (id,hand,data,date,net)=r?;Ok(json!({"id":id,"hand":hand,"decision":serde_json::from_str::<Value>(&data)?,"played_at":date,"net_bb":net}))}).collect::<Result<Vec<_>>>()?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next = if more {
        rows.last().and_then(|v| v["id"].as_i64())
    } else {
        None
    };
    Ok(
        json!({"opportunities":opportunities,"hands":hands,"net_bb":net_bb,"actions":actions,"rows":rows,"next_cursor":next,"coverage":coverage(c)?}),
    )
}

pub fn decision_by_id(c: &Connection, id: i64) -> Result<decision::Decision> {
    let data: String = c.query_row("SELECT data FROM study_decisions WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    Ok(serde_json::from_str(&data)?)
}

pub fn resolve(c: &Connection, r: &decision::DecisionRef) -> Result<(i64, decision::Decision)> {
    anyhow::ensure!(
        r.version == decision::VERSION,
        "Decision version changed; recreate this training card"
    );
    let (id,data):(i64,String)=c.query_row("SELECT h.id,d.data FROM study_decisions d JOIN hands h ON h.id=d.hand WHERE h.profile=?1 AND h.hand_id=?2 AND d.seq=?3",params![r.profile,r.hand_id,r.seq],|row|Ok((row.get(0)?,row.get(1)?)))?;
    Ok((id, serde_json::from_str(&data)?))
}

pub fn items(c: &Connection, kind: &str) -> Result<Value> {
    let mut s = c.prepare("SELECT id,data FROM study_items WHERE kind=?1 ORDER BY id")?;
    let rows = s
        .query_map([kind], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .map(|r| {
            let (id, data) = r?;
            Ok(json!({"id":id,"data":serde_json::from_str::<Value>(&data)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!(rows))
}

pub fn save_item(c: &Connection, kind: &str, id: Option<&str>, data: Value) -> Result<Value> {
    match kind {
        "spot" => {
            let name = data["name"].as_str().context("Missing spot name")?;
            check_name(name)?;
            validate_spot(&serde_json::from_value(data["spot"].clone())?)?;
        }
        "benchmark" => {
            let b: Benchmark = serde_json::from_value(data.clone())?;
            check_name(&b.name)?;
            validate_spot(&b.spot)?;
            anyhow::ensure!(
                decision::ACTIONS.contains(&b.action.as_str())
                    && b.low.is_finite()
                    && b.high.is_finite()
                    && 0. <= b.low
                    && b.low <= b.high
                    && b.high <= 100.
                    && (1..=1_000_000).contains(&b.min_samples)
                    && b.note.chars().count() <= 4000,
                "Invalid benchmark"
            );
        }
        _ => bail!("Invalid study item kind"),
    }
    let key = id.map(str::to_owned).unwrap_or_else(|| {
        format!(
            "{}-{}",
            kind,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    });
    anyhow::ensure!(key.len() <= 200, "Invalid identifier");
    c.execute("INSERT INTO study_items VALUES(?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET data=excluded.data",params![kind,key,serde_json::to_string(&data)?])?;
    Ok(json!({"id":key,"saved":true}))
}

fn check_name(name: &str) -> Result<()> {
    anyhow::ensure!(
        !name.trim().is_empty() && name.chars().count() <= 150,
        "Name must contain 1–150 characters"
    );
    Ok(())
}

pub fn wilson(hits: u64, n: u64) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let n = n as f64;
    let p = hits as f64 / n;
    let z = 1.959963984540054;
    let z2 = z * z;
    let den = 1. + z2 / n;
    let center = (p + z2 / (2. * n)) / den;
    let radius = z * ((p * (1. - p) + z2 / (4. * n)) / n).sqrt() / den;
    Some((
        (center - radius).max(0.) * 100.,
        (center + radius).min(1.) * 100.,
    ))
}

pub fn leaks(c: &Connection, filter: &Filter, pack: Option<&str>) -> Result<Value> {
    let mut rows = vec![];
    for row in items(c, "benchmark")?
        .as_array()
        .context("Invalid benchmarks")?
    {
        let b: Benchmark = serde_json::from_value(row["data"].clone())?;
        let q = StudyQuery {
            filter: filter.clone(),
            spot: b.spot.clone(),
            ..StudyQuery::default()
        };
        let (clause, vals) = predicate(&q)?;
        let mut args = vec![Sql::Text(b.action.clone())];
        args.extend(vals);
        let (hits, n): (u64, u64) = c.query_row(
            &format!("SELECT COALESCE(SUM(d.action=?),0),COUNT(*) FROM {FROM} WHERE {clause}"),
            params_from_iter(args),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let actual = if n > 0 {
            Some(hits as f64 / n as f64 * 100.)
        } else {
            None
        };
        let gap = actual.map(|v| {
            if v < b.low {
                v - b.low
            } else if v > b.high {
                v - b.high
            } else {
                0.
            }
        });
        rows.push(json!({"id":row["id"],"name":b.name,"spot":b.spot,"source":"custom","note":b.note,"action":b.action,"low":b.low,"high":b.high,"actual":actual,"gap":gap,"interval":wilson(hits,n),"opportunities":n,"hits":hits,"enough":n>=b.min_samples,"priority":if n>=b.min_samples{gap.unwrap_or(0.).abs()}else{0.}}));
    }
    if let Some(pack) = pack {
        rows.extend(strategy::leaks(c, filter, pack)?);
    }
    rows.sort_by(|a, b| {
        b["priority"]
            .as_f64()
            .unwrap_or(0.)
            .total_cmp(&a["priority"].as_f64().unwrap_or(0.))
    });
    Ok(json!({"rows":rows,"coverage":coverage(c)?}))
}

/// User-owned objects are separate from derived IDs and survive source rebuilds.
pub fn copy_user_data(source: &Connection, dest: &Connection) -> Result<()> {
    for (table, columns) in [
        ("study_items", vec!["kind", "id", "data"]),
        ("study_packs", vec!["id", "data"]),
        ("study_cards", vec!["id", "data", "due", "stage"]),
        ("study_sessions", vec!["id", "data"]),
        (
            "study_attempts",
            vec!["id", "card", "session", "created", "data"],
        ),
    ] {
        let exists: Option<String> = source
            .query_row(
                "SELECT name FROM sqlite_master WHERE name=?1",
                [table],
                |r| r.get(0),
            )
            .optional()?;
        if exists.is_none() {
            continue;
        }
        let mut s = source.prepare(&format!("SELECT {} FROM {table}", columns.join(",")))?;
        let mut rows = s.query([])?;
        while let Some(row) = rows.next()? {
            let vals = (0..columns.len())
                .map(|i| row.get::<_, Sql>(i))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            dest.execute(
                &format!(
                    "INSERT OR REPLACE INTO {table} VALUES({})",
                    vec!["?"; columns.len()].join(",")
                ),
                params_from_iter(vals),
            )?;
        }
    }
    Ok(())
}
