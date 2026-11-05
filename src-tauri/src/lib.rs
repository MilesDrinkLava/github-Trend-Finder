mod lexicon;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use lexicon::Lexicon;
use serde::{Deserialize, Serialize};
use tauri::Manager;

const API: &str = "https://api.github.com/search/repositories";
const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);

pub struct AppState {
    lex: Lexicon,
    client: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, Vec<Item>)>>,
    trend_cache: Mutex<HashMap<String, (Instant, Vec<Item>)>>,
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

#[tauri::command]
async fn search(
    query: String,
    limit: Option<usize>,
    app: tauri::AppHandle,
) -> SearchResponse {
    let state = app.state::<AppState>();
    let query = query.trim().to_string();
    let limit = limit.unwrap_or(10).clamp(1, 30);

    let analysis = state.lex.analyze(&query);
    let english = lexicon::to_english(&analysis.groups);

    let mut out = SearchResponse {
        query: query.clone(),
        english: english.clone(),
        groups: analysis.groups,
        raw_cn: analysis.raw_cn,
        source: "github".into(),
        items: Vec::new(),
        error: String::new(),
    };

    if english.is_empty() {
        out.error = "没认出中文里的关键词，换个说法试试".into();
        return out;
    }

    let cache_key = format!("{english} | archived:false");
    if let Ok(cache) = state.cache.lock() {
        if let Some((at, items)) = cache.get(&cache_key) {
            if at.elapsed() < CACHE_TTL {
                out.items = items.iter().take(limit).cloned().collect();
                out.source = "cache".into();
                return out;
            }
        }
    }

    let mut req = state
        .client
        .get(API)
        .query(&[
            ("q", format!("{english} archived:false")),
            ("per_page", limit.to_string()),
            ("order", "desc".to_string()),
        ])
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if let Some(token) = load_token() {
        req = req.bearer_auth(token);
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                out.error = short_error(status.as_u16());
                return out;
            }
            match resp.json::<GhResponse>().await {
                Ok(body) => {
                    let items: Vec<Item> = body.items.into_iter().take(limit).map(Item::from).collect();
                    if let Ok(mut cache) = state.cache.lock() {
                        cache.insert(cache_key, (Instant::now(), items.clone()));
                    }
                    out.items = items;
                }
                Err(e) => out.error = format!("解析 GitHub 响应失败：{e}"),
            }
        }
        Err(e) => {
            out.error = if e.is_timeout() {
                "连接 GitHub 超时，检查网络或代理".into()
            } else if e.is_connect() {
                "连不上 GitHub，检查网络或代理".into()
            } else {
                format!("网络错误：{e}")
            };
        }
    }

    out
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

    let cache_key = format!("trend|{q}");
    if let Ok(cache) = state.trend_cache.lock() {
        if let Some((at, items)) = cache.get(&cache_key) {
            if at.elapsed() < Duration::from_secs(1800) {
                out.items = items.iter().take(limit).cloned().collect();
                out.source = "cache".into();
                return out;
            }
        }
    }

    let mut req = state
        .client
        .get(API)
        .query(&[
            ("q", q),
            ("sort", "stars".to_string()),
            ("order", "desc".to_string()),
            ("per_page", limit.to_string()),
        ])
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if let Some(token) = load_token() {
        req = req.bearer_auth(token);
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                out.error = short_error(status.as_u16());
                return out;
            }
            match resp.json::<GhResponse>().await {
                Ok(body) => {
                    let items: Vec<Item> =
                        body.items.into_iter().take(limit).map(Item::from).collect();
                    if let Ok(mut cache) = state.trend_cache.lock() {
                        cache.insert(cache_key, (Instant::now(), items.clone()));
                    }
                    out.items = items;
                }
                Err(e) => out.error = format!("解析 GitHub 响应失败：{e}"),
            }
        }
        Err(e) => {
            out.error = if e.is_timeout() {
                "连接 GitHub 超时，检查网络或代理".into()
            } else if e.is_connect() {
                "连不上 GitHub，检查网络或代理".into()
            } else {
                format!("网络错误：{e}")
            };
        }
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
    if !url.starts_with("https://github.com/") {
        return Err("只允许打开 GitHub 链接".into());
    }
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
        .user_agent("gh-finder/1.0")
        .build()
        .unwrap_or_default();

    tauri::Builder::default()
        .manage(AppState {
            lex: Lexicon::load(),
            client,
            cache: Mutex::new(HashMap::new()),
            trend_cache: Mutex::new(HashMap::new()),
            trans_cache: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            search,
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
