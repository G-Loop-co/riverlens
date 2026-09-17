// Shared, bounded agent tools. No model or transport dependencies.
use crate::{model::*, store};
use anyhow::{bail, ensure, Context, Result};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn init(c: &Connection) -> Result<()> {
    c.execute_batch("
      CREATE TABLE IF NOT EXISTS ai_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
      INSERT OR IGNORE INTO ai_meta VALUES('epoch',lower(hex(randomblob(16))));
      INSERT OR IGNORE INTO ai_meta VALUES('revision','0');
      CREATE TABLE IF NOT EXISTS ai_evidence(id TEXT PRIMARY KEY,version TEXT NOT NULL,tool TEXT NOT NULL,args TEXT NOT NULL,data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS ai_drafts(id TEXT PRIMARY KEY,kind TEXT NOT NULL,title TEXT NOT NULL,body TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'draft',revision INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL DEFAULT(unixepoch()));
      CREATE TABLE IF NOT EXISTS ai_attempts(id INTEGER PRIMARY KEY,draft TEXT NOT NULL REFERENCES ai_drafts(id),item INTEGER NOT NULL,answer TEXT NOT NULL,created_at INTEGER NOT NULL DEFAULT(unixepoch()));
      CREATE TABLE IF NOT EXISTS ai_strategy_config(profile TEXT PRIMARY KEY,rake_id TEXT NOT NULL,tree_id TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS ai_strategies(id TEXT PRIMARY KEY,data TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS ai_action_path(hand INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,seq INTEGER NOT NULL,street TEXT NOT NULL,position TEXT NOT NULL,kind TEXT NOT NULL,to_bb REAL NOT NULL,PRIMARY KEY(hand,seq));
      CREATE INDEX IF NOT EXISTS ai_path_lookup ON ai_action_path(street,position,kind,hand,seq);
      CREATE TABLE IF NOT EXISTS ai_indexed(hand INTEGER PRIMARY KEY REFERENCES hands(id) ON DELETE CASCADE);
      CREATE VIRTUAL TABLE IF NOT EXISTS ai_search USING fts5(kind UNINDEXED,ref UNINDEXED,body);
    ")?;
    for table in [
        "hands",
        "annotations",
        "tags",
        "ai_strategies",
        "ai_strategy_config",
    ] {
        for event in ["INSERT", "UPDATE", "DELETE"] {
            c.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS ai_rev_{table}_{event} AFTER {event} ON {table} BEGIN UPDATE ai_meta SET value=CAST(value AS INTEGER)+1 WHERE key='revision'; END;"))?;
        }
    }
    Ok(())
}
pub fn version(c: &Connection) -> Result<String> {
    Ok(c.query_row("SELECT (SELECT value FROM ai_meta WHERE key='epoch') || ':' || (SELECT value FROM ai_meta WHERE key='revision')",[],|r|r.get(0))?)
}
pub fn invalidate(c: &Connection) -> Result<()> {
    init(c)?;
    c.execute(
        "UPDATE ai_meta SET value=lower(hex(randomblob(16))) WHERE key='epoch'",
        [],
    )?;
    Ok(())
}
pub fn index_hand(c: &Connection, id: i64, h: &Hand) -> Result<()> {
    let mut insert =
        c.prepare_cached("INSERT OR REPLACE INTO ai_action_path VALUES(?1,?2,?3,?4,?5,?6)")?;
    for a in h
        .actions
        .iter()
        .filter(|a| ["fold", "check", "call", "bet", "raise"].contains(&a.kind.as_str()))
    {
        if let Some(player) = h.players.iter().find(|p| Some(p.seat) == a.actor) {
            insert.execute(params![
                id,
                a.seq,
                a.street,
                player.position,
                a.kind,
                a.to as f64 / h.bb.max(1) as f64
            ])?;
        }
    }
    c.execute("INSERT OR IGNORE INTO ai_indexed VALUES(?1)", [id])?;
    Ok(())
}
pub fn backfill(c: &Connection) -> Result<()> {
    let ids = c
        .prepare("SELECT id FROM hands WHERE id NOT IN (SELECT hand FROM ai_indexed)")?
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for batch in ids.chunks(250) {
        let tx = c.unchecked_transaction()?;
        for id in batch {
            index_hand(&tx, *id, &store::get_hand(&tx, *id)?)?;
        }
        tx.commit()?;
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Args {
    pub filter: Filter,
    pub group: Option<String>,
    pub stat: Option<String>,
    pub cursor: Option<Value>,
    pub limit: Option<u32>,
    pub id: Option<i64>,
    pub seq: Option<u32>,
    pub version: Option<String>,
    pub path: Vec<PathStep>,
    pub query: Option<String>,
    pub draft: Option<Draft>,
    pub pack: Option<String>,
    pub target: Option<Target>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathStep {
    pub street: String,
    pub position: String,
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub metric: String,
    pub min: f64,
    pub max: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeItem {
    pub hand: i64,
    pub seq: u32,
    pub prompt: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub id: String,
    pub title: String,
    pub text: String,
    pub evidence: Vec<String>,
    #[serde(default)]
    pub items: Vec<PracticeItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<Organization>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Organization {
    pub category: String,
    pub tags: Vec<String>,
    pub sections: Vec<LearningSection>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningSection {
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub tags: Vec<String>,
}
impl Organization {
    pub fn validate(&self, text: &str) -> Result<()> {
        fn tags_valid(tags: &[String]) -> bool {
            tags.len() <= 12 && tags.iter().all(|t| !t.trim().is_empty() && t.len() <= 100)
        }
        ensure!(
            !self.category.trim().is_empty()
                && self.category.len() <= 100
                && tags_valid(&self.tags),
            "invalid classification"
        );
        ensure!(
            !self.sections.is_empty() && self.sections.len() <= 40,
            "invalid section count"
        );
        let mut cursor = 0;
        let lines = text.split('\n').count();
        for section in &self.sections {
            ensure!(
                !section.title.trim().is_empty()
                    && section.title.len() <= 200
                    && tags_valid(&section.tags),
                "invalid section metadata"
            );
            ensure!(
                section.start == cursor && section.end > section.start && section.end <= lines,
                "sections must cover original text in order without gaps or overlaps"
            );
            cursor = section.end;
        }
        ensure!(cursor == lines, "sections must preserve all source lines");
        Ok(())
    }
}
fn schema(props: Value, required: Value) -> Value {
    json!({"type":"object","properties":props,"required":required,"additionalProperties":false})
}
pub fn tools() -> Value {
    let mut filter = serde_json::Map::new();
    for (k, _) in serde_json::to_value(Filter::default())
        .unwrap()
        .as_object()
        .unwrap()
    {
        let ty = if ["paired", "showdown", "reviewed"].contains(&k.as_str()) {
            "boolean"
        } else if ["player_count", "flop_players"].contains(&k.as_str()) {
            "integer"
        } else if k.ends_with("_min") || k.ends_with("_max") {
            "number"
        } else {
            "string"
        };
        filter.insert(k.clone(), json!({"type":[ty,"null"]}));
    }
    let f = json!({"type":"object","properties":filter,"additionalProperties":false});
    let step = schema(
        json!({"street":{"type":"string","enum":["preflop","flop","turn","river"]},"position":{"type":"string"},"kind":{"type":"string","enum":["fold","check","call","bet","raise"]}}),
        json!(["street", "position", "kind"]),
    );
    let item = schema(
        json!({"hand":{"type":"integer","minimum":1},"seq":{"type":"integer","minimum":0},"prompt":{"type":"string"}}),
        json!(["hand", "seq", "prompt"]),
    );
    let draft = schema(
        json!({"id":{"type":"string","description":"Stable idempotency key; reuse on retry"},"title":{"type":"string"},"text":{"type":"string"},"evidence":{"type":"array","items":{"type":"string"},"minItems":1},"items":{"type":"array","items":item}}),
        json!(["id", "title", "text", "evidence"]),
    );
    let defs=vec![
        ("get_capabilities","Start here: discover tools, product boundaries and sharing rules.",json!({}),json!([])),
        ("get_data_catalog","Read available dates, sample coverage and dataset version before analysis.",json!({}),json!([])),
        ("get_metric_definitions","Read opportunity denominators before interpreting statistics.",json!({}),json!([])),
        ("query_stats","Compute exact engine statistics. Filters use source canonical values; percentages have opportunity denominators.",json!({"filter":f,"group":{"type":"string"},"version":{"type":"string"}}),json!([])),
        ("query_matrix","Read the observed 13x13 starting-hand matrix. Observed hands are not a GTO range.",json!({"filter":f,"stat":{"type":"string"},"version":{"type":"string"}}),json!([])),
        ("list_saved_filters","Read saved filter definitions for reproducible analysis. Names are omitted unless note sharing is enabled.",json!({}),json!([])),
        ("find_spots","Find hands with an ordered action subsequence. Results are bounded; follow next_cursor. Never infer leaks from losses alone.",json!({"filter":f,"path":{"type":"array","items":step,"maxItems":8},"cursor":{"type":"integer"},"limit":{"type":"integer","minimum":1,"maximum":200},"version":{"type":"string"}}),json!([])),
        ("get_hand","Read an anonymized completed hand for retrospective review, not practice questions.",json!({"id":{"type":"integer","minimum":1},"version":{"type":"string"}}),json!(["id"])),
        ("get_decision_context","Get ONLY information available before a Hero decision. No future board, outcome, notes or opponent hole cards.",json!({"id":{"type":"integer","minimum":1},"seq":{"type":"integer","minimum":0},"version":{"type":"string"}}),json!(["id","seq"])),
        ("find_leak_candidates","Compare a declared custom percentage target with an engine metric using Wilson intervals. Without target return review candidates only.",json!({"filter":f,"target":schema(json!({"metric":{"type":"string"},"min":{"type":"number","minimum":0,"maximum":100},"max":{"type":"number","minimum":0,"maximum":100}}),json!(["metric","min","max"])),"version":{"type":"string"}}),json!([])),
        ("list_strategy_packs","Read installed strategy sources, validation status and matching configuration.",json!({}),json!([])),
        ("compare_preflop","Compare a Hero preflop decision with a verified exact strategy or explicitly reference-only data. Never return action EV without a solver.",json!({"id":{"type":"integer"},"seq":{"type":"integer"},"pack":{"type":"string"},"version":{"type":"string"}}),json!(["id","seq","pack"])),
        ("get_learning_progress","Read saved drafts and practice attempts; self-review scores are not GTO scores.",json!({}),json!([])),
        ("search_learning","Search local reports by text. User notes are only included if sharing is enabled.",json!({"query":{"type":"string"}}),json!(["query"])),
        ("create_report_draft","Save an editable evidence-linked report draft. Does not apply annotations.",json!({"draft":draft,"version":{"type":"string"}}),json!(["draft","version"])),
        ("create_practice_draft","Save practice references, not answers. Questions are served by engine with future information hidden.",json!({"draft":draft,"version":{"type":"string"}}),json!(["draft","version"])),
        ("create_study_plan_draft","Save an editable evidence-linked study plan for user acceptance.",json!({"draft":draft,"version":{"type":"string"}}),json!(["draft","version"])),
    ];
    json!(defs.into_iter().map(|(name,description,p,r)|json!({"name":name,"description":description,"inputSchema":schema(p,r)})).collect::<Vec<_>>())
}
pub fn validate(name: &str, v: &Value) -> Result<Args> {
    let ts = tools();
    let tool = ts
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .context("unknown agent tool")?;
    let obj = v.as_object().context("arguments must be an object")?;
    for key in obj.keys() {
        ensure!(
            tool["inputSchema"]["properties"].get(key).is_some(),
            "unsupported argument: {key}"
        );
    }
    for key in tool["inputSchema"]["required"].as_array().unwrap() {
        ensure!(
            obj.get(key.as_str().unwrap()).is_some_and(|v| !v.is_null()),
            "required argument missing: {key}"
        );
    }
    if let Some(f) = obj.get("filter") {
        let allowed = serde_json::to_value(Filter::default())?;
        for k in f.as_object().context("filter must be an object")?.keys() {
            ensure!(allowed.get(k).is_some(), "unknown filter: {k}");
        }
    }
    let a: Args = serde_json::from_value(v.clone())?;
    ensure!(
        a.limit.unwrap_or(60) > 0 && a.limit.unwrap_or(60) <= 200,
        "limit must be 1..200"
    );
    ensure!(a.path.len() <= 8, "path too long");
    ensure!(
        a.draft.as_ref().is_none_or(|d| d.evidence.len() <= 50),
        "too many evidence references"
    );
    ensure!(a.id.is_none_or(|i| i > 0), "invalid hand id");
    Ok(a)
}
pub fn call(c: &mut Connection, name: &str, input: Value, share_notes: bool) -> Result<Value> {
    let a = validate(name, &input)?;
    let tx = c.transaction()?;
    let ver = version(&tx)?;
    if let Some(expected) = &a.version {
        ensure!(
            expected == &ver,
            "dataset_changed: refresh catalog and evidence"
        );
    }
    let data = match name {
        "get_capabilities" => {
            json!({"tools":tools(),"scope":"personal post-session","administration":"not exposed","raw_sharing":false,"notes_sharing":share_notes,"gto":"exact validated configurations only","postflop":"self-review; no action EV solver",
          "product_features":{"imports":"desktop only","filters":"available","statistics":"available","matrix":"available","replay":"available","annotations":"read with sharing consent; edit in desktop","backup_restore":"desktop only","all_in_equity":"available; not decision EV","strategy_comparison":"available","learning":"drafts and progress","live_play":"unsupported"}})
        }
        "get_data_catalog" => {
            let coverage:Value=tx.query_row("SELECT count(*),min(played_at),max(played_at),sum(status='valid') FROM hands",[],|r|Ok(json!({"hands":r.get::<_,i64>(0)?,"from":r.get::<_,Option<i64>>(1)?,"to":r.get::<_,Option<i64>>(2)?,"valid":r.get::<_,Option<i64>>(3)?})))?;
            let mut values = serde_json::Map::new();
            for field in [
                "profile", "game", "stakes", "position", "currency", "pot_type",
            ] {
                let v = tx
                    .prepare(&format!(
                        "SELECT DISTINCT {field} FROM hands ORDER BY {field} LIMIT 1001"
                    ))?
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                values.insert(field.into(),json!({"values":v.iter().take(1000).collect::<Vec<_>>(),"truncated":v.len()>1000}));
            }
            json!({"values":values,"coverage":coverage,"filter_fields":serde_json::to_value(Filter::default())?,"money":"Hand and decision amounts are decimal currency strings; explicitly named *_units fields are integer micro-units (1 currency unit = 1000000 micro-units).","stats_version":STATS_VERSION})
        }
        "get_metric_definitions" => json!(STAT_DEFINITIONS
            .iter()
            .map(|(id, label, definition)| json!({"id":id,"label":label,"definition":definition}))
            .collect::<Vec<_>>()),
        "query_stats" => store::report(&tx, &a.filter, a.group.as_deref().unwrap_or("position"))?,
        "query_matrix" => store::matrix(&tx, &a.filter, a.stat.as_deref().unwrap_or("hands"))?,
        "list_saved_filters" => {
            let mut filters = store::saved_filters(&tx)?;
            if !share_notes {
                for f in filters.as_array_mut().unwrap() {
                    f["name"] = json!(format!("Filter {}", f["id"]));
                }
            }
            filters
        }
        "find_spots" => find_spots(&tx, &a)?,
        "get_hand" => {
            let mut h = serde_json::to_value(store::get_hand(&tx, a.id.unwrap())?)?;
            for key in ["raw", "table_name", "profile", "local_time"] {
                h.as_object_mut().unwrap().remove(key);
            }
            for p in h["players"].as_array_mut().unwrap() {
                p["name"] = json!(if p["hero"] == true {
                    "Hero".to_string()
                } else {
                    format!("Seat {}", p["seat"])
                });
            }
            if share_notes {
                h["annotation"] = json!(store::annotation(&tx, a.id.unwrap())?);
            }
            h
        }
        "get_decision_context" => decision(&tx, a.id.unwrap(), a.seq.unwrap())?,
        "find_leak_candidates" => {
            let report = store::report(&tx, &a.filter, "position")?;
            let mut result = json!({"report":report,"status":"review_only","reason":"No declared target; losses do not establish a leak"});
            if let Some(t) = a.target {
                ensure!(
                    t.min.is_finite()
                        && t.max.is_finite()
                        && t.min >= 0.
                        && t.max <= 100.
                        && t.min <= t.max,
                    "invalid target"
                );
                let stat = report["stats"]
                    .as_array()
                    .context("stats missing")?
                    .iter()
                    .find(|s| s["id"] == t.metric)
                    .context("unknown metric")?;
                let n = stat["opportunities"].as_f64().unwrap_or(0.);
                let k = stat["numerator"].as_f64().unwrap_or(0.);
                let ci = wilson(k, n);
                result = json!({"report":report,"target":t,"interval_95":ci,"status":if ci.is_some_and(|(lo,hi)|hi<t.min||lo>t.max) {"candidate"} else {"insufficient_evidence"},"basis":"custom target; not solver EV"});
            }
            result
        }
        "list_strategy_packs" => strategy_list(&tx)?,
        "compare_preflop" => crate::strategy::compare(
            &tx,
            a.id.unwrap(),
            a.seq.unwrap(),
            a.pack.as_deref().unwrap(),
        )?,
        "get_learning_progress" => learning(&tx)?,
        "search_learning" => {
            let q = a.query.as_deref().unwrap();
            ensure!(q.len() <= 500, "query too long");
            tx.execute("DELETE FROM ai_search", [])?;
            tx.execute(
                "INSERT INTO ai_search SELECT 'draft',id,title || ' ' || body FROM ai_drafts",
                [],
            )?;
            if share_notes {
                tx.execute(
                    "INSERT INTO ai_search SELECT 'note',CAST(hand AS TEXT),note FROM annotations",
                    [],
                )?;
                tx.execute(
                    "INSERT INTO ai_search SELECT 'tag',CAST(hand AS TEXT),tag FROM tags",
                    [],
                )?;
            }
            let rows=tx.prepare("SELECT kind,ref,snippet(ai_search,2,'','', ' … ',32) FROM ai_search WHERE ai_search MATCH ?1 LIMIT 50")?
                .query_map([format!("\"{}\"",q.replace('"',"\"\""))],|r|Ok(json!({"kind":r.get::<_,String>(0)?,"ref":r.get::<_,String>(1)?,"text":r.get::<_,String>(2)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            json!({"rows":rows,"limit":50})
        }
        "create_report_draft" | "create_practice_draft" | "create_study_plan_draft" => {
            let d = a.draft.context("draft required")?;
            validate_draft(&tx, &d, &ver, name == "create_practice_draft")?;
            let body = serde_json::to_string(&d)?;
            let existing: Option<String> = tx
                .query_row("SELECT body FROM ai_drafts WHERE id=?1", [&d.id], |r| {
                    r.get(0)
                })
                .optional()?;
            if let Some(old) = existing {
                ensure!(old == body, "idempotency_conflict: use a new draft id");
            } else {
                tx.execute(
                    "INSERT INTO ai_drafts(id,kind,title,body) VALUES(?1,?2,?3,?4)",
                    params![d.id, name, d.title, body],
                )?;
            }
            json!({"id":d.id,"status":"draft","applied":false})
        }
        _ => bail!("unsupported tool"),
    };
    let id = format!(
        "e-{:x}",
        Sha256::digest(serde_json::to_vec(&json!([ver, name, input, data]))?)
    );
    tx.execute(
        "INSERT OR IGNORE INTO ai_evidence VALUES(?1,?2,?3,?4,?5)",
        params![id, ver, name, input.to_string(), data.to_string()],
    )?;
    tx.commit()?;
    Ok(json!({"evidence_id":id,"version":ver,"tool":name,"args":input,"data":data}))
}
pub fn wilson(k: f64, n: f64) -> Option<(f64, f64)> {
    if n <= 0. {
        return None;
    }
    let z = 1.96;
    let p = k / n;
    let d = 1. + z * z / n;
    let center = (p + z * z / (2. * n)) / d;
    let margin = z * ((p * (1. - p) + z * z / (4. * n)) / n).sqrt() / d;
    Some(((center - margin) * 100., (center + margin) * 100.))
}
fn find_spots(c: &Connection, a: &Args) -> Result<Value> {
    let (mut pred, mut values) = store::predicate(&a.filter, true)?;
    let mut parent = "h.id".to_string();
    for (i, p) in a.path.iter().enumerate() {
        ensure!(
            ["preflop", "flop", "turn", "river"].contains(&p.street.as_str()),
            "invalid street"
        );
        ensure!(
            ["fold", "check", "call", "bet", "raise"].contains(&p.kind.as_str()),
            "invalid action"
        );
        let alias = format!("a{i}");
        pred.push_str(&format!(" AND EXISTS(SELECT 1 FROM ai_action_path {alias} WHERE {alias}.hand={parent} AND {alias}.street=? AND {alias}.position=? AND {alias}.kind=?"));
        if i > 0 {
            pred.push_str(&format!(" AND {alias}.seq>a{}.seq", i - 1));
        }
        values.extend([
            p.street.clone().into(),
            p.position.clone().into(),
            p.kind.clone().into(),
        ]);
        parent = format!("{alias}.hand");
    }
    pred.push_str(&")".repeat(a.path.len()));
    if let Some(cursor) = &a.cursor {
        let cursor = cursor.as_i64().context("cursor must be integer")?;
        pred.push_str(" AND h.id<?");
        values.push(cursor.into());
    }
    let limit = a.limit.unwrap_or(60) as usize;
    let mut rows=c.prepare(&format!("SELECT h.id,h.position,h.hand_class,h.played_at FROM hands h WHERE {pred} ORDER BY h.id DESC LIMIT {}",limit+1))?
        .query_map(params_from_iter(values),|r|Ok(json!({"id":r.get::<_,i64>(0)?,"position":r.get::<_,String>(1)?,"hand_class":r.get::<_,String>(2)?,"played_at":r.get::<_,i64>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let more = rows.len() > limit;
    rows.truncate(limit);
    let next = if more {
        rows.last().map(|r| r["id"].clone())
    } else {
        None
    };
    Ok(
        json!({"rows":rows,"next_cursor":next,"truncated":more,"ordering":"id_desc","path_semantics":"ordered subsequence"}),
    )
}
pub fn decision(c: &Connection, id: i64, seq: u32) -> Result<Value> {
    let h = store::get_hand(c, id)?;
    let a = h
        .actions
        .iter()
        .find(|a| a.seq == seq as usize)
        .context("decision not found")?;
    ensure!(
        h.status == "valid"
            && a.actor == Some(h.hero_seat)
            && ["fold", "check", "call", "bet", "raise"].contains(&a.kind.as_str()),
        "must reference a valid Hero decision"
    );
    let actions: Vec<_> = h
        .actions
        .iter()
        .filter(|x| {
            x.seq < seq as usize && !["show", "collect", "cashout"].contains(&x.kind.as_str())
        })
        .collect();
    Ok(
        json!({"hand":id,"seq":seq,"street":a.street,"position":h.position,"sb":crate::money::format(h.sb),"bb":crate::money::format(h.bb),
        "players":h.players.iter().map(|p|json!({"seat":p.seat,"position":p.position,"stack":crate::money::format(p.stack),"hero":p.hero,"cards":if p.hero {p.cards.clone()}else{vec![]}})).collect::<Vec<_>>(),
        "actions":actions,"notice":"Decision-before view. Outcomes, future cards and opponent cards omitted."}),
    )
}
pub fn validate_draft(c: &Connection, d: &Draft, ver: &str, practice: bool) -> Result<()> {
    ensure!(
        !d.id.is_empty()
            && d.id.len() <= 100
            && !d.title.is_empty()
            && d.title.len() <= 500
            && d.text.len() <= 30000,
        "invalid draft fields"
    );
    ensure!(
        !d.evidence.is_empty() && d.evidence.len() <= 50 && d.items.len() <= 100,
        "invalid draft bounds"
    );
    if let Some(o) = &d.organization {
        o.validate(&d.text)?;
    }
    for id in &d.evidence {
        let v: String = c
            .query_row("SELECT version FROM ai_evidence WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .context("unknown evidence")?;
        ensure!(v == ver, "stale evidence");
    }
    ensure!(
        !practice || !d.items.is_empty(),
        "practice needs decision references"
    );
    for item in &d.items {
        decision(c, item.hand, item.seq)?;
    }
    Ok(())
}
pub fn learning(c: &Connection) -> Result<Value> {
    let drafts=c.prepare("SELECT id,kind,title,body,status,revision FROM ai_drafts ORDER BY created_at DESC LIMIT 100")?
        .query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"body":serde_json::from_str::<Value>(&r.get::<_,String>(3)?).unwrap_or(Value::Null),"status":r.get::<_,String>(4)?,"revision":r.get::<_,i64>(5)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let attempts=c.prepare("SELECT draft,item,answer,created_at FROM ai_attempts ORDER BY id DESC LIMIT 200")?
        .query_map([],|r|Ok(json!({"draft":r.get::<_,String>(0)?,"item":r.get::<_,i64>(1)?,"answer":r.get::<_,String>(2)?,"created_at":r.get::<_,i64>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(json!({"drafts":drafts,"attempts":attempts,"draft_limit":100,"attempt_limit":200}))
}
pub fn strategy_list(c: &Connection) -> Result<Value> {
    let rows = c
        .prepare("SELECT data FROM ai_strategies ORDER BY id")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(json!(rows
        .iter()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .map(|mut v| {
            v.as_object_mut().unwrap().remove("nodes");
            v
        })
        .collect::<Vec<_>>()))
}
