use serde::Serialize;
#[derive(Serialize)]
pub struct GithubStatus {
    pub connected: bool,
    pub login: Option<String>,
}

pub async fn profile(client: &reqwest::Client, base: &str, token: &str) -> Result<String, String> {
    if token.trim().is_empty() || token.len() > 512 || token.chars().any(char::is_control) {
        return Err("请填写有效的 GitHub Token".into());
    }
    let response = client
        .get(format!("{base}/user"))
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "无法连接 GitHub，请检查网络或代理")?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 => "GitHub Token 无效或已失效".into(),
            403 => "GitHub 拒绝读取账号信息，请检查令牌权限或限流状态".into(),
            n => format!("GitHub 返回 HTTP {n}"),
        });
    }
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "无法读取 GitHub 账号信息")?;
    value["login"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() < 100)
        .map(str::to_string)
        .ok_or("GitHub 未返回账号名称".into())
}
