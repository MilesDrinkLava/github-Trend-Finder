//! 中文 → 英文查询构词。
//!
//! 与 Python 版 expand.py / lexicon.py 保持一致：
//! 最长匹配扫描 → 概念组合塌缩 → 弱概念裁剪。
//! 词表由 export_lexicon.py 从 lexicon.py 导出，改词表请改 Python 再重跑。

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

const RAW: &str = include_str!("../lexicon.json");

#[derive(Deserialize)]
struct LexData {
    concepts: HashMap<String, Vec<String>>,
    filler: Vec<String>,
    combos: Vec<(Vec<String>, Vec<String>, Option<Vec<String>>)>,
    weak: Vec<String>,
}

pub struct Lexicon {
    concepts: HashMap<String, Vec<String>>,
    concept_keys: Vec<String>,
    filler_keys: Vec<String>,
    combos: Vec<(HashSet<String>, HashSet<String>, Option<Vec<String>>)>,
    weak: HashSet<String>,
}

pub struct Analysis {
    pub groups: Vec<Vec<String>>,
    pub keys: Vec<String>,
    pub raw_cn: Vec<String>,
    pub coverage: f64,
}

fn by_len_desc(a: &String, b: &String) -> std::cmp::Ordering {
    b.chars()
        .count()
        .cmp(&a.chars().count())
        .then_with(|| a.cmp(b))
}

fn is_ascii_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

fn is_cjk(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
}

impl Lexicon {
    pub fn load() -> Self {
        let d: LexData =
            serde_json::from_str(RAW).expect("lexicon.json 解析失败，请重跑 export_lexicon.py");

        let mut concept_keys: Vec<String> = d.concepts.keys().cloned().collect();
        concept_keys.sort_by(by_len_desc);

        let mut filler_keys = d.filler.clone();
        filler_keys.sort_by(by_len_desc);

        let combos = d
            .combos
            .into_iter()
            .map(|(t, c, r)| {
                (
                    t.into_iter().collect::<HashSet<_>>(),
                    c.into_iter().collect::<HashSet<_>>(),
                    r,
                )
            })
            .collect();

        Self {
            concepts: d.concepts,
            concept_keys,
            filler_keys,
            combos,
            weak: d.weak.into_iter().collect(),
        }
    }

    pub fn analyze(&self, query: &str) -> Analysis {
        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        let mut raw_cn: Vec<String> = Vec::new();
        let mut seen: HashSet<Vec<String>> = HashSet::new();
        let mut buf = String::new();
        let total_cjk = query.chars().filter(|c| is_cjk(*c)).count();
        let mut matched_chars = 0usize;

        fn flush(buf: &mut String, raw_cn: &mut Vec<String>) {
            let s = buf.trim();
            if !s.is_empty() {
                raw_cn.push(s.to_string());
            }
            buf.clear();
        }

        fn add(
            groups: &mut Vec<Vec<String>>,
            keys: &mut Vec<String>,
            seen: &mut HashSet<Vec<String>>,
            terms: Vec<String>,
            key: String,
        ) {
            if seen.insert(terms.clone()) {
                groups.push(terms);
                keys.push(key);
            }
        }

        let mut i = 0usize;
        while i < query.len() {
            let rest = &query[i..];
            let ch = rest.chars().next().unwrap();

            // ① ASCII 单词原样保留（ocr / epub / react 这类直接就是检索词）
            if is_ascii_word_char(ch) {
                flush(&mut buf, &mut raw_cn);
                let mut j = i;
                while j < query.len() {
                    let c = query[j..].chars().next().unwrap();
                    if is_ascii_word_char(c) {
                        j += c.len_utf8();
                    } else {
                        break;
                    }
                }
                let w = query[i..j]
                    .trim_matches(|c| c == '-' || c == '_' || c == '.')
                    .to_lowercase();
                if !w.is_empty() {
                    add(&mut groups, &mut keys, &mut seen, vec![w.clone()], w);
                }
                i = j;
                continue;
            }

            // ② 概念优先于填充词（「插件」两边都有时以概念为准）
            if let Some(k) = self.concept_keys.iter().find(|k| rest.starts_with(*k)) {
                flush(&mut buf, &mut raw_cn);
                let terms = self.concepts.get(k).cloned().unwrap_or_default();
                matched_chars += k.chars().count();
                add(&mut groups, &mut keys, &mut seen, terms, k.clone());
                i += k.len();
                continue;
            }

            // ③ 填充词是「有意丢弃」，不算未覆盖
            if let Some(k) = self.filler_keys.iter().find(|k| rest.starts_with(*k)) {
                flush(&mut buf, &mut raw_cn);
                matched_chars += k.chars().count();
                i += k.len();
                continue;
            }

            // ④ 没认出来的汉字先攒着，只跳一个字，绝不吞后面
            if is_cjk(ch) {
                buf.push(ch);
            }
            i += ch.len_utf8();
        }
        flush(&mut buf, &mut raw_cn);

        let (groups, keys) = self.postprocess(groups, keys);
        let coverage = if total_cjk == 0 {
            1.0
        } else {
            let _ = matched_chars;
            matched_chars as f64 / total_cjk as f64
        };

        Analysis {
            groups,
            keys,
            raw_cn,
            coverage,
        }
    }

    /// 组合塌缩 + 弱概念裁剪。
    ///
    /// GitHub 搜索是 AND 语义，每多一个词就按比例砍结果集，砍掉的还是
    /// 描述里没写这个词的正确仓库。
    fn postprocess(
        &self,
        groups: Vec<Vec<String>>,
        keys: Vec<String>,
    ) -> (Vec<Vec<String>>, Vec<String>) {
        let mut gs = groups;
        let mut ks = keys;

        for (triggers, consume, repl) in &self.combos {
            let present: HashSet<&String> = ks.iter().collect();
            if !triggers.iter().all(|t| present.contains(t)) {
                continue;
            }
            let drop: HashSet<usize> = ks
                .iter()
                .enumerate()
                .filter(|(_, k)| consume.contains(*k))
                .map(|(i, _)| i)
                .collect();
            if drop.is_empty() {
                continue;
            }
            let pos = *drop.iter().min().unwrap();
            let kept: Vec<usize> = (0..gs.len()).filter(|i| !drop.contains(i)).collect();

            let new_gs: Vec<Vec<String>> = kept.iter().map(|&i| gs[i].clone()).collect();
            let new_ks: Vec<String> = kept.iter().map(|&i| ks[i].clone()).collect();
            gs = new_gs;
            ks = new_ks;

            if let Some(r) = repl {
                if r.is_empty() {
                    continue;
                }
                let at = kept.iter().filter(|&&i| i < pos).count();
                gs.insert(at, r.clone());
                ks.insert(at, r[0].clone());
            }
        }

        if gs.len() >= 3 {
            let kept: Vec<usize> = ks
                .iter()
                .enumerate()
                .filter(|(_, k)| !self.weak.contains(*k))
                .map(|(i, _)| i)
                .collect();
            if kept.len() >= 2 {
                let new_gs: Vec<Vec<String>> = kept.iter().map(|&i| gs[i].clone()).collect();
                let new_ks: Vec<String> = kept.iter().map(|&i| ks[i].clone()).collect();
                gs = new_gs;
                ks = new_ks;
            }
        }

        (gs, ks)
    }
}

/// 每组取第一个英文说法，空格拼接。
///
/// 同义词顺序是人工按准确度排的，比词频统计靠谱（词频会把桌面→linux、
/// 数据库→db、爬虫→crawl）。
pub fn to_english(groups: &[Vec<String>]) -> String {
    groups
        .iter()
        .filter_map(|g| g.first())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}
