//! Shared, version-pinned bounded Coach/MCP queries.
use super::{facts, storage};
use anyhow::{ensure, Context, Result};
use duckdb::{params_from_iter, types::Value as SqlValue, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub bb_units: Option<i64>,
    pub position: Option<String>,
    pub opponent: Option<String>,
    pub street: Option<String>,
    pub pot_type: Option<String>,
    pub players: Option<u8>,
    pub hu_postflop: bool,
    pub effective_min: Option<f64>,
    pub effective_max: Option<f64>,
    pub bet_min: Option<f64>,
    pub bet_max: Option<f64>,
    pub board: Option<String>,
    pub texture: Option<String>,
    pub paired: Option<bool>,
    pub high_card: Option<String>,
    pub metric: Option<String>,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            date_from: None,
            date_to: None,
            bb_units: None,
            position: None,
            opponent: None,
            street: None,
            pot_type: None,
            players: None,
            hu_postflop: true,
            effective_min: None,
            effective_max: None,
            bet_min: None,
            bet_max: None,
            board: None,
            texture: None,
            paired: None,
            high_card: None,
            metric: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Args {
    pub dataset_id: String,
    pub version: Option<String>,
    pub filter: Filter,
    pub group: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub hand_key: Option<String>,
    pub seq: Option<u32>,
    pub min_opportunities: Option<u32>,
}
pub fn is_tool(name: &str) -> bool {
    [
        "get_mda_catalog",
        "query_mda_stats",
        "find_mda_candidates",
        "find_mda_spots",
        "get_mda_hand",
    ]
    .contains(&name)
}
pub fn definitions() -> Vec<Value> {
    let mut filter = serde_json::Map::new();
    for (key, v) in serde_json::to_value(Filter::default())
        .unwrap()
        .as_object()
        .unwrap()
    {
        let t = if ["bb_units", "players"].contains(&key.as_str()) {
            "integer"
        } else if key.starts_with("effective_") || key.starts_with("bet_") {
            "number"
        } else if v.is_boolean() || key == "paired" {
            "boolean"
        } else {
            "string"
        };
        filter.insert(key.clone(), json!({"type":t}));
    }
    [
        ("get_mda_catalog","Read ONE explicitly shared population dataset, version, source, stakes and opportunity definitions. HHDealer compatibility remains unverified."),
        ("query_mda_stats","Population numerators / legal opportunities. Separate stakes and streets. HU postflop default. Filters refer ONLY to information available at the decision; bet filters are facing-bet sizing."),
        ("find_mda_candidates","Research hypotheses with at least 200 opportunities, descriptive Wilson 95% and period consistency. No solver/GTO baseline. Report shown-card coverage and selection bias."),
        ("find_mda_spots","Bounded anonymous decision evidence. Follow next_cursor; identifiers are dataset-scoped opaque keys. No arbitrary SQL or player targeting."),
        ("get_mda_hand","Anonymous retrospective replay; optional seq returns ONLY pre-decision information, hiding future cards and outcomes."),
    ].into_iter().map(|(name,description)|{
        let mut p=serde_json::Map::from_iter([("dataset_id".into(),json!({"type":"string"})),("version".into(),json!({"type":"string"}))]);
        if !["get_mda_catalog","get_mda_hand"].contains(&name){p.insert("filter".into(),json!({"type":"object","properties":filter,"additionalProperties":false}));p.insert("limit".into(),json!({"type":"integer","minimum":1,"maximum":100}));p.insert("cursor".into(),json!({"type":"string"}));}
        if name=="query_mda_stats"||name=="find_mda_candidates" {p.insert("group".into(),json!({"type":"string","enum":["all","position","opponent","pot_type","texture","players"]}));}
        if name=="find_mda_candidates" {p.insert("min_opportunities".into(),json!({"type":"integer","minimum":200,"maximum":1000000}));}
        if name=="get_mda_hand" {p.insert("hand_key".into(),json!({"type":"string"}));p.insert("seq".into(),json!({"type":"integer","minimum":0}));}
        let required=if name=="get_mda_catalog" {json!(["dataset_id"])}else if name=="get_mda_hand" {json!(["dataset_id","version","hand_key"])}else{json!(["dataset_id","version"])};
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":p,"required":required,"additionalProperties":false}})
    }).collect()
}
pub fn validate(name: &str, args: &Value) -> Result<Args> {
    let defs = definitions();
    let def = defs
        .iter()
        .find(|t| t["name"] == name)
        .context("unknown MDA tool")?;
    let obj = args.as_object().context("arguments must be an object")?;
    for k in obj.keys() {
        ensure!(
            def["inputSchema"]["properties"].get(k).is_some(),
            "unsupported argument: {k}"
        );
    }
    for k in def["inputSchema"]["required"].as_array().unwrap() {
        ensure!(
            obj.get(k.as_str().unwrap()).is_some_and(|v| !v.is_null()),
            "required argument missing: {k}"
        );
    }
    let a: Args = serde_json::from_value(args.clone())?;
    ensure!(
        a.dataset_id.starts_with("mda-") && a.dataset_id.len() <= 80,
        "invalid dataset id"
    );
    ensure!(
        (1..=100).contains(&a.limit.unwrap_or(30)),
        "limit must be 1..100"
    );
    ensure!(
        (200..=1_000_000).contains(&a.min_opportunities.unwrap_or(200)),
        "minimum opportunities must be >=200"
    );
    ensure!(
        a.group.as_ref().is_none_or(|g| [
            "all", "position", "opponent", "pot_type", "texture", "players"
        ]
        .contains(&g.as_str())),
        "unsupported group"
    );
    ensure!(
        a.cursor.as_ref().is_none_or(|s| s.len() <= 80),
        "invalid cursor"
    );
    if let Some(k) = &a.hand_key {
        storage::parse_key(k)?;
    }
    let f = &a.filter;
    for p in [&f.position, &f.opponent].into_iter().flatten() {
        ensure!(
            ["SB", "BB", "UTG", "HJ", "CO", "BTN", "BTN/SB"].contains(&p.as_str()),
            "invalid position"
        );
    }
    ensure!(
        f.street
            .as_ref()
            .is_none_or(|s| ["preflop", "flop", "turn", "river"].contains(&s.as_str())),
        "invalid street"
    );
    ensure!(
        f.pot_type
            .as_ref()
            .is_none_or(|s| ["unopened", "limped", "SRP", "3-bet", "4-bet+"].contains(&s.as_str())),
        "invalid pot type"
    );
    ensure!(
        f.players.is_none_or(|n| (2..=6).contains(&n)) && f.bb_units.is_none_or(|n| n > 0),
        "invalid population / stakes"
    );
    for (lo, hi) in [(f.effective_min, f.effective_max), (f.bet_min, f.bet_max)] {
        ensure!(
            lo.is_none_or(|x| x.is_finite() && x >= 0.)
                && hi.is_none_or(|x| x.is_finite() && x >= 0.)
                && lo.zip(hi).is_none_or(|(a, b)| a <= b),
            "invalid numeric filter"
        );
    }
    if let Some(m) = &f.metric {
        ensure!(facts::bit(m).is_some(), "unknown metric");
    }
    if let Some(board) = &f.board {
        ensure!(
            board.len() <= 14
                && board.split_whitespace().all(|c| c.len() == 2
                    && "23456789TJQKA".contains(c.chars().next().unwrap())
                    && "cdhs".contains(c.chars().nth(1).unwrap())),
            "invalid board"
        );
    }
    ensure!(
        f.texture
            .as_ref()
            .is_none_or(|s| ["rainbow", "two-tone", "monotone"].contains(&s.as_str()))
            && f.high_card
                .as_ref()
                .is_none_or(|s| s.len() == 1 && "23456789TJQKA".contains(s)),
        "invalid board texture"
    );
    for d in [&f.date_from, &f.date_to].into_iter().flatten() {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")?;
    }
    ensure!(
        f.date_from
            .as_ref()
            .zip(f.date_to.as_ref())
            .is_none_or(|(lo, hi)| lo <= hi),
        "invalid date interval"
    );
    Ok(a)
}
pub fn catalog(c: &Connection) -> Result<Value> {
    let count: u64 = c.query_row("SELECT count(*) FROM hands", [], |r| r.get(0))?;
    let issues: u64 = c.query_row("SELECT count(*) FROM quarantine", [], |r| r.get(0))?;
    let stakes=c.prepare("SELECT bb,min(day),max(day),count(*) FROM hands GROUP BY bb ORDER BY bb")?.query_map([],|r|Ok(json!({"bb_units":r.get::<_,i64>(0)?,"date_min":r.get::<_,i32>(1)?,"date_max":r.get::<_,i32>(2)?,"hands":r.get::<_,u64>(3)?})))?.collect::<duckdb::Result<Vec<_>>>()?;
    Ok(
        json!({"dataset":storage::info(c)?,"version":storage::version(c)?,"hands":count,"quarantined":issues,"stakes":stakes,"metrics":facts::METRICS.iter().enumerate().map(|(i,(key,definition))|json!({"key":key,"bit":1u64<<i,"definition":definition})).collect::<Vec<_>>(),"unknown_cards":"unknown; never imputed","interval":"descriptive Wilson 95%; decisions from the same player/table are not independent","selection_bias":"shown cards are selected, not a random sample of complete ranges","limits":{"memory":"2GiB","threads":4,"page":100}}),
    )
}
fn where_clause(f: &Filter) -> Result<(String, Vec<SqlValue>)> {
    let mut w = vec!["TRUE".into()];
    let mut p = Vec::new();
    if f.hu_postflop && f.players.is_none() {
        w.push("(street='preflop' OR players=2)".into());
    }
    for (name, v) in [
        ("position", &f.position),
        ("opponent", &f.opponent),
        ("street", &f.street),
        ("pot_type", &f.pot_type),
        ("texture", &f.texture),
        ("high_card", &f.high_card),
        ("board", &f.board),
    ] {
        if let Some(v) = v {
            w.push(format!("{name}=?"));
            p.push(SqlValue::Text(v.clone()));
        }
    }
    for (name, v) in [
        ("bb", f.bb_units),
        ("players", f.players.map(i64::from)),
        ("paired", f.paired.map(i64::from)),
    ] {
        if let Some(v) = v {
            w.push(format!("{name}=?"));
            p.push(SqlValue::BigInt(v));
        }
    }
    for (name, op, v) in [
        ("effective_bb", ">=", f.effective_min),
        ("effective_bb", "<=", f.effective_max),
        ("facing_bet_pct", ">=", f.bet_min),
        ("facing_bet_pct", "<=", f.bet_max),
    ] {
        if let Some(v) = v {
            w.push(format!("{name}{op}?"));
            p.push(SqlValue::Double(v));
        }
    }
    for (op, d) in [(">=", &f.date_from), ("<=", &f.date_to)] {
        if let Some(d) = d {
            w.push(format!("day{op}?"));
            p.push(SqlValue::Int(d.replace('-', "").parse()?));
        }
    }
    if let Some(m) = &f.metric {
        w.push(format!("(opportunities & {})<>0", facts::bit(m).unwrap()));
    }
    Ok((w.join(" AND "), p))
}
pub fn stats(c: &Connection, a: &Args, candidates: bool) -> Result<Value> {
    let (w, p) = where_clause(&a.filter)?;
    let g = a.group.as_deref().unwrap_or("all");
    let group = if g == "all" { "''" } else { g };
    let detailed = a.filter.effective_min.is_some()
        || a.filter.effective_max.is_some()
        || a.filter.bet_min.is_some()
        || a.filter.bet_max.is_some()
        || a.filter.board.is_some();
    let (source, n, known, showdowns, shown) = if detailed {
        (
            "decisions",
            "1",
            "known_cards::INTEGER",
            "showdown::INTEGER",
            "(showdown AND known_cards)::INTEGER",
        )
    } else {
        ("rollups", "n", "known", "showdowns", "shown")
    };
    let offset = a.cursor.as_deref().unwrap_or("0").parse::<u32>()?;
    ensure!(offset <= 100_000, "cursor out of bounds");
    let limit = a.limit.unwrap_or(30);
    let metric_filter = a
        .filter
        .metric
        .as_ref()
        .map(|m| facts::METRICS.iter().position(|(n, _)| n == m).unwrap());
    let sql=format!("WITH selected AS (SELECT * FROM {source} WHERE {w}), expanded AS (SELECT bb,street,({group})::VARCHAR AS context,m.range::INTEGER AS metric,day,sum({n})::UBIGINT AS opportunities,sum(CASE WHEN (hits & (1::UBIGINT << m.range))<>0 THEN {n} ELSE 0 END)::UBIGINT AS hits,sum({known})::UBIGINT AS known,sum({showdowns})::UBIGINT AS showdowns,sum({shown})::UBIGINT AS shown FROM selected CROSS JOIN range({start},{end}) m WHERE (opportunities & (1::UBIGINT << m.range))<>0 GROUP BY bb,street,context,m.range,day), divided AS (SELECT *, min(day) OVER(PARTITION BY bb,street,context,metric) AS first_day,max(day) OVER(PARTITION BY bb,street,context,metric) AS last_day FROM expanded), split AS (SELECT *, strftime(make_timestamp(((epoch(strptime(first_day::VARCHAR, '%Y%m%d'))+epoch(strptime(last_day::VARCHAR, '%Y%m%d')))/2*1000000)::BIGINT), '%Y%m%d')::INTEGER AS cutoff FROM divided) SELECT bb,street,context,metric,sum(opportunities)::UBIGINT,sum(hits)::UBIGINT,sum(known)::UBIGINT,sum(showdowns)::UBIGINT,sum(shown)::UBIGINT,min(first_day),max(last_day),sum(CASE WHEN day<=cutoff THEN opportunities ELSE 0 END)::UBIGINT,sum(CASE WHEN day<=cutoff THEN hits ELSE 0 END)::UBIGINT,sum(CASE WHEN day>cutoff THEN opportunities ELSE 0 END)::UBIGINT,sum(CASE WHEN day>cutoff THEN hits ELSE 0 END)::UBIGINT FROM split GROUP BY bb,street,context,metric HAVING sum(opportunities)>={minimum} ORDER BY bb,street,context,metric LIMIT {plus} OFFSET {offset}",start=metric_filter.unwrap_or(0),end=metric_filter.map(|i|i+1).unwrap_or(facts::METRICS.len()),minimum=if candidates {a.min_opportunities.unwrap_or(200)}else{1},plus=limit+1);
    let mut rows=c.prepare(&sql)?.query_map(params_from_iter(p),|r|{
        let metric=r.get::<_,u32>(3)? as usize;let opportunities=r.get::<_,u64>(4)?;let hits=r.get::<_,u64>(5)?;
        let n1=r.get::<_,u64>(11)?;let k1=r.get::<_,u64>(12)?;let n2=r.get::<_,u64>(13)?;let k2=r.get::<_,u64>(14)?;
        let ci=crate::agent::wilson(hits as f64,opportunities as f64);
        let ci1=crate::agent::wilson(k1 as f64,n1 as f64);let ci2=crate::agent::wilson(k2 as f64,n2 as f64);
        let consistent=ci1.zip(ci2).map(|((l1,h1),(l2,h2))|l1<=h2&&l2<=h1);
        let showdown_count=r.get::<_,u64>(7)?;let shown_count=r.get::<_,u64>(8)?;
        Ok(json!({"metric":facts::METRICS[metric].0,"definition":facts::METRICS[metric].1,"bb_units":r.get::<_,i64>(0)?,"street":r.get::<_,String>(1)?,"context":r.get::<_,String>(2)?,"opportunities":opportunities,"numerator":hits,"pct":hits as f64/opportunities as f64*100.,"wilson95":ci,"known_cards":r.get::<_,u64>(6)?,"showdown_decisions":showdown_count,"shown_cards":shown_count,"showdown_coverage_pct":if showdown_count>0 {Some(shown_count as f64/showdown_count as f64*100.)}else{None},"periods":[{"opportunities":n1,"numerator":k1,"wilson95":ci1},{"opportunities":n2,"numerator":k2,"wilson95":ci2}],"date_min":r.get::<_,i32>(9)?,"date_max":r.get::<_,i32>(10)?,"period_interval_overlap":consistent}))
    })?.collect::<duckdb::Result<Vec<_>>>()?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    if candidates {
        for row in &mut rows {
            row["classification"] = json!("研究假設：缺少 GTO／牌池基準");
            row["adjustment"] = json!(adjustment(row["metric"].as_str().unwrap()));
        }
    }
    let bluff = if a.filter.street.as_deref() == Some("river")
        && a.filter.hu_postflop
        && a.filter.players.is_none_or(|p| p == 2)
        && a.filter
            .bet_min
            .zip(a.filter.bet_max)
            .is_some_and(|(a, b)| (a - b).abs() < 1e-9)
    {
        let b = a.filter.bet_min.unwrap() / 100.;
        Some(
            json!({"pure_bluff_break_even_fold_pct":b/(1.+b)*100.,"assumptions":["HU river","bluff zero showdown equity","opponent only folds/calls","fixed bet B against pre-bet pot P","no rake / raise / additional cost adjustment"],"use":"descriptive hypothesis only, not action EV"}),
        )
    } else {
        None
    };
    Ok(
        json!({"rows":rows,"next_cursor":if more{Some((offset+limit).to_string())}else{None},"filter":a.filter,"group":g,"source":source,"baseline":"none; research hypothesis","interval":"Wilson 95% descriptive; clustered player decisions, not causal proof","periods":"split observed source calendar-day range in half; one-period results have no consistency evidence","selection_bias":"known showdown cards are selected; coverage denominator is showdown decision opportunities, not complete ranges","river_bluff_model":bluff}),
    )
}
fn adjustment(metric: &str) -> &'static str {
    match metric {
    "fold_cbet"|"fold_bet"=>"若 fold 傾向經同情境案例確認：研究增加合適 bluff；value 可減少薄下注。尺寸須按面對的 bet/P 篩選另驗；不能把 fold% 直接當 EV。",
    "call_cbet"|"call_bet"=>"若 call 傾向經確認：研究增加 value／較薄 value，减少零 equity bluff；分尺寸、牌面及阻擋牌驗證。",
    "raise_cbet"|"raise_bet"|"check_raise"=>"研究對 raise 的防守範圍與較強 value；check-back／bluff 頻率須按尺寸、牌面和樣本另驗。",
    "rfi"|"three_bet"=>"檢視位置範圍結構，再研究盲位防守／4-bet value 與 blocker bluff；機會率不等於完整手牌範圍。",
    "blind_fold"|"fold_three_bet"=>"研究 open／3-bet bluff 的方向；需先核實位置、尺寸、有效籌碼及反擊成本，沒有基準不稱過度棄牌。",
    _=>"逐級別、位置、尺寸與牌面核實傾向；提出 value／bluff／防守調整及例外，再以匿名案例確認。",
}
}
pub fn spots(c: &Connection, a: &Args) -> Result<Value> {
    let (w, mut p) = where_clause(&a.filter)?;
    let (k, seq) = if let Some(cursor) = &a.cursor {
        let (k, s) = cursor.split_once(':').context("invalid spot cursor")?;
        (storage::parse_key(k)?, s.parse::<u32>()?)
    } else {
        (0, 0)
    };
    p.push(SqlValue::UBigInt(k));
    p.push(SqlValue::UBigInt(k));
    p.push(SqlValue::UInt(seq));
    let limit = a.limit.unwrap_or(20);
    let sql=format!("SELECT hand_key,seq,seat,bb,day,position,opponent,street,pot_type,players,board,action,pot_units,to_call_units,remaining_units,effective_bb,facing_bet_pct,opportunities,hits FROM decisions WHERE {w} AND (hand_key>? OR (hand_key=? AND seq>?)) ORDER BY hand_key,seq LIMIT {}",limit+1);
    let mut rows=c.prepare(&sql)?.query_map(params_from_iter(p),|r|Ok(json!({"hand_key":storage::hex_key(r.get::<_,u64>(0)?),"seq":r.get::<_,u32>(1)?,"seat":r.get::<_,u8>(2)?,"bb_units":r.get::<_,i64>(3)?,"day":r.get::<_,i32>(4)?,"position":r.get::<_,String>(5)?,"opponent":r.get::<_,String>(6)?,"street":r.get::<_,String>(7)?,"pot_type":r.get::<_,String>(8)?,"players":r.get::<_,u8>(9)?,"board":r.get::<_,String>(10)?,"action":r.get::<_,String>(11)?,"pot_units":r.get::<_,i64>(12)?,"to_call_units":r.get::<_,i64>(13)?,"remaining_units":r.get::<_,i64>(14)?,"effective_bb":r.get::<_,f64>(15)?,"facing_bet_pct":r.get::<_,Option<f64>>(16)?,"opportunities":r.get::<_,u64>(17)?,"hits":r.get::<_,u64>(18)?})))?.collect::<duckdb::Result<Vec<_>>>()?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next = if more {
        rows.last()
            .map(|r| format!("{}:{}", r["hand_key"].as_str().unwrap(), r["seq"]))
    } else {
        None
    };
    Ok(
        json!({"rows":rows,"next_cursor":next,"filter":a.filter,"decision_information":"pre-decision only; no future board or opponent cards"}),
    )
}
pub fn query(c: &Connection, name: &str, a: &Args) -> Result<Value> {
    match name {
        "get_mda_catalog" => catalog(c),
        "query_mda_stats" => stats(c, a, false),
        "find_mda_candidates" => stats(c, a, true),
        "find_mda_spots" => spots(c, a),
        "get_mda_hand" => {
            let k = storage::parse_key(a.hand_key.as_ref().context("hand key missing")?)?;
            let mut h = storage::anonymize(storage::hand(c, k)?, k);
            ensure!(h.actions.len() <= 300, "hand too large to share");
            if let Some(seq) = a.seq {
                ensure!(
                    h.actions.iter().any(|a| a.seq == seq as usize
                        && matches!(a.kind.as_str(), "fold" | "check" | "call" | "bet" | "raise")),
                    "decision not found"
                );
                h.actions.retain(|a| a.seq < seq as usize);
                if h.actions.iter().all(|a| a.street == "preflop") {
                    h.pot_type = match h.actions.iter().filter(|a| a.kind == "raise").count() {
                        0 => "unopened",
                        1 => "SRP",
                        2 => "3-bet",
                        _ => "4-bet+",
                    }
                    .into();
                }
                h.boards = vec![h
                    .actions
                    .iter()
                    .rfind(|a| a.kind == "board")
                    .map(|a| a.cards.clone())
                    .unwrap_or_default()];
                for p in &mut h.players {
                    p.cards.clear();
                }
                h.total_pot = 0;
                h.pots.clear();
                h.fees.clear();
                h.showdown = false;
                h.saw_flop = false;
                h.flop_players = 0;
                h.hu_effective_bb = None;
                h.texture.clear();
                h.paired = false;
                h.high_card.clear();
                h.cues.clear();
            }
            Ok(json!({"hand":h,"hand_key":a.hand_key,"seq":a.seq,"retrospective":a.seq.is_none()}))
        }
        _ => anyhow::bail!("unknown MDA tool"),
    }
}
