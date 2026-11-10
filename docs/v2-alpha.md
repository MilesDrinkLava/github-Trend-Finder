# V2.0 Alpha：中英文宽召回与 DeepSeek 筛选

V2.0 的目标是覆盖中文与英文开发者的项目，并根据实际需求分析采样结果。V1 的词库仅保留为历史实现和技术回归，不再是 V2 智能检索的必经入口。Skills 暂停开发。

## 当前实现

1. 始终保留用户原文作为独立查询；DeepSeek 另生成最多 3 个中文短查询与 3 个英文短查询。
2. 首轮采样按查询路数分配名额，随后轮流翻页，按 GitHub 仓库 ID 合并。程序不加最低 star、实现语言或 `archived:false` 限定。
3. 采样预算可选 300 / 600 / 1200 / 2000 个去重项目；每页最多 100 个，每路最多 10 页，每轮检索尝试预算最多 36 次。最终数量取决于匹配数量、重复、网络和配额，不能保证达到上限。
4. 全部采样候选以每批 25 个、最多两批并发的方式交给 DeepSeek 分析，而不是先按 star 只保留头部。
5. 对最多 12 个高相关与 8 个待核实候选补充默认 README，并以每批 5 个复审。首版未自动追踪所有中文 README 链接，也未下载或执行项目代码。
6. 保留模型判为相关、部分符合、待核实和不相关的全部样本。缺失/失败的分析保持“尚未分析”，不自动视为无关。
7. 模型只可引用真实候选 ID。支持性引文须逐字存在于提供的描述、topics 或 README；无法通过核验的正面结论降为待核实。引文存在只证明来源，不保证模型推理绝对正确。
8. UI 显示采样、分析与深读计数，保留原描述，展示理由、缺口、证据和召回词。分类、筛选、排序和相关度/热度权重均在已有样本上运行，不重复调用模型。
9. 支持停止、输入变化后取消旧会话、接口失败时保留已显示样本，以及导出完整 JSON 到本机 Downloads。
10. 原文搜索可独立运行，无需 DeepSeek。原文直接交给 GitHub，不经中文转英文词库。

检索固定为公开仓库，避免把有权限访问的私有项目送入模型；数据中的 private 候选也会被排除。GitHub token 仅发往 GitHub，DeepSeek key 仅发往官方 DeepSeek 服务。

## 使用

安装或运行 V2 Alpha 后，在设置中保存 DeepSeek API Key，选择 `deepseek-flash` 或 `deepseek-v4-pro`。界面不会回显已保存的密钥。可使用“测试连接”读取官方模型列表，此按钮不生成内容。

输入需求后，按 Enter 或点击“采样并筛选”。智能模式不会逐键发送模型请求。默认预算 600 个项目，最多约 1 次计划 + 24 次元数据批处理 + 4 次 README 复审；实际调用数和返回 token 用量在“检索详情与用量”中展示。取消在途请求不能保证取消服务端已经开始的计费。

检索与发现共用请求节流；读取配额冷却响应后停止继续消耗该轮检索。无 GitHub token 时大样本采样会更慢，可在设置中配置公开数据读取所需的 token。

同一需求、预算、模型及凭据在 30 分钟内可复用无警告且全部候选完成分析的结果；缓存命中时本次模型调用和 token 计数显示为零。采样页也有 30 分钟内存缓存。需要刷新时，在“检索详情与用量”点击“重新采样（忽略缓存）”；这会重新使用检索和模型配额。应用退出后内存缓存消失；导出的样本可长期保留供评测。

DeepSeek key 当前存于本机 `%APPDATA%\com.ghfinder.desktop\deepseek.json`，也支持用户自行设置的 `DEEPSEEK_API_KEY` 环境变量。清除文件中的 key 不会删除外部环境变量。配置文件尚未采用系统凭据加密；导出文件不包含模型密钥或 GitHub token。

## 验证与边界

离线端到端测试使用本地模拟 GitHub 和 DeepSeek：验证原生中文、英文路由、0 star 候选、去重、私有仓库排除、模型虚构 ID/引文、空结果、错误密钥与取消。模拟服务验证程序行为，不能代替真实 DeepSeek 搜索质量评测。

```text
python -m unittest discover -s tests -v
node --test tests/frontend.test.cjs tests/research-view.test.cjs
cd src-tauri
cargo fmt --check
cargo test --locked --test core --test research_flow
cargo check --locked
```

必要条件目前由模型基于证据写入判断理由与缺口，并非形式化的约束证明器。默认相关度权重 100%，热度不加分；用户可以主动改变权重。不相关候选仍可在“全部样本”看到。

每次会话最多 15 分钟。遇到限流、网络错误、空/截断 JSON 或解析失败时展示覆盖缺口；不宣称“未发现同类”或保证搜索全站。真实模型筛选质量需使用用户配置的 key 和真实需求继续评测。

协议依据：[DeepSeek JSON Output](https://api-docs.deepseek.com/guides/json_mode/)、[DeepSeek Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/)、[GitHub Search API](https://docs.github.com/en/rest/search/search)。模型名称按 2026-11-10 官方文档提供，后续服务变化需再核实。

## 本机验证记录（2026-11-10）

- Python 11 项、前端 9 项检查通过；Rust 核心 4 项与检索流程 10 项通过，包含 0.11 秒内取消正在等待模型响应的本地请求。
- 完成 Windows x64 NSIS Alpha 安装包构建。
- 两次真实公开 GitHub 检索均取得样本，快照在 `reports/v2-recall-smoke-2026-11-10.json`；这不是模型筛选效果评测。
- 真实 DeepSeek 付费生成尚未执行；需用户在应用中配置 Key 后继续验收。
- 隔离配置下的桌面程序可启动。Computer Use 助手在重置重试后仍超时，自动截图/点击验收未完成，不能据此声称界面已完整验收。
