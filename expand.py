"""查询扩展：中文自然语言 → 英文关键词组。

不做分词库依赖，用「最长优先 + 词表」的贪心扫描。

关键设计：遇到不认识的汉字时**只跳过这一个字**，绝不能吞掉后续整段——
否则「支持中文的命令行下载器」里的「中文的」会把后面的「命令行」一起吃掉。
"""

from lexicon import (
    CONCEPTS, FILLER, COMBOS, WEAK_CONCEPTS,
    _KEYS_BY_LEN, _FILLER_BY_LEN, ASCII_ALIASES, CHAR_MAP, CONCEPT_ALIASES,
)

_ASCII_WORD_CHARS = set("-_.+#")
CJK_START, CJK_END = "一", "鿿"


def normalize_query(query: str) -> str:
    """全角 ASCII / 空格和常见繁体字统一；字符数保持不变。"""
    return "".join(
        CHAR_MAP.get(ch, ch) if not ("！" <= ch <= "～" or ch == "　")
        else (" " if ch == "　" else chr(ord(ch) - 0xFEE0))
        for ch in query
    ).lower()


def _postprocess(groups, keys):
    """组合塌缩 + 弱概念裁剪。返回 (groups, keys)。

    GitHub 搜索是 AND 语义，每多一个词都把结果集按比例砍一刀，而且砍掉的是
    描述里没写这个词的正确仓库。实测 4 个词经常只剩十几条小仓库。
    """
    gs = [list(g) for g in groups]
    ks = list(keys)

    # ① 组合塌缩：识别+图片+文字 → ocr
    for triggers, consume, repl in COMBOS:
        if not triggers.issubset(ks):
            continue
        drop = {i for i, k in enumerate(ks) if k in consume}
        if not drop:
            continue
        pos = min(drop)
        kept = [i for i in range(len(gs)) if i not in drop]
        gs = [gs[i] for i in kept]
        ks = [ks[i] for i in kept]
        if repl is not None:
            at = sum(1 for i in kept if i < pos)
            gs.insert(at, list(repl))
            ks.insert(at, repl[0])

    # ② 弱概念裁剪：只在组数 ≥3 时做，且至少留 2 组
    if len(gs) >= 3:
        kept = [i for i, k in enumerate(ks) if k not in WEAK_CONCEPTS]
        if len(kept) >= 2:
            gs = [gs[i] for i in kept]
            ks = [ks[i] for i in kept]

    return gs, ks


def _is_ascii_word_char(ch: str) -> bool:
    return ch.isascii() and (ch.isalnum() or ch in _ASCII_WORD_CHARS)


def _is_cjk(ch: str) -> bool:
    return CJK_START <= ch <= CJK_END


def analyze(query: str) -> dict:
    """完整解析结果，供调试和覆盖率统计。"""
    original_query = query
    query = normalize_query(query)
    groups, keys, raw_cn, seen = [], [], [], set()
    buf = []
    matched_chars = 0
    total_cjk = sum(1 for ch in query if _is_cjk(ch))

    def flush():
        if buf:
            s = "".join(buf).strip()
            if s:
                raw_cn.append(s)
            buf.clear()

    def add(terms, key):
        sig = tuple(terms)
        if sig not in seen:
            seen.add(sig)
            groups.append(list(terms))
            keys.append(key)

    i, n = 0, len(query)
    while i < n:
        ch = query[i]

        if _is_ascii_word_char(ch):
            flush()
            j = i
            while j < n and _is_ascii_word_char(query[j]):
                j += 1
            raw_word = query[i:j].strip("-_")
            w = ASCII_ALIASES.get(raw_word, raw_word.strip("."))
            if w:
                add([w], w)
            i = j
            continue

        # 概念优先于填充词（「插件」这类词两边都有时以概念为准）
        hit = next((k for k in _KEYS_BY_LEN if query.startswith(k, i)), None)
        if hit:
            flush()
            add(CONCEPTS[hit], CONCEPT_ALIASES.get(hit, hit))
            matched_chars += sum(_is_cjk(c) for c in hit)
            i += len(hit)
            continue

        hit = next((k for k in _FILLER_BY_LEN if query.startswith(k, i)), None)
        if hit:
            flush()
            matched_chars += sum(_is_cjk(c) for c in hit)
            i += len(hit)
            continue

        if _is_cjk(ch):
            buf.append(ch)
        else:
            flush()
        i += 1

    flush()
    groups, keys = _postprocess(groups, keys)
    coverage = matched_chars / total_cjk if total_cjk else 1.0
    return {
        "query": original_query,
        "groups": groups,
        "keys": keys,
        "raw_cn": raw_cn,
        "coverage": coverage,
        "matched_chars": matched_chars,
        "total_cjk": total_cjk,
    }


def expand(query: str):
    """对外接口。返回 (groups, raw_cn)。"""
    a = analyze(query)
    return a["groups"], a["raw_cn"]


def describe(groups, raw_cn) -> str:
    """人能看懂的形式。raw_cn 不参与 AND，只用于 trigram 兜底。"""
    expr = " AND ".join("(" + " OR ".join(g) + ")" for g in groups) or "(空)"
    if raw_cn:
        expr += "   [词表未覆盖: " + " / ".join(raw_cn) + "]"
    return expr
