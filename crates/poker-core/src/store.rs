use crate::{model::*, money};
use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, TimeZone};
use chrono_tz::Tz;
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use rusqlite::{params, params_from_iter, types::Value as Sql, Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let c = Connection::open(path)?;
    c.busy_timeout(Duration::from_secs(15))?;
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA cache_size=-32000; PRAGMA temp_store=MEMORY;")?;
    Ok(c)
}
fn stat_columns() -> String {
    STAT_DEFINITIONS
        .iter()
        .flat_map(|(k, _, _)| {
            [
                format!("{k}_n INTEGER NOT NULL DEFAULT 0"),
                format!("{k}_d INTEGER NOT NULL DEFAULT 0"),
            ]
        })
        .collect::<Vec<_>>()
        .join(",")
}
fn stat_names() -> Vec<String> {
    STAT_DEFINITIONS
        .iter()
        .flat_map(|(k, _, _)| [format!("{k}_n"), format!("{k}_d")])
        .collect()
}
pub fn init(path: &Path) -> Result<()> {
    let c = open(path)?;
    c.execute_batch(&format!("
      CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
      INSERT OR IGNORE INTO metadata VALUES('schema_version','2');
      CREATE TABLE IF NOT EXISTS profiles(id TEXT PRIMARY KEY, data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY, data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS hands(
        id INTEGER PRIMARY KEY, profile TEXT NOT NULL, hand_id TEXT NOT NULL, fingerprint TEXT NOT NULL,
        source_file TEXT NOT NULL, played_at INTEGER NOT NULL, local_date TEXT NOT NULL,
        currency TEXT NOT NULL, game TEXT NOT NULL, stakes TEXT NOT NULL, position TEXT NOT NULL,
        player_count INTEGER NOT NULL, pot_type TEXT NOT NULL, hand_class TEXT NOT NULL,
        hero_stack_bb REAL NOT NULL, effective_bb REAL, flop_players INTEGER NOT NULL,
        texture TEXT NOT NULL, paired INTEGER NOT NULL, high_card TEXT NOT NULL,
        net INTEGER NOT NULL, net_bb REAL NOT NULL, showdown INTEGER NOT NULL,
        status TEXT NOT NULL, ev_status TEXT NOT NULL, ev_reason TEXT, adjusted_net INTEGER, equity REAL,
        session TEXT NOT NULL, {stats},
        UNIQUE(profile,hand_id));
      CREATE TABLE IF NOT EXISTS hand_payload(hand INTEGER PRIMARY KEY REFERENCES hands(id) ON DELETE CASCADE,detail BLOB NOT NULL);
      CREATE INDEX IF NOT EXISTS hand_time ON hands(played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_source_date ON hands(local_date,played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_status_time ON hands(status,played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_profile_time ON hands(profile,played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_position_time ON hands(position,played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_class_time ON hands(hand_class,played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_game_stakes ON hands(game,stakes,played_at DESC,id DESC);
      CREATE INDEX IF NOT EXISTS hand_ev ON hands(ev_status,id);
      CREATE INDEX IF NOT EXISTS hand_invalid ON hands(status) WHERE status!='valid';
      CREATE TABLE IF NOT EXISTS hero_actions(hand INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,street TEXT NOT NULL,kind TEXT NOT NULL,bet_pct REAL);
      CREATE INDEX IF NOT EXISTS action_filter ON hero_actions(street,kind,hand);
      CREATE TABLE IF NOT EXISTS cues(hand INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,kind TEXT NOT NULL, PRIMARY KEY(hand,kind));
      CREATE INDEX IF NOT EXISTS cue_filter ON cues(kind,hand);
      CREATE TABLE IF NOT EXISTS annotations(hand INTEGER PRIMARY KEY REFERENCES hands(id) ON DELETE CASCADE,note TEXT NOT NULL DEFAULT '',reviewed INTEGER NOT NULL DEFAULT 0);
      CREATE TABLE IF NOT EXISTS tags(hand INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,tag TEXT NOT NULL,PRIMARY KEY(hand,tag));
      CREATE INDEX IF NOT EXISTS tag_filter ON tags(tag,hand);
      CREATE TABLE IF NOT EXISTS saved_filters(id INTEGER PRIMARY KEY,name TEXT NOT NULL,data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS import_issues(id INTEGER PRIMARY KEY,job TEXT NOT NULL,source_file TEXT NOT NULL,hand_id TEXT,code TEXT NOT NULL,message TEXT NOT NULL,raw BLOB);
      CREATE TABLE IF NOT EXISTS equity_cache(key TEXT PRIMARY KEY,equity REAL NOT NULL);
      CREATE TABLE IF NOT EXISTS equity_results(key TEXT PRIMARY KEY,equity REAL NOT NULL,adjusted_net INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS rollups(dimension TEXT NOT NULL,value TEXT NOT NULL,profile TEXT NOT NULL,currency TEXT NOT NULL,hands INTEGER NOT NULL,net INTEGER NOT NULL,net_bb REAL NOT NULL,sd INTEGER NOT NULL,nsd INTEGER NOT NULL,{stats},PRIMARY KEY(dimension,value,profile,currency));
    ",stats=stat_columns()))?;
    let version: String = c.query_row(
        "SELECT value FROM metadata WHERE key='schema_version'",
        [],
        |r| r.get(0),
    )?;
    anyhow::ensure!(
        version == "1" || version == "2" || version == "3",
        "unsupported database schema {version}"
    );
    if version != "3" {
        let populated: bool =
            c.query_row("SELECT EXISTS(SELECT 1 FROM hands)", [], |r| r.get(0))?;
        if populated {
            backup(
                &c,
                &path.with_file_name(format!(
                    "before-study-{}.db",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                )),
            )?;
        }
    }
    if version == "1" {
        c.execute_batch("BEGIN IMMEDIATE; INSERT INTO hand_payload SELECT id,detail FROM hands; ALTER TABLE hands DROP COLUMN detail; UPDATE metadata SET value='2' WHERE key='schema_version'; COMMIT;")?;
    }
    crate::study::init(&c)?;
    ensure_session_rollups(&c)?;
    c.execute(
        "INSERT OR IGNORE INTO profiles(id,data) VALUES(?1,?2)",
        params![
            Profile::default().id,
            serde_json::to_string(&Profile::default())?
        ],
    )?;
    Ok(())
}

fn ensure_session_rollups(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE INDEX IF NOT EXISTS hand_invalid ON hands(status) WHERE status!='valid'",
    )?;
    // Keep broad stat-opportunity aggregation on narrow index pages. HH payload
    // and long provenance strings are intentionally absent from this index.
    let stat_cover = stat_names()
        .into_iter()
        .filter(|n| n != "vpip_d")
        .collect::<Vec<_>>()
        .join(",");
    c.execute_batch(&format!("CREATE INDEX IF NOT EXISTS hand_stat_cover ON hands(status,vpip_d,profile,local_date,currency,position,stakes,game,hand_class,net,net_bb,showdown,adjusted_net,ev_status,ev_reason,{stat_cover})"))?;
    for (marker, dimension, value) in [
        ("session_rollup_v1", "session", "session"),
        (
            "slice_rollup_v1",
            "slice",
            "json_array(position,stakes,game,local_date)",
        ),
    ] {
        let ready: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM metadata WHERE key=?1)",
            [marker],
            |r| r.get(0),
        )?;
        if ready {
            continue;
        }
        let names = stat_names();
        let sums = names
            .iter()
            .map(|n| format!("SUM({n})"))
            .collect::<Vec<_>>()
            .join(",");
        let tx = c.unchecked_transaction()?;
        tx.execute(&format!("INSERT OR REPLACE INTO rollups(dimension,value,profile,currency,hands,net,net_bb,sd,nsd,{}) SELECT '{dimension}',{value},profile,currency,COUNT(*),SUM(net),SUM(net_bb),SUM(CASE WHEN showdown=1 THEN net ELSE 0 END),SUM(CASE WHEN showdown=0 THEN net ELSE 0 END),{sums} FROM hands WHERE status='valid' GROUP BY {value},profile,currency", names.join(",")), [])?;
        tx.execute("INSERT OR REPLACE INTO metadata VALUES(?1,'1')", [marker])?;
        tx.commit()?;
    }
    Ok(())
}

fn pack(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::fast());
    e.write_all(bytes)?;
    Ok(e.finish()?)
}
fn unpack(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut d = ZlibDecoder::new(bytes);
    let mut out = vec![];
    d.read_to_end(&mut out)?;
    Ok(out)
}
pub fn fingerprint(raw: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(raw.replace("\r\n", "\n").trim().as_bytes())
    )
}

#[derive(Default)]
pub struct BatchResult {
    pub inserted: u64,
    pub duplicates: u64,
    pub quarantined: u64,
    pub conflicts: u64,
}

pub fn insert_batch(
    c: &mut Connection,
    job: &str,
    source_file: &str,
    hands: &[Hand],
) -> Result<BatchResult> {
    let tx = c.transaction()?;
    let mut result = BatchResult::default();
    let base="profile,hand_id,fingerprint,source_file,played_at,local_date,currency,game,stakes,position,player_count,pot_type,hand_class,hero_stack_bb,effective_bb,flop_players,texture,paired,high_card,net,net_bb,showdown,status,ev_status,ev_reason,adjusted_net,equity,session";
    let columns = format!("{base},{}", stat_names().join(","));
    let placeholders = std::iter::repeat_n("?", 28 + STAT_DEFINITIONS.len() * 2)
        .collect::<Vec<_>>()
        .join(",");
    let mut insert = tx.prepare_cached(&format!(
        "INSERT INTO hands({columns}) VALUES({placeholders})"
    ))?;
    let mut find =
        tx.prepare_cached("SELECT fingerprint FROM hands WHERE profile=?1 AND hand_id=?2")?;
    let mut action_insert = tx.prepare_cached("INSERT INTO hero_actions VALUES(?1,?2,?3,?4)")?;
    let mut cue_insert = tx.prepare_cached("INSERT INTO cues VALUES(?1,?2)")?;
    let mut grouped = BTreeMap::<(String, String, String, String), (Vec<i64>, f64)>::new();
    for h in hands {
        let fp = fingerprint(&h.raw);
        match find.query_row(params![h.profile, h.id], |r| r.get::<_, String>(0)) {
            Ok(existing) => {
                if existing == fp {
                    result.duplicates += 1;
                } else {
                    result.conflicts += 1;
                    tx.execute("INSERT INTO import_issues(job,source_file,hand_id,code,message,raw) VALUES(?1,?2,?3,'conflicting_duplicate','Same source profile and hand ID with different content',?4)",params![job,source_file,h.id,pack(h.raw.as_bytes())?])?;
                }
                continue;
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(e) => return Err(e.into()),
        }
        let stakes = format!("{}/{}", money::format(h.sb), money::format(h.bb));
        // Session is provenance-based, not inferred from a changing Rush table name.
        let session = format!("{}:{}", h.profile, source_file);
        let mut vals = vec![
            Sql::from(h.profile.clone()),
            Sql::from(h.id.clone()),
            Sql::from(fp),
            Sql::from(source_file.to_string()),
            Sql::from(h.played_at),
            Sql::from(h.local_time[..10].to_string()),
            Sql::from(h.currency.clone()),
            Sql::from(h.game.clone()),
            Sql::from(stakes.clone()),
            Sql::from(h.position.clone()),
            Sql::from(h.player_count as i64),
            Sql::from(h.pot_type.clone()),
            Sql::from(h.hand_class.clone()),
            Sql::from(h.hero_stack_bb),
            h.hu_effective_bb.map_or(Sql::Null, Sql::Real),
            Sql::from(h.flop_players as i64),
            Sql::from(h.texture.clone()),
            Sql::from(h.paired as i64),
            Sql::from(h.high_card.clone()),
            Sql::from(h.net),
            Sql::from(h.net as f64 / h.bb as f64),
            Sql::from(h.showdown as i64),
            Sql::from(h.status.clone()),
            Sql::from(h.ev_status.clone()),
            h.ev_reason.clone().map_or(Sql::Null, Sql::Text),
            Sql::Null,
            Sql::Null,
            Sql::from(session.clone()),
        ];
        for (key, _, _) in STAT_DEFINITIONS {
            let s = h.stats.get(*key).cloned().unwrap_or_default();
            vals.push(Sql::from(s.numerator as i64));
            vals.push(Sql::from(s.opportunities as i64));
        }
        insert.execute(params_from_iter(vals))?;
        let id = tx.last_insert_rowid();
        tx.prepare_cached("INSERT INTO hand_payload VALUES(?1,?2)")?
            .execute(params![id, pack(&serde_json::to_vec(h)?)?])?;
        crate::study::index_hand(&tx, id, h)?;
        for a in h.actions.iter().filter(|a| {
            a.actor == Some(h.hero_seat)
                && matches!(a.kind.as_str(), "fold" | "check" | "call" | "bet" | "raise")
        }) {
            let before = a.pot_after - a.amount;
            let pct = if before > 0 {
                Some(a.amount as f64 / before as f64 * 100.)
            } else {
                None
            };
            action_insert.execute(params![id, a.street, a.kind, pct])?;
        }
        for cue in &h.cues {
            cue_insert.execute(params![id, cue])?;
        }
        for i in &h.issues {
            tx.execute("INSERT INTO import_issues(job,source_file,hand_id,code,message) VALUES(?1,?2,?3,?4,?5)",params![job,source_file,h.id,i.code,i.message])?;
        }
        result.inserted += 1;
        if h.status != "valid" {
            result.quarantined += 1;
            continue;
        }
        for (dimension, value) in [
            ("all", "all".to_string()),
            ("date", h.local_time[..10].to_string()),
            ("position", h.position.clone()),
            ("stakes", stakes.clone()),
            ("game", h.game.clone()),
            ("hand_class", h.hand_class.clone()),
            ("session", session),
            (
                "slice",
                serde_json::to_string(&[
                    h.position.as_str(),
                    stakes.as_str(),
                    h.game.as_str(),
                    &h.local_time[..10],
                ])?,
            ),
        ] {
            let (g, bb) = grouped
                .entry((
                    dimension.into(),
                    value,
                    h.profile.clone(),
                    h.currency.clone(),
                ))
                .or_insert_with(|| (vec![0; 5 + STAT_DEFINITIONS.len() * 2], 0.));
            g[0] += 1;
            g[1] += h.net;
            *bb += h.net as f64 / h.bb as f64;
            g[if h.showdown { 3 } else { 4 }] += h.net;
            for (i, (key, _, _)) in STAT_DEFINITIONS.iter().enumerate() {
                let s = &h.stats[*key];
                g[5 + i * 2] += s.numerator as i64;
                g[6 + i * 2] += s.opportunities as i64;
            }
        }
    }
    drop(insert);
    drop(find);
    drop(action_insert);
    drop(cue_insert);
    let mut names = vec![
        "hands".into(),
        "net".into(),
        "net_bb".into(),
        "sd".into(),
        "nsd".into(),
    ];
    names.extend(stat_names());
    let updates = names
        .iter()
        .map(|n| format!("{n}={n}+excluded.{n}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql=format!("INSERT INTO rollups(dimension,value,profile,currency,{}) VALUES({}) ON CONFLICT(dimension,value,profile,currency) DO UPDATE SET {updates}",names.join(","),std::iter::repeat_n("?",names.len()+4).collect::<Vec<_>>().join(","));
    {
        let mut statement = tx.prepare_cached(&sql)?;
        for ((d, v, p, c), (totals, bb)) in grouped {
            let mut vals = vec![Sql::Text(d), Sql::Text(v), Sql::Text(p), Sql::Text(c)];
            for (i, n) in totals.into_iter().enumerate() {
                vals.push(if i == 2 {
                    Sql::Real(bb)
                } else {
                    Sql::Integer(n)
                });
            }
            statement.execute(params_from_iter(vals))?;
        }
    }
    tx.commit()?;
    Ok(result)
}

pub fn reject(c: &Connection, job: &str, file: &str, raw: &str, message: &str) -> Result<()> {
    c.execute("INSERT INTO import_issues(job,source_file,code,message,raw) VALUES(?1,?2,'unsupported_or_invalid',?3,?4)",params![job,file,message,pack(raw.as_bytes())?])?;
    Ok(())
}
pub fn save_job(c: &Connection, job: &Job) -> Result<()> {
    c.execute(
        "INSERT INTO jobs VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![job.id, serde_json::to_string(job)?],
    )?;
    Ok(())
}
pub fn jobs(c: &Connection) -> Result<Vec<Job>> {
    let mut s = c.prepare("SELECT data FROM jobs ORDER BY rowid DESC LIMIT 100")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
}
pub fn profiles(c: &Connection) -> Result<Vec<Profile>> {
    let mut s = c.prepare("SELECT data FROM profiles ORDER BY rowid")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
}
pub fn save_profile(c: &Connection, p: &Profile) -> Result<()> {
    anyhow::ensure!(
        !p.id.is_empty() && !p.hero.is_empty(),
        "profile ID and Hero are required"
    );
    let _: Tz = p.timezone.parse()?;
    if c.query_row(
        "SELECT EXISTS(SELECT 1 FROM hands WHERE profile=?1)",
        [&p.id],
        |r| r.get::<_, bool>(0),
    )? {
        let old: Profile = serde_json::from_str(&c.query_row(
            "SELECT data FROM profiles WHERE id=?1",
            [&p.id],
            |r| r.get::<_, String>(0),
        )?)?;
        anyhow::ensure!(
            old.hero == p.hero && old.timezone == p.timezone && old.brand == p.brand,
            "來源已有手牌；更改 Hero／時區／品牌請建立新來源，以免混合不同解析設定"
        );
    }
    c.execute(
        "INSERT INTO profiles VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![p.id, serde_json::to_string(p)?],
    )?;
    Ok(())
}

pub fn predicate(f: &Filter, include_status: bool) -> Result<(String, Vec<Sql>)> {
    let mut clauses = vec!["1=1".to_string()];
    let mut vals = vec![];
    let mut eq = |col: &str, v: Option<&str>| {
        if let Some(v) = v.filter(|v| !v.is_empty()) {
            clauses.push(format!("h.{col}=?"));
            vals.push(Sql::from(v.to_string()));
        }
    };
    eq("profile", f.profile.as_deref());
    eq("position", f.position.as_deref());
    eq("game", f.game.as_deref());
    eq("stakes", f.stakes.as_deref());
    eq("currency", f.currency.as_deref());
    eq("pot_type", f.pot_type.as_deref());
    eq("hand_class", f.hand_class.as_deref());
    eq("texture", f.texture.as_deref());
    eq("high_card", f.high_card.as_deref());
    eq("session", f.session.as_deref());
    eq("local_date", f.source_date.as_deref());
    eq("ev_status", f.ev_status.as_deref());
    if include_status {
        eq("status", Some(f.status.as_deref().unwrap_or("valid")));
    }
    let tz: Tz = f.timezone.as_deref().unwrap_or("Asia/Hong_Kong").parse()?;
    for (key, date, end) in [
        ("played_at", &f.date_from, false),
        ("played_at", &f.date_to, true),
    ] {
        if let Some(date) = date.as_ref().filter(|v| !v.is_empty()) {
            let mut d = NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
            if end {
                d = d.succ_opt().context("date overflow")?;
            }
            let t = tz
                .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .context("ambiguous filter date")?
                .timestamp();
            clauses.push(format!("h.{key} {} ?", if end { "<" } else { ">=" }));
            vals.push(Sql::Integer(t));
        }
    }
    for (col, v) in [
        ("player_count", f.player_count.map(i64::from)),
        ("flop_players", f.flop_players.map(i64::from)),
        ("paired", f.paired.map(i64::from)),
        ("showdown", f.showdown.map(i64::from)),
    ] {
        if let Some(v) = v {
            clauses.push(format!("h.{col}=?"));
            vals.push(Sql::Integer(v));
        }
    }
    for (col, op, v) in [
        ("hero_stack_bb", ">=", f.stack_min),
        ("hero_stack_bb", "<=", f.stack_max),
        ("effective_bb", ">=", f.effective_min),
        ("effective_bb", "<=", f.effective_max),
    ] {
        if let Some(v) = v {
            anyhow::ensure!(v.is_finite(), "invalid numeric filter");
            clauses.push(format!("h.{col}{op}?"));
            vals.push(Sql::Real(v));
        }
    }
    if let Some(stat) = &f.stat {
        anyhow::ensure!(
            STAT_DEFINITIONS.iter().any(|(k, _, _)| k == stat),
            "unknown stat"
        );
        let suffix = if f.stat_mode.as_deref() == Some("numerator") {
            "n"
        } else {
            "d"
        };
        clauses.push(format!("h.{stat}_{suffix}>0"));
    }
    if let Some(cue) = &f.cue {
        clauses.push("EXISTS(SELECT 1 FROM cues c WHERE c.hand=h.id AND c.kind=?)".into());
        vals.push(Sql::Text(cue.clone()));
    }
    if let Some(tag) = &f.tag {
        clauses.push("EXISTS(SELECT 1 FROM tags t WHERE t.hand=h.id AND t.tag=?)".into());
        vals.push(Sql::Text(tag.clone()));
    }
    if let Some(reviewed) = f.reviewed {
        clauses.push("COALESCE((SELECT reviewed FROM annotations a WHERE a.hand=h.id),0)=?".into());
        vals.push(Sql::Integer(reviewed as i64));
    }
    if let Some(search) = &f.search {
        clauses.push("h.hand_id LIKE ?".into());
        vals.push(Sql::Text(format!("%{search}%")));
    }
    if let Some(result) = &f.result {
        match result.as_str() {
            "win" => clauses.push("h.net>0".into()),
            "loss" => clauses.push("h.net<0".into()),
            _ => {}
        }
    }
    if f.street.is_some() || f.action.is_some() || f.bet_min.is_some() || f.bet_max.is_some() {
        let mut a = vec!["a.hand=h.id".to_string()];
        if let Some(v) = &f.street {
            a.push("a.street=?".into());
            vals.push(Sql::Text(v.clone()));
        }
        if let Some(v) = &f.action {
            a.push("a.kind=?".into());
            vals.push(Sql::Text(v.clone()));
        }
        if let Some(v) = f.bet_min {
            a.push("a.bet_pct>=?".into());
            vals.push(Sql::Real(v));
        }
        if let Some(v) = f.bet_max {
            a.push("a.bet_pct<=?".into());
            vals.push(Sql::Real(v));
        }
        clauses.push(format!(
            "EXISTS(SELECT 1 FROM hero_actions a WHERE {})",
            a.join(" AND ")
        ));
    }
    Ok((clauses.join(" AND "), vals))
}

pub fn report(c: &Connection, f: &Filter, group: &str) -> Result<Value> {
    // Broad reports read exact incremental rollups, never decompress or scan HH payloads.
    let basic = serde_json::to_value(f)?
        .as_object()
        .unwrap()
        .iter()
        .all(|(k, v)| {
            v.is_null()
                || matches!(
                    k.as_str(),
                    "profile"
                        | "currency"
                        | "timezone"
                        | "position"
                        | "stakes"
                        | "game"
                        | "date_from"
                        | "date_to"
                )
                || (k == "status" && v == "valid")
        });
    let date_filter = f.date_from.is_some() || f.date_to.is_some();
    let date_compatible = !date_filter
        || profiles(c)?
            .iter()
            .filter(|p| f.profile.as_ref().is_none_or(|id| id == &p.id))
            .all(|p| p.timezone == f.timezone.as_deref().unwrap_or("Asia/Hong_Kong"));
    if basic
        && date_compatible
        && matches!(
            group,
            "date" | "position" | "stakes" | "game" | "hand_class" | "session"
        )
        && (!(f.position.is_some() || f.stakes.is_some() || f.game.is_some() || date_filter)
            || matches!(group, "date" | "position" | "stakes" | "game"))
    {
        return rollup_report(c, f, group);
    }
    let (mut pred, vals) = predicate(f, true)?;
    pred.push_str(" AND h.status='valid'");
    let (coverage_pred, coverage_vals) = predicate(f, false)?;
    let from = "hands h";
    let stat_sql = STAT_DEFINITIONS
        .iter()
        .flat_map(|(k, _, _)| {
            [
                format!("COALESCE(SUM({k}_n),0)"),
                format!("COALESCE(SUM({k}_d),0)"),
            ]
        })
        .collect::<Vec<_>>()
        .join(",");
    let sql=format!("SELECT COUNT(*),COALESCE(SUM(net),0),COALESCE(SUM(net_bb),0),COALESCE(SUM(CASE WHEN showdown=1 THEN net ELSE 0 END),0),COALESCE(SUM(CASE WHEN showdown=0 THEN net ELSE 0 END),0),COALESCE(SUM(COALESCE(adjusted_net,net)),0),SUM(ev_status='complete'),SUM(ev_status='pending'),SUM(ev_status='excluded'),{stat_sql} FROM {from} WHERE {pred}");
    let (count,net,net_bb,sd,nsd,adj,complete,pending,excluded,stats)=c.query_row(&sql,params_from_iter(vals.iter()),|r|{
        let mut stats=vec![];for(i,(id,label,definition))in STAT_DEFINITIONS.iter().enumerate(){let n:i64=r.get(9+i*2)?;let d:i64=r.get(10+i*2)?;stats.push(json!({"id":id,"label":label,"definition":definition,"numerator":n,"opportunities":d,"value":if d>0{Some(n as f64/d as f64*100.)}else{None}}));}
        Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,f64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,Option<i64>>(6)?.unwrap_or(0),r.get::<_,Option<i64>>(7)?.unwrap_or(0),r.get::<_,Option<i64>>(8)?.unwrap_or(0),stats))
    })?;
    let mut coverage = BTreeMap::new();
    let mut st = c.prepare(&format!(
        "SELECT status,COUNT(*) FROM {from} WHERE {coverage_pred} GROUP BY status"
    ))?;
    for row in st.query_map(params_from_iter(coverage_vals.iter()), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (k, v) = row?;
        coverage.insert(k, v);
    }
    coverage.entry("valid".into()).or_insert(0);
    coverage.entry("quarantined".into()).or_insert(0);
    let col = match group {
        "position" => "position",
        "stakes" => "stakes",
        "game" => "game",
        "session" => "session",
        "hand_class" => "hand_class",
        _ => "local_date",
    };
    let mut groups = vec![];
    let mut st=c.prepare(&format!("SELECT {col},COUNT(*),SUM(net),SUM(net_bb),SUM(CASE WHEN showdown=1 THEN net ELSE 0 END),SUM(COALESCE(adjusted_net,net)) FROM {from} WHERE {pred} GROUP BY {col} ORDER BY {col} LIMIT 2001"))?;
    for r in st.query_map(params_from_iter(vals.iter()), |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, f64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })? {
        let (key, n, net, bb, sd, adj) = r?;
        groups.push(json!({"key":key,"hands":n,"net":money::format(net),"net_bb":bb,"bb100":bb/n as f64*100.,"sd":money::format(sd),"nsd":money::format(net-sd),"adjusted":money::format(adj)}));
    }
    let groups_truncated = groups.len() > 2000;
    groups.truncate(2000);
    let mut currencies = vec![];
    let mut st = c.prepare(&format!(
        "SELECT DISTINCT currency FROM {from} WHERE {pred}"
    ))?;
    for r in st.query_map(params_from_iter(vals.iter()), |r| r.get::<_, String>(0))? {
        currencies.push(r?);
    }
    let mut reasons = BTreeMap::new();
    let mut st=c.prepare(&format!("SELECT ev_reason,COUNT(*) FROM {from} WHERE {pred} AND ev_status='excluded' GROUP BY ev_reason"))?;
    for r in st.query_map(params_from_iter(vals.iter()), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (k, v) = r?;
        reasons.insert(k, v);
    }
    Ok(
        json!({"hands":count,"net":if currencies.len()<=1{Some(money::format(net))}else{None},"currency":currencies.first().cloned().unwrap_or_else(||"USD".into()),"net_bb":net_bb,"bb100":if count>0{net_bb/count as f64*100.}else{0.},"sd":money::format(sd),"nsd":money::format(nsd),"adjusted":money::format(adj),"stats":stats,"groups":groups,"groups_truncated":groups_truncated,"coverage":coverage,"ev":{"complete":complete,"pending":pending,"excluded":excluded,"reasons":reasons},"stats_version":STATS_VERSION}),
    )
}

fn rollup_report(c: &Connection, f: &Filter, group: &str) -> Result<Value> {
    let sliced = f.position.is_some()
        || f.stakes.is_some()
        || f.game.is_some()
        || f.date_from.is_some()
        || f.date_to.is_some();
    let mut pred = "1=1".to_string();
    let mut vals = vec![];
    for (key, value) in [("profile", &f.profile), ("currency", &f.currency)] {
        if let Some(v) = value {
            pred += &format!(" AND {key}=?");
            vals.push(Sql::Text(v.clone()));
        }
    }
    let mut hand_pred = pred.clone();
    for (index, key, value) in [
        (0, "position", &f.position),
        (1, "stakes", &f.stakes),
        (2, "game", &f.game),
    ] {
        if let Some(v) = value {
            pred += &format!(" AND json_extract(value,'$[{index}]')=?");
            hand_pred += &format!(" AND {key}=?");
            vals.push(Sql::Text(v.clone()));
        }
    }
    for (operator, date) in [(">=", &f.date_from), ("<=", &f.date_to)] {
        if let Some(date) = date {
            let normalized = NaiveDate::parse_from_str(date, "%Y-%m-%d")?
                .format("%Y-%m-%d")
                .to_string();
            pred += &format!(" AND json_extract(value,'$[3]'){operator}?");
            hand_pred += &format!(" AND local_date{operator}?");
            vals.push(Sql::Text(normalized));
        }
    }
    let dimension = if sliced { "slice" } else { "all" };
    let sums = stat_names()
        .iter()
        .map(|k| format!("COALESCE(SUM({k}),0)"))
        .collect::<Vec<_>>()
        .join(",");
    let sql=format!("SELECT COALESCE(SUM(hands),0),COALESCE(SUM(net),0),COALESCE(SUM(net_bb),0),COALESCE(SUM(sd),0),COALESCE(SUM(nsd),0),{sums} FROM rollups WHERE dimension='{dimension}' AND {pred}");
    let (count,net,bb,sd,nsd,stats)=c.query_row(&sql,params_from_iter(vals.iter()),|r| {
        let mut stats=vec![];for (i,(id,label,definition)) in STAT_DEFINITIONS.iter().enumerate(){let n:i64=r.get(5+i*2)?;let d:i64=r.get(6+i*2)?;stats.push(json!({"id":id,"label":label,"definition":definition,"numerator":n,"opportunities":d,"value":if d>0{Some(n as f64/d as f64*100.)}else{None}}));}
        Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,f64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,stats))
    })?;
    let mut evcounts = BTreeMap::<String, i64>::new();
    let mut reasons = BTreeMap::new();
    let mut delta = 0i64;
    let mut delta_groups = BTreeMap::<String, i64>::new();
    let col = match group {
        "date" => "local_date",
        _ => group,
    };
    let mut s=c.prepare(&format!("SELECT ev_status,ev_reason,{col},COALESCE(adjusted_net-net,0) FROM hands INDEXED BY hand_ev WHERE status='valid' AND ev_status IN ('complete','pending','excluded') AND {hand_pred}"))?;
    let mut rows = s.query(params_from_iter(vals.iter()))?;
    while let Some(r) = rows.next()? {
        let status: String = r.get(0)?;
        *evcounts.entry(status.clone()).or_default() += 1;
        if status == "excluded" {
            let reason: String = r.get(1)?;
            *reasons.entry(reason).or_insert(0i64) += 1;
        }
        let d: i64 = r.get(3)?;
        delta += d;
        *delta_groups.entry(r.get::<_, String>(2)?).or_default() += d;
    }
    let excluded: i64 = c.query_row(
        &format!("SELECT COUNT(*) FROM hands INDEXED BY hand_invalid WHERE status!='valid' AND {hand_pred}"),
        params_from_iter(vals.iter()),
        |r| r.get(0),
    )?;
    let grouped_value = if sliced {
        match group {
            "position" => "json_extract(value,'$[0]')",
            "stakes" => "json_extract(value,'$[1]')",
            "game" => "json_extract(value,'$[2]')",
            _ => "json_extract(value,'$[3]')",
        }
    } else {
        "value"
    };
    let mut s=c.prepare(&format!("SELECT {grouped_value},SUM(hands),SUM(net),SUM(net_bb),SUM(sd),SUM(nsd) FROM rollups WHERE dimension=? AND {pred} GROUP BY {grouped_value} ORDER BY {grouped_value}"))?;
    let mut args = vec![Sql::Text(if sliced { "slice" } else { group }.into())];
    args.extend(vals.clone());
    let mut rows = s.query(params_from_iter(args))?;
    let mut groups = vec![];
    while let Some(r) = rows.next()? {
        let key: String = r.get(0)?;
        let n: i64 = r.get(1)?;
        let result: i64 = r.get(2)?;
        let bb: f64 = r.get(3)?;
        let adj = result + delta_groups.get(&key).copied().unwrap_or(0);
        groups.push(json!({"key":key,"hands":n,"net":money::format(result),"net_bb":bb,"bb100":bb/n as f64*100.,"sd":money::format(r.get(4)?),"nsd":money::format(r.get(5)?),"adjusted":money::format(adj)}));
    }
    Ok(
        json!({"hands":count,"net":money::format(net),"currency":f.currency.as_deref().unwrap_or("USD"),"net_bb":bb,"bb100":if count>0{bb/count as f64*100.}else{0.},"sd":money::format(sd),"nsd":money::format(nsd),"adjusted":money::format(net+delta),"stats":stats,"groups":groups,"groups_truncated":false,"coverage":{"valid":count,"quarantined":excluded},"ev":{"complete":evcounts.get("complete").copied().unwrap_or(0),"pending":evcounts.get("pending").copied().unwrap_or(0),"excluded":evcounts.get("excluded").copied().unwrap_or(0),"reasons":reasons},"stats_version":STATS_VERSION}),
    )
}

pub fn list_hands(
    c: &Connection,
    f: &Filter,
    sort: &str,
    cursor: Option<&Value>,
    limit: u32,
) -> Result<Value> {
    let (mut pred, mut vals) = predicate(f, true)?;
    let (sort_col, direction) = match sort {
        "win" => ("net_bb", "DESC"),
        "loss" => ("net_bb", "ASC"),
        _ => ("played_at", "DESC"),
    };
    if let Some(cursor) = cursor {
        let v = cursor
            .get("value")
            .and_then(Value::as_f64)
            .context("invalid cursor value")?;
        let id = cursor
            .get("id")
            .and_then(Value::as_i64)
            .context("invalid cursor id")?;
        let op = if direction == "DESC" { "<" } else { ">" };
        pred += &format!(" AND (h.{sort_col}{op}? OR (h.{sort_col}=? AND h.id<?))");
        vals.extend([Sql::Real(v), Sql::Real(v), Sql::Integer(id)]);
    }
    vals.push(Sql::Integer(limit.clamp(1, 200) as i64 + 1));
    let mut st=c.prepare(&format!("SELECT h.id,hand_id,played_at,local_date,position,hand_class,net,net_bb,stakes,pot_type,game,status,ev_status,COALESCE(a.reviewed,0),h.{sort_col} FROM hands h LEFT JOIN annotations a ON a.hand=h.id WHERE {pred} ORDER BY h.{sort_col} {direction},h.id DESC LIMIT ?"))?;
    let mut rows = vec![];
    for r in st.query_map(params_from_iter(vals.iter()),|r|Ok(json!({"id":r.get::<_,i64>(0)?,"hand_id":r.get::<_,String>(1)?,"played_at":r.get::<_,i64>(2)?,"date":r.get::<_,String>(3)?,"position":r.get::<_,String>(4)?,"hand_class":r.get::<_,String>(5)?,"net":money::format(r.get(6)?),"net_bb":r.get::<_,f64>(7)?,"stakes":r.get::<_,String>(8)?,"pot_type":r.get::<_,String>(9)?,"game":r.get::<_,String>(10)?,"status":r.get::<_,String>(11)?,"ev_status":r.get::<_,String>(12)?,"reviewed":r.get::<_,bool>(13)?,"sort_value":r.get::<_,f64>(14)?})))?{rows.push(r?);}
    let has_more = rows.len() > limit.clamp(1, 200) as usize;
    if has_more {
        rows.pop();
    }
    let next = if has_more {
        rows.last()
            .map(|r| json!({"value":r["sort_value"],"id":r["id"]}))
    } else {
        None
    };
    Ok(json!({"rows":rows,"next_cursor":next}))
}

pub fn get_hand(c: &Connection, id: i64) -> Result<Hand> {
    let (blob,ev,equity,adj):(Vec<u8>,String,Option<f64>,Option<i64>)=c.query_row("SELECT p.detail,h.ev_status,h.equity,h.adjusted_net FROM hands h JOIN hand_payload p ON p.hand=h.id WHERE h.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    let mut h: Hand = serde_json::from_slice(&unpack(&blob)?)?;
    h.ev_status = ev;
    h.equity = equity;
    h.adjusted_net = adj.map(money::format);
    Ok(h)
}
pub fn annotation(c: &Connection, id: i64) -> Result<Annotation> {
    let mut a = Annotation::default();
    match c.query_row(
        "SELECT note,reviewed FROM annotations WHERE hand=?1",
        [id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)),
    ) {
        Ok((n, r)) => {
            a.note = n;
            a.reviewed = r;
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {}
        Err(e) => return Err(e.into()),
    };
    let mut st = c.prepare("SELECT tag FROM tags WHERE hand=?1 ORDER BY tag")?;
    for r in st.query_map([id], |r| r.get::<_, String>(0))? {
        a.tags.push(r?);
    }
    Ok(a)
}
pub fn save_annotation(c: &mut Connection, id: i64, a: &Annotation) -> Result<()> {
    anyhow::ensure!(
        a.note.len() <= 50_000 && a.tags.len() <= 30,
        "annotation too large"
    );
    let tx = c.transaction()?;
    tx.execute("INSERT INTO annotations VALUES(?1,?2,?3) ON CONFLICT(hand) DO UPDATE SET note=excluded.note,reviewed=excluded.reviewed",params![id,a.note,a.reviewed])?;
    tx.execute("DELETE FROM tags WHERE hand=?1", [id])?;
    for tag in &a.tags {
        let t = tag.trim();
        if !t.is_empty() {
            anyhow::ensure!(t.len() <= 100, "tag too long");
            tx.execute("INSERT OR IGNORE INTO tags VALUES(?1,?2)", params![id, t])?;
        }
    }
    tx.commit()?;
    Ok(())
}
pub fn matrix(c: &Connection, f: &Filter, stat: &str) -> Result<Value> {
    anyhow::ensure!(
        STAT_DEFINITIONS.iter().any(|(k, _, _)| *k == stat),
        "unknown matrix stat"
    );
    let (mut pred, vals) = predicate(f, true)?;
    pred.push_str(" AND h.status='valid'");
    let mut s=c.prepare(&format!("SELECT hand_class,COUNT(*),SUM(net),SUM(net_bb),SUM({stat}_n),SUM({stat}_d) FROM hands h WHERE {pred} GROUP BY hand_class"))?;
    let rows=s.query_map(params_from_iter(vals.iter()),|r|{let n:i64=r.get(1)?;let bb:f64=r.get(3)?;Ok(json!({"hand":r.get::<_,String>(0)?,"hands":n,"net":money::format(r.get(2)?),"net_bb":bb,"bb100":bb/n as f64*100.,"numerator":r.get::<_,i64>(4)?,"opportunities":r.get::<_,i64>(5)?}))})?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!(rows))
}
pub fn issues(c: &Connection) -> Result<Value> {
    issues_page(c, None)
}
pub fn issues_page(c: &Connection, before: Option<i64>) -> Result<Value> {
    // Conflicts/rejected input have their own raw text: never route them to the stored hand.
    // Normalized issues must resolve within the importing profile, not another user's same ID.
    let mut s = c.prepare(
        "SELECT i.id,i.job,i.source_file,i.hand_id,i.code,i.message,
        CASE WHEN i.raw IS NULL THEN (SELECT h.id FROM hands h
          WHERE h.hand_id=i.hand_id AND h.profile=json_extract(j.data,'$.profile.id') LIMIT 1) END
        FROM import_issues i LEFT JOIN jobs j ON j.id=i.job
        WHERE (?1 IS NULL OR i.id<?1) ORDER BY i.id DESC LIMIT 301",
    )?;
    let mut rows=s.query_map([before],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"job":r.get::<_,String>(1)?,"file":r.get::<_,String>(2)?,"hand_id":r.get::<_,Option<String>>(3)?,"code":r.get::<_,String>(4)?,"message":r.get::<_,String>(5)?,"hand_row":r.get::<_,Option<i64>>(6)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    let more = rows.len() > 300;
    rows.truncate(300);
    let next_cursor = if more {
        rows.last().and_then(|r| r["id"].as_i64())
    } else {
        None
    };
    Ok(
        json!({"rows":rows,"next_cursor":next_cursor,"total":c.query_row("SELECT COUNT(*) FROM import_issues",[],|r|r.get::<_,i64>(0))?}),
    )
}
pub fn saved_filters(c: &Connection) -> Result<Value> {
    let mut s = c.prepare("SELECT id,name,data FROM saved_filters ORDER BY id DESC")?;
    let rows = s.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut out = vec![];
    for r in rows {
        let (id, name, data) = r?;
        out.push(json!({"id":id,"name":name,"filter":serde_json::from_str::<Value>(&data)?}));
    }
    Ok(json!(out))
}
pub fn save_filter(c: &Connection, name: &str, f: &Filter) -> Result<()> {
    anyhow::ensure!(
        !name.trim().is_empty() && name.chars().count() <= 150,
        "invalid filter name"
    );
    predicate(f, true)?;
    c.execute(
        "INSERT INTO saved_filters(name,data) VALUES(?1,?2)",
        params![name, serde_json::to_string(f)?],
    )?;
    Ok(())
}

pub fn backup(c: &Connection, path: &Path) -> Result<()> {
    anyhow::ensure!(
        !path.exists(),
        "Backup destination already exists; choose a new filename"
    );
    // A native save dialog can grant one selected file without granting its
    // SQLite journal/sidecar paths. Build a standalone image in our own folder,
    // close SQLite, then write only the selected file.
    let staged = snapshot_file(c)?;
    let mut dest = Connection::open(staged.path())?;
    {
        let b = rusqlite::backup::Backup::new(c, &mut dest)?;
        finish_backup(&b)?;
    }
    dest.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE")?;
    drop(dest);
    (|| -> Result<()> {
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        let copied = std::io::copy(&mut std::fs::File::open(staged.path())?, &mut output)
            .and_then(|_| output.sync_all());
        drop(output);
        if let Err(error) = copied {
            let _ = std::fs::remove_file(path);
            return Err(error.into());
        }
        Ok(())
    })()
}

fn snapshot_file(c: &Connection) -> Result<tempfile::NamedTempFile> {
    let parent = Path::new(c.path().context("snapshot needs a file database")?)
        .parent()
        .context("database has no parent directory")?;
    Ok(tempfile::Builder::new()
        .prefix(".riverlens-snapshot-")
        .suffix(".db")
        .tempfile_in(parent)?)
}

fn finish_backup(backup: &rusqlite::backup::Backup<'_, '_>) -> Result<()> {
    use rusqlite::backup::StepResult;
    let mut blocked_since = None;
    loop {
        match backup.step(1000)? {
            StepResult::Done => break,
            StepResult::More => blocked_since = None,
            StepResult::Busy | StepResult::Locked => {
                let since = blocked_since.get_or_insert_with(Instant::now);
                anyhow::ensure!(since.elapsed() < Duration::from_secs(15), "SQLite snapshot remained locked for 15 seconds; close other database writers and retry");
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => anyhow::bail!("unknown SQLite backup step"),
        }
    }
    Ok(())
}
pub fn restore(c: &mut Connection, path: &Path) -> Result<()> {
    // Open the selected backup without asking SQLite to create external WAL/SHM files.
    let staged = snapshot_file(c)?;
    std::fs::copy(path, staged.path())?;
    let src = Connection::open_with_flags(staged.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let check: String = src.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    anyhow::ensure!(check == "ok", "backup integrity check failed");
    let schema: String = src.query_row(
        "SELECT value FROM metadata WHERE key='schema_version'",
        [],
        |r| r.get(0),
    )?;
    anyhow::ensure!(
        schema == "1" || schema == "2" || schema == "3",
        "unsupported backup schema; open older database with this version first to migrate it"
    );
    drop(src);
    init(staged.path())?;
    let src = Connection::open_with_flags(staged.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    {
        let b = rusqlite::backup::Backup::new(&src, c)?;
        finish_backup(&b)?;
    }
    ensure_session_rollups(c)?;
    Ok(())
}
pub fn export(
    c: &Connection,
    f: &Filter,
    path: &Path,
    format: &str,
    selected: &[i64],
) -> Result<u64> {
    let (mut pred, mut vals) = predicate(f, true)?;
    if !selected.is_empty() {
        anyhow::ensure!(selected.len() <= 1000, "select up to 1000 hands");
        pred += &format!(
            " AND h.id IN ({})",
            std::iter::repeat_n("?", selected.len())
                .collect::<Vec<_>>()
                .join(",")
        );
        vals.extend(selected.iter().map(|id| Sql::Integer(*id)));
    }
    let grouped = format
        .strip_prefix("report:")
        .map(|group| report(c, f, group))
        .transpose()?;
    if let Some(report) = &grouped {
        anyhow::ensure!(
            report["groups_truncated"] != true,
            "報表超過 2,000 組；請縮窄日期後匯出，避免不完整 CSV"
        );
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let mut n = 0;
    if let Some(report) = grouped {
        writeln!(
            file,
            "group,hands,net_usd,net_bb,bb_per_100,showdown_usd,non_showdown_usd,adjusted_usd"
        )?;
        for row in report["groups"]
            .as_array()
            .context("missing report groups")?
        {
            writeln!(
                file,
                "{},{},{},{},{},{},{},{}",
                csv(row["key"].as_str().unwrap_or("")),
                row["hands"],
                row["net"].as_str().unwrap_or(""),
                row["net_bb"],
                row["bb100"],
                row["sd"].as_str().unwrap_or(""),
                row["nsd"].as_str().unwrap_or(""),
                row["adjusted"].as_str().unwrap_or("")
            )?;
            n += 1;
        }
    } else if format == "csv" {
        writeln!(
            file,
            "hand_id,time_utc,position,starting_hand,stakes,net,net_bb,ev_status"
        )?;
        let mut s=c.prepare(&format!("SELECT hand_id,played_at,position,hand_class,stakes,net,net_bb,ev_status FROM hands h WHERE {pred} ORDER BY played_at,id"))?;
        for r in s.query_map(params_from_iter(vals.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, f64>(6)?,
                r.get::<_, String>(7)?,
            ))
        })? {
            let (id, time, pos, card, stakes, net, bb, ev) = r?;
            writeln!(
                file,
                "{},{},{},{},{},{},{:.4},{}",
                csv(&id),
                time,
                csv(&pos),
                csv(&card),
                csv(&stakes),
                money::format(net),
                bb,
                csv(&ev)
            )?;
            n += 1;
        }
    } else if format == "hh" {
        let mut s=c.prepare(&format!("SELECT p.detail FROM hands h JOIN hand_payload p ON p.hand=h.id WHERE {pred} ORDER BY played_at,id"))?;
        for b in s.query_map(params_from_iter(vals.iter()), |r| r.get::<_, Vec<u8>>(0))? {
            let h: Hand = serde_json::from_slice(&unpack(&b?)?)?;
            writeln!(file, "{}\n\n", h.raw)?;
            n += 1;
        }
    } else {
        bail!("unsupported export format");
    }
    Ok(n)
}
fn csv(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.into()
    };
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub fn db_path(path: impl Into<PathBuf>) -> PathBuf {
    path.into()
}
