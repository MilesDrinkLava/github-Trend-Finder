"""Phase 0 命令行入口。

  python cli.py crawl            建库
  python cli.py expand "查询"    只看查询扩展（不需要 token）
  python cli.py gh "查询"        中文 → 英文 → GitHub 官方搜索（推荐）
  python cli.py search "查询"    本地索引搜索（离线兜底）
  python cli.py eval             20 条查询三方对比
  python cli.py stats            看库
"""

import sys

# Windows 控制台中文
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

import db
import expand as expand_mod
import search as search_mod


def cmd_crawl(args):
    import crawl
    limit = len(crawl.TOPICS)
    if args and args[0].isdigit():
        limit = int(args[0])
    crawl.run(crawl.TOPICS[:limit])


def cmd_expand(args):
    q = " ".join(args)
    a = expand_mod.analyze(q)
    groups, raw_cn = a["groups"], a["raw_cn"]
    print(f"输入   : {q}")
    print(f"分组   : {groups}")
    print(f"未覆盖 : {raw_cn}")
    print(f"覆盖率 : {a['coverage']:.0%}  ({a['matched_chars']}/{a['total_cjk']} 个汉字命中词表)")
    print(f"表达式 : {expand_mod.describe(groups, raw_cn)}")
    print(f"FTS    : {search_mod.build_match(groups)}")


def cmd_search(args):
    q = " ".join(a for a in args if a != "--debug")
    debug = "--debug" in args
    con = db.connect()
    results = search_mod.search(con, q, limit=20, debug=debug)
    print(search_mod.format_results(results))
    con.close()


def cmd_gh(args):
    """薄客户端：中文 → 英文关键词 → GitHub 官方搜索。"""
    import remote

    limit, extra, words, keep_archived = 10, [], [], False
    it = iter(args)
    for a in it:
        if a == "--limit":
            limit = int(next(it))
        elif a == "--stars":
            extra.append(f"stars:>={next(it)}")
        elif a == "--lang":
            extra.append(f"language:{next(it)}")
        elif a == "--archived":
            keep_archived = True
        else:
            words.append(a)

    # 归档仓库基本是「已经不用了」，默认让 GitHub 自己过滤掉
    if not keep_archived:
        extra.insert(0, "archived:false")

    q = " ".join(words)
    if not q:
        print("用法: python cli.py gh \"能批量重命名文件的工具\" [--limit 10] [--stars 1000] [--lang python]")
        return
    result = remote.search(q, limit=limit, extra=" ".join(extra))
    print(remote.format_results(result))


def cmd_eval(args):
    import evaluate
    evaluate.run(evaluate.load_queries())


def cmd_stats(args):
    con = db.connect()
    n = db.stats(con)["repos"]
    langs = con.execute(
        "SELECT language, COUNT(*) c FROM repos GROUP BY language ORDER BY c DESC LIMIT 10"
    ).fetchall()
    recent = con.execute(
        "SELECT COUNT(*) FROM repos WHERE pushed_at >= date('now','-90 day')"
    ).fetchone()[0]
    print(f"仓库总数   : {n}")
    print(f"90天活跃   : {recent}  ({recent / max(n, 1):.0%})")
    print("语言分布   : " + ", ".join(f"{r['language'] or 'None'} {r['c']}" for r in langs))
    con.close()


COMMANDS = {
    "crawl": cmd_crawl,
    "expand": cmd_expand,
    "gh": cmd_gh,
    "search": cmd_search,
    "eval": cmd_eval,
    "stats": cmd_stats,
}


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        print(__doc__)
        return
    COMMANDS[sys.argv[1]](sys.argv[2:])


if __name__ == "__main__":
    main()
