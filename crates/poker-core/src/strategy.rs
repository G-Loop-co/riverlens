// Source weights are preserved. Nexus reach weights are never silently normalized.
use crate::{agent, store};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

fn weights(text: &str) -> Result<BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    for item in text.trim().split(',').filter(|s| !s.trim().is_empty()) {
        let (hand, w) = item.trim().split_once(':').unwrap_or((item.trim(), "1"));
        let w: f64 = w.parse()?;
        ensure!(
            w.is_finite() && (0.0..=1.0).contains(&w),
            "invalid range weight"
        );
        ensure!(hand.len() == 2 || hand.len() == 3, "invalid hand class");
        ensure!(
            out.insert(hand.to_string(), w).is_none(),
            "duplicate hand class"
        );
    }
    Ok(out)
}
pub fn import(c: &Connection, path: &Path) -> Result<Value> {
    ensure!(path.is_file(), "select a strategy manifest JSON");
    ensure!(
        std::fs::metadata(path)?.len() <= 10_000_000,
        "manifest too large"
    );
    let m: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    if m["format"] == "riverlens.strategy/1" {
        return import_verified(c, m);
    }
    let files = m["files"]
        .as_array()
        .context("Nexus manifest requires files")?;
    ensure!(files.len() <= 10000, "too many range files");
    let root = path
        .parent()
        .context("manifest directory missing")?
        .canonicalize()?;
    let mut nodes = BTreeMap::<String, Value>::new();
    for f in files {
        let rel = f["relativePath"].as_str().context("relativePath missing")?;
        let file = root.join(rel).canonicalize()?;
        ensure!(
            file.starts_with(&root),
            "range file escapes manifest directory"
        );
        ensure!(
            std::fs::metadata(&file)?.len() <= 1_000_000,
            "range file too large"
        );
        let w = weights(&std::fs::read_to_string(file)?)?;
        let source_path = f["sourcePath"].as_str().context("sourcePath missing")?;
        let hero = f["hero"].as_str().context("hero missing")?;
        let key = format!("{hero}|{source_path}");
        let node = nodes.entry(key).or_insert_with(
            || json!({"hero":hero,"path":source_path,"actions":[],"validation":"reference_only"}),
        );
        node["actions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"action":f["action"],"weights":w,"source":f["sourceUrl"]}));
    }
    // Parent range is the player's previous action reach, not all players' reach.
    let copy = nodes.clone();
    let mut invalid = 0;
    for node in nodes.values_mut() {
        let hero = node["hero"].as_str().unwrap();
        let path = node["path"].as_str().unwrap();
        let parent = copy
            .values()
            .filter(|n| n["hero"] == hero)
            .flat_map(|n| n["actions"].as_array().unwrap().iter().map(move |a| (n, a)))
            .filter(|(n, a)| {
                let prefix = format!(
                    "{}{}{}",
                    n["path"].as_str().unwrap(),
                    if n["path"] == "" { "" } else { "-" },
                    action_token(a["action"].as_str().unwrap_or(""))
                );
                path == prefix || path.starts_with(&(prefix + "-"))
            })
            .max_by_key(|(n, _)| n["path"].as_str().unwrap().len());
        let mut overflow = 0.0f64;
        let mut sums = BTreeMap::<String, f64>::new();
        for a in node["actions"].as_array().unwrap() {
            for (h, w) in a["weights"].as_object().unwrap() {
                *sums.entry(h.clone()).or_default() += w.as_f64().unwrap();
            }
        }
        for (h, sum) in sums {
            let reach = parent.map_or(1., |(_, a)| a["weights"][&h].as_f64().unwrap_or(0.));
            overflow = overflow.max(sum - reach);
        }
        node["max_absolute_overflow"] = json!(overflow);
        node["weights_consistent"] = json!(overflow <= 0.00001);
        if overflow > 0.00001 {
            invalid += 1;
        }
    }
    let id = m["source"]["gametype"]
        .as_str()
        .context("source gametype missing")?;
    let data = json!({"id":id,"source":m["source"],"nodes":nodes,"node_count":copy.len(),"invalid_nodes":invalid,
        "validation":"reference_only","reason":"Imported reach weights; independent source verification and actual rake configuration are not available. Exact scoring disabled."});
    c.execute(
        "INSERT INTO ai_strategies VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![id, data.to_string()],
    )?;
    agent::strategy_list(c)
}
fn action_token(action: &str) -> String {
    if action.starts_with("Allin") {
        "RAI".into()
    } else if action.starts_with("Raise ") {
        format!("R{}", action.trim_start_matches("Raise "))
    } else if action == "Call" {
        "C".into()
    } else if action == "Check" {
        "X".into()
    } else {
        "F".into()
    }
}
pub fn compare(c: &Connection, id: i64, seq: u32, pack: &str) -> Result<Value> {
    agent::decision(c, id, seq)?;
    let h = store::get_hand(c, id)?;
    let action = h
        .actions
        .iter()
        .find(|a| a.seq == seq as usize)
        .context("decision missing")?;
    ensure!(
        action.street == "preflop",
        "preflop comparison only; postflop is self-review"
    );
    let raw: String = c
        .query_row("SELECT data FROM ai_strategies WHERE id=?1", [pack], |r| {
            r.get(0)
        })
        .context("strategy pack not installed")?;
    let p: Value = serde_json::from_str(&raw)?;
    let path = h
        .actions
        .iter()
        .filter(|a| a.seq < seq as usize && a.street == "preflop")
        .filter_map(|a| match a.kind.as_str() {
            "fold" => Some("F".into()),
            "call" => Some("C".into()),
            "check" => Some("X".into()),
            "raise" => Some(if a.all_in {
                "RAI".into()
            } else {
                format!("R{}", a.to as f64 / h.bb as f64)
            }),
            _ => None,
        })
        .collect::<Vec<String>>()
        .join("-");
    let key = format!("{}|{}", h.position, path);
    let node = &p["nodes"][&key];
    if node.is_null() {
        return Ok(
            json!({"status":"unmatched","path":path,"reason":"No matching position and action path","counts_in_deviation":false}),
        );
    }
    if p["format"] == "riverlens.strategy/1" {
        let cfg = &p["configuration"];
        let actual_cfg: Option<(String, String)> = c
            .query_row(
                "SELECT rake_id,tree_id FROM ai_strategy_config WHERE profile=?1",
                [&h.profile],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        let mut reasons = vec![];
        if actual_cfg.as_ref().is_none_or(|(r, t)| {
            Some(r.as_str()) != cfg["rake_id"].as_str()
                || Some(t.as_str()) != cfg["tree_id"].as_str()
        }) {
            reasons.push("rake/tree configuration not confirmed");
        }
        if h.max_seats as u64 != cfg["max_seats"].as_u64().unwrap_or(0)
            || h.player_count as u64 != cfg["max_seats"].as_u64().unwrap_or(0)
        {
            reasons.push("table configuration mismatch");
        }
        if h.currency != cfg["currency"].as_str().unwrap_or("")
            || h.bb.to_string() != cfg["bb_units"].as_str().unwrap_or("")
            || h.game != cfg["game"].as_str().unwrap_or("")
        {
            reasons.push("game or stakes mismatch");
        }
        let depth = cfg["depth_bb"].as_f64().unwrap_or(0.);
        if h.players
            .iter()
            .any(|p| (p.stack as f64 / h.bb as f64 - depth).abs() > 0.000001)
        {
            reasons.push("stack configuration mismatch");
        }
        if h.actions.iter().any(|a| {
            a.seq < seq as usize && ["ante", "straddle", "dead_blind"].contains(&a.kind.as_str())
        }) {
            reasons.push("unsupported forced bet");
        }
        let frequencies:Vec<_>=node["actions"].as_array().unwrap().iter().map(|a|json!({"action":a["action"],"frequency":a["weights"][&h.hand_class].as_f64().unwrap_or(0.)})).collect();
        let total: f64 = frequencies
            .iter()
            .map(|a| a["frequency"].as_f64().unwrap())
            .sum();
        if (total - 1.).abs() > 0.000001 {
            reasons.push("hand absent from verified node");
        }
        let observed = match action.kind.as_str() {
            "raise" => {
                if action.all_in {
                    "Allin".to_string()
                } else {
                    format!("Raise {}", action.to as f64 / h.bb as f64)
                }
            }
            "call" => "Call".into(),
            "check" => "Check".into(),
            _ => "Fold".into(),
        };
        let chosen = frequencies
            .iter()
            .find(|a| a["action"] == observed)
            .and_then(|a| a["frequency"].as_f64())
            .unwrap_or(0.);
        return Ok(
            json!({"status":if reasons.is_empty(){"exact"}else{"reference_only"},"counts_in_deviation":reasons.is_empty(),
            "hand_class":h.hand_class,"frequencies":frequencies,"observed_action":observed,"chosen_frequency":chosen,
            "off_strategy":reasons.is_empty()&&chosen==0.,"reasons":reasons,"source":p["source"],
            "notice":"Mixed strategies are not graded by selecting only the highest-frequency action. No action EV loss is inferred."}),
        );
    }
    let weights=node["actions"].as_array().unwrap().iter().map(|a|json!({"action":a["action"],"source_weight":a["weights"][&h.hand_class].as_f64().unwrap_or(0.),"source":a["source"]})).collect::<Vec<_>>();
    Ok(
        json!({"status":"reference_only","counts_in_deviation":false,"hand_class":h.hand_class,"path":path,
        "source":p["source"],"weights":weights,"weights_consistent":node["weights_consistent"],
        "reason":p["reason"],"warning":"Reach weights are not conditional action frequencies; no normalization or action EV loss is reported."}),
    )
}

fn import_verified(c: &Connection, m: Value) -> Result<Value> {
    let id = m["id"].as_str().context("pack id missing")?;
    ensure!(!id.is_empty() && id.len() <= 100, "invalid pack id");
    let cfg = &m["configuration"];
    for field in ["rake_id", "tree_id", "currency", "bb_units", "game"] {
        ensure!(
            cfg[field].as_str().is_some_and(|s| !s.is_empty()),
            "configuration field missing: {field}"
        );
    }
    ensure!(
        cfg["max_seats"]
            .as_u64()
            .is_some_and(|n| (2..=9).contains(&n)),
        "invalid max_seats"
    );
    ensure!(
        cfg["depth_bb"]
            .as_f64()
            .is_some_and(|n| n > 0. && n.is_finite()),
        "invalid stack depth"
    );
    ensure!(
        m["source"]["verification_reference"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "independent verification reference required"
    );
    ensure!(
        m["source"]["url"]
            .as_str()
            .is_some_and(|s| s.starts_with("https://")),
        "source URL required"
    );
    let nodes = m["nodes"]
        .as_object()
        .context("nodes must map position|path to verified conditional strategies")?;
    ensure!(
        !nodes.is_empty() && nodes.len() <= 10000,
        "invalid node count"
    );
    for (key, n) in nodes {
        ensure!(
            n["hero"].as_str().is_some() && n["path"].as_str().is_some(),
            "node identity missing"
        );
        ensure!(
            key == &format!(
                "{}|{}",
                n["hero"].as_str().unwrap(),
                n["path"].as_str().unwrap()
            ),
            "node key mismatch"
        );
        let actions = n["actions"].as_array().context("node actions missing")?;
        ensure!(
            !actions.is_empty() && actions.len() <= 20,
            "invalid actions"
        );
        let mut sums = BTreeMap::<String, f64>::new();
        let mut seen = std::collections::BTreeSet::new();
        for a in actions {
            let name = a["action"].as_str().context("action missing")?;
            ensure!(seen.insert(name), "duplicate action");
            ensure!(
                ["Fold", "Call", "Check", "Allin"].contains(&name)
                    || name
                        .strip_prefix("Raise ")
                        .is_some_and(|n| n.parse::<f64>().is_ok_and(|v| v > 0. && v.is_finite())),
                "invalid action"
            );
            for (hand, w) in a["weights"].as_object().context("weights missing")? {
                let w = w.as_f64().context("numeric frequency required")?;
                ensure!(
                    w.is_finite() && (0.0..=1.0).contains(&w),
                    "invalid conditional frequency"
                );
                *sums.entry(hand.clone()).or_default() += w;
            }
        }
        ensure!(
            !sums.is_empty() && sums.values().all(|v| (v - 1.).abs() < 0.000001),
            "conditional action frequencies must sum to one; no silent normalization"
        );
    }
    let mut m = m.clone();
    m["node_count"] = json!(nodes.len());
    m["invalid_nodes"] = json!(0);
    m["validation"] = json!("conditional_weights_validated");
    c.execute(
        "INSERT INTO ai_strategies VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![id, m.to_string()],
    )?;
    agent::strategy_list(c)
}
