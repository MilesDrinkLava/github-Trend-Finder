#[allow(dead_code)]
#[path = "../src/connections.rs"]
mod connections;
#[path = "../src/policy.rs"]
mod policy;
#[allow(dead_code)]
#[path = "../src/preferences.rs"]
mod preferences;
#[allow(dead_code)]
#[path = "../src/prefilter.rs"]
mod prefilter;
#[allow(dead_code)]
#[path = "../src/research.rs"]
mod research;
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gh-finder-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn default_budget_is_five_calls_and_4600_output_tokens() {
    let mut b = research::ModelBudget::for_reviews(20);
    b.reserve(&json!({"plan":"中文需求"}), 600).unwrap();
    for _ in 0..4 {
        b.reserve(&json!({"five_candidates":true}), 1000).unwrap();
    }
    assert_eq!(b.calls, 5);
    assert_eq!(b.output_reserved, 4600);
    assert!(b.reserve(&json!({}), 1).is_err());
    assert_eq!(b.calls, 5);
}
#[test]
fn excessive_input_is_rejected_before_reserving_a_call() {
    let mut b = research::ModelBudget::for_reviews(20);
    assert!(b
        .reserve(&json!({"text":"中".repeat(20_000)}), 1000)
        .is_err());
    assert_eq!(b.calls, 0);
    assert_eq!(b.input_bytes, 0);
}
#[test]
fn six_hundred_candidates_only_select_twenty_and_keep_zero_star_chinese() {
    let mut pool = Vec::new();
    let mut index = HashMap::new();
    for id in 1..=600 {
        let repo=serde_json::from_value(json!({"id":id,"full_name":format!("owner/repo{id}"),"description":"中文文件工具","stargazers_count":if id==1{0}else{100000}})).unwrap();
        research::merge_candidates(
            &mut pool,
            &mut index,
            vec![repo],
            "中文文件 [name,description]",
            600,
        );
        pool.last_mut()
            .unwrap()
            .route_ranks
            .insert("中文文件 [name,description]".into(), id as usize);
    }
    let chosen = prefilter::choose(&mut pool, 20, "中文文件工具");
    assert_eq!(chosen.len(), 20);
    assert_eq!(pool.len(), 600);
    assert!(chosen.contains(&0));
    assert_eq!(pool.iter().filter(|c| c.selected_for_ai).count(), 20);
}
#[test]
fn direct_github_matches_beat_readme_only_noise_without_using_stars() {
    let mut pool = Vec::new();
    let mut index = HashMap::new();
    for id in 1..=30 {
        let r=serde_json::from_value(json!({"id":id,"full_name":format!("owner/r{id}"),"description":"file rename","stargazers_count":0})).unwrap();
        research::merge_candidates(
            &mut pool,
            &mut index,
            vec![r],
            "file rename [name,description]",
            40,
        );
        pool.last_mut()
            .unwrap()
            .route_ranks
            .insert("file rename [name,description]".into(), id as usize);
    }
    let r=serde_json::from_value(json!({"id":99,"full_name":"popular/everything","description":"API manager","stargazers_count":500000})).unwrap();
    research::merge_candidates(&mut pool, &mut index, vec![r], "批量改名 [readme]", 40);
    pool.last_mut()
        .unwrap()
        .route_ranks
        .insert("批量改名 [readme]".into(), 1);
    prefilter::choose(&mut pool, 20, "批量改名");
    assert!(pool[0].prefilter_score > pool[30].prefilter_score);
}
#[test]
fn excerpts_find_later_matching_paragraphs_and_never_split_utf8() {
    let s = format!(
        "介绍\n\n{}\n\n支持离线批量重命名文件，并提供预览。",
        "无关说明".repeat(500)
    );
    let x = prefilter::excerpt(&s, "离线批量重命名", 180);
    assert!(x.contains("离线批量重命名"));
    assert!(x.len() <= 180);
}
#[test]
fn directory_and_network_preferences_round_trip() {
    let dir = Temp::new();
    let file = dir.0.join("config/preferences.json");
    let mut p = preferences::Preferences::default();
    p.sample_dir = dir.0.join("samples").to_string_lossy().into();
    p.proxy_mode = preferences::ProxyMode::Custom;
    p.proxy_url = "http://127.0.0.1:7897".into();
    preferences::save_to(&file, &p).unwrap();
    let q = preferences::load_from(&file).unwrap();
    assert_eq!(q.proxy_mode, preferences::ProxyMode::Custom);
    assert_eq!(q.proxy_url, p.proxy_url);
    assert_eq!(q.sample_dir, p.sample_dir);
    assert!(preferences::session_directory(&q.sample_dir)
        .unwrap()
        .is_dir());
    assert!(preferences::prepare_directory("relative/path").is_err());
}
#[test]
fn invalid_proxy_is_not_silently_used() {
    let mut p = preferences::Preferences::default();
    p.proxy_mode = preferences::ProxyMode::Custom;
    for address in [
        "garbage",
        "socks5://127.0.0.1:7897",
        "http://user:secret@localhost:7897",
    ] {
        p.proxy_url = address.into();
        assert!(preferences::client(&p).is_err());
    }
}
#[test]
fn configured_http_proxy_really_receives_the_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(4);
        loop {
            if let Ok((mut s, _)) = listener.accept() {
                s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut buf = [0; 8192];
                let n = s.read(&mut buf).unwrap();
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                    .unwrap();
                return request;
            }
            if Instant::now() > until {
                panic!("configured proxy received no request");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let mut p = preferences::Preferences::default();
    p.proxy_mode = preferences::ProxyMode::Custom;
    p.proxy_url = format!("http://{address}");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(rt
        .block_on(async {
            preferences::client(&p)
                .unwrap()
                .get("http://example.invalid/probe")
                .send()
                .await
        })
        .unwrap()
        .status()
        .is_success());
    assert!(worker
        .join()
        .unwrap()
        .starts_with("GET http://example.invalid/probe"));
}
#[test]
fn github_connection_checks_real_profile_response() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut buf = [0; 8192];
        let n = s.read(&mut buf).unwrap();
        let input = String::from_utf8_lossy(&buf[..n]);
        assert!(input.starts_with("GET /user"));
        assert!(input
            .to_lowercase()
            .contains("authorization: bearer fake-test-token"));
        let body = b"{\"login\":\"verified-user\"}";
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        s.write_all(body).unwrap();
    });
    let mut p = preferences::Preferences::default();
    p.proxy_mode = preferences::ProxyMode::Direct;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let name = rt
        .block_on(connections::profile(
            &preferences::client(&p).unwrap(),
            &format!("http://{address}"),
            "fake-test-token",
        ))
        .unwrap();
    assert_eq!(name, "verified-user");
    worker.join().unwrap();
}
