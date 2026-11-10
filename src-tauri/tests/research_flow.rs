#[allow(dead_code)]
#[path = "../src/intent.rs"]
mod intent;
#[allow(dead_code)]
#[path = "../src/judgment.rs"]
mod judgment;
#[path = "../src/policy.rs"]
mod policy;
#[path = "../src/prefilter.rs"]
#[allow(dead_code)]
mod prefilter;
#[path = "../src/research.rs"]
mod research;
#[allow(dead_code)]
#[path = "../src/retrieval.rs"]
mod retrieval;
#[path = "../src/review_session.rs"]
mod review_session;
use research::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn repo(id: u64, desc: Option<&str>, stars: u64) -> Repository {
    serde_json::from_value(json!({"id":id,"full_name":format!("owner/project{id}"),
        "description":desc,"stargazers_count":stars,"topics":[],"language":null}))
    .unwrap()
}
fn candidate() -> Candidate {
    let mut pool = vec![];
    merge_candidates(
        &mut pool,
        &mut HashMap::new(),
        vec![repo(1, Some("离线批量重命名文件"), 0)],
        "批量改名",
        100,
    );
    pool.remove(0)
}
fn plan() -> Plan {
    Plan {
        intent: "批量重命名".into(),
        requirements: vec![],
        queries_zh: vec!["批量改名".into()],
        queries_en: vec!["batch rename".into()],
        criteria: vec![intent::Criterion {
            id: String::new(),
            text: String::new(),
            mode: intent::Mode::Core,
            quote: "批量".into(),
            terms: vec!["批量改名".into(), "batch rename".into()],
        }],
        contract_valid: false,
    }
}

#[test]
fn original_chinese_is_mandatory_and_generated_filters_cannot_narrow_it() {
    let mut p = plan();
    p.queries_zh
        .extend(["stars:>500".into(), "language:English".into()]);
    let r = routes("支持中文的批量重命名", &p);
    assert_eq!(r[0], "支持中文的批量重命名");
    assert!(r.contains(&"批量改名".into()));
    assert!(r.contains(&"batch rename".into()));
    assert!(!r.iter().any(|x| x.contains(':')));
}
#[test]
fn zero_star_chinese_repositories_survive_and_private_repositories_do_not() {
    let mut pool = vec![];
    let mut indexes = HashMap::new();
    let mut private = repo(9, Some("private secret"), 100);
    private.private = true;
    merge_candidates(
        &mut pool,
        &mut indexes,
        vec![repo(1, Some("中文工具"), 0), private],
        "中文",
        100,
    );
    assert_eq!(pool.len(), 1);
    assert_eq!(pool[0].repo.stars, 0);
    assert_eq!(
        merge_candidates(
            &mut pool,
            &mut indexes,
            vec![repo(1, Some("中文工具"), 0)],
            "English",
            100
        ),
        1
    );
    assert_eq!(pool[0].recalled_by.len(), 2);
}
#[test]
fn invented_ids_and_invented_quotes_cannot_become_verified_matches() {
    let c = candidate();
    let item = |id, quote| json!({"id":id,"verdict":"match","relevance":99,"category":"app","reason":"符合", "evidence":[{"source":"description","quote":quote}]});
    let result = parse_assessments(
        json!({"items":[item(999,"离线批量重命名"),item(1,"不存在的 Windows 支持")]}),
        &[c],
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].verdict, Verdict::Uncertain);
    assert!(result[0].evidence.is_empty());
}
#[test]
fn missing_results_remain_unassessed_not_irrelevant() {
    let c = candidate();
    assert!(parse_assessments(json!({"items":[]}), &[c.clone()])
        .unwrap()
        .is_empty());
    assert!(c.assessment.is_none());
}

#[test]
fn compact_response_keeps_verified_evidence() {
    let rows=parse_assessments(json!({"items":[{"id":1,"v":"match","s":91,"c":"app","r":"支持离线改名","g":[],"e":[{"s":"description","q":"离线批量重命名文件"}]}]}), &[candidate()]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].verdict, Verdict::Match);
    assert_eq!(rows[0].evidence.len(), 1);
}

struct Server {
    base: String,
    seen: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}
fn server(fail_model: bool) -> Server {
    server_with_delay(fail_model, Duration::ZERO)
}
fn server_with_delay(fail_model: bool, delay: Duration) -> Server {
    server_with_options(fail_model, delay, false)
}
fn server_with_options(fail_model: bool, delay: Duration, large: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(vec![]));
    let stop = Arc::new(AtomicBool::new(false));
    let (requests, done) = (seen.clone(), stop.clone());
    let thread = std::thread::spawn(move || {
        while !done.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    let requests = requests.clone();
                    std::thread::spawn(move || {
                        socket.set_nonblocking(false).unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_secs(15)))
                            .unwrap();
                        let mut bytes = vec![];
                        let mut buf = [0u8; 8192];
                        let end = loop {
                            let n = socket.read(&mut buf).unwrap();
                            if n == 0 {
                                return;
                            }
                            bytes.extend_from_slice(&buf[..n]);
                            if let Some(i) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
                                break i + 4;
                            }
                        };
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_string();
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|x| x.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        while bytes.len() < end + length {
                            let n = socket.read(&mut buf).unwrap();
                            if n == 0 {
                                break;
                            }
                            bytes.extend_from_slice(&buf[..n]);
                        }
                        let path = headers
                            .lines()
                            .next()
                            .unwrap()
                            .split_whitespace()
                            .nth(1)
                            .unwrap()
                            .to_string();
                        let body: Value =
                            serde_json::from_slice(&bytes[end..]).unwrap_or(Value::Null);
                        requests
                            .lock()
                            .unwrap()
                            .push(json!({"path":path,"body":body}));
                        let mut code = 200;
                        let response = if path.starts_with("/chat/completions") {
                            if fail_model {
                                code = 401;
                                json!({"error":"invalid key"})
                            } else {
                                let input: Value = serde_json::from_str(
                                    body["messages"][1]["content"].as_str().unwrap(),
                                )
                                .unwrap();
                                let output = if input.get("request").is_some() {
                                    if large {
                                        json!({"intent":"批量改名","criteria":[
                                            {"mode":"core","quote":"批量改名","terms":["批量改名","batch rename"]},
                                            {"mode":"must","quote":"Windows","terms":["Windows"]},
                                            {"mode":"must","quote":"离线","terms":["离线","offline"]},
                                            {"mode":"must","quote":"中文界面","terms":["中文界面","Chinese UI"]},
                                            {"mode":"must","quote":"预览","terms":["预览","preview"]},
                                            {"mode":"prefer","quote":"最好有撤销","terms":["撤销","undo"]}],
                                            "queries_zh":["批量改名","文件重命名"],"queries_en":["batch rename","file renamer"]})
                                    } else {
                                        serde_json::to_value(plan()).unwrap()
                                    }
                                } else {
                                    json!({"items":input["candidates"].as_array().unwrap().iter().map(|c|{
                                        let id=c["id"].as_u64().unwrap();
                                        json!({"id":id,"type":"app","complete":true,"checks":input["criteria"].as_array().unwrap().iter().map(|criterion|json!([criterion["id"],if id==1 {"y"}else{"n"},c["e"][0]["k"]])).collect::<Vec<_>>()})
                                    }).collect::<Vec<_>>()})
                                };
                                json!({"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}],"usage":{"prompt_tokens":50,"completion_tokens":30}})
                            }
                        } else if path.starts_with("/search/repositories") {
                            if large {
                                let url =
                                    reqwest::Url::parse(&format!("http://local{path}")).unwrap();
                                let size: usize = url
                                    .query_pairs()
                                    .find(|(k, _)| k == "per_page")
                                    .unwrap()
                                    .1
                                    .parse()
                                    .unwrap();
                                let page_count = requests
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .filter(|r| r["path"].as_str().unwrap().starts_with("/search/"))
                                    .count();
                                json!({"total_count":1000,"incomplete_results":false,"items":(0..size).map(|n|repo(((page_count-1)*50+n+1) as u64,Some("离线批量改名 Windows 中文界面 预览 撤销"),0)).collect::<Vec<_>>()})
                            } else {
                                json!({"total_count":2,"incomplete_results":false,"items":[repo(1,Some("离线批量重命名文件"),0),repo(2,Some("JavaScript learning resources"),500000)]})
                            }
                        } else {
                            if large {
                                use base64::Engine as _;
                                let readme=format!("批量改名工具。\n\n{}\n\nWindows supported.\n\n离线运行无需联网，支持中文界面。\n\nSupports preview and undo.","通用介绍与示例。".repeat(3000));
                                json!({"content":base64::engine::general_purpose::STANDARD.encode(readme),"sha":"large","html_url":"https://github.com/owner/project1/blob/main/README.md"})
                            } else {
                                json!({"content":"5pSv5oyB56a757q/5om56YeP6YeN5ZG95ZCN5paH5Lu2","sha":"abc123","html_url":"https://github.com/owner/project1/blob/main/README.md"})
                            }
                        };
                        if path.starts_with("/chat/completions") {
                            std::thread::sleep(delay);
                        }
                        let data = response.to_string();
                        let response=format!("HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{data}",data.len());
                        let _ = socket.write_all(response.as_bytes());
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    Server {
        base,
        seen,
        stop,
        thread: Some(thread),
    }
}
fn engine(base: &str) -> Engine {
    Engine {
        client: reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
        endpoints: Endpoints {
            github: base.into(),
            model: base.into(),
        },
        github_token: Some("test-github".into()),
        api_key: "test-deepseek".into(),
        model: DEFAULT_MODEL.into(),
        gate: Arc::new(tokio::sync::Mutex::new(Instant::now())),
        cache: Arc::new(Mutex::new(HashMap::new())),
        cancel: Cancellation {
            generation: Arc::new(AtomicU64::new(1)),
            id: 1,
        },
        review_limit: 20,
        budget: Arc::new(Mutex::new(ModelBudget::for_reviews(20))),
        checkpoint: None,
        snapshot: None,
        record_prefix: String::new(),
    }
}
#[test]
fn complete_session_samples_both_languages_and_judges_real_ids() {
    let server = server(false);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let progress = Mutex::new(vec![]);
    let dir = std::env::temp_dir().join(format!(
        "gh-finder-checkpoint-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(dir.clone());
    let mut e = engine(&server.base);
    e.checkpoint = Some(dir.clone());
    let result = runtime
        .block_on(e.run("中文批量重命名", 100, "test", |p| {
            progress.lock().unwrap().push(p)
        }))
        .unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.stats.assessed_count, 2);
    assert_eq!(result.stats.model_calls, 2);
    assert_eq!(result.stats.prompt_tokens, 100);
    assert_eq!(result.stats.completion_tokens, 60);
    let snapshot: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("latest.json")).unwrap()).unwrap();
    assert_eq!(snapshot["stage"], "done");
    assert_eq!(snapshot["items"].as_array().unwrap().len(), 2);
    assert_eq!(snapshot["stats"]["shortlisted_count"], 2);
    assert!(dir.join("model-request-02.json").is_file());
    assert!(dir.join("model-response-02.json").is_file());
    for entry in std::fs::read_dir(&dir).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains("test-github") && !text.contains("test-deepseek"));
    }
    assert!(result.stats.duplicates >= 2);
    assert_eq!(result.items[0].repo.stars, 0);
    assert_eq!(
        result.items[0].assessment.as_ref().unwrap().verdict,
        Verdict::Match
    );
    assert_eq!(
        result.items[1].assessment.as_ref().unwrap().verdict,
        Verdict::Irrelevant
    );
    let seen = server.seen.lock().unwrap();
    let queries: Vec<String> = seen
        .iter()
        .filter(|r| r["path"].as_str().unwrap().starts_with("/search/"))
        .map(|r| {
            reqwest::Url::parse(&format!("http://local{}", r["path"].as_str().unwrap()))
                .unwrap()
                .query_pairs()
                .find(|(k, _)| k == "q")
                .unwrap()
                .1
                .to_string()
        })
        .collect();
    assert!(queries.iter().any(|q| q.starts_with("中文批量重命名")));
    assert!(queries.iter().any(|q| q.starts_with("批量改名")));
    assert!(queries.iter().any(|q| q.starts_with("batch rename")));
    assert!(queries.iter().all(|q| q.contains("is:public")
        && !q.contains("stars:")
        && !q.contains("archived:false")));
    assert!(progress
        .lock()
        .unwrap()
        .iter()
        .any(|p| p.stage == "sampled"));
}
#[test]
fn initial_six_hundred_pool_with_six_conditions_finishes_twenty_within_wire_budget() {
    let server = server_with_options(false, Duration::ZERO, true);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let e = engine(&server.base);
    // Local transport needs no GitHub rate wait; keep the production request accounting intact.
    let result = runtime
        .block_on(e.run(
            "批量改名，Windows，离线，中文界面，预览，最好有撤销",
            600,
            "large",
            |_| {
                if let Ok(mut gate) = e.gate.try_lock() {
                    *gate = Instant::now();
                }
            },
        ))
        .unwrap();
    assert_eq!(result.items.len(), 600);
    assert_eq!(result.plan.criteria.len(), 6);
    assert!(result.plan.contract_valid);
    assert_eq!(result.stats.assessed_count, 20);
    assert_eq!(result.stats.model_calls, 5);
    assert!(result.stats.model_input_bytes <= 50_000);
    assert_eq!(result.stats.model_output_limit, 4600);
    let seen = server.seen.lock().unwrap();
    let calls: Vec<_> = seen
        .iter()
        .filter(|r| r["path"].as_str().unwrap().starts_with("/chat/"))
        .collect();
    let total: usize = calls.iter().map(|r| r["body"].to_string().len()).sum();
    assert_eq!(total, result.stats.model_input_bytes);
    for call in calls.iter().skip(1) {
        let input: Value =
            serde_json::from_str(call["body"]["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(input["criteria"].as_array().unwrap().len(), 6);
        assert_eq!(input["candidates"].as_array().unwrap().len(), 5);
        assert_eq!(call["body"]["thinking"]["type"], "disabled");
        assert_eq!(call["body"]["max_tokens"], 1000);
    }
    println!(
        "600 -> 20; {} calls; {} request bytes; {} search requests",
        result.stats.model_calls, total, result.stats.search_requests
    );
}
#[test]
fn invalid_model_key_stops_before_sampling() {
    let server = server(true);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(engine(&server.base).run("中文", 100, "test", |_| {}));
    assert_eq!(result.err().unwrap(), "DeepSeek 密钥无效");
    assert_eq!(server.seen.lock().unwrap().len(), 1);
}
#[test]
fn cancelled_session_does_not_send_new_requests() {
    let server = server(false);
    let e = engine(&server.base);
    e.cancel.generation.store(2, Ordering::Relaxed);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert_eq!(
        runtime
            .block_on(e.run("中文", 100, "test", |_| {}))
            .err()
            .unwrap(),
        "搜索已取消"
    );
    assert!(server.seen.lock().unwrap().is_empty());
}

#[test]
fn cancellation_interrupts_an_inflight_model_request() {
    let server = server_with_delay(false, Duration::from_secs(2));
    let e = engine(&server.base);
    let generation = e.cancel.generation.clone();
    let seen = server.seen.clone();
    let cancel = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(2);
        while seen.lock().unwrap().is_empty() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        generation.store(2, Ordering::Relaxed);
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let start = Instant::now();
    assert_eq!(
        runtime
            .block_on(e.run("中文", 100, "cancel", |_| {}))
            .err()
            .unwrap(),
        "搜索已取消"
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    cancel.join().unwrap();
}

fn prior_pool() -> ResearchResult {
    let mut items = Vec::new();
    let mut index = HashMap::new();
    for id in 1..=600 {
        merge_candidates(
            &mut items,
            &mut index,
            vec![repo(id, Some("离线批量重命名文件"), 0)],
            "批量改名",
            600,
        );
        let c = items.last_mut().unwrap();
        c.prefilter_score = 1.0 / id as f64;
        if id <= 20 {
            c.selected_for_ai = true;
            c.assessment = Some(Assessment {
                id,
                verdict: Verdict::Match,
                relevance: 90,
                category: "app".into(),
                reason: "已完成的判断".into(),
                matched: vec![],
                gaps: vec![],
                evidence: vec![],
                checks: vec![],
            });
        }
    }
    ResearchResult {
        query: "批量改名".into(),
        session: "current-search".into(),
        plan: intent::compile("批量改名", plan()).0,
        queries: vec!["批量改名".into()],
        items,
        stats: Stats {
            unique_count: 600,
            assessed_count: 20,
            shortlisted_count: 20,
            model_calls: 5,
            prompt_tokens: 500,
            completion_tokens: 200,
            total_model_calls: 5,
            total_prompt_tokens: 500,
            total_completion_tokens: 200,
            ..Stats::default()
        },
        warnings: vec![],
        source: "test".into(),
        sample_directory: None,
    }
}
fn continuation(base: &str, previous: &ResearchResult, n: usize) -> Engine {
    let mut e = engine(base);
    e.review_limit = n;
    e.budget = Arc::new(Mutex::new(ModelBudget::for_additional(n, &previous.stats)));
    e.snapshot = Some(Arc::new(Mutex::new(Some(previous.clone()))));
    e.record_prefix = "additional-test-".into();
    e
}
fn model_ids(server: &Server) -> Vec<u64> {
    server
        .seen
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r["path"] == "/chat/completions")
        .flat_map(|r| {
            let user: Value =
                serde_json::from_str(r["body"]["messages"][1]["content"].as_str().unwrap())
                    .unwrap();
            assert!(
                user.get("request").is_none(),
                "continuation must not regenerate queries"
            );
            user["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["id"].as_u64().unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn continuation_uses_next_twenty_without_new_search_or_rejudging_completed_projects() {
    let server = server(false);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let prior = prior_pool();
    let e = continuation(&server.base, &prior, 20);
    let first = rt.block_on(e.review_more(prior, None, |_| {})).unwrap();
    assert_eq!(first.items.len(), 600);
    assert_eq!(first.stats.assessed_count, 40);
    assert_eq!(first.stats.model_calls, 4);
    assert_eq!(first.stats.model_call_limit, 4);
    assert_eq!(first.stats.model_output_limit, 4000);
    assert_eq!(first.stats.total_model_calls, 9);
    assert_eq!(first.stats.total_prompt_tokens, 700);
    assert_eq!(first.stats.total_completion_tokens, 320);
    assert_eq!(
        first.items[0].assessment.as_ref().unwrap().reason,
        "已完成的判断"
    );
    let next = continuation(&server.base, &first, 20);
    let final_result = rt.block_on(next.review_more(first, None, |_| {})).unwrap();
    assert_eq!(final_result.stats.assessed_count, 60);
    assert_eq!(final_result.stats.total_model_calls, 13);
    assert_eq!(model_ids(&server), (21..=60).collect::<Vec<_>>());
    assert!(!server
        .seen
        .lock()
        .unwrap()
        .iter()
        .any(|r| r["path"].as_str().unwrap().starts_with("/search/")));
}

#[test]
fn explicit_tail_project_is_reviewed_alone_and_unknown_or_completed_ids_make_no_calls() {
    let server = server(false);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let prior = prior_pool();
    let e = continuation(&server.base, &prior, 1);
    let result = rt
        .block_on(e.review_more(prior, Some(600), |_| {}))
        .unwrap();
    assert!(result.items[599].assessment.is_some());
    assert_eq!(result.stats.assessed_count, 21);
    assert_eq!(result.stats.model_calls, 1);
    assert_eq!(result.stats.model_output_limit, 1000);
    assert_eq!(model_ids(&server), vec![600]);
    let before = server.seen.lock().unwrap().len();
    for id in [600, 9000] {
        let e = continuation(&server.base, &result, 1);
        assert!(rt
            .block_on(e.review_more(result.clone(), Some(id), |_| {}))
            .is_err());
    }
    assert_eq!(server.seen.lock().unwrap().len(), before);
    assert_eq!(
        additional_candidates(&result, 20, None).unwrap(),
        (20..40).collect::<Vec<_>>()
    );
}

#[test]
fn additional_budget_has_no_planning_allowance_and_keeps_prior_usage() {
    let prior = prior_pool();
    let mut budget = ModelBudget::for_additional(1, &prior.stats);
    budget.reserve(&json!({"candidate":600}), 1000).unwrap();
    assert!(budget.reserve(&json!({}), 1).is_err());
    let mut stats = Stats::default();
    budget.sync(&mut stats);
    assert_eq!(stats.model_calls, 1);
    assert_eq!(stats.total_model_calls, 6);
    assert_eq!(stats.total_prompt_tokens, 500);
}

#[test]
fn additional_rounds_save_the_full_pool_without_overwriting_previous_request_logs() {
    let server = server(false);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let dir = std::env::temp_dir().join(format!(
        "gh-finder-additional-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(dir.clone());
    std::fs::write(
        dir.join("model-request-01.json"),
        "initial request sentinel",
    )
    .unwrap();
    let mut current = prior_pool();
    current.sample_directory = Some(dir.to_string_lossy().into());
    current.warnings.push("本次分析中断：搜索已取消".into());
    for (id, prefix) in [(600, "additional-first-"), (599, "additional-second-")] {
        let mut e = continuation(&server.base, &current, 1);
        e.checkpoint = Some(dir.clone());
        e.record_prefix = prefix.into();
        current = rt
            .block_on(e.review_more(current, Some(id), |_| {}))
            .unwrap();
        assert!(dir.join(format!("{prefix}model-request-01.json")).is_file());
        assert!(dir
            .join(format!("{prefix}model-response-01.json"))
            .is_file());
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("model-request-01.json")).unwrap(),
        "initial request sentinel"
    );
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("latest.json")).unwrap()).unwrap();
    assert_eq!(saved["items"].as_array().unwrap().len(), 600);
    assert_eq!(saved["stats"]["assessed_count"], 22);
    assert_eq!(saved["stats"]["total_model_calls"], 7);
    assert!(saved["warnings"].as_array().unwrap().is_empty());
    for entry in std::fs::read_dir(&dir).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains("test-deepseek") && !text.contains("test-github"));
    }
}

#[test]
fn session_reservation_blocks_duplicate_and_stale_operations() {
    let state = Arc::new(Mutex::new(review_session::Sessions::default()));
    state.lock().unwrap().start_search("current-search", 1);
    let operation = review_session::Operation {
        sessions: state.clone(),
        id: 1,
    };
    operation.finish(Some(prior_pool()));
    state.lock().unwrap().reserve("current-search", 2).unwrap();
    assert!(state.lock().unwrap().reserve("current-search", 3).is_err());
    drop(operation); // old completion must not unlock the newer reservation
    assert!(state.lock().unwrap().ready("current-search").is_err());
    state.lock().unwrap().finish(2, None);
    assert_eq!(
        state
            .lock()
            .unwrap()
            .ready("current-search")
            .unwrap()
            .items
            .len(),
        600
    );
    state.lock().unwrap().start_search("new-search", 3);
    state.lock().unwrap().finish(2, Some(prior_pool()));
    assert!(state.lock().unwrap().ready("current-search").is_err());
    assert!(state.lock().unwrap().ready("new-search").is_err());
}

#[test]
fn continuation_ranks_a_pool_stopped_before_initial_prefilter() {
    let server = server(false);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut prior = prior_pool();
    prior.stats.shortlisted_count = 0;
    prior.stats.assessed_count = 0;
    for c in &mut prior.items {
        c.assessment = None;
        c.selected_for_ai = false;
        c.prefilter_score = 0.0;
        c.route_ranks.insert(
            "batch rename [name,description]".into(),
            if c.repo.id == 600 {
                1
            } else {
                c.repo.id as usize + 1
            },
        );
    }
    let e = continuation(&server.base, &prior, 1);
    let result = rt.block_on(e.review_more(prior, None, |_| {})).unwrap();
    assert_eq!(model_ids(&server), vec![600]);
    assert_eq!(result.stats.assessed_count, 1);
    assert!(!result.items[599].prefilter_reasons.is_empty());
}

#[test]
fn stopping_additional_review_keeps_the_completed_batch_for_resume() {
    let server = server_with_delay(false, Duration::from_millis(250));
    let prior = prior_pool();
    let e = continuation(&server.base, &prior, 20);
    let generation = e.cancel.generation.clone();
    let seen = server.seen.clone();
    let cancel = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(4);
        while seen
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r["path"] == "/chat/completions")
            .count()
            < 2
        {
            assert!(Instant::now() < until, "second batch was never sent");
            std::thread::sleep(Duration::from_millis(5));
        }
        generation.store(2, Ordering::Relaxed);
    });
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(rt.block_on(e.review_more(prior, None, |_| {})).is_err());
    cancel.join().unwrap();
    let snapshot = e
        .snapshot
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .clone()
        .unwrap();
    assert_eq!(snapshot.stats.assessed_count, 25);
    assert_eq!(snapshot.stats.total_model_calls, 7);
    assert_eq!(additional_candidates(&snapshot, 20, None).unwrap()[0], 25);
    assert!(snapshot.items[24].assessment.is_some());
}
