use super::{
    decision::{Decision, DecisionRef},
    *,
};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAction {
    pub code: String,
    pub label: String,
    pub kind: String,
    pub size_bb: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyNode {
    pub id: String,
    pub hero: String,
    pub path: String,
    pub source_url: String,
    pub actions: Vec<StrategyAction>,
    pub frequencies: BTreeMap<String, BTreeMap<String, f64>>,
    pub issues: BTreeMap<String, String>,
    pub reach: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPack {
    pub id: String,
    pub name: String,
    pub source: Value,
    pub imported_at: i64,
    pub files: usize,
    pub raw: BTreeMap<String, String>,
    pub nodes: Vec<StrategyNode>,
}

pub fn classes() -> Vec<String> {
    let ranks: Vec<_> = "AKQJT98765432".chars().collect();
    ranks
        .iter()
        .enumerate()
        .flat_map(|(i, a)| {
            ranks.iter().enumerate().map(move |(j, b)| {
                if i == j {
                    format!("{a}{b}")
                } else if i < j {
                    format!("{a}{b}s")
                } else {
                    format!("{b}{a}o")
                }
            })
        })
        .collect()
}

fn weights(text: &str) -> Result<BTreeMap<String, f64>> {
    let allowed: BTreeSet<_> = classes().into_iter().collect();
    let mut values = BTreeMap::new();
    for token in text.trim().split(',') {
        let (hand, raw) = token.trim().split_once(':').unwrap_or((token.trim(), "1"));
        let weight: f64 = raw.parse().context("Invalid range weight")?;
        anyhow::ensure!(
            allowed.contains(hand) && weight.is_finite() && weight > 0. && weight <= 1.,
            "Invalid range entry: {token}"
        );
        anyhow::ensure!(
            values.insert(hand.into(), weight).is_none(),
            "Duplicate range entry: {hand}"
        );
    }
    Ok(values)
}

fn action(label: &str) -> Result<StrategyAction> {
    let (code, kind, size) = if label == "Call" {
        ("C".into(), "call", None)
    } else if label == "Check" {
        ("X".into(), "check", None)
    } else if label == "Fold" {
        ("F".into(), "fold", None)
    } else if let Some(size) = label.strip_prefix("Raise ") {
        let n: f64 = size.parse()?;
        anyhow::ensure!(n.is_finite() && n > 1. && n <= 100., "Invalid raise size");
        (format!("R{n}"), "raise", Some(n))
    } else if label == "Allin 100" {
        ("RAI".into(), "raise", Some(100.))
    } else {
        bail!("Unsupported strategy action: {label}")
    };
    Ok(StrategyAction {
        code,
        label: label.into(),
        kind: kind.into(),
        size_bb: size,
    })
}

type RawNodes = BTreeMap<(String, String), Vec<(StrategyAction, BTreeMap<String, f64>, String)>>;

/// Returns the acting position, its previous action, and whether folding is legal.
fn ancestry(path: &str, hero: &str) -> Result<(Option<(String, String)>, bool)> {
    let seats = ["UTG", "HJ", "CO", "BTN", "SB", "BB"];
    let mut actor = 0;
    let mut skipped = BTreeSet::new();
    let mut prefix = vec![];
    let mut prior = None;
    let mut live = [0., 0., 0., 0., 0.5, 1.];
    let mut level: f64 = 1.;
    for code in path.split('-').filter(|t| !t.is_empty()) {
        if seats[actor] == hero {
            prior = Some((prefix.join("-"), code.into()));
        }
        match code {
            "F" => {
                skipped.insert(actor);
            }
            "C" => {
                live[actor] = level;
            }
            "X" => anyhow::ensure!(live[actor] == level, "Illegal source check"),
            "RAI" => {
                live[actor] = 100.;
                level = 100.;
                skipped.insert(actor);
            }
            _ => {
                let n: f64 = code
                    .strip_prefix('R')
                    .context("Invalid source path")?
                    .parse()?;
                anyhow::ensure!(n > level && n <= 100., "Invalid source raise");
                live[actor] = n;
                level = n;
            }
        }
        prefix.push(code);
        actor = (actor + 1) % 6;
        anyhow::ensure!(skipped.len() < 6, "No actor in source path");
        while skipped.contains(&actor) {
            actor = (actor + 1) % 6;
        }
    }
    anyhow::ensure!(
        seats[actor] == hero,
        "Source path actor does not match {hero}"
    );
    Ok((prior, level > live[actor]))
}

pub fn parse_nexus(path: &Path) -> Result<StrategyPack> {
    anyhow::ensure!(
        fs::metadata(path)?.len() <= 2_000_000,
        "Manifest is too large"
    );
    let manifest = fs::read_to_string(path)?;
    let m: Value = serde_json::from_str(&manifest)?;
    anyhow::ensure!(
        m["source"]["gametype"] == "Cash6mGeneral_6mNL10R25" && m["source"]["depthBb"] == 100,
        "This adapter requires the saved 6-max NL10 / 100bb Nexus manifest"
    );
    let files = m["files"].as_array().context("Missing range files")?;
    anyhow::ensure!(
        !files.is_empty() && files.len() <= 1000,
        "Invalid file count"
    );
    let root = path
        .parent()
        .context("Missing manifest directory")?
        .canonicalize()?;
    let mut nodes = RawNodes::new();
    let mut raw = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for f in files {
        let relative = f["relativePath"].as_str().context("Missing relativePath")?;
        let file = root.join(relative).canonicalize()?;
        anyhow::ensure!(
            file.starts_with(&root)
                && file.extension().is_some_and(|s| s == "txt")
                && fs::metadata(&file)?.len() <= 200_000,
            "Range path must be a small TXT within the selected directory"
        );
        let text = fs::read_to_string(&file)?;
        let range = weights(&text)?;
        anyhow::ensure!(
            Some(range.len() as u64) == f["hands"].as_u64(),
            "Range hand count mismatch: {relative}"
        );
        let hero = f["hero"]
            .as_str()
            .context("Missing source position")?
            .to_string();
        let source_path = f["sourcePath"]
            .as_str()
            .context("Missing action path")?
            .to_string();
        let a = action(f["action"].as_str().context("Missing action")?)?;
        anyhow::ensure!(
            seen.insert((source_path.clone(), hero.clone(), a.code.clone())),
            "Duplicate source action"
        );
        let url = f["sourceUrl"].as_str().unwrap_or("").to_string();
        anyhow::ensure!(
            url.starts_with("https://app.gtowizard.com/"),
            "Invalid source URL"
        );
        nodes
            .entry((source_path, hero))
            .or_default()
            .push((a, range, url));
        raw.insert(relative.to_string(), text);
    }
    anyhow::ensure!(
        Some(nodes.len() as u64) == m["estimate"]["decisionSpots"].as_u64(),
        "Decision node count mismatch"
    );
    let mut output = vec![];
    for ((path, hero), ranges) in &nodes {
        let (prior, fold_allowed) = ancestry(path, hero)?;
        let reach = if let Some((parent, code)) = prior {
            nodes
                .get(&(parent, hero.clone()))
                .and_then(|rs| rs.iter().find(|(a, _, _)| a.code == code))
                .map(|(_, r, _)| r.clone())
                .context("Missing parent action range")?
        } else {
            classes().into_iter().map(|h| (h, 1.)).collect()
        };
        let mut frequencies = BTreeMap::new();
        let mut issues = BTreeMap::new();
        let mut actions: Vec<_> = ranges.iter().map(|(a, _, _)| a.clone()).collect();
        if fold_allowed && !actions.iter().any(|a| a.code == "F") {
            actions.push(action("Fold")?);
        }
        for hand in classes() {
            let arrival = reach.get(&hand).copied().unwrap_or(0.);
            let sum: f64 = ranges
                .iter()
                .map(|(_, r, _)| r.get(&hand).copied().unwrap_or(0.))
                .sum();
            if arrival == 0. {
                if sum > 0. {
                    issues.insert(hand, "missing_arrival".into());
                }
                continue;
            }
            // Tolerance is floating representation only. Do not conceal rounding in
            // a source export by normalizing conditional probabilities over 100%.
            if sum > arrival + 1e-12
                || ((!fold_allowed || ranges.iter().any(|(a, _, _)| a.code == "F"))
                    && (sum - arrival).abs() > 1e-12)
            {
                issues.insert(hand, "source_weight_inconsistent".into());
                continue;
            }
            let mut freq: BTreeMap<String, f64> = ranges
                .iter()
                .map(|(a, r, _)| {
                    (
                        a.code.clone(),
                        r.get(&hand).copied().unwrap_or(0.) / arrival,
                    )
                })
                .collect();
            if fold_allowed && !freq.contains_key("F") {
                freq.insert("F".into(), ((arrival - sum) / arrival).max(0.));
            }
            frequencies.insert(hand, freq);
        }
        output.push(StrategyNode {
            id: format!("{hero}:{path}"),
            hero: hero.clone(),
            path: path.clone(),
            source_url: ranges[0].2.clone(),
            actions,
            frequencies,
            issues,
            reach,
        });
    }
    propagate_issues(&mut output)?;
    let id = store::fingerprint(&format!("{}{}", manifest, serde_json::to_string(&raw)?));
    Ok(StrategyPack {
        id,
        name: "GTO Wizard · NL10 · 6-max · 100bb".into(),
        source: m["source"].clone(),
        imported_at: chrono::Utc::now().timestamp(),
        files: files.len(),
        raw,
        nodes: output,
    })
}

// Re-run on stored packs too: raw values and immutable pack IDs stay unchanged.
fn propagate_issues(nodes: &mut [StrategyNode]) -> Result<()> {
    let mut order: Vec<_> = (0..nodes.len()).collect();
    order.sort_by_key(|&i| nodes[i].path.split('-').filter(|s| !s.is_empty()).count());
    for i in order {
        let (prior, _) = ancestry(&nodes[i].path, &nodes[i].hero)?;
        if let Some((path, _)) = prior {
            let parent = nodes
                .iter()
                .find(|n| n.path == path && n.hero == nodes[i].hero)
                .context("Missing parent strategy node")?;
            let issues: Vec<_> = parent.issues.keys().cloned().collect();
            for hand in issues {
                nodes[i].frequencies.remove(&hand);
                nodes[i]
                    .issues
                    .entry(hand)
                    .or_insert_with(|| "ancestor_weight_inconsistent".into());
            }
        }
    }
    Ok(())
}

/// Neither the export's model identifier nor a hand history verifies the rake
/// model. Until this adapter can compare independently verified model metadata,
/// historical comparisons remain reference-only. Numerical theory drills remain
/// available for cells whose entire arrival ancestry is internally consistent.
pub fn model_reasons(p: &StrategyPack) -> Vec<String> {
    let mut reasons = vec!["hand_model_unverified".into()];
    if p.source["treeId"]
        .as_str()
        .is_none_or(|s| s.trim().is_empty())
    {
        reasons.push("source_tree_unverified".into());
    }
    if p.source["rake"].is_null() {
        reasons.push("source_rake_unverified".into());
    }
    reasons
}

pub fn import(c: &Connection, path: &str) -> Result<Value> {
    let pack = parse_nexus(Path::new(path))?;
    c.execute(
        "INSERT OR IGNORE INTO study_packs VALUES(?1,?2)",
        params![pack.id, serde_json::to_string(&pack)?],
    )?;
    Ok(summary(&pack))
}
pub fn load(c: &Connection, id: &str) -> Result<StrategyPack> {
    let s: String = c.query_row("SELECT data FROM study_packs WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    let mut pack: StrategyPack = serde_json::from_str(&s)?;
    propagate_issues(&mut pack.nodes)?;
    Ok(pack)
}
fn summary(p: &StrategyPack) -> Value {
    json!({"id":p.id,"name":p.name,"source":p.source,"files":p.files,"nodes":p.nodes.len(),"ready_cells":p.nodes.iter().map(|n|n.frequencies.len()).sum::<usize>(),"reference_cells":p.nodes.iter().map(|n|n.issues.len()).sum::<usize>(),"imported_at":p.imported_at})
}
pub fn list(c: &Connection) -> Result<Value> {
    let mut s = c.prepare("SELECT data FROM study_packs ORDER BY id")?;
    let rows = s
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|r| {
            let mut p: StrategyPack = serde_json::from_str(&r?)?;
            propagate_issues(&mut p.nodes)?;
            Ok(summary(&p))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!(rows))
}
pub fn nodes(c: &Connection, id: &str) -> Result<Value> {
    let p = load(c, id)?;
    Ok(json!(p.nodes.iter().map(|n|json!({"id":n.id,"hero":n.hero,"path":n.path,"actions":n.actions,"ready_cells":n.frequencies.len(),"reference_cells":n.issues.len()})).collect::<Vec<_>>()))
}

pub fn mismatches(d: &Decision, node: &StrategyNode, pack: &StrategyPack) -> Vec<String> {
    let mut reasons = configuration_mismatches(d);
    reasons.extend(model_reasons(pack));
    if d.street != "preflop" {
        reasons.push("street".into());
    }
    if d.position != node.hero {
        reasons.push("position".into());
    }
    if d.preflop_path != node.path {
        reasons.push("action_path".into());
    }
    add_hand_reason(&mut reasons, &d.hand_class, node);
    reasons
}
fn configuration_mismatches(d: &Decision) -> Vec<String> {
    let mut reasons = vec![];
    if d.player_count != 6 {
        reasons.push("player_count".into());
    }
    if d.stacks_bb.len() != 6 || d.stacks_bb.values().any(|v| (v - 100.).abs() > 1e-8) {
        reasons.push("stack_depth".into());
    }
    if d.currency != "USD"
        || crate::money::parse(&d.bb).ok() != Some(100_000)
        || (d.sb_bb - 0.5).abs() > 1e-8
    {
        reasons.push("stakes".into());
    }
    if d.special {
        reasons.push("special_betting".into());
    }
    reasons
}
fn add_hand_reason(reasons: &mut Vec<String>, hand: &str, node: &StrategyNode) {
    if !node.frequencies.contains_key(hand) {
        reasons.push(
            node.issues
                .get(hand)
                .map(String::as_str)
                .unwrap_or("unreachable_hand")
                .into(),
        );
    }
}

/// Small, versioned index projection. Group before decoding so large databases
/// never deserialize a full decision snapshot for every preflop opportunity.
#[derive(Serialize, Deserialize)]
pub struct AggregateFacts {
    position: String,
    path: String,
    hand: String,
    reasons: Vec<String>,
    action: String,
    size: Option<f64>,
}
pub fn aggregate_facts(d: &Decision) -> Result<String> {
    if d.street != "preflop" {
        return Ok(String::new());
    }
    Ok(serde_json::to_string(&AggregateFacts {
        position: d.position.clone(),
        path: d.preflop_path.clone(),
        hand: d.hand_class.clone(),
        reasons: configuration_mismatches(d),
        action: d.action.clone(),
        size: d.size_bb,
    })?)
}
fn observed_code(kind: &str, size: Option<f64>, node: &StrategyNode) -> Option<String> {
    node.actions
        .iter()
        .find(|a| {
            a.kind == kind
                && (a.size_bb.is_none()
                    || a.size_bb
                        .zip(size)
                        .is_some_and(|(a, b)| (a - b).abs() < 1e-8))
        })
        .map(|a| a.code.clone())
}

pub fn action_code(d: &Decision, node: &StrategyNode) -> Option<String> {
    observed_code(&d.action, d.size_bb, node)
}

pub fn compare_decision(
    c: &Connection,
    pack: &str,
    r: &DecisionRef,
    node: Option<&str>,
) -> Result<Value> {
    let (_, d) = super::resolve(c, r)?;
    let p = load(c, pack)?;
    let n = if let Some(id) = node {
        p.nodes.iter().find(|n| n.id == id)
    } else {
        p.nodes
            .iter()
            .find(|n| n.hero == d.position && n.path == d.preflop_path)
    };
    let Some(n) = n else {
        return Ok(
            json!({"status":"unmatched","reasons":["action_path"],"decision":d,"source":p.source}),
        );
    };
    let reasons = mismatches(&d, n, &p);
    let status = if reasons.is_empty() {
        "matched"
    } else {
        "reference"
    };
    Ok(
        json!({"status":status,"reasons":reasons,"decision":d,"node":n,"source":p.source,"frequency":n.frequencies.get(&d.hand_class),"observed_action":action_code(&d,n)}),
    )
}

#[derive(Default, Serialize)]
struct Observed {
    opportunities: u64,
    matched: u64,
    actions: BTreeMap<String, u64>,
    matched_actions: BTreeMap<String, u64>,
    expected: BTreeMap<String, f64>,
    reference_expected: BTreeMap<String, f64>,
}

pub fn matrix(c: &Connection, id: &str, node: &str, filter: &Filter) -> Result<Value> {
    let pack = load(c, id)?;
    let node = pack
        .nodes
        .iter()
        .find(|n| n.id == node)
        .context("Unknown strategy node")?;
    let q = StudyQuery {
        filter: filter.clone(),
        spot: SpotDefinition {
            street: Some("preflop".into()),
            position: Some(node.hero.clone()),
            ..SpotDefinition::default()
        },
        ..StudyQuery::default()
    };
    let (clause, mut args) = super::predicate(&q)?;
    args.push(Sql::Text(node.path.clone()));
    let mut s=c.prepare(&format!("SELECT d.strategy_data,COUNT(*) FROM {FROM} WHERE {clause} AND d.preflop_path=? GROUP BY d.strategy_data"))?;
    let mut observed = BTreeMap::<String, Observed>::new();
    let mut reasons = BTreeMap::<String, u64>::new();
    let mut rows = s.query(params_from_iter(args))?;
    while let Some(row) = rows.next()? {
        let d: AggregateFacts = serde_json::from_str(&row.get::<_, String>(0)?)?;
        let count: u64 = row.get(1)?;
        let entry = observed.entry(d.hand.clone()).or_default();
        entry.opportunities += count;
        let code = observed_code(&d.action, d.size, node).unwrap_or_else(|| "off_tree".into());
        *entry.actions.entry(code.clone()).or_default() += count;
        if let Some(frequencies) = node.frequencies.get(&d.hand) {
            for (action, frequency) in frequencies {
                *entry.reference_expected.entry(action.clone()).or_default() +=
                    frequency * count as f64;
            }
        }
        let mut mismatch = d.reasons.clone();
        mismatch.extend(model_reasons(&pack));
        add_hand_reason(&mut mismatch, &d.hand, node);
        if mismatch.is_empty() {
            entry.matched += count;
            *entry.matched_actions.entry(code).or_default() += count;
            for (a, f) in &node.frequencies[&d.hand] {
                *entry.expected.entry(a.clone()).or_default() += f * count as f64;
            }
        } else {
            for reason in mismatch {
                *reasons.entry(reason).or_default() += count;
            }
        }
    }
    let cells:Vec<_>=classes().into_iter().map(|hand|json!({"hand":hand,"strategy":node.frequencies.get(&hand),"issue":node.issues.get(&hand),"observed":observed.get(&hand)})).collect();
    Ok(
        json!({"node":node,"source":pack.source,"cells":cells,"reasons":reasons,"coverage":super::coverage(c)?}),
    )
}

pub fn leaks(c: &Connection, filter: &Filter, id: &str) -> Result<Vec<Value>> {
    let p = load(c, id)?;
    let q = StudyQuery {
        filter: filter.clone(),
        spot: SpotDefinition {
            street: Some("preflop".into()),
            ..SpotDefinition::default()
        },
        ..StudyQuery::default()
    };
    let (clause, args) = super::predicate(&q)?;
    let mut s = c.prepare(&format!(
        "SELECT d.strategy_data,COUNT(*) FROM {FROM} WHERE {clause} GROUP BY d.strategy_data"
    ))?;
    let mut rows = s.query(params_from_iter(args))?;
    let map: BTreeMap<_, _> = p
        .nodes
        .iter()
        .map(|n| ((n.hero.as_str(), n.path.as_str()), n))
        .collect();
    let mut groups = BTreeMap::<String, Observed>::new();
    while let Some(row) = rows.next()? {
        let d: AggregateFacts = serde_json::from_str(&row.get::<_, String>(0)?)?;
        let count: u64 = row.get(1)?;
        let Some(n) = map.get(&(d.position.as_str(), d.path.as_str())) else {
            continue;
        };
        let mut reasons = d.reasons.clone();
        reasons.extend(model_reasons(&p));
        add_hand_reason(&mut reasons, &d.hand, n);
        if !reasons.is_empty() {
            continue;
        }
        let obs = groups.entry(n.id.clone()).or_default();
        obs.matched += count;
        *obs.matched_actions
            .entry(observed_code(&d.action, d.size, n).unwrap_or_else(|| "off_tree".into()))
            .or_default() += count;
        for (a, f) in &n.frequencies[&d.hand] {
            *obs.expected.entry(a.clone()).or_default() += f * count as f64;
        }
    }
    let mut result = vec![];
    for (node, obs) in groups {
        let n = p
            .nodes
            .iter()
            .find(|n| n.id == node)
            .context("Missing node")?;
        for code in obs
            .expected
            .keys()
            .chain(obs.matched_actions.keys())
            .collect::<BTreeSet<_>>()
        {
            let count = obs.matched_actions.get(code).copied().unwrap_or(0);
            let target = obs.expected.get(code).copied().unwrap_or(0.) / obs.matched as f64 * 100.;
            let actual = count as f64 / obs.matched as f64 * 100.;
            result.push(json!({"id":format!("{id}:{node}:{code}"),"name":format!("{} · {}",n.hero,n.path),"node":node,"pack":id,"source":"gto","note":p.name,"action":code,"actual":actual,"low":target,"high":target,"gap":actual-target,"opportunities":obs.matched,"hits":count,"interval":super::wilson(count,obs.matched),"enough":obs.matched>=100,"priority":if obs.matched>=100{(actual-target).abs()}else{0.}}));
        }
    }
    Ok(result)
}
