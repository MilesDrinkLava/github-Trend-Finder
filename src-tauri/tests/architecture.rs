//! Offline behavioral evaluation. Fixed public snapshots are not full relevance labels.
#![allow(dead_code)]
#[path = "baselines/alpha3_prefilter.rs"]
mod baseline;
#[path = "../src/intent.rs"]
mod intent;
#[path = "../src/judgment.rs"]
mod judgment;
#[path = "../src/policy.rs"]
mod policy;
#[path = "../src/prefilter.rs"]
mod prefilter;
#[path = "../src/research.rs"]
mod research;
#[path = "../src/retrieval.rs"]
mod retrieval;
use research::{Candidate, Plan, Verdict};
use serde_json::{json, Value};
use std::collections::HashMap;

fn contract(query: &str, clauses: &[(&str, &str, &[&str])]) -> Plan {
    intent::compile(query, serde_json::from_value(json!({"intent":query,"criteria":clauses.iter()
        .map(|(mode,quote,terms)| json!({"mode":mode,"quote":quote,"terms":terms})).collect::<Vec<_>>()})).unwrap()).0
}
fn ocr_plan() -> Plan {
    contract(
        "离线 OCR，支持 Windows，最好有中文界面",
        &[
            ("core", "OCR", &["OCR", "文字识别"]),
            ("must", "离线", &["离线", "offline"]),
            ("must", "支持 Windows", &["Windows"]),
            ("prefer", "最好有中文界面", &["中文界面", "Chinese UI"]),
        ],
    )
}
fn candidate(id: u64, desc: &str, rank: usize) -> Candidate {
    let repo = serde_json::from_value(json!({"id":id,"full_name":format!("owner/project{id}"),"description":desc,"stargazers_count":0})).unwrap();
    let mut pool = vec![];
    research::merge_candidates(
        &mut pool,
        &mut HashMap::new(),
        vec![repo],
        "OCR [name,description]",
        1,
    );
    let mut c = pool.remove(0);
    c.original_rank = rank;
    c.route_ranks
        .insert("OCR [name,description]".into(), rank + 1);
    c
}
fn packet(id: u64) -> judgment::Packet {
    judgment::Packet {
        id,
        name: format!("owner/project{id}"),
        readme_available: true,
        e: vec![
            judgment::Snippet {
                k: "r0".into(),
                s: "readme".into(),
                t: "Offline OCR. Windows supported.".into(),
            },
            judgment::Snippet {
                k: "r1".into(),
                s: "readme".into(),
                t: "Windows is not supported.".into(),
            },
        ],
    }
}
fn judge(checks: Value, complete: bool) -> research::Assessment {
    judgment::parse(json!({"items":[{"id":1,"complete":complete,"checks":checks,"score":100,"verdict":"match"}]}),&ocr_plan(),&[packet(1)]).unwrap().remove(0)
}

#[test]
fn compiler_uses_original_conditions_and_rejects_invented_constraints() {
    let raw: Plan = serde_json::from_value(json!({"intent":"OCR","criteria":[
        {"mode":"core","quote":"OCR","terms":["文字识别"]},
        {"mode":"must","quote":"不要云API","text":"必须使用云API"},
        {"mode":"must","quote":"Windows","text":"仅Windows"}]}))
    .unwrap();
    let (p, w) = intent::compile("OCR，不要云API", raw);
    assert_eq!(p.criteria[1].text, "不要云API");
    assert_eq!(p.criteria.len(), 2);
    assert!(!p.contract_valid);
    assert!(!w.is_empty());
    assert!(!intent::fallback("复杂需求").contract_valid);
}
#[test]
fn excessive_conditions_do_not_silently_become_complete() {
    let q = "OCR Windows Linux macOS offline Chinese preview undo";
    let entries: Vec<_> = q
        .split_whitespace()
        .enumerate()
        .map(|(i, s)| json!({"mode":if i==0{"core"}else{"must"},"quote":s}))
        .collect();
    let (p, _) = intent::compile(
        q,
        serde_json::from_value(json!({"intent":q,"criteria":entries})).unwrap(),
    );
    assert_eq!(p.criteria.len(), 6);
    assert!(!p.contract_valid);
}
#[test]
fn query_routes_preserve_bilingual_core_without_query_permutation_duplicates() {
    let mut p = ocr_plan();
    p.queries_en = vec![
        "offline OCR".into(),
        "OCR offline".into(),
        "OCR stars:>100".into(),
    ];
    let r = research::routes("离线文字识别", &p);
    assert_eq!(r[0], "离线文字识别");
    assert!(r.contains(&"文字识别".into()) && r.contains(&"OCR".into()));
    assert_eq!(r.iter().filter(|q| q.contains("offline")).count(), 1);
    assert!(!r.iter().any(|q| q.contains(':')));
}
#[test]
fn ascii_fragments_are_not_core_function_matches() {
    assert_eq!(
        judgment::match_terms("language parser plan", &["LAN".into()]),
        0.0
    );
    assert_eq!(
        judgment::match_terms("LAN file transfer", &["LAN".into()]),
        1.0
    );
    assert_eq!(
        judgment::match_terms("离线文字识别工具", &["文字识别".into()]),
        1.0
    );
}
#[test]
fn duplicate_query_routes_cannot_multiply_relevance() {
    let mut a: Vec<_> = (1..=20)
        .map(|id| candidate(id, "OCR", id as usize))
        .collect();
    let mut b = a.clone();
    for c in &mut b {
        for n in 0..12 {
            c.route_ranks.insert(
                format!("OCR variant{n} [name,description]"),
                c.original_rank + 1,
            );
        }
    }
    prefilter::choose_with_plan(&mut a, 5, "OCR", &ocr_plan());
    prefilter::choose_with_plan(&mut b, 5, "OCR", &ocr_plan());
    for (a, b) in a.iter().zip(b.iter()) {
        assert!((a.prefilter_score - b.prefilter_score).abs() < 1e-9);
    }
}
#[test]
fn diverse_selection_does_not_spend_every_slot_on_identical_forks() {
    let mut pool: Vec<_> = (1..=30)
        .map(|id| {
            candidate(
                id,
                "OCR identical upstream desktop image recognition fork",
                id as usize,
            )
        })
        .collect();
    let mut distinct = candidate(100, "文字识别，移动端摄像头扫描整理", 30);
    distinct.route_ranks.clear();
    distinct
        .route_ranks
        .insert("中文OCR [name,description]".into(), 2);
    pool.push(distinct);
    let chosen = prefilter::choose_with_plan(&mut pool, 5, "OCR", &ocr_plan());
    assert!(chosen.contains(&30));
    assert_eq!(pool.len(), 31);
}
#[test]
fn exploration_can_preserve_an_opaque_native_chinese_project() {
    let mut pool: Vec<_> = (1..=25)
        .map(|id| candidate(id, "OCR app", id as usize))
        .collect();
    let mut opaque = candidate(100, "", 25);
    opaque.repo.full_name = "中文作者/小工具".into();
    opaque.route_ranks.clear();
    opaque.route_ranks.insert("文字识别 [readme]".into(), 1);
    pool.push(opaque);
    let chosen = prefilter::choose_with_plan(&mut pool, 20, "OCR", &ocr_plan());
    assert!(chosen.contains(&25));
    assert_eq!(pool[25].repo.stars, 0);
}
#[test]
fn unknown_required_platform_cannot_be_a_confirmed_match() {
    let a = judge(
        json!([
            ["c1", "y", "r0"],
            ["c2", "y", "r0"],
            ["c3", "u", ""],
            ["c4", "u", ""]
        ]),
        true,
    );
    assert_eq!(a.verdict, Verdict::Partial);
    assert!(a.relevance <= 65);
    assert!(a.gaps.iter().any(|s| s.contains("Windows")));
}
#[test]
fn hard_conflict_and_unknown_core_have_separate_outcomes() {
    let mixed = judge(
        json!([["c1", "u", ""], ["c2", "y", "r0"], ["c3", "n", "r1"]]),
        true,
    );
    assert_eq!(mixed.verdict, Verdict::Irrelevant);
    assert!(mixed.reason.contains("证据冲突：支持 Windows"));
    let a = judge(
        json!([["c1", "y", "r0"], ["c2", "y", "r0"], ["c3", "n", "r1"]]),
        true,
    );
    assert_eq!(a.verdict, Verdict::Irrelevant);
    assert!(a.relevance <= 20);
    let a = judge(
        json!([["c1", "u", ""], ["c2", "y", "r0"], ["c3", "y", "r0"]]),
        true,
    );
    assert_eq!(a.verdict, Verdict::Uncertain);
    assert!(a.relevance <= 40);
}
#[test]
fn missing_checks_and_forged_evidence_are_unknown_not_verified() {
    let a = judge(
        json!([
            ["c1", "y", "invented"],
            ["c2", "y", "r0"],
            ["c2", "n", "r1"]
        ]),
        true,
    );
    assert!(a
        .checks
        .iter()
        .all(|c| c.status == judgment::Status::Unknown));
    assert!(a.evidence.is_empty());
    assert_eq!(a.verdict, Verdict::Uncertain);
}
#[test]
fn model_cannot_set_scores_or_bypass_contract_completeness() {
    let checks = json!([
        ["c1", "y", "r0"],
        ["c2", "y", "r0"],
        ["c3", "y", "r0"],
        ["c4", "u", ""]
    ]);
    let a = judge(checks.clone(), true);
    assert_eq!(a.verdict, Verdict::Match);
    assert_eq!(a.relevance, 80);
    assert_eq!(judge(checks, false).verdict, Verdict::Uncertain);
}
#[test]
fn duplicate_ids_are_rejected_and_batch_order_cannot_change_scores() {
    let row = |id| json!({"id":id,"complete":true,"checks":[["c1","y","r0"],["c2","y","r0"],["c3","y","r0"]]});
    let p = ocr_plan();
    let packets = [packet(1), packet(2)];
    assert!(
        judgment::parse(json!({"items":[row(1),row(1)]}), &p, &packets)
            .unwrap()
            .is_empty()
    );
    let a = judgment::parse(json!({"items":[row(1),row(2)]}), &p, &packets).unwrap();
    let b = judgment::parse(json!({"items":[row(2),row(1)]}), &p, &packets).unwrap();
    assert_eq!(a[0].relevance, b[1].relevance);
}
#[test]
fn evidence_packets_keep_late_limitations_and_exact_source_text_within_budget() {
    let mut c = candidate(1, "OCR utility", 0);
    c.readme = format!(
        "OCR reads images offline.\n\n{}\n\nWindows is not supported.\n\n{}",
        "General documentation and examples. ".repeat(180),
        "更多说明。".repeat(100)
    );
    let p = judgment::packet(&c, &ocr_plan(), 1800);
    assert!(serde_json::to_vec(&p).unwrap().len() <= 1800);
    assert!(
        p.e.iter().any(|e| e.t.contains("Windows is not supported")),
        "{:?}",
        p
    );
    for e in p.e.iter().filter(|e| e.s == "readme") {
        assert!(c.readme.contains(&e.t));
    }
    for cap in [300, 500, 900, 2200] {
        assert!(
            serde_json::to_vec(&judgment::packet(&c, &ocr_plan(), cap))
                .unwrap()
                .len()
                <= cap
        );
    }
}
#[test]
fn scheduler_warms_both_languages_then_prefers_new_candidates() {
    let mut r = retrieval::build(&["文字识别".into(), "OCR".into()], 600);
    assert_eq!(r.len(), 4);
    for i in 0..r.len() {
        assert_eq!(retrieval::next(&r), Some(i));
        let size = r[i].page_size;
        r[i].observe(
            size,
            if i == 1 { size } else { 0 },
            if i == 1 { size } else { 0 },
            1000,
            false,
        );
    }
    assert_eq!(retrieval::next(&r), Some(1));
    let size = r[0].page_size;
    r[0].observe(size, 0, 0, 1000, false);
    assert!(r[0].exhausted);
}
#[test]
fn incomplete_github_response_is_not_treated_as_exhaustive() {
    let mut r = retrieval::build(&["OCR".into()], 600).remove(0);
    r.observe(1, 1, 1, 1, true);
    assert!(!r.exhausted);
    r.observe(0, 0, 0, 1, true);
    assert!(!r.exhausted);
    r.observe(0, 0, 0, 1, true);
    assert!(r.exhausted);
}

fn replay(case: &Value) -> Vec<Candidate> {
    let mut pool = vec![];
    let mut index = HashMap::new();
    for route in case["routes"].as_array().unwrap() {
        let label = route["route"].as_str().unwrap();
        for (rank, item) in route["items"].as_array().unwrap().iter().enumerate() {
            let repo: research::Repository = serde_json::from_value(item.clone()).unwrap();
            let id = repo.id;
            research::merge_candidates(&mut pool, &mut index, vec![repo], label, 600);
            pool[index[&id]].route_ranks.insert(label.into(), rank + 1);
        }
    }
    pool
}
#[test]
fn compare_fixed_public_snapshots_and_constructed_noise_without_model_calls() {
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../reports/v2-prefilter-live-2026-11-10.json"
    ))
    .unwrap();
    let core_sets = [
        vec!["OCR", "文字识别"],
        vec!["file transfer", "文件传输", "传文件"],
        vec!["batch rename", "file renamer", "重命名", "改名"],
    ];
    let mut cases = vec![];
    for (n, case) in snapshot["cases"].as_array().unwrap().iter().enumerate() {
        let query = case["query"].as_str().unwrap();
        let mut old = replay(case);
        let mut new = old.clone();
        let p = contract(query, &[("core", query, &core_sets[n])]);
        let before = baseline::choose(&mut old, 20, query);
        let after = prefilter::choose_with_plan(&mut new, 20, query, &p);
        let rows = |pool: &[Candidate], selected: &[usize]| {
            selected.iter().map(|&i|json!({"repo":pool[i].repo.full_name,"description":pool[i].repo.description,"score":pool[i].prefilter_score})).collect::<Vec<_>>()
        };
        for expected in case["expected"].as_array().unwrap() {
            assert!(after
                .iter()
                .any(|&i| new[i].repo.full_name == expected.as_str().unwrap()));
        }
        cases.push(json!({"query":query,"pool":new.len(),"anchors":case["expected"],"alpha3":rows(&old,&before),"alpha4":rows(&new,&after)}));
    }
    // Labeled by construction; these are algorithm stress cases, not measured real-world precision.
    let mut synthetic = vec![];
    for (query, description) in [
        ("OCR", "OCR text recognition"),
        ("文件传输", "局域网文件传输"),
        ("改名", "批量改名工具"),
        ("PDF", "PDF editor"),
        ("backup", "file backup"),
        ("字幕", "字幕编辑器"),
    ] {
        let p = contract(query, &[("core", query, &[query])]);
        let mut pool: Vec<_> = (1..=40)
            .map(|id| {
                let mut c = candidate(
                    id,
                    "General learning links and unrelated API examples",
                    id as usize,
                );
                for n in 0..6 {
                    c.route_ranks
                        .insert(format!("variant{n} [name,description]"), id as usize);
                }
                c
            })
            .collect();
        for id in 41..=50 {
            let mut c = candidate(id, description, id as usize);
            c.route_ranks.clear();
            c.route_ranks
                .insert(format!("{query} [name,description]"), (id - 40) as usize);
            pool.push(c);
        }
        let mut old = pool.clone();
        let before = baseline::choose(&mut old, 20, query);
        let after = prefilter::choose_with_plan(&mut pool, 20, query, &p);
        let count = |pool: &[Candidate], indices: &[usize]| {
            indices.iter().filter(|&&i| pool[i].repo.id >= 41).count()
        };
        let a = count(&old, &before);
        let b = count(&pool, &after);
        assert!(b > a, "{query}: {a} -> {b}");
        synthetic.push(json!({"query":query,"relevant_total":10,"alpha3_relevant_at20":a,"alpha4_relevant_at20":b}));
    }
    let report = json!({"fixture":"v2-prefilter-live-2026-11-10.json","paid_model_calls":0,"scope":"fixed snapshots, limited anchors and constructed stress cases; not a full precision or model-quality evaluation","public_snapshots":cases,"constructed_stress_cases":synthetic});
    if let Ok(path) = std::env::var("GH_FINDER_EVAL_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{}", report["constructed_stress_cases"]);
}
