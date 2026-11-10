use super::*;

#[tauri::command]
pub fn app_preferences() -> Result<preferences::Preferences, String> {
    preferences::load()
}

#[tauri::command]
pub fn save_app_preferences(
    proxy_mode: preferences::ProxyMode,
    proxy_url: String,
    sample_dir: String,
    app: tauri::AppHandle,
) -> Result<preferences::Preferences, String> {
    let mut p = preferences::load().unwrap_or_default();
    let changed = p.proxy_mode != proxy_mode || p.proxy_url != proxy_url;
    p.proxy_mode = proxy_mode;
    p.proxy_url = proxy_url.trim().into();
    p.sample_dir = sample_dir.trim().into();
    let client = preferences::client(&p)?;
    preferences::save(&p)?;
    let state = app.state::<AppState>();
    *state.client.lock().map_err(|_| "网络设置锁异常")? = client;
    if changed {
        state.research_generation.fetch_add(1, Ordering::Relaxed);
        state.search_generation.fetch_add(1, Ordering::Relaxed);
        state.trend_generation.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut c) = state.cache.lock() {
            c.clear();
        }
        if let Ok(mut c) = state.research_cache.lock() {
            c.clear();
        }
    }
    Ok(p)
}
#[tauri::command]
pub async fn choose_sample_directory() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::spawn(preferences::choose_directory)
            .join()
            .map_err(|_| "文件夹窗口关闭异常".to_string())?
    })
    .await
    .map_err(|_| "文件夹选择任务异常".to_string())?
}
#[tauri::command]
pub fn open_samples_folder() -> Result<(), String> {
    let dir = preferences::prepare_directory(&preferences::load()?.sample_dir)?;
    open::that(dir).map_err(|_| "无法打开样本文件夹".into())
}
#[tauri::command]
pub async fn test_network(
    proxy_mode: preferences::ProxyMode,
    proxy_url: String,
) -> Result<String, String> {
    let mut p = preferences::Preferences::default();
    p.proxy_mode = proxy_mode;
    p.proxy_url = proxy_url;
    let response = preferences::client(&p)?
        .get("https://api.github.com/rate_limit")
        .send()
        .await
        .map_err(|_| "连接失败，请检查代理是否运行、地址和端口是否正确")?;
    if response.status().is_success() {
        Ok("网络连接正常，GitHub 可以访问".into())
    } else {
        Err(format!(
            "连接已到达 GitHub，但返回 HTTP {}",
            response.status().as_u16()
        ))
    }
}
#[tauri::command]
pub fn github_status() -> connections::GithubStatus {
    let p = preferences::load().unwrap_or_default();
    connections::GithubStatus {
        connected: load_token().is_some(),
        login: p.github_login,
    }
}
#[tauri::command]
pub async fn connect_github(
    token: String,
    app: tauri::AppHandle,
) -> Result<connections::GithubStatus, String> {
    let token = if token.trim().is_empty() {
        load_token().ok_or("请填写 GitHub Token")?
    } else {
        token.trim().to_string()
    };
    let login = connections::profile(
        &app.state::<AppState>().http_client(),
        "https://api.github.com",
        &token,
    )
    .await?;
    save_token(token)?;
    let mut p = preferences::load().unwrap_or_default();
    p.github_disconnected = false;
    p.github_login = Some(login.clone());
    preferences::store(&p)?;
    Ok(connections::GithubStatus {
        connected: true,
        login: Some(login),
    })
}
#[tauri::command]
pub fn disconnect_github(app: tauri::AppHandle) -> Result<connections::GithubStatus, String> {
    let mut p = preferences::load().unwrap_or_default();
    p.github_disconnected = true;
    p.github_login = None;
    preferences::store(&p)?;
    let key = preferences::config_dir()?.join("token.txt");
    if key.is_file() {
        std::fs::remove_file(key).map_err(|_| "已断开，但无法删除本机旧 Token 文件")?;
    }
    app.state::<AppState>()
        .research_generation
        .fetch_add(1, Ordering::Relaxed);
    Ok(connections::GithubStatus {
        connected: false,
        login: None,
    })
}
