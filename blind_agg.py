"""汇总盲评结果：把 X/Y/Z 还原成 A/B/C。

用法：python -X utf8 blind_agg.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
BLIND = os.path.join(HERE, "reports", "blind")

RE_Q = re.compile(r"^##\s*Q(\d+)", re.M)
RE_LINE = re.compile(
    r"^[-*]\s*([XYZ])[：:\s]*相关[^\d]*(\d+)[^\d]+.*?点开[^\d]*(\d+)",
    re.M,
)
RE_LOOSE = re.compile(r"^[-*]\s*([XYZ])[：:]", re.M)


def parse_file(path):
    text = open(path, encoding="utf-8").read()
    # 按 Q 切块
    marks = [(m.start(), int(m.group(1))) for m in RE_Q.finditer(text)]
    out = {}
    for i, (pos, num) in enumerate(marks):
        end = marks[i + 1][0] if i + 1 < len(marks) else len(text)
        block = text[pos:end]
        scores = {}
        for line in block.split("\n"):
            m = re.match(r"^[-*]\s*([XYZ])[：:]", line.strip())
            if not m:
                continue
            tag = m.group(1)
            rel = re.search(r"相关[^\d]*(\d+)\s*/\s*10", line)
            clk = re.search(r"点开[^\d]*(\d+)\s*条", line)
            if rel and clk:
                scores[tag] = (int(rel.group(1)), int(clk.group(1)))
            elif rel:
                scores[tag] = (int(rel.group(1)), 0)
        if scores:
            out[num] = scores
    return out


def main():
    key = json.load(open(os.path.join(BLIND, "key.json"), encoding="utf-8"))
    data = {}
    for n in (1, 2, 3):
        p = os.path.join(BLIND, f"result{n}.md")
        if os.path.exists(p):
            data.update(parse_file(p))

    print(f"解析到 {len(data)} 条查询的评分\n")

    rows = []
    for num in sorted(data):
        qk = f"q{num:02d}"
        if qk not in key:
            continue
        tag_of = key[qk]                       # {"A": "Y", ...}
        inv = {v: k for k, v in tag_of.items()}  # {"Y": "A", ...}
        s = {inv.get(t, t): v for t, v in data[num].items()}
        if len(s) < 3:
            print(f"  Q{num:02d} 缺列：{sorted(s)}")
            continue
        rows.append((num, s))

    if not rows:
        print("没有可汇总的数据")
        return

    print(f"{'Q':<4}{'A 相关/点开':>14}{'B 相关/点开':>14}{'C 相关/点开':>14}   胜者")
    wins = {k: 0 for k in "ABC"}
    rel = {k: 0 for k in "ABC"}
    clk = {k: 0 for k in "ABC"}
    for num, s in rows:
        for k in "ABC":
            rel[k] += s[k][0]
            clk[k] += s[k][1]
        best = max("ABC", key=lambda k: (s[k][0], s[k][1]))
        top = s[best]
        ties = [k for k in "ABC" if s[k] == top]
        winner = "并列" if len(ties) > 1 else best
        if winner != "并列":
            wins[winner] += 1
        print(
            f"Q{num:02d}  "
            f"{s['A'][0]:>6}/{s['A'][1]:<6}"
            f"{s['B'][0]:>6}/{s['B'][1]:<6}"
            f"{s['C'][0]:>6}/{s['C'][1]:<6}"
            f"   {winner}"
        )

    n = len(rows)
    print("\n=== 汇总 ===")
    for k, label in (("A", "GitHub 原生中文"), ("B", "GitHub 原生英文"), ("C", "本系统")):
        print(
            f"{label:<16} 平均相关 {rel[k] / n:5.2f}/10   "
            f"平均点开 {clk[k] / n:4.2f} 条   胜场 {wins[k]}/{n}"
        )

    print("\n=== C 逐条对比 ===")
    c_vs_a = c_vs_b = 0
    for num, s in rows:
        d_a, d_b = s["C"][0] - s["A"][0], s["C"][0] - s["B"][0]
        c_vs_a += d_a > 0
        c_vs_b += d_b > 0
    print(f"C 相关性高于 A 的查询：{c_vs_a}/{n}")
    print(f"C 相关性高于 B 的查询：{c_vs_b}/{n}")


if __name__ == "__main__":
    main()
