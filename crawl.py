"""拉取候选池。

搜索 API 限速 30 次/分钟（认证），所以固定 2.1 秒间隔，不抢跑。
100 个 topic ≈ 3.5 分钟。
"""

import json
import os
import time
from datetime import datetime, timezone

import requests

import db
from topics import TOPICS

API = "https://api.github.com/search/repositories"
HERE = os.path.dirname(os.path.abspath(__file__))

MIN_STARS = 100
PER_PAGE = 100
INTERVAL = 2.1       # 秒，卡在 30 次/分钟以内（认证）
ANON_INTERVAL = 6.2  # 秒，卡在 10 次/分钟以内（未认证）


def load_token():
    for key in ("GITHUB_TOKEN", "GH_TOKEN"):
        if os.environ.get(key):
            return os.environ[key].strip()
    p = os.path.join(HERE, "token.txt")
    if os.path.exists(p):
        t = open(p, encoding="utf-8").read().strip()
        if t:
            return t
    return None


def _row(item: dict, topic: str) -> tuple:
    lic = item.get("license") or {}
    return (
        item["id"],
        item["full_name"],
        (item.get("owner") or {}).get("login", ""),
        item["name"],
        item.get("description") or "",
        item.get("language") or "",
        item.get("stargazers_count", 0),
        item.get("forks_count", 0),
        " ".join(item.get("topics") or []),
        lic.get("spdx_id") or "",
        1 if item.get("archived") else 0,
        0,  # has_release：搜索 API 不返回，Phase 0 不做额外请求
        item.get("pushed_at") or "",
        item.get("created_at") or "",
        item.get("html_url") or "",
        item.get("homepage") or "",
        topic,
        datetime.now(timezone.utc).isoformat(timespec="seconds"),
    )


UPSERT = """
INSERT INTO repos (
    id, full_name, owner, name, description, language, stars, forks,
    topics, license, archived, has_release, pushed_at, created_at,
    html_url, homepage, source_topic, fetched_at
) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
ON CONFLICT(full_name) DO UPDATE SET
    stars       = excluded.stars,
    forks       = excluded.forks,
    description = excluded.description,
    topics      = excluded.topics,
    license     = excluded.license,
    archived    = excluded.archived,
    pushed_at   = excluded.pushed_at,
    fetched_at  = excluded.fetched_at
"""


def fetch(topic: str, token) -> list[dict]:
    params = {
        "q": f"topic:{topic} stars:>={MIN_STARS}",
        "sort": "stars",
        "order": "desc",
        "per_page": PER_PAGE,
    }
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    for attempt in range(3):
        r = requests.get(API, params=params, headers=headers, timeout=40)
        if r.status_code == 200:
            return r.json().get("items", [])
        if r.status_code in (403, 429):
            wait = int(r.headers.get("retry-after") or 60)
            print(f"  限速，等待 {wait}s …")
            time.sleep(wait)
            continue
        print(f"  HTTP {r.status_code}: {r.text[:200]}")
        if attempt == 2:
            return []
        time.sleep(5)
    return []


def run(topics=None):
    topics = topics or TOPICS
    token = load_token()
    interval = INTERVAL if token else ANON_INTERVAL
    mode = "认证 30次/分" if token else "未认证 10次/分"
    print(f"模式 {mode}，{len(topics)} 个 topic，预计 {len(topics) * interval / 60:.0f} 分钟\n")

    con = db.connect()
    total = 0

    for idx, topic in enumerate(topics, 1):
        items = fetch(topic, token)
        rows = [_row(it, topic) for it in items]
        con.executemany(UPSERT, rows)
        con.commit()
        total += len(rows)
        n = db.stats(con)["repos"]
        print(f"[{idx:>3}/{len(topics)}] {topic:<28} 拿到 {len(rows):>3} 条，库内 {n:>6}")
        if idx < len(topics):
            time.sleep(interval)

    print(f"\n完成。原始 {total} 条，去重后 {db.stats(con)['repos']} 个仓库。")
    con.close()


if __name__ == "__main__":
    run()
