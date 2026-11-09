use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn retry_delay(headers: &reqwest::header::HeaderMap) -> Duration {
    let seconds = |name: &str| headers.get(name)?.to_str().ok()?.parse::<u64>().ok();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let wait = seconds("retry-after")
        .or_else(|| seconds("x-ratelimit-reset").map(|reset| reset.saturating_sub(now) + 1))
        .unwrap_or(60)
        .clamp(1, 3600);
    Duration::from_secs(wait)
}

pub fn validate_github_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "链接无效".to_string())?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
    {
        return Err("只允许打开 GitHub 链接".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_header_is_respected_and_bounded() {
        let mut headers = reqwest::header::HeaderMap::new();
        assert_eq!(retry_delay(&headers).as_secs(), 60);
        headers.insert("retry-after", "120".parse().unwrap());
        assert_eq!(retry_delay(&headers).as_secs(), 120);
        headers.insert("retry-after", "0".parse().unwrap());
        assert_eq!(retry_delay(&headers).as_secs(), 1);
        headers.insert("retry-after", "invalid".parse().unwrap());
        assert_eq!(retry_delay(&headers).as_secs(), 60);
    }

    #[test]
    fn rejects_non_github_destinations() {
        for url in [
            "https://example.com",
            "https://github.com.evil.test/a",
            "file:///tmp/x",
            "https://github.com@evil.test/a",
        ] {
            assert!(validate_github_url(url).is_err(), "{url}");
        }
    }
}
