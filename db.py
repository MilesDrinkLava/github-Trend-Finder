"""SQLite schema 与连接。"""

import os
import sqlite3

HERE = os.path.dirname(os.path.abspath(__file__))
DB_PATH = os.path.join(HERE, "data", "ghfinder.db")

SCHEMA = """
CREATE TABLE IF NOT EXISTS repos (
    id           INTEGER PRIMARY KEY,
    full_name    TEXT UNIQUE NOT NULL,
    owner        TEXT,
    name         TEXT,
    description  TEXT,
    language     TEXT,
    stars        INTEGER DEFAULT 0,
    forks        INTEGER DEFAULT 0,
    topics       TEXT DEFAULT '',
    license      TEXT,
    archived     INTEGER DEFAULT 0,
    has_release  INTEGER DEFAULT 0,
    pushed_at    TEXT,
    created_at   TEXT,
    html_url     TEXT,
    homepage     TEXT,
    source_topic TEXT,
    fetched_at   TEXT
);

CREATE INDEX IF NOT EXISTS idx_repos_pushed ON repos(pushed_at);
CREATE INDEX IF NOT EXISTS idx_repos_stars  ON repos(stars);

-- 远程搜索缓存：同样的扩展查询 6 小时内不再打 GitHub
CREATE TABLE IF NOT EXISTS query_cache (
    q          TEXT PRIMARY KEY,
    payload    TEXT NOT NULL,
    fetched_at TEXT NOT NULL
);

-- 主索引：英文词匹配 + BM25
CREATE VIRTUAL TABLE IF NOT EXISTS repos_fts USING fts5(
    full_name, name, description, topics,
    content='repos', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

-- 兜底索引：子串匹配（中英文都能切）
CREATE VIRTUAL TABLE IF NOT EXISTS repos_tri USING fts5(
    full_name, name, description, topics,
    content='repos', content_rowid='id',
    tokenize='trigram'
);

CREATE TRIGGER IF NOT EXISTS repos_ai AFTER INSERT ON repos BEGIN
    INSERT INTO repos_fts(rowid, full_name, name, description, topics)
    VALUES (new.id, new.full_name, new.name, new.description, new.topics);
    INSERT INTO repos_tri(rowid, full_name, name, description, topics)
    VALUES (new.id, new.full_name, new.name, new.description, new.topics);
END;

CREATE TRIGGER IF NOT EXISTS repos_ad AFTER DELETE ON repos BEGIN
    INSERT INTO repos_fts(repos_fts, rowid, full_name, name, description, topics)
    VALUES ('delete', old.id, old.full_name, old.name, old.description, old.topics);
    INSERT INTO repos_tri(repos_tri, rowid, full_name, name, description, topics)
    VALUES ('delete', old.id, old.full_name, old.name, old.description, old.topics);
END;

CREATE TRIGGER IF NOT EXISTS repos_au AFTER UPDATE ON repos BEGIN
    INSERT INTO repos_fts(repos_fts, rowid, full_name, name, description, topics)
    VALUES ('delete', old.id, old.full_name, old.name, old.description, old.topics);
    INSERT INTO repos_fts(rowid, full_name, name, description, topics)
    VALUES (new.id, new.full_name, new.name, new.description, new.topics);
    INSERT INTO repos_tri(repos_tri, rowid, full_name, name, description, topics)
    VALUES ('delete', old.id, old.full_name, old.name, old.description, old.topics);
    INSERT INTO repos_tri(rowid, full_name, name, description, topics)
    VALUES (new.id, new.full_name, new.name, new.description, new.topics);
END;
"""


def connect(path: str = DB_PATH) -> sqlite3.Connection:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    con = sqlite3.connect(path)
    con.row_factory = sqlite3.Row
    con.executescript(SCHEMA)
    return con


def stats(con: sqlite3.Connection) -> dict:
    n = con.execute("SELECT COUNT(*) FROM repos").fetchone()[0]
    return {"repos": n}
