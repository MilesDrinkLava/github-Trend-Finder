"""薄客户端：中文 → 英文关键词 → GitHub 官方搜索。

设计取舍（2026-11-05 盲评之后定的）：

  我们**不**自己做排序。盲评结论是本地排序层（C 列）不如直接拿英文关键词
  搜 GitHub（B 列，相关性 7.20 vs 6.40）。所以这一层只补 GitHub 缺的那块：
  中文到英文的术语映射。搜索、排序、相关度全部交给 GitHub。

  本地索引降级成两个用途：
    1. 查询扩展的诊断（词表覆盖率）
    2. 断网 / 限速时的兜底
"""

import json
import hashlib
import os
import time
from datetime import datetime, timezone

import requests

import db
import expand as expand_mod
import search as search_mod
from crawl import load_token

API = "https://api.github.com/search/repositories"
HERE = os.path.dirname(os.path.abspath(__file__))
CACHE_TTL_HOURS = 6


def to_english(groups) -> str:
    """每个概念组取第一个英文说法，空格拼接。

    试过「按本地词频挑最常见的同义词」，反而更差：
    桌面→linux、数据库→db、爬虫→crawl。同义词表的顺序是人工按准确度排的，
    比语料统计更靠谱。
    """
    return " ".join(g[0] for g in groups if g)


def build(query: str):
    """返回 (英文查询, 概念组, 未覆盖中文)。"""
    groups, raw_cn = expand_mod.expand(query)
    return to_english(groups), groups, raw_cn


def _cache_get(con, q: str):
    row = con.execute(
        "SELECT payload, fetched_at FROM query_cache WHERE q = ?", (q,)
    ).fetchone()
    if not row:
        return None
    try:
        age = datetime.now(timezone.utc) - datetime.fromisoformat(row["fetched_at"])
    except (ValueError, TypeError):
        return None
    if age.total_seconds() > CACHE_TTL_HOURS * 3600:
        return None
    try:
        items = json.loads(row["payload"])
        return items if isinstance(items, list) and all(isinstance(x, dict) for x in items) else None
    except (ValueError, TypeError):
        return None


def _cache_put(con, q: str, items):
    con.execute(
        "INSERT INTO query_cache (q, payload, fetched_at) VALUES (?,?,?) "
        "ON CONFLICT(q) DO UPDATE SET payload=excluded.payload, fetched_at=excluded.fetched_at",
        (q, json.dumps(items, ensure_ascii=False),
         datetime.now(timezone.utc).isoformat(timespec="seconds")),
    )
    con.commit()


def _normalize(it: dict) -> dict:
    return {
        "full_name": it.get("full_name", ""),
        "description": it.get("description") or "",
        "stars": it.get("stargazers_count", 0),
        "language": it.get("language") or "",
        "pushed_at": it.get("pushed_at") or "",
        "archived": bool(it.get("archived")),
        "topics": it.get("topics") or [],
        "html_url": it.get("html_url") or "",
    }


def _api(q: str, limit: int, token, extra: str, retries: int = 1):
    params = {
        "q": (q + (" " + extra if extra else "")).strip(),
        "per_page": limit,
        "sort": "",          # 空 = GitHub 的 best match，盲评里表现最好
        "order": "desc",
    }
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"

    r = requests.get(API, params=params, headers=headers, timeout=30)
    if r.status_code in (403, 429) and retries > 0:
        wait = int(r.headers.get("retry-after") or 0)
        if wait:
            time.sleep(min(wait, 60))
            return _api(q, limit, token, extra, retries - 1)
    if r.status_code != 200:
        raise RuntimeError(f"HTTP {r.status_code}: {r.text[:160]}")
    return [_normalize(x) for x in r.json().get("items", [])[:limit]]


def search(query: str, limit: int = 10, token=None, extra: str = "",
           con=None, use_cache: bool = True):
    """返回 dict：{query, english, groups, raw_cn, source, items, error}。

    source 取值：
      github —— 官方 API
      cache  —— 6 小时内的缓存
      local  —— 断网/限速时退回本地索引（结果质量低于 GitHub）
    """
    english, groups, raw_cn = build(query)
    limit = max(1, min(100, int(limit)))
    credential = token if token is not None else load_token()
    out = {
        "query": query,
        "english": english,
        "groups": groups,
        "raw_cn": raw_cn,
        "source": "github",
        "items": [],
        "error": "",
    }

    close = False
    if con is None:
        con = db.connect()
        close = True

    try:
        if not english:
            out["error"] = "中文没命中词表，无法构造英文查询"
            return out

        # 不同条数/凭据的响应不能互用，避免小页缓存截断大页或串用私有结果。
        scope = hashlib.sha256((credential or "").encode()).hexdigest()
        key = json.dumps(["v2", english, extra, limit, scope], ensure_ascii=False)
        if use_cache:
            hit = _cache_get(con, key)
            if hit is not None:
                out["items"] = hit[:limit]
                out["source"] = "cache"
                return out

        try:
            items = _api(english, limit, credential, extra)
            out["items"] = items
            _cache_put(con, key, items)
        except Exception as e:
            out["error"] = f"{e}，已退回本地索引"
            out["source"] = "local"
            # 本地索引不实现完整 GitHub 限定符；不能悄悄忽略 stars/language 等过滤。
            supported = extra.strip() in ("", "archived:false")
            candidates = search_mod.search(con, query, limit=100) if supported else []
            out["items"] = [
                {
                    "full_name": r["full_name"],
                    "description": r["description"] or "",
                    "stars": r["stars"],
                    "language": r["language"] or "",
                    "pushed_at": r["pushed_at"] or "",
                    "archived": bool(r["archived"]),
                    "topics": (r["topics"] or "").split(),
                    "html_url": r["html_url"] or "",
                }
                for _, r in candidates
                if extra.strip() != "archived:false" or not r["archived"]
            ][:limit]
            if not supported:
                out["error"] = f"{e}；本地索引不支持此筛选条件，未返回未筛选的结果"
        return out
    finally:
        if close:
            con.close()


def format_results(result: dict) -> str:
    items = result["items"]
    head = f"查询   : {result['query']}"
    if result["english"]:
        head += f"\n英文   : {result['english']}"
    if result["raw_cn"]:
        head += f"\n未覆盖 : {' / '.join(result['raw_cn'])}"
    src = {"github": "GitHub 官方搜索", "cache": "缓存（6 小时内）",
           "local": "本地索引兜底"}[result["source"]]
    head += f"\n来源   : {src}"
    if result["error"]:
        head += f"\n提示   : {result['error']}"

    if not items:
        return head + "\n\n（无结果）"

    lines = [head, ""]
    for i, r in enumerate(items, 1):
        d = search_mod._days_since(r["pushed_at"])
        age = f"{d}天前更新" if d < 9999 else "未知"
        lines.append(
            f"{i:>2}. {r['full_name']}  ★{r['stars']:,}  [{r['language'] or '-'}]  {age}"
            + ("  [已归档]" if r["archived"] else "")
        )
        if r["description"]:
            lines.append(f"    {r['description'][:110]}")
        lines.append(f"    {r['html_url']}")
    return "\n".join(lines)
