# 版本记录

## V2.0 Alpha（2.0.0-alpha.1）— 本地开发中

- 保留原始中文，DeepSeek 扩展中英文查询，多路采样和仓库 ID 去重。
- 全部候选分批判断，少量 README 深读、引用核验、待核实状态与完整样本导出。
- 可配置官方 DeepSeek、采样预算、分类和本地调权；支持进度、取消和缓存。
- 新增本地模拟服务端到端测试；真实 DeepSeek 检索质量待配置密钥后评测。
- Skills 暂停；本版尚未创建 GitHub 标签或 Release。

## V1.1（1.1.0）— 2026-11-09

- 本次代码审计与词库扩充后的版本，包含审计提交 `355af95`。
- 词库扩充为 380 个概念、788 条英文术语，增加繁体、全角与技术名映射。
- 修复 OCR 误判、缓存隔离、离线筛选、异步请求乱序及预览目录暴露问题。
- 增加共享限流、查询回归与安全边界检查；详见 [审计报告](docs/audit-2026-11-09.md)。
- Git 标签：`v1.1.0`。[独立发布页与安装包](https://github.com/MilesDrinkLava/github-Trend-Finder/releases/tag/v1.1.0)。

## V1.0（1.0.0）

- 本次审计前的原有版本：中文搜索、异步描述翻译、发现页、主题与 PAT 设置。
- 保留原 Git 标签 `v1.0.0`，指向 `1937d76`，不移动历史标签。[原版发布页与安装包](https://github.com/MilesDrinkLava/github-Trend-Finder/releases/tag/v1.0.0)。
