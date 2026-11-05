"""排序公式的量化对照实验（Phase 0 一次性工具，不进产品）。

用法：python -X utf8 tune.py
输出每种打分方案的 recall@10 / MRR，用数字决定改不改，而不是靠感觉。
"""

import math

import db
import expand as expand_mod
import search as search_mod

# 只收录「确实在候选池里」的标准答案，否则测的是覆盖率不是排序
GROUND_TRUTH = {
    "python 爬虫框架": [
        "scrapy/scrapy",
        "binux/pyspider",
        "Boris-code/feapder",
        "lorien/grab",
        "howie6879/ruia",
    ],
    "终端里的文件管理器": [
        "gokcehan/lf",
        "jarun/nnn",
        "sxyazi/yazi",
        "kamiyaa/joshuto",
        "yorukot/superfile",
        "sayanarijit/xplr",
    ],
    "把视频转成 gif 的工具": [
        "sindresorhus/Gifski",
        "vvo/gifify",
        "NickeManarin/ScreenToGif",
        "lettier/gifcurry",
    ],
    "桌面上好看的 markdown 笔记软件": [
        "laurent22/joplin",
        "marktext/marktext",
        "logseq/logseq",
    ],
    "命令行里查看 json 的工具": [
        "antonmedv/fx",
        "ynqa/jnv",
        "tomnomnom/gron",
        "mikefarah/yq",
    ],
    "开源的视频剪辑软件": [
        "mifi/lossless-cut",
        "OpenShot/openshot-qt",
        "olive-editor/olive",
    ],
}

TOP = 10


def _quality(text_rel, bm, row, coverage, bonus, w):
    """可调权重的打分函数。

    w["text_raw"] 存在时用原始 -bm25（现状）；否则用归一化到 [0,1] 的相关度。
    """
    if w.get("text_raw"):
        s = -bm * w["text_raw"]
    else:
        s = text_rel * w["text"]
    s += coverage * w["coverage"]
    s += bonus                        # bonus 已是 0.6 / 0.0，不再乘权重
    s += math.log10(max(row["stars"], 1) + 1) * w["stars"]
    d = search_mod._days_since(row["pushed_at"])
    if d <= 90:
        s += w["fresh"]
    elif d <= 365:
        s += w["fresh"] * 0.5
    if row["has_release"]:
        s += w["release"]
    if (row["license"] or "").upper() in search_mod.PERMISSIVE:
        s += w["license"]
    if row["archived"]:
        s -= w["archived_penalty"]
    if search_mod.NOISE_PATTERNS.search(
        f"{row['full_name']} {row['description'] or ''}"
    ):
        s -= 3.0
    return s


def keep(flags, weights, mode):
    """过滤规则的几种候选。head=现状（首概念必修）。"""
    if not flags:
        return True
    total = sum(weights) or 1.0
    cov = sum(x for f, x in zip(flags, weights) if f) / total
    if mode == "head":
        return flags[0] and cov >= 0.6
    if mode == "cov70":
        return cov >= 0.7
    if mode == "cov60":
        return cov >= 0.6
    if mode == "cov50":
        return cov >= 0.5
    raise ValueError(mode)


def rank(con, query, w):
    groups, raw_cn = expand_mod.expand(query)
    match = search_mod.build_match(groups)
    rows = con.execute(
        """
        SELECT r.*, bm25(repos_fts, 10.0, 6.0, 3.0, 1.0) AS bm
        FROM repos_fts JOIN repos r ON r.id = repos_fts.rowid
        WHERE repos_fts MATCH ?
        ORDER BY bm LIMIT 5000
        """,
        (match,),
    ).fetchall()

    weights = search_mod.group_weights(con, groups)
    total_w = sum(weights) or 1.0

    kept = []
    for r in rows:
        text = f"{r['full_name']} {r['description'] or ''} {r['topics'] or ''}"
        flags = search_mod.group_flags(text, groups)
        if not keep(flags, weights, w.get("filter", "head")):
            continue
        kept.append((r, sum(x for f, x in zip(flags, weights) if f) / total_w))

    if not kept:
        return []
    best = min(r["bm"] for r, _ in kept)      # 最负 = 最相关
    worst = max(r["bm"] for r, _ in kept)
    span = (worst - best) or 1.0

    scored = []
    for r, cov in kept:
        text_rel = (worst - r["bm"]) / span
        scored.append(
            (
                _quality(text_rel, r["bm"], r, cov, search_mod.head_bonus(r, groups), w),
                r["full_name"],
            )
        )
    scored.sort(reverse=True)
    return [n for _, n in scored[:TOP]]


def evaluate(name, w):
    con = db.connect()
    hits = 0
    total = 0
    rr = 0.0
    print(f"\n=== {name} ===")
    for q, gt in GROUND_TRUTH.items():
        got = rank(con, q, w)
        pos = next((i for i, n in enumerate(got, 1) if n in gt), None)
        # 命中率：Top10 里覆盖了几个标准答案
        covered = len(set(got) & set(gt))
        hits += covered
        total += len(gt)
        if pos:
            rr += 1.0 / pos
        print(f"  {q:<22} 覆盖 {covered}/{len(gt)}  首个第{pos or '-'}位  {got[:5]}")
    con.close()
    print(f"  ---- 覆盖率 {hits}/{total} = {hits / total:.0%}   MRR {rr / len(GROUND_TRUTH):.3f}")
    return hits / total, rr / len(GROUND_TRUTH)


BASE = dict(text=8.0, coverage=6.0, bonus=0.6, stars=1.5, fresh=1.0,
            release=0.3, license=0.2, archived_penalty=2.0)

VARIANTS = {
    "V0 现状（原始 -BM25×1.0，首概念必修）": dict(BASE, text_raw=1.0, text=0.0),
    "V1 归一化BM25=8 + 首概念必修": dict(BASE, filter="head"),
    "V3 归一化BM25=5 + 星标2.5 + 活跃1.5 + 首概念必修": dict(
        BASE, text=5.0, stars=2.5, fresh=1.5, filter="head"
    ),
    "V3a 同上，但取消首概念必修（cov≥0.6）": dict(
        BASE, text=5.0, stars=2.5, fresh=1.5, filter="cov60"
    ),
    "V3b 同上，cov≥0.7": dict(BASE, text=5.0, stars=2.5, fresh=1.5, filter="cov70"),
    "V3c 同上，cov≥0.5": dict(BASE, text=5.0, stars=2.5, fresh=1.5, filter="cov50"),
    "V1a 归一化BM25=8 + cov≥0.6": dict(BASE, filter="cov60"),
}

if __name__ == "__main__":
    for name, w in VARIANTS.items():
        evaluate(name, w)
