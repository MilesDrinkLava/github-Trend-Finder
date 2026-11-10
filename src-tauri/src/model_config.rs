use crate::research::DEFAULT_MODEL;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
}
fn default_model() -> String {
    DEFAULT_MODEL.into()
}
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_model(),
        }
    }
}
#[derive(Serialize)]
pub struct ModelStatus {
    pub configured: bool,
    pub model: String,
}
impl ModelConfig {
    pub fn status(&self) -> ModelStatus {
        ModelStatus {
            configured: !self.api_key.is_empty(),
            model: self.model.clone(),
        }
    }
}
fn path() -> Result<PathBuf, String> {
    Ok(
        PathBuf::from(std::env::var("APPDATA").map_err(|_| "找不到本机配置目录")?)
            .join("com.ghfinder.desktop")
            .join("deepseek.json"),
    )
}
pub fn load() -> Result<ModelConfig, String> {
    let p = path()?;
    let mut config = if p.exists() {
        serde_json::from_slice::<ModelConfig>(&std::fs::read(p).map_err(|_| "读取模型设置失败")?)
            .map_err(|_| "模型设置文件损坏，请重新保存")?
    } else {
        ModelConfig::default()
    };
    if config.api_key.is_empty() {
        config.api_key = std::env::var("DEEPSEEK_API_KEY")
            .unwrap_or_default()
            .trim()
            .into();
    }
    Ok(config)
}
pub fn save(key: String, model: String, clear: bool) -> Result<ModelStatus, String> {
    let mut config = load().unwrap_or_default();
    let key = key.trim();
    let model = model.trim();
    if !["deepseek-flash", "deepseek-v4-pro"].contains(&model) {
        return Err("请选择支持的 DeepSeek 模型".into());
    }
    if key.chars().any(char::is_control) || key.len() > 512 {
        return Err("API Key 格式无效".into());
    }
    config.model = model.into();
    if clear {
        config.api_key.clear();
    } else if !key.is_empty() {
        config.api_key = key.into();
    }
    let p = path()?;
    std::fs::create_dir_all(p.parent().ok_or("配置路径无效")?).map_err(|_| "创建配置目录失败")?;
    let content = serde_json::to_vec_pretty(&config).map_err(|_| "编码模型设置失败")?;
    std::fs::write(p, content).map_err(|_| "保存模型设置失败")?;
    Ok(config.status())
}
