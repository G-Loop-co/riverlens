use super::{
    decision::{Decision, DecisionRef},
    *,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCard {
    pub reference: Option<DecisionRef>,
    pub question: Value,
    pub frequencies: Option<BTreeMap<String, f64>>,
    pub observed: Option<String>,
    pub source: String,
    pub pack: Option<String>,
    pub source_note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSession {
    pub id: String,
    pub cards: Vec<String>,
    pub index: usize,
    pub started_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrainingAnswer {
    Action {
        action: String,
        note: String,
    },
    Frequency {
        frequencies: BTreeMap<String, f64>,
        note: String,
    },
}

fn from_decision(d: &Decision) -> Value {
    json!({"reference":d.reference,"street":d.street,"position":d.position,"opponent":d.opponent,"role":d.role,
        "board":d.board,"cards":d.cards,"hand_class":d.hand_class,"pot_before":d.pot_before,"to_call":d.to_call,
        "line":d.line,"preflop_path":d.preflop_path,"legal":d.legal,"stacks_bb":d.stacks_bb,"facing":d.facing})
}

pub fn enqueue(
    c: &Connection,
    refs: &[DecisionRef],
    pack: Option<&str>,
    node: Option<&str>,
    class: Option<&str>,
) -> Result<Value> {
    anyhow::ensure!(refs.len() <= 200, "Add at most 200 decisions at once");
    let source = pack.map(|p| strategy::load(c, p)).transpose()?;
    let mut cards = vec![];
    for r in refs {
        let (hand, d) = super::resolve(c, r)?;
        let n = source.as_ref().and_then(|p| {
            p.nodes
                .iter()
                .find(|n| n.hero == d.position && n.path == d.preflop_path)
        });
        let usable = n.filter(|n| {
            strategy::mismatches(&d, n, source.as_ref().expect("node has pack")).is_empty()
        });
        let mut question = from_decision(&d);
        let frequencies = usable
            .and_then(|n| n.frequencies.get(&d.hand_class))
            .cloned();
        if let Some(n) = usable {
            question["options"] = json!(n.actions);
        }
        let card = TrainingCard {
            reference: Some(r.clone()),
            question,
            frequencies,
            observed: Some(format!(
                "{}{}",
                d.action,
                d.size_bb.map(|s| format!(" {s}bb")).unwrap_or_default()
            )),
            source: if usable.is_some() { "gto" } else { "self" }.into(),
            pack: if usable.is_some() {
                pack.map(str::to_owned)
            } else {
                None
            },
            source_note: store::annotation(c, hand)?.note,
        };
        let key = store::fingerprint(&serde_json::to_string(&(r, &card.pack))?);
        cards.push((key, card));
    }
    if let Some(node_id) = node {
        anyhow::ensure!(
            refs.is_empty(),
            "Choose historical decisions or a strategy card"
        );
        let source = source.as_ref().context("Choose a strategy pack")?;
        let n = source
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .context("Unknown strategy node")?;
        let hand = class.context("Choose a starting hand")?;
        let frequency = n
            .frequencies
            .get(hand)
            .context("This hand has no verified conditional frequencies")?;
        let rank: Vec<_> = hand.chars().collect();
        let cards = vec![
            format!("{}s", rank[0]),
            format!("{}{}", rank[1], if hand.ends_with('s') { "s" } else { "h" }),
        ];
        let card = TrainingCard {
            reference: None,
            question: json!({"street":"preflop","position":n.hero,"preflop_path":n.path,"hand_class":hand,"cards":cards,"board":[],"line":[],"options":n.actions,"theory":true}),
            frequencies: Some(frequency.clone()),
            observed: None,
            source: "gto".into(),
            pack: Some(source.id.clone()),
            source_note: source.name.clone(),
        };
        let key = store::fingerprint(&format!("{}:{node_id}:{hand}", source.id));
        // Immutable pack IDs keep previous attempts attached to their original model.
        return insert_cards(c, &[(key, card)]);
    }
    anyhow::ensure!(!cards.is_empty(), "Choose at least one decision");
    insert_cards(c, &cards)
}

fn insert_cards(c: &Connection, cards: &[(String, TrainingCard)]) -> Result<Value> {
    let tx = c.unchecked_transaction()?;
    let mut added = 0;
    for (id, card) in cards {
        added += tx.execute(
            "INSERT OR IGNORE INTO study_cards VALUES(?1,?2,?3,0)",
            params![
                id,
                serde_json::to_string(card)?,
                chrono::Utc::now().timestamp()
            ],
        )?;
    }
    tx.commit()?;
    Ok(json!({"added":added,"duplicates":cards.len()-added}))
}

fn session(c: &Connection, id: &str) -> Result<TrainingSession> {
    let data: String = c.query_row("SELECT data FROM study_sessions WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    Ok(serde_json::from_str(&data)?)
}
fn card(c: &Connection, id: &str) -> Result<TrainingCard> {
    let data: String = c.query_row("SELECT data FROM study_cards WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    let mut card: TrainingCard = serde_json::from_str(&data)?;
    if card.source == "gto" {
        let pack = card
            .pack
            .as_deref()
            .map(|id| strategy::load(c, id))
            .transpose()?;
        let node = pack.as_ref().and_then(|p| {
            p.nodes.iter().find(|n| {
                n.hero == card.question["position"].as_str().unwrap_or("")
                    && n.path == card.question["preflop_path"].as_str().unwrap_or("")
            })
        });
        let class = card.question["hand_class"].as_str().unwrap_or("");
        let valid = node.and_then(|n| n.frequencies.get(class));
        if let Some(reference) = &card.reference {
            let resolved = super::resolve(c, reference).ok();
            let matched = resolved
                .as_ref()
                .zip(node)
                .zip(pack.as_ref())
                .is_some_and(|(((_, d), n), p)| strategy::mismatches(d, n, p).is_empty());
            if !matched {
                card.source = "self".into();
                card.frequencies = None;
                if let Some((_, d)) = resolved {
                    card.question = from_decision(&d);
                }
            }
        } else if valid.is_none() {
            // Keep progress and previous attempts, but never grade a stale theory key.
            card.question["invalid_strategy"] = json!(true);
            card.frequencies = None;
        }
    }
    Ok(card)
}

pub fn state(c: &Connection) -> Result<Value> {
    let now = chrono::Utc::now().timestamp();
    let (total, due): (u64, u64) = c.query_row(
        "SELECT COUNT(*),COALESCE(SUM(due<=?1),0) FROM study_cards",
        [now],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let active:Option<String>=c.query_row("SELECT id FROM study_sessions WHERE json_extract(data,'$.index')<json_array_length(data,'$.cards') ORDER BY json_extract(data,'$.started_at') DESC LIMIT 1",[],|r|r.get(0)).optional()?;
    let completed: u64 = c.query_row(
        "SELECT COUNT(*) FROM study_attempts WHERE json_extract(data,'$.rating') IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let recent = {
        let mut s=c.prepare("SELECT data FROM study_attempts WHERE json_extract(data,'$.rating') IS NOT NULL ORDER BY created DESC LIMIT 20")?;
        let rows = s
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|r| Ok(serde_json::from_str::<Value>(&r?)?))
            .collect::<Result<Vec<_>>>()?;
        rows
    };
    Ok(json!({"total":total,"due":due,"completed":completed,"active":active,"recent":recent}))
}

pub fn start(c: &Connection, limit: u32) -> Result<Value> {
    if let Some(id) = state(c)?["active"].as_str() {
        return current(c, id);
    }
    let cards = {
        let mut s =
            c.prepare("SELECT id FROM study_cards WHERE due<=?1 ORDER BY due,id LIMIT ?2")?;
        let rows = s
            .query_map(
                params![chrono::Utc::now().timestamp(), limit.clamp(1, 200)],
                |r| r.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows
    };
    let id = format!(
        "session-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let session = TrainingSession {
        id: id.clone(),
        cards,
        index: 0,
        started_at: chrono::Utc::now().timestamp(),
    };
    c.execute(
        "INSERT INTO study_sessions VALUES(?1,?2)",
        params![id, serde_json::to_string(&session)?],
    )?;
    current(c, &id)
}

pub fn current(c: &Connection, id: &str) -> Result<Value> {
    let s = session(c, id)?;
    if s.index >= s.cards.len() {
        return Ok(json!({"id":id,"complete":true,"index":s.index,"total":s.cards.len()}));
    }
    let card_id = &s.cards[s.index];
    let card = card(c, card_id)?;
    let attempt: Option<String> = c
        .query_row(
            "SELECT data FROM study_attempts WHERE id=?1",
            [format!("{id}:{}", s.index)],
            |r| r.get(0),
        )
        .optional()?;
    let mut feedback = attempt
        .map(|a| serde_json::from_str::<Value>(&a))
        .transpose()?;
    let resolved = card
        .reference
        .as_ref()
        .and_then(|r| super::resolve(c, r).ok());
    let stale = (card.reference.is_some() && resolved.is_none())
        || card.question["invalid_strategy"] == true;
    if let Some(feedback) = feedback.as_mut() {
        feedback["replay_hand"] = json!(resolved.as_ref().map(|(id, _)| id));
    }
    // Whitelist question fields: neither original action nor frequencies are
    // returned before submission, including hidden DOM/JSON attributes.
    Ok(
        json!({"id":id,"complete":false,"index":s.index,"total":s.cards.len(),"card_id":card_id,"question":card.question,"source":card.source,"pack":card.pack,"feedback":feedback,"stale":stale}),
    )
}

pub fn answer(c: &Connection, id: &str, card_id: &str, input: &TrainingAnswer) -> Result<Value> {
    let s = session(c, id)?;
    anyhow::ensure!(
        s.cards.get(s.index).is_some_and(|c| c == card_id),
        "Training question changed; reload the session"
    );
    let key = format!("{id}:{}", s.index);
    let old: Option<String> = c
        .query_row("SELECT data FROM study_attempts WHERE id=?1", [&key], |r| {
            r.get(0)
        })
        .optional()?;
    if old.is_some() {
        return current(c, id);
    }
    let card = card(c, card_id)?;
    anyhow::ensure!(
        card.question["invalid_strategy"] != true,
        "Strategy ancestry is no longer verified"
    );
    if let Some(r) = &card.reference {
        super::resolve(c, r)?;
    }
    let (note, feedback) = match input {
        TrainingAnswer::Action { action, note } => {
            let legal = if let Some(freq) = &card.frequencies {
                freq.contains_key(action)
            } else {
                card.question["legal"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|a| a.as_str() == Some(action)))
            };
            anyhow::ensure!(legal, "Choose an available action");
            (
                note,
                json!({"chosen":action,"in_strategy":card.frequencies.as_ref().map(|f|f.get(action).copied().unwrap_or(0.)>0.)}),
            )
        }
        TrainingAnswer::Frequency { frequencies, note } => {
            let expected = card
                .frequencies
                .as_ref()
                .context("Frequency grading requires a strategy answer")?;
            anyhow::ensure!(
                frequencies.keys().eq(expected.keys())
                    && frequencies
                        .values()
                        .all(|v| v.is_finite() && (0.0..=100.).contains(v))
                    && (frequencies.values().sum::<f64>() - 100.).abs() <= 0.01,
                "Enter every action frequency; total must be 100%"
            );
            let differences: BTreeMap<_, _> = expected
                .iter()
                .map(|(a, v)| (a.clone(), frequencies[a] - v * 100.))
                .collect();
            let max_gap = differences.values().map(|v| v.abs()).fold(0., f64::max);
            (note, json!({"differences":differences,"max_gap":max_gap}))
        }
    };
    anyhow::ensure!(note.chars().count() <= 4000, "Training note is too long");
    let hand = card
        .reference
        .as_ref()
        .map(|r| super::resolve(c, r).map(|(id, _)| id))
        .transpose()?;
    let data = json!({"answer":input,"feedback":feedback,"expected":card.frequencies,"observed":card.observed,"source_note":card.source_note,"source":card.source,"pack":card.pack,"reference":card.reference,"replay_hand":hand,"rating":null,"created":chrono::Utc::now().timestamp()});
    c.execute(
        "INSERT INTO study_attempts VALUES(?1,?2,?3,?4,?5)",
        params![
            key,
            card_id,
            id,
            chrono::Utc::now().timestamp(),
            data.to_string()
        ],
    )?;
    current(c, id)
}

pub fn rate(c: &mut Connection, id: &str, card_id: &str, rating: &str) -> Result<Value> {
    anyhow::ensure!(
        matches!(rating, "repeat" | "advance" | "skip"),
        "Invalid review rating"
    );
    let tx = c.transaction()?;
    let mut s = session(&tx, id)?;
    if s.cards.get(s.index).is_none_or(|c| c != card_id) {
        return current(&tx, id);
    }
    let key = format!("{id}:{}", s.index);
    if rating == "skip" {
        let card = card(&tx, card_id)?;
        anyhow::ensure!(
            card.reference
                .as_ref()
                .is_some_and(|r| super::resolve(&tx, r).is_err())
                || card.question["invalid_strategy"] == true,
            "Only unavailable decisions may be skipped"
        );
    } else {
        let data: String = tx
            .query_row("SELECT data FROM study_attempts WHERE id=?1", [&key], |r| {
                r.get(0)
            })
            .context("Submit an answer before rating")?;
        let mut data: Value = serde_json::from_str(&data)?;
        data["rating"] = json!(rating);
        tx.execute(
            "UPDATE study_attempts SET data=?1 WHERE id=?2",
            params![data.to_string(), key],
        )?;
    }
    let stage: usize = tx.query_row(
        "SELECT stage FROM study_cards WHERE id=?1",
        [card_id],
        |r| r.get(0),
    )?;
    let delays = [1, 3, 7, 14, 30];
    let next = if rating == "advance" {
        (stage + 1).min(5)
    } else {
        0
    };
    let days = if next == 0 { 1 } else { delays[next - 1] };
    tx.execute(
        "UPDATE study_cards SET due=?1,stage=?2 WHERE id=?3",
        params![chrono::Utc::now().timestamp() + days * 86400, next, card_id],
    )?;
    s.index += 1;
    tx.execute(
        "UPDATE study_sessions SET data=?1 WHERE id=?2",
        params![serde_json::to_string(&s)?, id],
    )?;
    tx.commit()?;
    current(c, id)
}
