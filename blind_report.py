"""把盲评评分渲染成 reports/agent-eval.md。"""

import json
import os

from blind_agg import BLIND, parse_file

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    key = json.load(open(os.path.join(BLIND, "key.json"), encoding="utf-8"))
    data = {}
    for n in (1, 2, 3):
        data.update(parse_file(os.path.join(BLIND, f"result{n}.md")))

    qs = [l.strip() for l in open(os.path.join(HERE, "queries.txt"), encoding="utf-8")
          if l.strip() and not l.startswith("#")]

    rows = []
    for num in sorted(data):
        inv = {v: k for k, v in key["q%02d" % num].items()}
        s = {inv.get(t, t): v for t, v in data[num].items()}
        rows.append((num, qs[num - 1] if num <= len(qs) else "?", s))

    rel = {k: 0 for k in "ABC"}
    clk = {k: 0 for k in "ABC"}
    wins = {k: 0 for k in "ABC"}
    for _, _, s in rows:
        for k in "ABC":
            rel[k] += s[k][0]
            clk[k] += s[k][1]
        top = max(s[k][0] for k in "ABC")
        ties = [k for k in "ABC" if s[k][0] == top]
        if len(ties) == 1:
            wins[ties[0]] += 1
    n = len(rows)

    out = [
        "# 子 agent 盲评结果",
        "",
        "3 个独立 agent 评审，A/B/C 打乱为 X/Y/Z，互相不可见，不联网，"
        "只看仓库名与 star。共 %d 条查询。" % n,
        "",
        "| 来源 | 平均相关性 | 平均愿意点开 | 胜场 |",
        "|---|---|---|---|",
        "| A · GitHub 原生（中文） | %.2f/10 | %.2f 条 | %d/%d |" % (rel["A"] / n, clk["A"] / n, wins["A"], n),
        "| B · GitHub 原生（英文） | %.2f/10 | %.2f 条 | %d/%d |" % (rel["B"] / n, clk["B"] / n, wins["B"], n),
        "| C · 本系统 | %.2f/10 | %.2f 条 | %d/%d |" % (rel["C"] / n, clk["C"] / n, wins["C"], n),
        "",
        "## 逐条",
        "",
        "| # | 查询 | A 相关/点开 | B 相关/点开 | C 相关/点开 | 胜者 |",
        "|---|---|---|---|---|---|",
    ]
    for num, q, s in rows:
        top = max(s[k][0] for k in "ABC")
        ties = [k for k in "ABC" if s[k][0] == top]
        w = "并列" if len(ties) > 1 else ties[0]
        out.append("| %d | %s | %d/%d | %d/%d | %d/%d | %s |" % (
            num, q, s["A"][0], s["A"][1], s["B"][0], s["B"][1], s["C"][0], s["C"][1], w))

    out += [
        "",
        "## 结论",
        "",
        "- **A（GitHub 原生中文）基本不可用**：平均 1.30/10，19 条查询输给本系统。"
        "大量结果被 `china-dictatorship` 等无关仓库刷屏。",
        "- **B（英文关键词）反超 C（本系统）**：相关性 7.20 vs 6.40，胜场 11 vs 8。"
        "结论是「翻译层有价值，排序层目前是负收益」。",
        "- **C 的优势场景**：查询本身带中文特有概念时（markdown 笔记、OCR、代码补全、"
        "命令行下载器），扩展层能拉出 B 搜不到的东西。",
        "- **C 的劣势场景**：泛关键词查询（gif、pdf、向量库）。软 AND 放进来一批"
        "「沾边但不对口」的仓库——视频播放器支持导出 GIF、PDF 库的 topics 里有 html。",
        "",
        "## 方法学提醒",
        "",
        "评审只能看到仓库名和 star，因此**对知名项目有天然偏袒**。"
        "B 列来自 GitHub 原生排序，天然是高分知名项目；C 列会推出冷门但正确的项目，"
        "评审不认识就按 0 分算。所以 B 的实际领先幅度可能小于表上的 0.8 分。",
        "反过来，C 列里被点名的噪音仓库是真实存在的，这部分结论不受此偏差影响。",
    ]

    path = os.path.join(HERE, "reports", "agent-eval.md")
    open(path, "w", encoding="utf-8").write("\n".join(out) + "\n")
    print("已写入", path)


if __name__ == "__main__":
    main()
