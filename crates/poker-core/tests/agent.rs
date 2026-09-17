use poker_core::{
    agent, fixtures,
    model::Profile,
    parser,
    service::{Request, Service},
    store,
};
use serde_json::json;
fn db() -> (tempfile::TempDir, std::sync::Arc<Service>) {
    let d = tempfile::tempdir().unwrap();
    let s = Service::new(d.path().join("test.db")).unwrap();
    let mut c = store::open(&s.path).unwrap();
    let h = parser::parse(&fixtures::cash_hand(42), &Profile::default()).unwrap();
    store::insert_batch(&mut c, "test", "private-source.txt", &[h]).unwrap();
    (d, s)
}
fn call(
    s: &std::sync::Arc<Service>,
    name: &str,
    args: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    s.handle(Request::AgentTool {
        name: name.into(),
        arguments: args,
        share_notes: false,
    })
}
#[test]
fn statistics_match_existing_engine_and_require_versions() {
    let (_d, s) = db();
    let expected = s
        .handle(Request::Overview {
            filter: Default::default(),
            group: Some("position".into()),
        })
        .unwrap();
    let e = call(&s, "query_stats", json!({"filter":{}})).unwrap();
    assert_eq!(e["data"], expected);
    assert!(call(&s, "query_stats", json!({"version":"old"})).is_err());
    assert!(call(&s, "get_hand", json!({"id":null})).is_err());
    assert!(call(&s, "get_hand", json!({"id":-1})).is_err());
    assert!(call(&s, "restore", json!({"path":"x"})).is_err());
    assert!(call(&s, "query_stats", json!({"filter":{"made_up":true}})).is_err());
}
#[test]
fn hand_redaction_and_decision_do_not_reveal_future() {
    let (_d, s) = db();
    let c = store::open(&s.path).unwrap();
    let h = store::get_hand(&c, 1).unwrap();
    let seq = h
        .actions
        .iter()
        .find(|a| a.actor == Some(h.hero_seat) && a.kind == "call")
        .unwrap()
        .seq;
    let e = call(&s, "get_decision_context", json!({"id":1,"seq":seq})).unwrap();
    assert_eq!(e["data"]["bb"], poker_core::money::format(h.bb));
    assert_eq!(e["data"]["sb"], poker_core::money::format(h.sb));
    for (actual, expected) in e["data"]["players"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&h.players)
    {
        assert_eq!(actual["stack"], poker_core::money::format(expected.stack));
    }
    assert!(e["data"].get("raw").is_none());
    assert!(e["data"].get("net").is_none());
    assert!(e["data"].get("boards").is_none());
    for a in e["data"]["actions"].as_array().unwrap() {
        assert!(a["seq"].as_u64().unwrap() < seq as u64);
    }
    for p in e["data"]["players"].as_array().unwrap() {
        if p["hero"] == false {
            assert!(p["cards"].as_array().unwrap().is_empty());
        }
    }
    let hand = call(&s, "get_hand", json!({"id":1})).unwrap();
    assert!(hand["data"].get("raw").is_none());
    assert!(hand["data"].get("annotation").is_none());
    assert!(!hand.to_string().contains("private-source"));
}
#[test]
fn paths_are_ordered_and_input_is_parameterized() {
    let (_d, s) = db();
    let c = store::open(&s.path).unwrap();
    let h = store::get_hand(&c, 1).unwrap();
    let acts = h
        .actions
        .iter()
        .filter(|a| a.actor.is_some() && ["call", "raise", "fold"].contains(&a.kind.as_str()))
        .collect::<Vec<_>>();
    let steps=acts.iter().take(2).map(|a|json!({"street":a.street,"kind":a.kind,"position":h.players.iter().find(|p|Some(p.seat)==a.actor).unwrap().position})).collect::<Vec<_>>();
    let good = call(&s, "find_spots", json!({"path":steps})).unwrap();
    assert_eq!(good["data"]["rows"].as_array().unwrap().len(), 1);
    let reversed = steps.into_iter().rev().collect::<Vec<_>>();
    let bad = call(&s, "find_spots", json!({"path":reversed})).unwrap();
    assert!(bad["data"]["rows"].as_array().unwrap().is_empty());
    let injection = call(
        &s,
        "find_spots",
        json!({"filter":{"position":"' OR 1=1 --"}}),
    )
    .unwrap();
    assert!(injection["data"]["rows"].as_array().unwrap().is_empty());
}
#[test]
fn draft_idempotency_confirmation_and_backup() {
    let (d, s) = db();
    let e = call(&s, "query_stats", json!({})).unwrap();
    let draft = json!({"id":"draft-1","title":"Report","text":"Evidence-backed review","evidence":[e["evidence_id"]],"items":[]});
    let args = json!({"draft":draft,"version":e["version"]});
    call(&s, "create_report_draft", args.clone()).unwrap();
    call(&s, "create_report_draft", args.clone()).unwrap();
    let mut conflict = args.clone();
    conflict["draft"]["text"] = json!("different");
    assert!(call(&s, "create_report_draft", conflict).is_err());
    let learning = call(&s, "get_learning_progress", json!({})).unwrap();
    assert_eq!(learning["data"]["drafts"].as_array().unwrap().len(), 1);
    s.handle(Request::AgentReview {
        id: "draft-1".into(),
        revision: 1,
        draft: serde_json::from_value(draft.clone()).unwrap(),
        accept: true,
    })
    .unwrap();
    assert!(s
        .handle(Request::AgentReview {
            id: "draft-1".into(),
            revision: 1,
            draft: serde_json::from_value(draft).unwrap(),
            accept: true
        })
        .is_err());
    let backup = d.path().join("backup.db");
    s.handle(Request::Backup {
        path: backup.to_string_lossy().into(),
    })
    .unwrap();
    let c = store::open(&backup).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM ai_drafts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn nonexistent_evidence_and_stale_evidence_rejected() {
    let (_d, s) = db();
    let e = call(&s, "query_stats", json!({})).unwrap();
    let mut d = json!({"id":"x","title":"x","text":"x","evidence":["invented"],"items":[]});
    assert!(call(
        &s,
        "create_report_draft",
        json!({"draft":d,"version":e["version"]})
    )
    .is_err());
    let c = store::open(&s.path).unwrap();
    agent::invalidate(&c).unwrap();
    d["evidence"] = json!([e["evidence_id"]]);
    assert!(call(
        &s,
        "create_report_draft",
        json!({"draft":d,"version":agent::version(&c).unwrap()})
    )
    .is_err());
}
#[test]
fn wilson_empty_and_uncertain_are_not_leaks() {
    assert_eq!(agent::wilson(0., 0.), None);
    let (lo, hi) = agent::wilson(1., 1.).unwrap();
    assert!(lo < 30. && hi > 99.);
    let (_d, s) = db();
    let r = call(&s, "find_leak_candidates", json!({})).unwrap();
    assert_eq!(r["data"]["status"], "review_only");
}

#[test]
fn exact_strategy_requires_verified_frequencies_and_matching_configuration() {
    let (d, s) = db();
    let c = store::open(&s.path).unwrap();
    let h = store::get_hand(&c, 1).unwrap();
    let action = h
        .actions
        .iter()
        .find(|a| a.actor == Some(h.hero_seat) && a.kind == "raise" && a.street == "preflop")
        .unwrap();
    let path = h
        .actions
        .iter()
        .filter(|a| a.seq < action.seq && a.street == "preflop")
        .filter_map(|a| match a.kind.as_str() {
            "fold" => Some("F".into()),
            "call" => Some("C".into()),
            "check" => Some("X".into()),
            "raise" => Some(format!("R{}", a.to as f64 / h.bb as f64)),
            _ => None,
        })
        .collect::<Vec<String>>()
        .join("-");
    let mut nodes = serde_json::Map::new();
    nodes.insert(format!("{}|{}",h.position,path),json!({"hero":h.position,"path":path,"actions":[{"action":"Raise 3","weights":{h.hand_class.clone():0.4}},{"action":"Fold","weights":{h.hand_class.clone():0.6}}]}));
    let pack = json!({"format":"riverlens.strategy/1","id":"synthetic","source":{"url":"https://example.org/synthetic-test","verification_reference":"SYNTHETIC FIXTURE, NOT POKER ADVICE"},"configuration":{"max_seats":h.max_seats,"depth_bb":h.players[0].stack as f64/h.bb as f64,"currency":h.currency,"bb_units":h.bb.to_string(),"game":h.game,"rake_id":"test","tree_id":"test"},"nodes":nodes});
    let file = d.path().join("pack.json");
    std::fs::write(&file, pack.to_string()).unwrap();
    poker_core::strategy::import(&c, &file).unwrap();
    let args = json!({"id":1,"seq":action.seq,"pack":"synthetic"});
    let reference = call(&s, "compare_preflop", args.clone()).unwrap();
    assert_eq!(reference["data"]["status"], "reference_only");
    s.handle(Request::AgentStrategyConfig {
        profile: h.profile.clone(),
        rake_id: "test".into(),
        tree_id: "test".into(),
    })
    .unwrap();
    let exact = call(&s, "compare_preflop", args).unwrap();
    assert_eq!(exact["data"]["status"], "exact");
    assert_eq!(exact["data"]["chosen_frequency"], 0.4);
    assert_eq!(exact["data"]["off_strategy"], false);
    let mut invalid = pack;
    invalid["nodes"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["actions"][0]["weights"][&h.hand_class] = json!(0.5);
    std::fs::write(&file, invalid.to_string()).unwrap();
    assert!(poker_core::strategy::import(&c, &file).is_err());
}
#[test]
fn practice_requires_acceptance_and_records_actual_only_after_answer() {
    let (_d, s) = db();
    let c = store::open(&s.path).unwrap();
    let h = store::get_hand(&c, 1).unwrap();
    let seq = h
        .actions
        .iter()
        .find(|a| a.actor == Some(h.hero_seat) && a.kind == "call")
        .unwrap()
        .seq;
    let e = call(&s, "get_decision_context", json!({"id":1,"seq":seq})).unwrap();
    let draft = json!({"id":"practice","title":"Review","text":"Self review","evidence":[e["evidence_id"]],"items":[{"hand":1,"seq":seq,"prompt":"Decide"}]});
    call(
        &s,
        "create_practice_draft",
        json!({"draft":draft,"version":e["version"]}),
    )
    .unwrap();
    assert!(s
        .handle(Request::AgentPractice {
            id: "practice".into(),
            item: 0,
            answer: None
        })
        .is_err());
    s.handle(Request::AgentReview {
        id: "practice".into(),
        revision: 1,
        draft: serde_json::from_value(draft).unwrap(),
        accept: true,
    })
    .unwrap();
    let q = s
        .handle(Request::AgentPractice {
            id: "practice".into(),
            item: 0,
            answer: None,
        })
        .unwrap();
    assert!(q.get("actual_action").is_none());
    let a = s
        .handle(Request::AgentPractice {
            id: "practice".into(),
            item: 0,
            answer: Some("Call".into()),
        })
        .unwrap();
    assert_eq!(a["actual_action"]["kind"], "call");
    assert_eq!(
        call(&s, "get_learning_progress", json!({})).unwrap()["data"]["attempts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn local_strategy_manifest_when_supplied() {
    let Ok(path) = std::env::var("RIVERLENS_STRATEGY_MANIFEST") else {
        return;
    };
    let (_d, s) = db();
    let c = store::open(&s.path).unwrap();
    let summary = poker_core::strategy::import(&c, std::path::Path::new(&path)).unwrap();
    for pack in summary.as_array().unwrap() {
        println!(
            "strategy audit: id={} nodes={} inconsistent={} validation={}",
            pack["id"], pack["node_count"], pack["invalid_nodes"], pack["validation"]
        );
        assert_eq!(pack["validation"], "reference_only");
    }
}

#[test]
fn notes_search_requires_consent_and_survives_rebuild_restore() {
    let (d, s) = db();
    s.handle(Request::SaveAnnotation {
        id: 1,
        annotation: poker_core::model::Annotation {
            note: "private-study-marker".into(),
            tags: vec!["special-tag".into()],
            reviewed: false,
        },
    })
    .unwrap();
    let hidden = call(
        &s,
        "search_learning",
        json!({"query":"private-study-marker"}),
    )
    .unwrap();
    assert!(hidden["data"]["rows"].as_array().unwrap().is_empty());
    let visible = s
        .handle(Request::AgentTool {
            name: "search_learning".into(),
            arguments: json!({"query":"private-study-marker"}),
            share_notes: true,
        })
        .unwrap();
    assert_eq!(visible["data"]["rows"].as_array().unwrap().len(), 1);
    let e = call(&s, "query_stats", json!({})).unwrap();
    let draft = json!({"id":"persist","title":"Report","text":"review","evidence":[e["evidence_id"]],"items":[]});
    call(
        &s,
        "create_report_draft",
        json!({"draft":draft,"version":e["version"]}),
    )
    .unwrap();
    let backup = d.path().join("before.db");
    s.handle(Request::Backup {
        path: backup.to_string_lossy().into(),
    })
    .unwrap();
    s.handle(Request::Rebuild).unwrap();
    assert_eq!(
        call(&s, "get_learning_progress", json!({})).unwrap()["data"]["drafts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    s.handle(Request::Restore {
        path: backup.to_string_lossy().into(),
    })
    .unwrap();
    assert_eq!(
        call(&s, "get_learning_progress", json!({})).unwrap()["data"]["drafts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(call(&s, "query_stats", json!({"version":e["version"]})).is_err());
}

#[test]
fn ai_and_offline_study_records_survive_together() {
    use poker_core::study::StudyRequest;
    let (dir, service) = db();
    service
        .handle(Request::Study {
            request: Box::new(StudyRequest::SaveItem {
                kind: "spot".into(),
                id: Some("keep-offline".into()),
                data: json!({"name":"Saved offline spot","spot":{"street":"preflop","line":[]}}),
            }),
        })
        .unwrap();
    let evidence = call(&service, "query_stats", json!({})).unwrap();
    call(&service, "create_report_draft", json!({"version":evidence["version"],"draft":{
        "id":"keep-ai","title":"AI report","text":"Synthetic integration", "evidence":[evidence["evidence_id"]],"items":[]
    }})).unwrap();
    let backup = dir.path().join("both.db");
    service
        .handle(Request::Backup {
            path: backup.to_string_lossy().into(),
        })
        .unwrap();
    service.handle(Request::Rebuild).unwrap();
    fn verify(service: &std::sync::Arc<Service>) {
        let spots = service
            .handle(Request::Study {
                request: Box::new(StudyRequest::Items {
                    kind: "spot".into(),
                }),
            })
            .unwrap();
        assert_eq!(spots[0]["id"], "keep-offline");
        let reports = call(service, "get_learning_progress", json!({})).unwrap();
        assert_eq!(reports["data"]["drafts"][0]["id"], "keep-ai");
    }
    verify(&service);
    service
        .handle(Request::Restore {
            path: backup.to_string_lossy().into(),
        })
        .unwrap();
    verify(&service);
}

#[test]
fn saved_evidence_preserves_envelope_and_organization_preserves_source() {
    let (_d, s) = db();
    let e = call(&s, "get_data_catalog", json!({})).unwrap();
    assert_eq!(
        s.handle(Request::AgentEvidence {
            id: e["evidence_id"].as_str().unwrap().into()
        })
        .unwrap(),
        e
    );
    assert!(s
        .handle(Request::AgentEvidence {
            id: "missing".into()
        })
        .is_err());
    let draft = json!({"id":"organized","title":"Report","text":"Preflop\n\nRiver","evidence":[e["evidence_id"]],"items":[]});
    call(
        &s,
        "create_report_draft",
        json!({"draft":draft,"version":e["version"]}),
    )
    .unwrap();
    let organization: agent::Organization = serde_json::from_value(json!({"category":"Strategy","tags":["preflop","river"],"sections":[{"title":"Preflop","start":0,"end":2,"tags":["preflop"]},{"title":"River","start":2,"end":3,"tags":["river"]}]})).unwrap();
    let request = Request::AgentOrganize {
        id: "organized".into(),
        revision: 1,
        organization: organization.clone(),
    };
    s.handle(request.clone()).unwrap();
    assert!(s.handle(request).is_err());
    let saved = call(&s, "get_learning_progress", json!({})).unwrap();
    let row = &saved["data"]["drafts"][0];
    assert_eq!(row["body"]["text"], draft["text"]);
    assert_eq!(row["body"]["evidence"], draft["evidence"]);
    assert_eq!(row["status"], "draft");
    assert_eq!(row["revision"], 2);
    let mut invalid = organization;
    invalid.sections[1].start = 1;
    assert!(invalid.validate("Preflop\n\nRiver").is_err());
    invalid.sections[1].start = 2;
    invalid.sections[1].end = 2;
    assert!(invalid.validate("Preflop\n\nRiver").is_err());
    invalid.sections.pop();
    assert!(invalid.validate("Preflop\n\nRiver").is_err());
}
