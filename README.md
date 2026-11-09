# GitHub Trend & Finder

当前版本：**V1.1**（内部版本 `1.1.0`）；上一版为 **V1.0**（`1.0.0`）。版本记录见 [CHANGELOG.md](CHANGELOG.md)。

> 用中文搜 GitHub 开源项目。只补中文到英文的术语映射，搜索与排序全部交回 GitHub。

![搜索界面](docs/screenshot-search.png)

## 为什么做

GitHub 原生搜索对中文几乎不可用，命中效果不好。例如：同样一条需求：

| 输入 | 结果 |
|---|---|
| `图片压缩` | 基本找不到对应的开源项目 |
| `image compression` | 前列就是正确的那几个 |

问题不在 GitHub 的索引，而在**中文意图到英文关键词**这一步。所以，本项目只做这层翻译：中文 → 英文术语 → GitHub Search API，
之所以不自己重排结果是因为——Phase 0 的盲评证明：未精确调试的自研排序目前是负收益（详见 [docs/phase0.md](docs/phase0.md)）。

## 功能

- **中文搜索**：例如-输入「把视频转成 gif 的工具」，自动构词为 `video convert gif` 并附带 `archived:false`
- **术语与字形映射**：380 个概念、788 条英文术语，支持常见繁体写法、全角输入和 `C++ / C# / .NET / JS / TS / k8s` 等技术名；`C++` 会转为 `cpp`，不会丢成 `c`
- **异步翻译**：结果先以英文出现，中文译文随后淡入替换，不阻塞列表（当然也可以选择关闭翻译）
- **发现页**：按今日 / 本周 / 本月筛选近期创建、按 star 排序的项目，并且支持语言过滤
- **两种主题**：跟随系统 / 浅色 / 深色
- **无边框窗口**：自绘标题栏、圆角、透明窗口，做到尽可能丝滑且简洁的UI
- **可选 token**：未配置时 10 次/分钟，配置 PAT 后 30 次/分钟（token 只存本机）
- **共享限流**：搜索与发现共用请求队列，按匿名 / 认证状态节流，遵循 GitHub 限流冷却；排队的过期搜索会被跳过
- **键盘操作**：`↑` `↓` 选择，`Enter` 在浏览器打开

![设置界面](docs/screenshot-settings.png)

## 安装

Windows 10 / 11 x64：

1. 从 [Releases](https://github.com/MilesDrinkLava/github-Trend-Finder/releases) 下载 `*-setup.exe`
2. 双击安装（NSIS，默认装到 `%LOCALAPPDATA%`）
3. 桌面或开始菜单启动

> 应用未签名，SmartScreen 可能提示「未知发布者」，选择「更多信息 → 仍要运行」即可。

## 使用

| 操作 | 快捷键 |
|---|---|
| 选择结果 | `↑` / `↓` |
| 在浏览器打开 | `Enter`（或双击） |
| 切换主题 | 侧栏底部太阳图标 |

**提高请求配额**：在 GitHub → Settings → Developer settings → Personal access tokens 生成一个 **不需要任何 scope** 的 token，粘贴到设置页保存即可。

## 工作原理

```
中文输入「把视频转成 gif 的工具」
   ↓  词表最长匹配 + 同义词展开
[video] [convert] [gif]
   ↓  概念组合塌缩 / 弱概念裁剪（通常压到 3 个词）
"video convert gif" + archived:false
   ↓  GitHub Search API（best match）
结果先渲染 → 并发 6 路调用翻译 → 译文逐条替换
```

## 从源码构建

前端是单文件 `dist/index.html`，由 Rust 在编译时嵌入，不需要 Node。

```bash
# 1. 安装 Rust（stable）与 Tauri CLI
cargo install tauri-cli --version "^2"

# 2. 构建
cd src-tauri
cargo tauri build
```

Windows 上用 MSVC 工具链即可。开发环境也可以使用 GNU 宿主工具链（`stable-x86_64-pc-windows-gnu` + MinGW-w64）。

构建前检查词库是否同步：`python export_lexicon.py --check`。修改 `lexicon.py` 后运行 `python export_lexicon.py` 更新桌面端嵌入的 JSON。`build.bat` 会检查同步状态。

`build.bat` 会优先读取环境变量 `GH_FINDER_TOOLS_DIR`，其次检查仓库同级的 `tools` 目录（即 `..\tools`）；都没有时直接使用系统 PATH 里的 Rust / Tauri CLI。

> 如果 NSIS 下载超时，可手动准备离线缓存：把 `nsis-3.13.zip` 解压，将解压目录里的内容直接放到 `src-tauri/target/.tauri/NSIS/`（确保 `NSIS.exe` 就在这一层），再把 `nsis_tauri_utils.dll` 放到 `src-tauri/target/.tauri/NSIS/Plugins/x86-unicode/additional/`，并保留 `tauri.conf.json` 里的 `"useLocalToolsDir": true`。

## 目录结构

| 路径 | 作用 |
|---|---|
| `dist/index.html` | 前端全部 UI / 交互 / 样式（单文件） |
| `src-tauri/` | Tauri 2 后端：搜索、发现、翻译、token、窗口控制 |
| `lexicon.py` / `expand.py` | 中文词表与查询扩展（Phase 0 的 Python 实现） |
| `remote.py` / `cli.py` | 命令行版搜索入口 |
| `reports/` | Phase 0 盲评结果 |
| `docs/phase0.md` | 早期工程笔记与踩坑记录 |
| `app/` | 浏览器预览服务（开发用） |

## 已知限制

- 仅提供 Windows 安装包
- 网络代理目前跟随系统设置，自定义代理为预留项
- 翻译使用 Google 免费接口，可能受网络或限流影响
- 登录方式目前为手动 PAT，OAuth 为预留项
- 未做自动更新与代码签名
- 术语库使用规则与人工映射，未实现通用语义理解或拼写纠错；常见繁体字形对照不等于完整繁简转换
- 「支持中文」会保留为 `chinese` 检索词，但不代表已经验证仓库具备中文界面或文档
- token 当前以本机配置文件保存，尚未接入系统凭据保险库

## 验证与维护

```bash
pip install -r requirements.txt
python export_lexicon.py --check
python -m unittest discover -s tests -v
node --test tests/frontend.test.cjs
cd src-tauri
cargo fmt --check
cargo test --locked --test core
cargo check --locked
```

Python 与 Rust 共用 [`tests/queries.json`](tests/queries.json) 的 51 条手工查询，覆盖技术名符号、繁体 / 全角、OCR 与物体识别边界。两端还逐条验证所有词库概念。测试使用模拟 API 或本地服务，不消耗 GitHub 搜索配额。审计范围与已修复问题见 [`docs/audit-2026-11-09.md`](docs/audit-2026-11-09.md)。

## 评测

Phase 0 用 20 条真实中文查询做了三路盲评（3 个独立评审、匿名打分）：

| 来源 | 平均相关性 | 平均愿意点开 | 胜场 |
|---|---|---|---|
| GitHub 原生（中文） | 1.30 / 10 | 0.50 条 | 1 / 20 |
| GitHub 原生（英文关键词） | **7.20 / 10** | 4.45 条 | **11 / 20** |
| 本系统 | 6.40 / 10 | 4.45 条 | 8 / 20 |

结论：有效的提高了效率，翻译层成立，自研排序是负收益。完整报告见 [`reports/eval.md`](reports/eval.md) 与 [`reports/agent-eval.md`](reports/agent-eval.md)。

## License

MIT © 2026 [MilesDrinkLava](https://github.com/MilesDrinkLava)
