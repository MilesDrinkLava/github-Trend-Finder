"""把 reports/eval.md 转成匿名盲评包。

A/B/C → 随机 X/Y/Z，映射写进 key.json（评分者看不到）。
每批一个 markdown，方便分给不同子 agent。
"""

import json
import os
import random
import re
import sqlite3

HERE = os.path.dirname(os.path.abspath(__file__))
EVAL = os.path.join(HERE, "reports", "eval.md")
OUT = os.path.join(HERE, "reports", "blind")
DB = os.path.join(HERE, "data", "ghfinder.db")

BATCHES = 3
SEED = 20261105


def parse(path):
    text = open(path, encoding="utf-8").read()
    out = []
    for blk in text.split("## ")[1:]:
        lines = blk.strip().split("\n")
        if not re.match(r"^\d+\.", lines[0]):
            continue
        title = lines[0].strip()
        expansion = ""
        for l in lines:
            if l.startswith("**扩展**"):
                m = re.search(r"`([^`]*)`", l)
                if m:
                    expansion = m.group(1)
        cols = {"A": [], "B": [], "C": []}
        for l in lines:
            if not (l.startswith("|") and "---" not in l and "A ·" not in l):
                continue
            cells = l.split("|")
            if len(cells) < 5:
                continue
            for i, key in enumerate(("A", "B", "C")):
                cell = cells[2 + i].strip()
                if cell:
                    name, _, stars = cell.rpartition(" ★")
                    cols[key].append((name, stars))
        num = int(title.split(".")[0])
        q = title.split(". ", 1)[1]
        out.append({"num": num, "q": q, "expansion": expansion, "cols": cols})
    return out


def load_desc():
    con = sqlite3.connect(DB)
    d = {}
    for name, desc, pushed, lang, archived in con.execute(
        "SELECT full_name, description, pushed_at, language, archived FROM repos"
    ):
        d[name.lower()] = (desc or "", pushed or "", lang or "", archived)
    con.close()
    return d


def line(i, name, stars, desc):
    extra = ""
    if desc:
        text = desc if len(desc) <= 110 else desc[:110] + "…"
        extra = f" — {text}"
    return f"{i}. **{name}** ★{stars}{extra}"


def main():
    queries = parse(EVAL)
    desc = load_desc()
    os.makedirs(OUT, exist_ok=True)

    rng = random.Random(SEED)
    keys, pages = {}, {b: [] for b in range(1, BATCHES + 1)}

    for item in queries:
        labels = ["A", "B", "C"]
        shuffled = ["X", "Y", "Z"]
        rng.shuffle(shuffled)
        mapping = dict(zip(labels, shuffled))
        keys[f"q{item['num']:02d}"] = mapping

        blocks = []
        for label in labels:
            tag = mapping[label]
            # 不写描述：库里只覆盖 46%，且集中在 C 列，写出来等于泄露答案
            body = "\n".join(
                line(i, n, s, "")
                for i, (n, s) in enumerate(item["cols"][label], 1)
            ) or "（空）"
            blocks.append(f"**列表 {tag}**\n\n{body}")

        page = [
            f"## Q{item['num']:02d}　{item['q']}",
            "",
            "\n\n".join(blocks),
            "",
            "评分区：",
            "- X：相关 __/10，愿意点开 __ 条，理由：",
            "- Y：相关 __/10，愿意点开 __ 条，理由：",
            "- Z：相关 __/10，愿意点开 __ 条，理由：",
            "",
            "---",
            "",
        ]
        b = (item["num"] - 1) * BATCHES // len(queries) + 1
        pages[b].append("\n".join(page))

    for b, blocks in pages.items():
        path = os.path.join(OUT, f"batch{b}.md")
        header = (
            f"# 盲评批次 {b}\n\n"
            "下面是若干条中文搜索意图，每条给出三份候选榜单（X/Y/Z）。\n"
            "三份榜单来自不同来源，顺序已打乱，你不知道哪份是谁的——这是故意的。\n\n"
            "评分规则：\n"
            "1. 只看「这份榜单能不能让我找到想要的东西」\n"
            "2. 只给了仓库名和 star，按名字、star 和你自己的了解判断；判断不了就按 0 算\n"
            "3. 「愿意点开」= 看完仓库名你会真的点进去看 README\n"
            "4. 每项必须填数字，理由一句话\n"
            "5. 三份榜单的条数可能不同，条数少不是缺点——只看质量\n\n---\n\n"
        )
        open(path, "w", encoding="utf-8").write(header + "\n".join(blocks))

    json.dump(keys, open(os.path.join(OUT, "key.json"), "w"), indent=1)
    print(f"生成 {BATCHES} 批：", {b: len(v) for b, v in pages.items()})
    print("映射表：", os.path.join(OUT, "key.json"))


if __name__ == "__main__":
    main()
