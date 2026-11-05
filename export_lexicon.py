"""把 Python 词表导出成 Rust 后端可嵌入的 JSON。

词表仍以 lexicon.py 为唯一事实来源；改完词表跑一次：

    python export_lexicon.py

生成 src-tauri/lexicon.json，Rust 端用 include_str! 编进二进制。
"""

import json
import os

import lexicon

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "src-tauri", "lexicon.json")


def main():
    data = {
        "concepts": {k: list(v) for k, v in lexicon.CONCEPTS.items()},
        "filler": sorted(lexicon.FILLER),
        "combos": [
            [sorted(triggers), sorted(consume), list(repl) if repl else None]
            for triggers, consume, repl in lexicon.COMBOS
        ],
        "weak": sorted(lexicon.WEAK_CONCEPTS),
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
    print(f"已写入 {OUT}")
    print(f"概念 {len(data['concepts'])} · 填充词 {len(data['filler'])} · "
          f"组合 {len(data['combos'])} · 弱概念 {len(data['weak'])}")


if __name__ == "__main__":
    main()
