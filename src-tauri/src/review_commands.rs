use super::*;

/// Keep the last completed batch when a request is cancelled or times out.
pub fn retain(
    engine: &research::Engine,
    operation: &review_session::Operation,
    outcome: &Result<research::ResearchResult, String>,
) {
    let result = match outcome {
        Ok(result) => Some(result.clone()),
        Err(error) => engine
            .snapshot
            .as_ref()
            .and_then(|s| s.lock().ok()?.clone())
            .map(|mut result| {
                if let Ok(b) = engine.budget.lock() {
                    b.sync(&mut result.stats);
                }
                result.warnings.push(format!("本次分析中断：{error}"));
                let _ = engine.checkpoint_result("interrupted", &result);
                result
            }),
    };
    operation.finish(result);
}

#[tauri::command]
pub fn research_session(
    session: String,
    app: tauri::AppHandle,
) -> Result<research::ResearchResult, String> {
    app.state::<AppState>()
        .review_sessions
        .lock()
        .map_err(|_| "会话锁异常")?
        .ready(&session)
}

#[tauri::command]
pub async fn continue_research(
    session: String,
    candidate_id: Option<u64>,
    review_limit: Option<usize>,
    app: tauri::AppHandle,
) -> Result<research::ResearchResult, String> {
    let state = app.state::<AppState>();
    let limit = review_limit.unwrap_or(20).clamp(1, 30);
    let (result, selected, operation) = {
        let mut active = state.active_research.lock().map_err(|_| "会话锁异常")?;
        let mut sessions = state.review_sessions.lock().map_err(|_| "会话锁异常")?;
        let result = sessions.ready(&session)?;
        let selected = research::additional_candidates(&result, limit, candidate_id)?;
        if selected.is_empty() {
            return Ok(result);
        }
        let id = state.research_generation.fetch_add(1, Ordering::Relaxed) + 1;
        sessions.reserve(&session, id)?;
        *active = session.clone();
        (
            result,
            selected,
            review_session::Operation {
                sessions: state.review_sessions.clone(),
                id,
            },
        )
    };
    let config = model_config::load()?;
    if config.api_key.is_empty() {
        return Err("请先保存 DeepSeek API Key".into());
    }
    // Reuse only an application-owned result; never accept arbitrary repositories or a client-supplied path.
    let directory = preferences::prepare_directory(
        result
            .sample_directory
            .as_deref()
            .ok_or("当前搜索没有样本目录")?,
    )?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_nanos();
    let engine = research::Engine {
        client: state.http_client(),
        endpoints: research::Endpoints::default(),
        github_token: load_token(),
        api_key: config.api_key,
        model: config.model,
        gate: state.request_gate.clone(),
        cache: state.research_cache.clone(),
        cancel: research::Cancellation {
            generation: state.research_generation.clone(),
            id: operation.id,
        },
        review_limit: selected.len(),
        budget: Arc::new(Mutex::new(research::ModelBudget::for_additional(
            selected.len(),
            &result.stats,
        ))),
        checkpoint: Some(directory),
        snapshot: Some(Arc::new(Mutex::new(Some(result.clone())))),
        record_prefix: format!("additional-{stamp}-"),
    };
    let outcome = tokio::time::timeout(
        Duration::from_secs(900),
        engine.review_more(result, candidate_id, |p| {
            let _ = app.emit("research-progress", p);
        }),
    )
    .await
    .unwrap_or_else(|_| Err("本次分析已到 15 分钟上限，已完成的结果保留".into()));
    retain(&engine, &operation, &outcome);
    outcome
}
