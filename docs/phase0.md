# gh-finder · Phase 0

验证「中文意图 → 英文关键词 → 本地模糊匹配」这条链路是否成立。

Phase 0 **不做 UI**，只产出：一个本地索引 + 一个搜索函数 + 一份对比评测报告。

## Phase 0 结论（2026-11-05，子 agent 盲评）

20 条查询，A/B/C 打乱后由 3 个独立 agent 盲评（详见 `reports/agent-eval.md`）：

| 来源 | 平均相关性 | 平均愿意点开 | 胜场 |
|---|---|---|---|
| A · GitHub 原生（中文） | 1.30 / 10 | 0.50 条 | 1 / 20 |
| B · GitHub 原生（英文关键词） | **7.20 / 10** | 4.45 条 | **11 / 20** |
| C · 本系统 | 6.40 / 10 | 4.45 条 | 8 / 20 |

**翻译层成立，排序层目前是负收益。** C 在 19/20 条查询上碾压 A，但输给「人肉把中文翻成英文再搜」。

根因不是权重没调好，是**纯词法匹配的天花板**：

- 软 AND 会放进「沾边但不对口」的仓库——视频播放器确实支持导出 GIF，PDF 库的 topics 里确实有 `html`
- 试过「topics 命中降权」「全匹配加分」，标准答案覆盖率反而从 80% 掉到 72%/68%

**因此产品方向改为「只补 GitHub 缺的那块」**：我们做中文 → 英文术语映射，
搜索和排序交回 GitHub。本地索引降级为离线兜底。

---

## 当前架构

```
中文输入「把视频转成 gif 的工具」
   ↓  词表最长匹配 + 同义词展开（lexicon.py）
[video] [convert] [gif]
   ↓  概念组合塌缩 / 弱概念裁剪（expand.py）
[video] [convert] [gif]          # 这条不用动
   ↓  每组取第一个术语
"video convert gif" + archived:false
   ↓  GitHub Search API（best match 排序）
结果
```

| 层 | 谁负责 | 代码 |
|---|---|---|
| 中文分词与术语映射 | **我们** | `lexicon.py` / `expand.py` |
| 检索与排序 | GitHub | `remote.py` |
| 缓存（6 小时） | 我们 | `query_cache` 表 |
| 断网/限速兜底 | 我们 | `search.py`（本地 FTS5） |

为什么排序不自己做：盲评里 B 列（英文关键词直搜 GitHub）相关性 7.20，
C 列（本地 FTS5 + 自研重排）只有 6.40。重复造这个轮子目前是负收益。

---

## 要回答的四个问题

| # | 问题 | 判定标准 |
|---|---|---|
| 1 | 中文 → 英文关键词，准确率够不够？ | 20 条真实查询，Top 10 里「愿意见到」的项目命中数 |
| 2 | 本地索引能不能召回词形变化/部分命中？ | `rename` 能命中 `renamer` / `batch-rename` |
| 3 | 我们的排序是否优于 GitHub 原生 relevance？ | 盲评对比，我们的 Top 10 vs GitHub 的 Top 10 |
| 4 | 建库成本是否可接受？ | 时间 / 体积 / 配额消耗 |

## 已锁定的设计决策

- **模糊匹配范围**：中文语义扩展 + 子串/词形。**拼写容错（recat→react）后置到 Phase 1**
- **排序方向**：活跃优先（90 天内有提交 > star 高）
- **测试集**：由用户提供 20 条真实查询

## 候选池

```
180 个 topic × Top 100（按 star 排序，stars:>100）
12,440 个去重后仓库（其中 63% 在 90 天内有提交）
180 次搜索请求 ≈ 19 分钟（未认证 10 次/分钟）
```

**关键原则：候选池必须已经包含「正确答案」**，否则测试的是池子而不是算法。所以 topic 覆盖刻意做宽。

## 存储

SQLite 3.40 + FTS5，两张全文索引：

| 索引 | tokenizer | 用途 |
|---|---|---|
| `repos_fts` | `unicode61` | 主索引，英文词匹配 + BM25 排序 + 前缀查询 |
| `repos_tri` | `trigram` | 兜底，子串匹配 / 中文描述匹配 |

> 实测：trigram **不做**编辑距离，`recat` 匹配不到 `react`。它只做子串。

## 查询扩展流水线

```
「能批量重命名文件的工具」
  ↓ ① 最长匹配分词（中文词表）
[批量重命名] [文件] [工具]
  ↓ ② 术语映射
[batch rename] [file] [tool]
  ↓ ③ 同义词扩展
[[batch rename, bulk rename, renamer, mass rename], [file, files], [tool, cli]]
  ↓ ④ FTS5 MATCH
("batch rename" OR "bulk rename" OR renamer OR "mass rename") AND (file OR files) AND (tool OR cli)
```

## 排序公式

```python
score =  text_rel               × 5.0   # bm25 归一化到 [0,1]，1 = 本查询内最相关
        + 加权概念覆盖率         × 6.0   # 漏掉一个稀有概念，直接掉 2 分以上
        + 首概念出现在名字/描述  × 1.0
        + log10(stars+1)       × 2.5
        + 活跃度(pushed_at)     × 1.5   # ≤90天:1.5  ≤365天:0.75  否则:0
        + has_release          × 0.3
        + 宽松许可证            × 0.2
        - archived             × 2.0
        - awesome/教程/资源集合  × 3.0
```

### 为什么 bm25 必须归一化

第一版直接拿 `-bm25` 当分数，跑评测才发现：

- `-bm25` 的绝对量级在 **25–45** 之间，而星标只贡献 0–7 分、活跃度 1 分
- 等于「活跃优先」这个设计**根本没生效**
- 后果：描述里堆满关键词的小仓库把 scrapy（★65k）挤到**第 25 名**

`bm25` 的绝对值只在同一条查询内部可比，跨查询没有意义，所以先按本次召回集的
min/max 归一化到 `[0,1]`。改完后 scrapy 第 1、Joplin 第 1、lossless-cut 第 1。

用 `tune.py` 在 6 条有标准答案的查询上做过 A/B（覆盖率 / MRR）：

| 方案 | 覆盖率 | MRR |
|---|---|---|
| 原始 `-bm25` × 1.0（第一版） | 72% | 0.49 |
| 归一化 bm25=8，其余不动 | 80% | 0.54 |
| **归一化 bm25=5 + 星标2.5 + 活跃1.5** | **80%** | **0.92** |

最后一项是重点：GitHub 搜「batch rename」会推一堆 `awesome-xxx` 列表仓库，必须压下去。

## 构词规则：为什么必须塌缩

GitHub 仓库搜索是 **AND 语义**，每多一个词就把结果集砍一刀，而且砍掉的是
「描述里恰好没写这个词」的正确仓库。2026-11-05 实测（每组都是同一意图的不同说法）：

| 构词 | 结果数 | 首条 |
|---|---|---|
| `code completion editor plugin` | 14 | notepad4（无关） |
| `code completion plugin` | 313 | nvim-cmp ★9.5k |
| `recognition image text offline` | 29 | 全是垃圾 |
| `ocr offline` | 2,582 | Umi-OCR ★48k |
| `web page save pdf` | 128 | WebPDF ★13 |
| `web page pdf` | 2,443 | url-to-pdf-api ★7.1k |
| `android phone ssh client` | 17 | 无关 |
| `android ssh` | 1,242 | ConnectBot ★3.6k |
| `cli json` | 22,871 | httpie、urfave/cli（撞仓库名） |
| `command line json` | 6,349 | **jq** ★36k |

由此定下两条规则（`expand.py::_postprocess`）：

**① 概念组合塌缩**（`lexicon.py::COMBOS`）——中文常把两三个词并排说，
英文生态里其实是一个专门的词：

| 中文说法 | 逐词翻译（错） | 塌缩后（对） |
|---|---|---|
| 识别图片里的文字 | `recognition image text` | `ocr` |
| 把 epub 转成 mobi | `epub convert mobi` | `ebook converter` |
| 终端文本编辑器 | `terminal text editor` | `terminal editor` |

**② 弱概念裁剪**（`WEAK_CONCEPTS`）——`桌面 / 客户端 / 保存 / 生成` 这类
平台或包装词，只在组数 ≥3 时丢弃，且至少保留 2 组。数量少时它们就是本体
（「视频剪辑软件」只剩 `video editor`，editor 不能丢）。

**③ 首选词要选生态里真写的那个**——`命令行` 的首选是 `command line` 而不是 `cli`；
`下载器` 用 `downloader` 而不是 `download`。缩写会被 GitHub 拿去撞仓库名。

## 词表踩过的坑

这些是 20 条评测跑下来最贵的一课，改词表前先看：

| 坑 | 现象 | 结论 |
|---|---|---|
| 泛化词当概念 | 「支持中文的X」把 `Chinese-CLIP`、`flink-learning` 全捞进来 | `中文` 是修饰语，进 FILLER |
| 中文本义要落到英文说法上 | `桌面` → `desktop`，结果 Joplin/MarkText 全被过滤掉 | 桌面软件描述里写的是 `Windows/macOS/Linux` |
| 概念与填充词重复 | 「插件」同时出现在两处，行为取决于匹配顺序 | 概念优先，FILLER 里必须删掉 |
| 词形盲区 | `scraper*` 匹配不到 `scraping`；`recat` 匹配不到 `react` | 前缀匹配不是词干还原，同义词要写全 |
| 首概念必修过严 | 弱限定词（桌面、手机）当门禁，正确答案全被毙 | 至少让最后的候选答案能通过 `passes` |

## 文件

| 文件 | 作用 |
|---|---|
| `topics.py` | 180 个采集 topic |
| `lexicon.py` | 中文 → 英文术语表 + 同义词族 |
| `db.py` | SQLite schema 与连接 |
| `crawl.py` | 拉取候选池 |
| `expand.py` | 查询扩展 |
| `remote.py` | 薄客户端：调 GitHub 官方搜索 + 缓存 + 兜底 |
| `search.py` | 本地索引检索 + 重排（离线兜底） |
| `evaluate.py` | 20 条查询对比评测 |
| `tune.py` | 排序公式的量化对照实验（一次性工具） |
| `cli.py` | 命令行入口 |

## 用法

```bash
# 1. 放 token（不需要任何 scope，30 次/分；不放也能用，10 次/分）
echo "ghp_xxx" > token.txt

# 2. 直接搜（推荐，走 GitHub 官方搜索）
python cli.py gh "把视频转成 gif 的工具"
python cli.py gh "本地的向量数据库" --stars 500 --lang python --limit 20

# 3. 只看中文被翻译成了什么（不联网）
python cli.py expand "支持中文的命令行下载器"

# 4. 离线兜底（建库约 19 分钟，可跳过）
python cli.py crawl
python cli.py search "批量重命名文件的工具"

# 5. 评测（需要 queries.txt，每行一条）
python cli.py eval
```
