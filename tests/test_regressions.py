import ast
import json
import threading
import unittest
import urllib.error
import urllib.request
from pathlib import Path
from unittest.mock import patch

import db
import expand
import export_lexicon
import lexicon
import remote
import search
from app.server import Handler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]


class LexiconTests(unittest.TestCase):
    def test_shared_queries(self):
        fixtures = json.loads((ROOT / "tests/queries.json").read_text(encoding="utf-8"))
        for case in fixtures:
            with self.subTest(query=case["query"]):
                a = expand.analyze(case["query"])
                self.assertEqual(remote.to_english(a["groups"]), case["english"])
                self.assertTrue(0 <= a["coverage"] <= 1)
                if "raw_cn" in case:
                    self.assertEqual(a["raw_cn"], case["raw_cn"])

    def test_all_concepts_parse_and_export_is_current(self):
        for word, terms in lexicon.CONCEPTS.items():
            with self.subTest(word=word):
                self.assertTrue(terms and all(t.strip() == t and t for t in terms))
                a = expand.analyze(word)
                self.assertEqual(a["groups"], [terms])
                self.assertEqual(a["raw_cn"], [])
        self.assertFalse(set(lexicon.CONCEPTS) & lexicon.FILLER)
        data = json.loads((ROOT / "src-tauri/lexicon.json").read_text(encoding="utf-8"))
        self.assertEqual(data, export_lexicon.build_data())

    def test_no_silent_duplicate_keys(self):
        tree = ast.parse((ROOT / "lexicon.py").read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Dict):
                keys = [ast.literal_eval(k) for k in node.keys if k is not None]
                self.assertEqual(len(keys), len(set(keys)), keys)


class CacheTests(unittest.TestCase):
    def setUp(self):
        self.con = db.connect(":memory:")

    def tearDown(self):
        self.con.close()

    def test_limit_and_credentials_isolate_cache(self):
        def api(q, limit, token, extra):
            return [{"full_name": f"{token}/{i}"} for i in range(limit)]
        with patch.object(remote, "_api", side_effect=api) as mock:
            one = remote.search("图片压缩", limit=1, token="a", con=self.con)
            many = remote.search("图片压缩", limit=30, token="a", con=self.con)
            other = remote.search("图片压缩", limit=1, token="b", con=self.con)
            cached = remote.search("图片压缩", limit=1, token="a", con=self.con)
        self.assertEqual(len(one["items"]), 1)
        self.assertEqual(len(many["items"]), 30)
        self.assertEqual(other["items"][0]["full_name"], "b/0")
        self.assertEqual(cached["source"], "cache")
        self.assertEqual(mock.call_count, 3)

    def test_corrupt_cache_is_a_miss(self):
        self.con.execute("INSERT INTO query_cache VALUES ('bad', '{broken', datetime('now'))")
        self.assertIsNone(remote._cache_get(self.con, "bad"))
        self.con.execute("UPDATE query_cache SET fetched_at = 'not-a-date'")
        self.assertIsNone(remote._cache_get(self.con, "bad"))

    def test_raw_mode_preserves_unrecognized_chinese(self):
        with patch.object(remote, "_api", return_value=[]) as api:
            remote.search("仅原生中文的小众词汇", token="", con=self.con, raw=True)
        self.assertEqual(api.call_args.args[0], "仅原生中文的小众词汇")

    def test_fallback_preserves_archived_filter(self):
        row = dict(full_name="a/archived", description="", stars=1, language="Python",
                   pushed_at="", archived=1, topics="", html_url="")
        with patch.object(remote, "_api", side_effect=RuntimeError("offline")), \
             patch.object(search, "search", return_value=[(1, row)]):
            result = remote.search("数据库", token="", extra="archived:false", con=self.con)
            self.assertEqual(result["items"], [])

    def test_unsupported_offline_filters_fail_closed(self):
        with patch.object(remote, "_api", side_effect=RuntimeError("offline")), \
             patch.object(search, "search") as local:
            result = remote.search("数据库", token="", extra="language:Rust", con=self.con)
        self.assertEqual(result["items"], [])
        self.assertIn("不支持此筛选", result["error"])
        local.assert_not_called()

    def test_timestamp_and_database_basename(self):
        self.assertGreaterEqual(search._days_since("2024-01-01"), 0)
        self.assertEqual(search._days_since("2099-01-01T00:00:00Z"), 0)
        self.assertIn("APACHE-2.0", search.PERMISSIVE)

    def test_substring_recall_and_short_chinese(self):
        self.con.executemany(
            "INSERT INTO repos (id, full_name, name, description, stars) VALUES (?,?,?,?,?)",
            [(1, "a/bulkrenamer", "bulkrenamer", "batchrenaming files", 100),
             (2, "b/rename", "rename", "rename files", 100),
             (3, "c/mars", "mars", "火星探测器", 100)],
        )
        self.con.commit()
        names = {r["full_name"] for _, r in search.search(self.con, "rename")}
        self.assertEqual(names, {"a/bulkrenamer", "b/rename"})
        chinese = search.search(self.con, "火星")
        self.assertEqual(chinese[0][1]["full_name"], "c/mars")


class PreviewTests(unittest.TestCase):
    def test_only_frontend_is_served(self):
        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=httpd.serve_forever, daemon=True)
        worker.start()
        base = f"http://127.0.0.1:{httpd.server_port}"
        try:
            with urllib.request.urlopen(base + "/") as r:
                self.assertEqual(r.status, 200)
                self.assertIn(b"<html", r.read())
            for path in ("/token.txt", "/.git/config", "/lexicon.py", "/../lexicon.py"):
                with self.subTest(path=path), self.assertRaises(urllib.error.HTTPError) as ctx:
                    urllib.request.urlopen(base + path)
                self.assertEqual(ctx.exception.code, 404)
        finally:
            httpd.shutdown()
            httpd.server_close()
            worker.join()


if __name__ == "__main__":
    unittest.main()
