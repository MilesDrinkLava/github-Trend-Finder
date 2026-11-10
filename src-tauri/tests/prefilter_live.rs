#[allow(dead_code)]
#[path = "../src/policy.rs"]
mod policy;
#[path = "../src/prefilter.rs"]
mod prefilter;
#[allow(dead_code)]
#[path = "../src/research.rs"]
mod research;
use std::collections::HashMap;
#[test]
fn replay_live_github_routes_through_production_prefilter() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../reports/v2-prefilter-live-2026-11-10.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut pool = Vec::new();
        let mut index = HashMap::new();
        for route in case["routes"].as_array().unwrap() {
            let label = route["route"].as_str().unwrap();
            for (rank, item) in route["items"].as_array().unwrap().iter().enumerate() {
                let repo: research::Repository = serde_json::from_value(item.clone()).unwrap();
                let id = repo.id;
                research::merge_candidates(&mut pool, &mut index, vec![repo], label, 600);
                let at = index[&id];
                pool[at].route_ranks.insert(label.into(), rank + 1);
            }
        }
        let chosen = prefilter::choose(&mut pool, 20, case["query"].as_str().unwrap());
        let names: Vec<_> = chosen
            .iter()
            .map(|&i| pool[i].repo.full_name.as_str())
            .collect();
        println!(
            "{}: {} candidates -> {} shortlist; {:?}",
            case["query"],
            pool.len(),
            chosen.len(),
            names
        );
        for expected in case["expected"].as_array().unwrap() {
            let name = expected.as_str().unwrap();
            assert!(
                pool.iter().any(|c| c.repo.full_name == name),
                "anchor {name} missed in recall"
            );
            assert!(
                names.contains(&name),
                "anchor {name} excluded by initial filter"
            );
        }
    }
}
