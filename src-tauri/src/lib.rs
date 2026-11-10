mod lexicon;
mod model_config;
mod policy;
mod research;
use policy::retry_delay;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use lexicon::Lexicon;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

const API: &str = "https://api.github.com/search/repositories";
const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);

pub struct AppState {
    lex: Lexicon,
    client: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, Vec<Item>)>>,
    request_gate: Arc<tokio::sync::Mutex<Instant>>,
    research_generation: Arc<AtomicU64>,
    research_cache: research::PageCache,
    active_research: Mutex<String>,
    research_results: Mutex<HashMap<String, (Instant, research::ResearchResult)>>,
    search_generation: AtomicU64,
    trend_generation: AtomicU64,
    trans_cache: Mutex<HashMap<String, String>>,
}

#[derive(Clone, Serialize)]
pub struct Item {
    pub full_name: String,
    pub description: String,
    pub stars: u64,
    pub language: String,
    pub pushed_at: String,
    pub created_at: String,
    pub archived: bool,
    pub topics: Vec<String>,
    pub html_url: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub english: String,
    pub groups: Vec<Vec<String>>,
    pub keys: Vec<String>,
    pub coverage: f64,
    pub raw_cn: Vec<String>,
    pub source: String,
    pub items: Vec<Item>,
    pub error: String,
}

#[derive(Deserialize)]
struct GhResponse {
    #[serde(default)]
    items: Vec<GhItem>,
}

#[derive(Deserialize)]
struct GhItem {
    #[serde(default)]
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    stargazers_count: u64,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    pushed_at: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    topics: Option<Vec<String>>,
    #[serde(default)]
    html_url: Option<String>,
}

impl From<GhItem> for Item {
    fn from(it: GhItem) -> Self {
        let full_name = it.full_name;
        let html_url = it
            .html_url
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| format!("https://github.com/{full_name}"));
        Self {
            description: it.description.unwrap_or_default(),
            stars: it.stargazers_count,
            language: it.language.unwrap_or_default(),
            pushed_at: it.pushed_at.unwrap_or_default(),
            created_at: it.created_at.unwrap_or_default(),
            archived: it.archived,
            topics: it.topics.unwrap_or_default(),
            html_url,
            full_name,
        }
    }
}

fn token_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("GH_FINDER_TOKEN") {
        let p = p.trim();
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        // 与 tauri.conf.json 的 identifier 对齐，卸载器才能一并清理。
        out.push(
            PathBuf::from(&appdata)
                .join("com.ghfinder.desktop")
                .join("token.txt"),
        );
        // 兼容 v1.0.0 早期构建的旧路径。
        out.push(PathBuf::from(appdata).join("gh-finder").join("token.txt"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("token.txt"));
        out.push(cwd.join("..").join("token.txt"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("token.txt"));
        }
    }
    out
}

/// 把旧版 `%APPDATA%\gh-finder\token.txt` 迁到卸载器会清理的目录。
/// 只在旧文件存在、新文件不存在且复制成功时删除旧文件。
fn migrate_legacy_token() {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return;
    };
    let old_dir = PathBuf::from(&appdata).join("gh-finder");
    let old = old_dir.join("token.txt");
    let new_dir = PathBuf::from(&appdata).join("com.ghfinder.desktop");
    let new = new_dir.join("token.txt");
    if old.is_file() && !new.exists() && std::fs::create_dir_all(&new_dir).is_ok() {
        if std::fs::copy(&old, &new).is_ok() {
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::remove_dir(&old_dir);
        }
    }
}

fn load_token() -> Option<String> {
    if let Ok(t) = std::env::var("GH_FINDER_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() && !t.contains('/') && !t.contains('\\') {
            return Some(t);
        }
    }
    for p in token_paths() {
        if let Ok(s) = std::fs::read_to_string(&p) {
            let t = s.trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

fn short_error(status: u16) -> String {
    match status {
        401 => "token 无效，请检查设置".into(),
        403 | 429 => "GitHub 限速了，等一分钟再试（登录后额度更高）".into(),
        422 => "GitHub 无法理解这个查询，换个说法试试".into(),
        _ => format!("GitHub 返回 HTTP {status}"),
    }
}

// 搜索与发现共享配额和在途锁。缓存按凭据、条数、排序隔离；排队旧查询不会继续烧配额。
impl AppState {
    async fn github_items(
        &self,
        q: &str,
        limit: usize,
        sort: &str,
        ttl: Duration,
        generation: &AtomicU64,
        request_id: u64,
    ) -> Result<(Vec<Item>, bool), String> {
        let token = load_token();
        let mut scope = std::collections::hash_map::DefaultHasher::new();
        token.hash(&mut scope);
        let key = format!("{}|{limit}|{sort}|{q}", scope.finish());
        let cached = || {
            self.cache.lock().ok().and_then(|cache| {
                cache
                    .get(&key)
                    .filter(|(at, _)| at.elapsed() < ttl)
                    .map(|(_, items)| items.clone())
            })
        };
        if let Some(items) = cached() {
            return Ok((items, true));
        }
        let mut next = self.request_gate.lock().await;
        if let Some(items) = cached() {
            return Ok((items, true));
        }
        if *next > Instant::now() {
            let remaining = next.saturating_duration_since(Instant::now());
            if remaining > Duration::from_secs(7) {
                return Err(format!(
                    "GitHub 请求冷却中，请在 {} 秒后再试",
                    remaining.as_secs() + 1
                ));
            }
            tokio::time::sleep_until((*next).into()).await;
        }
        if generation.load(Ordering::Relaxed) != request_id {
            return Err("查询已被更新".into());
        }
        *next = Instant::now() + Duration::from_millis(if token.is_some() { 2100 } else { 6200 });
        let mut req = self
            .client
            .get(API)
            .query(&[
                ("q", q.to_string()),
                ("per_page", limit.to_string()),
                ("order", "desc".into()),
            ])
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if !sort.is_empty() {
            req = req.query(&[("sort", sort)]);
        }
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                "连接 GitHub 超时，检查网络或代理".into()
            } else if e.is_connect() {
                "连不上 GitHub，检查网络或代理".into()
            } else {
                format!("网络错误：{e}")
            }
        })?;
        let status = resp.status();
        if status.as_u16() == 429 || status.as_u16() == 403 {
            let delay = retry_delay(resp.headers());
            *next = Instant::now() + delay;
            return Err(format!(
                "GitHub 暂时拒绝请求，请在 {} 秒后再试",
                delay.as_secs()
            ));
        }
        if !status.is_success() {
            return Err(short_error(status.as_u16()));
        }
        let body = resp
            .json::<GhResponse>()
            .await
            .map_err(|e| format!("解析 GitHub 响应失败：{e}"))?;
        let items: Vec<Item> = body.items.into_iter().take(limit).map(Item::from).collect();
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
            if cache.len() >= 256 {
                cache.clear();
            }
            cache.insert(key, (Instant::now(), items.clone()));
        }
        Ok((items, false))
    }
}

#[tauri::command]
async fn search(query: String, limit: Option<usize>, app: tauri::AppHandle) -> SearchResponse {
    let state = app.state::<AppState>();
    let request_id = state.search_generation.fetch_add(1, Ordering::Relaxed) + 1;
    let query = query.trim().to_string();
    let limit = limit.unwrap_or(10).clamp(1, 30);

    let analysis = state.lex.analyze(&query);
    let english = lexicon::to_english(&analysis.groups);

    let mut out = SearchResponse {
        query: query.clone(),
        english: english.clone(),
        groups: analysis.groups,
        keys: analysis.keys,
        coverage: analysis.coverage,
        raw_cn: analysis.raw_cn,
        source: "github".into(),
        items: Vec::new(),
        error: String::new(),
    };

    if query.is_empty() {
        out.error = "请输入搜索内容".into();
        return out;
    }
    match state
        .github_items(
            &format!(
                "{} is:public in:name,description,readme",
                query.chars().take(220).collect::<String>()
            ),
            limit,
            "",
            CACHE_TTL,
            &state.search_generation,
            request_id,
        )
        .await
    {
        Ok((items, cached)) => {
            out.items = items;
            if cached {
                out.source = "cache".into();
            }
        }
        Err(e) => out.error = e,
    }

    out
}

#[tauri::command]
async fn smart_search(
    query: String,
    target: Option<usize>,
    refresh: Option<bool>,
    session: String,
    app: tauri::AppHandle,
) -> Result<research::ResearchResult, String> {
    let state = app.state::<AppState>();
    let id = {
        let mut active = state.active_research.lock().map_err(|_| "会话锁异常")?;
        let id = state.research_generation.fetch_add(1, Ordering::Relaxed) + 1;
        *active = session.clone();
        id
    };
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 500 {
        return Err("请输入 1–500 字的搜索需求".into());
    }
    let config = model_config::load()?;
    if config.api_key.is_empty() {
        return Err("请先在设置中保存 DeepSeek API Key，或切换原文搜索".into());
    }
    let target = target.unwrap_or(600).clamp(100, 2000);
    let github_token = load_token();
    let mut scope = std::collections::hash_map::DefaultHasher::new();
    (query, target, &config.model, &config.api_key, &github_token).hash(&mut scope);
    let cache_key = scope.finish().to_string();
    if refresh.unwrap_or(false) {
        if let Ok(mut cache) = state.research_results.lock() {
            cache.remove(&cache_key);
        }
        if let Ok(mut cache) = state.research_cache.lock() {
            cache.clear();
        }
    }
    let cached = {
        state.research_results.lock().ok().and_then(|cache| {
            cache
                .get(&cache_key)
                .filter(|(t, _)| t.elapsed() < Duration::from_secs(1800))
                .map(|(_, r)| r.clone())
        })
    };
    if let Some(mut result) = cached {
        result.session = session;
        result.source = "cache".into();
        result.stats.model_calls = 0;
        result.stats.prompt_tokens = 0;
        result.stats.completion_tokens = 0;
        return Ok(result);
    }
    let engine = research::Engine {
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("gh-finder/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| "初始化研究客户端失败")?,
        endpoints: research::Endpoints::default(),
        github_token,
        api_key: config.api_key,
        model: config.model,
        gate: state.request_gate.clone(),
        cache: state.research_cache.clone(),
        cancel: research::Cancellation {
            generation: state.research_generation.clone(),
            id,
        },
    };
    let run = engine.run(query, target, &session, |progress| {
        let _ = app.emit("research-progress", progress);
    });
    let result = tokio::time::timeout(Duration::from_secs(900), run)
        .await
        .map_err(|_| "已到 15 分钟会话上限，当前已显示的样本仍可查看".to_string())??;
    if result.warnings.is_empty() && result.stats.assessed_count == result.items.len() {
        if let Ok(mut cache) = state.research_results.lock() {
            if cache.len() >= 8 {
                cache.clear();
            }
            cache.insert(cache_key, (Instant::now(), result.clone()));
        }
    }
    Ok(result)
}

#[tauri::command]
fn cancel_smart_search(session: String, app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(active) = state.active_research.lock() {
        if *active == session {
            state.research_generation.fetch_add(1, Ordering::Relaxed);
        }
    };
}
#[tauri::command]
fn model_settings() -> Result<model_config::ModelStatus, String> {
    Ok(model_config::load()?.status())
}
#[tauri::command]
fn save_model_settings(
    api_key: String,
    model: String,
    clear_key: bool,
    app: tauri::AppHandle,
) -> Result<model_config::ModelStatus, String> {
    app.state::<AppState>()
        .research_generation
        .fetch_add(1, Ordering::Relaxed);
    model_config::save(api_key, model, clear_key)
}
#[tauri::command]
fn export_research(payload: String) -> Result<String, String> {
    if payload.len() > 16 * 1024 * 1024 {
        return Err("样本文件过大".into());
    }
    let value: serde_json::Value = serde_json::from_str(&payload).map_err(|_| "样本 JSON 无效")?;
    if !value["items"]
        .as_array()
        .map(|x| x.len() <= 2000)
        .unwrap_or(false)
    {
        return Err("样本数量无效".into());
    }
    let root = PathBuf::from(std::env::var("USERPROFILE").map_err(|_| "找不到用户目录")?)
        .join("Downloads");
    std::fs::create_dir_all(&root).map_err(|_| "创建下载目录失败")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_millis();
    let path = root.join(format!("gh-finder-v2-samples-{stamp}.json"));
    std::fs::write(&path, payload).map_err(|_| "保存样本失败")?;
    Ok(path.display().to_string())
}
#[tauri::command]
async fn test_model_connection() -> Result<String, String> {
    let config = model_config::load()?;
    if config.api_key.is_empty() {
        return Err("请先保存 DeepSeek API Key".into());
    }
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "初始化连接失败")?
        .get("https://api.deepseek.com/models")
        .timeout(Duration::from_secs(20))
        .bearer_auth(&config.api_key)
        .send()
        .await
        .map_err(|_| "连接 DeepSeek 失败，请检查网络")?;
    if !response.status().is_success() {
        return Err(format!("DeepSeek 返回 HTTP {}", response.status().as_u16()));
    }
    let value: serde_json::Value = response.json().await.map_err(|_| "模型列表格式错误")?;
    let available = value["data"]
        .as_array()
        .map(|a| a.iter().any(|m| m["id"].as_str() == Some(&config.model)))
        .unwrap_or(false);
    if available {
        Ok(format!("连接成功，{} 可用（未生成内容）", config.model))
    } else {
        Err("密钥有效，但当前账号的模型列表中没有所选模型".into())
    }
}

/// 自 1970-01-01 起的天数 → (年, 月, 日)。Howard Hinnant 的 civil_from_days。
fn ymd_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn cutoff_date(days_ago: i64) -> String {
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    let (y, m, d) = ymd_from_days(today - days_ago);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 发现页：最近创建、star 最多的新项目（GitHub 没有官方 trending API）。
#[tauri::command]
async fn trending(
    period: Option<String>,
    lang: Option<String>,
    limit: Option<usize>,
    app: tauri::AppHandle,
) -> SearchResponse {
    let state = app.state::<AppState>();
    let request_id = state.trend_generation.fetch_add(1, Ordering::Relaxed) + 1;
    let period = period.unwrap_or_else(|| "day".into());
    let (days, min_stars) = match period.as_str() {
        "week" => (7, 20),
        "month" => (30, 50),
        _ => (1, 5),
    };
    let limit = limit.unwrap_or(20).clamp(1, 30);
    let lang = lang.unwrap_or_default();

    let mut out = SearchResponse {
        query: String::new(),
        english: String::new(),
        groups: Vec::new(),
        keys: Vec::new(),
        coverage: 1.0,
        raw_cn: Vec::new(),
        source: "github".into(),
        items: Vec::new(),
        error: String::new(),
    };

    let cutoff = cutoff_date(days);
    let mut q = format!("created:>{cutoff} stars:>{min_stars} archived:false");
    if !lang.trim().is_empty() && lang != "all" {
        q.push_str(&format!(" language:{}", lang.trim()));
    }

    match state
        .github_items(
            &q,
            limit,
            "stars",
            Duration::from_secs(1800),
            &state.trend_generation,
            request_id,
        )
        .await
    {
        Ok((items, cached)) => {
            out.items = items;
            if cached {
                out.source = "cache".into();
            }
        }
        Err(e) => out.error = e,
    }

    out
}

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

async fn translate_once(client: &reqwest::Client, text: &str) -> Option<String> {
    let resp = client
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", "en"),
            ("tl", "zh-CN"),
            ("dt", "t"),
            ("q", text),
        ])
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    let segs = v.get(0)?.as_array()?;
    let mut out = String::new();
    for seg in segs {
        if let Some(t) = seg.get(0).and_then(|x| x.as_str()) {
            out.push_str(t);
        }
    }
    let out = out.trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// 免费翻译接口偶尔抖一下，失败后重试一次。
async fn translate_one(client: &reqwest::Client, text: &str) -> Option<String> {
    if let Some(v) = translate_once(client, text).await {
        return Some(v);
    }
    tokio::time::sleep(Duration::from_millis(350)).await;
    translate_once(client, text).await
}

/// 批量翻译仓库描述。永远不阻塞首屏：前端先把英文画出来，译文到了再替换。
#[tauri::command]
async fn translate(texts: Vec<String>, app: tauri::AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let texts: Vec<String> = texts.into_iter().take(30).collect();
    let mut out = texts.clone();
    let mut pending: Vec<usize> = Vec::new();

    for (i, t) in texts.iter().enumerate() {
        if t.trim().is_empty() || has_cjk(t) {
            continue;
        }
        let cached = state
            .trans_cache
            .lock()
            .ok()
            .and_then(|c| c.get(t).cloned());
        if let Some(zh) = cached {
            out[i] = zh;
        } else {
            pending.push(i);
        }
    }

    if pending.is_empty() {
        return out;
    }

    // 限流并发：一次最多 6 个请求，免费接口不容易被掐。
    let client = state.client.clone();
    let jobs: Vec<(usize, String)> = pending.iter().map(|&i| (i, texts[i].clone())).collect();
    let results: Vec<(usize, Option<String>)> =
        futures_util::stream::iter(jobs.into_iter().map(|(i, text)| {
            let client = client.clone();
            async move { (i, translate_one(&client, &text).await) }
        }))
        .buffer_unordered(6)
        .collect()
        .await;

    for (i, zh) in results {
        if let Some(zh) = zh {
            out[i] = zh.clone();
            if let Ok(mut cache) = state.trans_cache.lock() {
                if cache.len() >= 2000 {
                    cache.clear();
                }
                cache.insert(texts[i].clone(), zh);
            }
        }
    }
    out
}

#[tauri::command]
fn token_status() -> bool {
    load_token().is_some()
}

#[tauri::command]
fn save_token(token: String) -> Result<bool, String> {
    let t = token.trim();
    if t.is_empty() {
        return Ok(false);
    }
    let appdata = std::env::var("APPDATA").map_err(|_| "找不到 APPDATA 目录".to_string())?;
    let dir = PathBuf::from(appdata).join("com.ghfinder.desktop");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败：{e}"))?;
    std::fs::write(dir.join("token.txt"), t).map_err(|e| format!("写入 token 失败：{e}"))?;
    Ok(true)
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    policy::validate_github_url(&url)?;
    open::that(url).map_err(|e| format!("打不开浏览器：{e}"))
}

#[tauri::command]
fn win_minimize(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn win_close(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    migrate_legacy_token();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("gh-finder/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_default();

    tauri::Builder::default()
        .manage(AppState {
            lex: Lexicon::load(),
            client,
            cache: Mutex::new(HashMap::new()),
            request_gate: Arc::new(tokio::sync::Mutex::new(Instant::now())),
            research_generation: Arc::new(AtomicU64::new(0)),
            research_cache: Arc::new(Mutex::new(HashMap::new())),
            active_research: Mutex::new(String::new()),
            research_results: Mutex::new(HashMap::new()),
            search_generation: AtomicU64::new(0),
            trend_generation: AtomicU64::new(0),
            trans_cache: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            search,
            smart_search,
            cancel_smart_search,
            model_settings,
            save_model_settings,
            test_model_connection,
            export_research,
            trending,
            translate,
            token_status,
            save_token,
            open_url,
            win_minimize,
            win_close
        ])
        .run(tauri::generate_context!())
        .expect("gh-finder 启动失败");
}
