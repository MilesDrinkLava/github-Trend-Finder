"""检索 + 重排。

三段式：
  1. FTS5 宽召回     —— 所有同义词 OR 起来，只要沾边就捞
  2. 软 AND 过滤     —— 3 组以上时允许漏掉 1 组（硬 AND 会大量误杀）
  3. 质量重排        —— 活跃优先，压掉归档 / awesome 列表 / 低星
"""

import math
import re
from datetime import datetime, timezone

import db
import expand as expand_mod

PERMISSIVE = {
    "MIT", "APACHE-2.0", "BSD-3-CLAUSE", "BSD-2-CLAUSE", "ISC",
    "UNLICENSE", "0BSD", "CC0-1.0",
}

# 这些词出现在名字/描述里，说明它是「资源集合」而不是「能用的工具」
NOISE_PATTERNS = re.compile(
    r"\b(awesome[- ]|curated list|collection of|list of|cheat[- ]?sheet|"
    r"tutorial|course|learning|roadmap|handbook|resources)\b",
    re.IGNORECASE,
)

_TOKEN_SPLIT = re.compile(r"[^a-z0-9]+")


def _parts(term: str):
    return tuple(p for p in _TOKEN_SPLIT.split(term.lower()) if len(p) >= 2)


def _term_expr(term: str):
    parts = _parts(term)
    if not parts:
        return None
    if len(parts) == 1:
        return f"{parts[0]}*"
    return "(" + " AND ".join(f"{p}*" for p in parts) + ")"


def build_match(groups) -> str:
    """宽召回式：所有组的同义词全部 OR 起来，去重后交给 FTS5。"""
    seen, terms = set(), []
    for group in groups:
        for raw in group:
            parts = _parts(raw)
            if not parts or parts in seen:
                continue
            seen.add(parts)
            expr = _term_expr(raw)
            if expr:
                terms.append(expr)
    return " OR ".join(terms)


def _tokens(text: str):
    return set(_TOKEN_SPLIT.split(text.lower())) - {""}


def _has_part(tokens, part: str) -> bool:
    return any(t.startswith(part) for t in tokens)


def group_flags(text: str, groups, substring: bool = False):
    """逐组判断是否命中，返回布尔列表（顺序与 groups 一致）。"""
    tokens = _tokens(text)
    flags = []
    for group in groups:
        hit = False
        for term in group:
            parts = _parts(term)
            if parts and all(
                any(p in t for t in tokens) if substring else _has_part(tokens, p)
                for p in parts
            ):
                hit = True
                break
        flags.append(hit)
    return flags


def group_hits(text: str, groups) -> int:
    return sum(group_flags(text, groups))


def group_weights(con, groups):
    """每个概念组的权重 = 逆文档频率。

    「跨平台的音乐播放器」里，cross-platform 和 player 都很常见，
    music 才是区分度最高的那个。按组计数加权，漏掉稀有组就会被判不及格。
    """
    weights = []
    for g in groups:
        m = build_match([g])
        n = 1
        if m:
            try:
                n = con.execute(
                    "SELECT COUNT(*) FROM repos_fts WHERE repos_fts MATCH ?", (m,)
                ).fetchone()[0]
            except Exception:
                n = 1
        weights.append(1.0 / math.log(2 + n))
    return weights


def required_hits(n_groups: int) -> int:
    """旧接口保留：组数口径的及格线。"""
    if n_groups == 0:
        return 0
    return max(1, round(n_groups * 0.6))


def passes(flags, weights, ratio: float = 0.6) -> bool:
    """首概念（最左边的词，通常是核心动作）必须命中，且加权命中率 ≥ ratio。"""
    if not flags:
        return True
    if not flags[0]:
        return False
    total = sum(weights) or 1.0
    got = sum(w for f, w in zip(flags, weights) if f)
    return got / total >= ratio


def _days_since(ts: str) -> int:
    if not ts:
        return 9999
    try:
        dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
    except (ValueError, TypeError):
        return 9999
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return max(0, (datetime.now(timezone.utc) - dt).days)


def head_bonus(row, groups) -> float:
    """首概念出现在名字或描述里，比只出现在 topics 里可信得多。

    反例：jarun/nnn 的 topics 里带 `batch-rename`（它确实支持），
    但名字和描述里完全看不出「批量重命名」——这种要降权。
    """
    if not groups:
        return 0.0
    head = f"{row['full_name']} {row['description'] or ''}"
    return 0.6 if group_flags(head, [groups[0]])[0] else 0.0


def quality_score(text_rel: float, row, coverage: float = 0.0, bonus: float = 0.0) -> float:
    """text_rel 是归一化到 [0,1] 的文本相关度（1 = 本查询内最相关）。

    早期直接拿 -bm25 当分数，实测它的绝对值在 25–45 之间，而星标只贡献
    0–7 分、活跃度 1 分——等于「活跃优先」的设计完全没生效，
    scrapy(★65k) 会被关键词堆满描述的小仓库挤到第 25 名。
    """
    s = text_rel * 5.0
    s += coverage * 6.0                 # 加权命中率：全中 vs 漏一组，差 2 分以上
    s += bonus                          # 首概念出现在名字/描述里
    s += math.log10(max(row["stars"], 1) + 1) * 2.5

    d = _days_since(row["pushed_at"])
    if d <= 90:
        s += 1.5
    elif d <= 365:
        s += 0.75

    if row["has_release"]:
        s += 0.3
    if (row["license"] or "").upper() in PERMISSIVE:
        s += 0.2
    if row["archived"]:
        s -= 2.0
    if NOISE_PATTERNS.search(f"{row['full_name']} {row['description'] or ''}"):
        s -= 3.0                        # awesome 列表：搜索里最吵的一类噪音
    return s


def search(con, query: str, limit: int = 20, debug: bool = False, ratio: float = 0.6):
    groups, raw_cn = expand_mod.expand(query)
    match = build_match(groups)

    if debug:
        print(f"[扩展] {expand_mod.describe(groups, raw_cn)}")
        print(f"[召回] {match}")

    rows = []
    if match:
        rows = con.execute(
            """
            SELECT r.*, bm25(repos_fts, 10.0, 6.0, 3.0, 1.0) AS bm
            FROM repos_fts JOIN repos r ON r.id = repos_fts.rowid
            WHERE repos_fts MATCH ?
            ORDER BY bm LIMIT 5000
            """,
            (match,),
        ).fetchall()

    # 子串召回必须合并，而不是仅在未识别中文且主索引为空时才运行。
    # trigram 不支持编辑距离；少于三个字符的未知中文用受限的扫描兜底。
    tri_terms = set()
    for group in groups:
        for term in group:
            parts = [p for p in _parts(term) if len(p) >= 3]
            if parts:
                tri_terms.add("(" + " AND ".join('"' + p + '"' for p in parts) + ")")
    tri_terms.update('"' + s.replace('"', '""') + '"' for s in raw_cn if len(s) >= 3)
    tri_rows = []
    if tri_terms:
        tri_rows = con.execute(
            """
            SELECT r.*, bm25(repos_tri, 10.0, 6.0, 3.0, 1.0) AS bm
            FROM repos_tri JOIN repos r ON r.id = repos_tri.rowid
            WHERE repos_tri MATCH ?
            ORDER BY bm LIMIT 200
            """,
            (" OR ".join(sorted(tri_terms)),),
        ).fetchall()
    if not rows and not tri_rows and raw_cn:
        terms = [s for s in raw_cn if len(s) < 3]
        if terms:
            tri_rows = con.execute(
                "SELECT r.*, 0.0 AS bm FROM repos r WHERE " +
                " OR ".join("instr(coalesce(description, '') || full_name || coalesce(topics, ''), ?) > 0" for _ in terms) +
                " ORDER BY stars DESC LIMIT 200", terms,
            ).fetchall()
    tri_ids = {r["id"] for r in tri_rows}
    existing_ids = {r["id"] for r in rows}
    # 两个 FTS 索引的 BM25 量级不同，子串补充项用主召回的最低分。
    if tri_rows:
        floor = max((r["bm"] for r in rows), default=0.0)
        rows.extend(dict(r, bm=floor) for r in tri_rows if r["id"] not in existing_ids)

    weights = group_weights(con, groups)
    total_w = sum(weights) or 1.0

    kept = []
    for r in rows:
        text = f"{r['full_name']} {r['description'] or ''} {r['topics'] or ''}"
        flags = group_flags(text, groups, substring=r["id"] in tri_ids)
        if not passes(flags, weights, ratio):
            continue
        coverage = sum(w for f, w in zip(flags, weights) if f) / total_w
        kept.append((r, coverage))

    if not kept:
        return []

    # bm25 的绝对值只在同一条查询内可比，且量级会盖掉星标和活跃度，先归一化
    best = min(r["bm"] for r, _ in kept)   # 最负 = 最相关
    worst = max(r["bm"] for r, _ in kept)
    span = (worst - best) or 1.0

    scored = [
        (quality_score((worst - r["bm"]) / span, r, cov, head_bonus(r, groups)), r)
        for r, cov in kept
    ]
    scored.sort(key=lambda x: x[0], reverse=True)
    return scored[:limit]


def format_results(results) -> str:
    if not results:
        return "（无结果）"
    out = []
    for i, (sc, r) in enumerate(results, 1):
        d = _days_since(r["pushed_at"])
        age = f"{d}天前更新" if d < 9999 else "未知"
        out.append(
            f"{i:>2}. {r['full_name']}  ★{r['stars']:,}  [{r['language'] or '-'}]  {age}\n"
            f"    {(r['description'] or '')[:110]}\n"
            f"    score={sc:.2f}  {r['html_url']}"
        )
    return "\n".join(out)
