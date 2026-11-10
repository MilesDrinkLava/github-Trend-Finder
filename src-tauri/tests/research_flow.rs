#[path = "../src/policy.rs"]
mod policy;
#[path = "../src/research.rs"]
mod research;
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
                        socket
                            .set_read_timeout(Some(Duration::from_secs(5)))
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
                                    serde_json::to_value(plan()).unwrap()
                                } else {
                                    json!({"items":input["candidates"].as_array().unwrap().iter().map(|c|{
                            let id=c["id"].as_u64().unwrap();
                            if id==1 {json!({"id":id,"verdict":"match","relevance":99,"category":"app","reason":"中文原生离线改名工具","evidence":[{"source":"description","quote":"离线批量重命名文件"}]})}
                            else {json!({"id":id,"verdict":"irrelevant","relevance":1,"category":"resource","reason":"是教程集合，不提供改名功能","evidence":[]})}
                        }).collect::<Vec<_>>()})
                                };
                                json!({"choices":[{"finish_reason":"stop","message":{"content":output.to_string()}}],"usage":{"prompt_tokens":50,"completion_tokens":30}})
                            }
                        } else if path.starts_with("/search/repositories") {
                            json!({"total_count":2,"incomplete_results":false,"items":[repo(1,Some("离线批量重命名文件"),0),repo(2,Some("JavaScript learning resources"),500000)]})
                        } else {
                            json!({"content":"5pSv5oyB56a757q/5om56YeP6YeN5ZG95ZCN5paH5Lu2","sha":"abc123","html_url":"https://github.com/owner/project1/blob/main/README.md"})
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
    let result = runtime
        .block_on(
            engine(&server.base).run("中文批量重命名", 100, "test", |p| {
                progress.lock().unwrap().push(p)
            }),
        )
        .unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.stats.assessed_count, 2);
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
