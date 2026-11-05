"""20 条查询的三方对比评测。

三列的意义：
  A. GitHub 原生（中文原句）—— 用户不借助任何工具的现状
  B. GitHub 原生（英文关键词）—— 只加「翻译层」，不加「排序层」
  C. 我们的系统            —— 翻译层 + 排序层

A vs B 量出翻译层的价值，B vs C 量出排序层的价值。
"""

import json
import os
import time
from datetime import datetime, timezone

import requests

import db
import expand as expand_mod
import search as search_mod
from crawl import load_token

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT_DIR = os.path.join(HERE, "reports")
API = "https://api.github.com/search/repositories"


def github_search(query: str, token=None, limit: int = 10, _retries: int = 3):
    if not query.strip():
        return []
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"

    r = requests.get(
        API, params={"q": query, "per_page": limit}, headers=headers, timeout=40
    )
    if r.status_code in (403, 429) and _retries > 0:
        # 403 有两种：真限速（带 retry-after / 剩余 0）和「搜索校验失败」。
        # 后者重试也没用，直接当空结果，避免白白等 60 秒。
        remaining = r.headers.get("x-ratelimit-remaining")
        if remaining not in (None, "0"):
            print(f"    HTTP 403（非限速）：{r.text[:120]}", flush=True)
            return []
        wait = int(r.headers.get("retry-after") or 30)
        print(f"    限速，等待 {wait}s …", flush=True)
        time.sleep(wait)
        return github_search(query, token, limit, _retries - 1)
    if r.status_code != 200:
        print(f"    HTTP {r.status_code}：{r.text[:120]}", flush=True)
        return []
    return r.json().get("items", [])[:limit]


def cell_gh(item) -> str:
    stars = item.get("stargazers_count", 0)
    return f"{item['full_name']} ★{stars / 1000:.1f}k" if stars >= 1000 else f"{item['full_name']} ★{stars}"


def cell_ours(row) -> str:
    stars = row["stars"]
    s = f"{row['full_name']} ★{stars / 1000:.1f}k" if stars >= 1000 else f"{row['full_name']} ★{stars}"
    return s


def _gh_item(it):
    """把 GitHub API 返回压成盲评需要的字段。"""
    return {
        "full_name": it.get("full_name", ""),
        "stars": it.get("stargazers_count", 0),
        "description": it.get("description") or "",
        "language": it.get("language") or "",
        "pushed_at": it.get("pushed_at") or "",
        "archived": bool(it.get("archived")),
        "topics": it.get("topics") or [],
    }


def _our_row(r):
    return {
        "full_name": r["full_name"],
        "stars": r["stars"],
        "description": r["description"] or "",
        "language": r["language"] or "",
        "pushed_at": r["pushed_at"] or "",
        "archived": bool(r["archived"]),
        "topics": (r["topics"] or "").split(),
    }


def english_query(groups) -> str:
    """用每组第一个词拼出「人工翻译后会打的查询」，作为 B 列基准。"""
    return " ".join(g[0] for g in groups if g)


def run(queries, out_name="eval.md"):
    token = load_token()
    # 未认证时搜索 API 只有 10 次/分钟。实测按 6.2 秒卡极限会被同一个 IP 上
    # 的其他请求挤爆，留 25% 余量更稳。
    interval = 2.1 if token else 7.5
    con = db.connect()
    os.makedirs(REPORT_DIR, exist_ok=True)

    lines = [
        "# Phase 0 对比评测",
        "",
        f"生成时间：{datetime.now(timezone.utc).astimezone().strftime('%Y-%m-%d %H:%M')}",
        "",
        "| 列 | 含义 |",
        "|---|---|",
        "| A | GitHub 原生，输入中文原句 |",
        "| B | GitHub 原生，输入扩展后的英文关键词 |",
        "| C | 本系统（翻译层 + 排序层） |",
        "",
    ]

    empty = {"A": 0, "B": 0, "C": 0}
    dump = []                      # 盲评用的原始数据，带描述
    path = os.path.join(REPORT_DIR, out_name)

    def _write():
        """每跑完一条就落盘。跑到一半被打断也不至于颗粒无收。"""
        tail = [
            "## 汇总",
            "",
            "| 列 | 空结果查询数 |",
            "|---|---|",
            f"| A · GitHub 原生（中文） | {empty['A']} / {len(queries)} |",
            f"| B · GitHub 原生（英文） | {empty['B']} / {len(queries)} |",
            f"| C · 本系统 | {empty['C']} / {len(queries)} |",
        ]
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines + tail))

    for i, q in enumerate(queries, 1):
        groups, raw_cn = expand_mod.expand(q)
        en = english_query(groups)

        print(f"[{i}/{len(queries)}] {q}  →  {en}", flush=True)
        ours = search_mod.search(con, q, limit=10)
        print(f"    本地 C 列 {len(ours)} 条", flush=True)
        a = github_search(q, token, 10)
        print(f"    A 列 {len(a)} 条", flush=True)
        time.sleep(interval)
        b = github_search(en, token, 10)
        print(f"    B 列 {len(b)} 条", flush=True)
        time.sleep(interval)

        empty["A"] += 0 if a else 1
        empty["B"] += 0 if b else 1
        empty["C"] += 0 if ours else 1

        lines += [
            f"## {i}. {q}",
            "",
            f"**扩展**：`{expand_mod.describe(groups, raw_cn)}`",
            "",
            f"**B 列查询**：`{en}`",
            "",
            "| # | A · GitHub 原生（中文） | B · GitHub 原生（英文） | C · 本系统 |",
            "|---|---|---|---|",
        ]
        for k in range(10):
            a_cell = cell_gh(a[k]) if k < len(a) else ""
            b_cell = cell_gh(b[k]) if k < len(b) else ""
            c_cell = cell_ours(ours[k][1]) if k < len(ours) else ""
            lines.append(f"| {k + 1} | {a_cell} | {b_cell} | {c_cell} |")
        lines += ["", "**盲评**：A ___ / B ___ / C ___ （各列 Top10 里你愿点开几个）", "", "---", ""]
        dump.append({
            "q": q,
            "expansion": expand_mod.describe(groups, raw_cn),
            "b_query": en,
            "A": [_gh_item(x) for x in a],
            "B": [_gh_item(x) for x in b],
            "C": [_our_row(r) for _, r in ours],
        })
        _write()

    jpath = os.path.splitext(path)[0] + ".json"
    with open(jpath, "w", encoding="utf-8") as f:
        json.dump(
            {"generated": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
             "queries": dump},
            f, ensure_ascii=False, indent=1,
        )

    print(f"报告已写入 {path}", flush=True)
    print(f"盲评数据已写入 {jpath}", flush=True)
    con.close()
    return path


def load_queries(path=None):
    path = path or os.path.join(HERE, "queries.txt")
    if not os.path.exists(path):
        raise SystemExit(
            f"缺少 {path}。每行一条查询，例如：\n"
            "能批量重命名文件的工具\n"
            "支持中文的命令行下载器\n"
        )
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                out.append(line)
    if len(out) < 5:
        raise SystemExit(f"只有 {len(out)} 条查询，至少给 10 条更有意义。")
    return out


if __name__ == "__main__":
    run(load_queries())
