"""本地预览服务：把 dist/index.html 接到真实的 GitHub 搜索。

    python app/server.py
    # 浏览器打开 http://127.0.0.1:8765

这层只是开发用的胶水，不是最终形态。前端与 Tauri 版共用同一个
dist/index.html；后端复用现有的 remote.search()（中文映射 → GitHub API
→ 6 小时缓存 → 断网回退本地索引）。
"""

import json
import os
import sys
import urllib.parse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)

import remote  # noqa: E402

PORT = 8765


def _short_error(e: Exception) -> str:
    s = str(e)
    if "rate limit" in s.lower() or "403" in s:
        return "GitHub 限速了，等一分钟再试（或配置 token 提升到 30 次/分）"
    if "timed out" in s.lower() or "connection" in s.lower():
        return "连不上 GitHub，检查网络或代理设置"
    return s[:160]


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=os.path.join(ROOT, "dist"), **kwargs)

    def do_GET(self):
        path = urllib.parse.urlparse(self.path).path
        if path == "/api/search":
            return self._search(urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query))
        if path == "/":
            self.path = "/index.html"
        return super().do_GET()

    def _search(self, qs):
        query = (qs.get("q") or [""])[0].strip()
        try:
            limit = max(1, min(30, int((qs.get("limit") or ["10"])[0])))
        except ValueError:
            limit = 10

        if not query:
            return self._json({"error": "空查询", "english": "", "items": [],
                               "raw_cn": [], "source": "none"})

        try:
            result = remote.search(query, limit=limit, extra="is:public in:name,description,readme", raw=True)
        except Exception as e:  # remote.search 内部已兜底，这里只防意外
            return self._json({"error": _short_error(e), "english": "", "items": [],
                               "raw_cn": [], "source": "error"}, 500)
        return self._json(result)

    def _json(self, obj, status: int = 200):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        sys.stderr.write("[preview] " + (fmt % args) + "\n")


def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    token = remote.load_token()
    print(f"预览服务: http://127.0.0.1:{PORT}")
    print(f"GitHub  : {'已配置 token（30 次/分）' if token else '未配置 token（10 次/分）'}")
    print("Ctrl+C 停止")
    ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
