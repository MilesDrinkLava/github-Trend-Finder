use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    #[default]
    System,
    Direct,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub proxy_mode: ProxyMode,
    pub proxy_url: String,
    pub sample_dir: String,
    pub github_disconnected: bool,
    pub github_login: Option<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        Self {
            proxy_mode: ProxyMode::System,
            proxy_url: "http://127.0.0.1:7897".into(),
            sample_dir: home
                .join("Documents")
                .join("GitHub Trend Finder")
                .join("Samples")
                .to_string_lossy()
                .into(),
            github_disconnected: false,
            github_login: None,
        }
    }
}
pub fn config_dir() -> Result<PathBuf, String> {
    Ok(
        PathBuf::from(std::env::var("APPDATA").map_err(|_| "找不到本机配置目录")?)
            .join("com.ghfinder.desktop"),
    )
}
pub fn load_from(path: &Path) -> Result<Preferences, String> {
    if !path.exists() {
        return Ok(Preferences::default());
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|_| "无法读取应用设置")?)
        .map_err(|_| "应用设置文件损坏，请重新保存".into())
}
pub fn load() -> Result<Preferences, String> {
    load_from(&config_dir()?.join("preferences.json"))
}

pub fn validate_proxy(p: &Preferences) -> Result<(), String> {
    if p.proxy_mode == ProxyMode::Custom {
        let url = reqwest::Url::parse(p.proxy_url.trim())
            .map_err(|_| "代理地址格式无效，例如 http://127.0.0.1:7897")?;
        if !["http", "https"].contains(&url.scheme())
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("目前支持不带用户名密码的 HTTP/HTTPS 代理地址".into());
        }
    }
    Ok(())
}
pub fn client(p: &Preferences) -> Result<reqwest::Client, String> {
    validate_proxy(p)?;
    let builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("gh-finder/", env!("CARGO_PKG_VERSION")));
    let builder = match p.proxy_mode {
        ProxyMode::Direct => builder.no_proxy(),
        ProxyMode::System => builder,
        ProxyMode::Custom => builder
            .no_proxy()
            .proxy(reqwest::Proxy::all(p.proxy_url.trim()).map_err(|_| "无法创建代理连接")?),
    };
    builder.build().map_err(|_| "初始化网络连接失败".into())
}
pub fn prepare_directory(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() {
        return Err("请选择完整的样本目录路径".into());
    }
    std::fs::create_dir_all(&p).map_err(|_| "无法创建样本目录，请检查位置和权限")?;
    if !p.is_dir() {
        return Err("样本位置必须是文件夹".into());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_nanos();
    let probe = p.join(format!(".gh-finder-write-test-{stamp}"));
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
        .map_err(|_| "样本目录不可写，请换一个目录")?;
    std::fs::remove_file(probe).map_err(|_| "无法完成样本目录写入检查")?;
    Ok(p)
}
pub fn store_to(path: &Path, p: &Preferences) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("配置路径无效")?)
        .map_err(|_| "创建设置目录失败")?;
    std::fs::write(
        path,
        serde_json::to_vec_pretty(p).map_err(|_| "设置编码失败")?,
    )
    .map_err(|_| "保存设置失败".into())
}
pub fn save_to(path: &Path, p: &Preferences) -> Result<(), String> {
    validate_proxy(p)?;
    prepare_directory(&p.sample_dir)?;
    store_to(path, p)
}
pub fn save(p: &Preferences) -> Result<(), String> {
    save_to(&config_dir()?.join("preferences.json"), p)
}
pub fn store(p: &Preferences) -> Result<(), String> {
    store_to(&config_dir()?.join("preferences.json"), p)
}
pub fn session_directory(base: &str) -> Result<PathBuf, String> {
    let root = prepare_directory(base)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_nanos();
    let path = root.join(format!("search-{stamp}"));
    std::fs::create_dir(&path).map_err(|_| "无法创建本次搜索的样本目录")?;
    Ok(path)
}

#[cfg(windows)]
pub fn choose_directory() -> Result<Option<String>, String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST,
        FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|_| "无法初始化文件夹选择窗口")?;
        let _com = Com;
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|_| "无法打开文件夹选择窗口")?;
        dialog
            .SetOptions(FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR)
            .map_err(|_| "无法设置文件夹选择窗口")?;
        dialog
            .SetTitle(windows::core::w!("选择样本保存位置"))
            .map_err(|_| "无法设置窗口标题")?;
        if let Err(e) = dialog.Show(None) {
            if e.code().0 as u32 == 0x800704c7 {
                return Ok(None);
            }
            return Err("无法显示文件夹选择窗口".into());
        }
        let item = dialog.GetResult().map_err(|_| "无法读取所选目录")?;
        let display = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|_| "无法读取目录路径")?;
        let result = display.to_string().map_err(|_| "目录路径编码无效");
        CoTaskMemFree(Some(display.0.cast()));
        Ok(Some(result?))
    }
}
#[cfg(not(windows))]
pub fn choose_directory() -> Result<Option<String>, String> {
    Err("请在目录输入框填写完整路径".into())
}
