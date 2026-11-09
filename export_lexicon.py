"""把 Python 词表导出成 Rust 后端可嵌入的 JSON。

词表仍以 lexicon.py 为唯一事实来源；改完词表跑一次：

    python export_lexicon.py

生成 src-tauri/lexicon.json，Rust 端用 include_str! 编进二进制。
"""

import json
import os
import sys

import lexicon

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "src-tauri", "lexicon.json")


def build_data():
    """稳定、可验证的导出结构，Python 是词库唯一来源。"""
    return {
        "concepts": {k: list(v) for k, v in lexicon.CONCEPTS.items()},
        "filler": sorted(lexicon.FILLER),
        "combos": [
            [sorted(triggers), sorted(consume), list(repl) if repl else None]
            for triggers, consume, repl in lexicon.COMBOS
        ],
        "weak": sorted(lexicon.WEAK_CONCEPTS),
        "ascii_aliases": lexicon.ASCII_ALIASES,
        "char_map": lexicon.CHAR_MAP,
        "concept_aliases": lexicon.CONCEPT_ALIASES,
    }


def main():
    data = build_data()
    content = json.dumps(data, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if "--check" in sys.argv:
        try:
            with open(OUT, encoding="utf-8") as f:
                current = f.read()
        except FileNotFoundError:
            current = ""
        if current != content:
            raise SystemExit("词库导出已过期，请运行 python export_lexicon.py")
        print("词库导出一致")
        return
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    print(f"已写入 {OUT}")
    print(f"概念 {len(data['concepts'])} · 填充词 {len(data['filler'])} · "
          f"组合 {len(data['combos'])} · 弱概念 {len(data['weak'])}")


if __name__ == "__main__":
    main()
