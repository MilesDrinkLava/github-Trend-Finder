//! Fuse GitHub's own per-query relevance rankings; do not turn popularity into relevance.
use crate::research::Candidate;
use std::collections::HashSet;

pub fn choose(pool: &mut [Candidate], limit: usize, query: &str) -> Vec<usize> {
    let wants_resources = [
        "教程",
        "资源",
        "学习",
        "awesome",
        "tutorial",
        "course",
        "collection",
    ]
    .iter()
    .any(|x| query.to_lowercase().contains(x));
    for c in pool.iter_mut() {
        c.selected_for_ai = false;
        let positions: Vec<usize> = if c.route_ranks.is_empty() {
            vec![c.original_rank + 1]
        } else {
            c.route_ranks.values().copied().collect()
        };
        c.prefilter_score = if c.route_ranks.is_empty() {
            1.0 / (40.0 + c.original_rank as f64 + 1.0)
        } else {
            c.route_ranks
                .iter()
                .map(|(scope, r)| {
                    (if scope.ends_with("[readme]") {
                        0.35
                    } else {
                        1.0
                    }) / (40.0 + *r as f64)
                })
                .sum::<f64>()
        };
        let name = c.repo.full_name.to_lowercase();
        let is_list =
            name.contains("awesome-") || name.contains("/awesome") || name.contains("-resources");
        if is_list && !wants_resources {
            c.prefilter_score *= 0.45;
        }
        c.prefilter_reasons = vec![
            format!(
                "GitHub 最靠前排名：第 {} 位",
                positions.iter().min().unwrap_or(&1)
            ),
            format!("命中 {} 路检索", positions.len()),
        ];
        if is_list && !wants_resources {
            c.prefilter_reasons
                .push("资源汇总类降权，仍保留在检索结果中".into());
        }
    }
    let mut ranked: Vec<usize> = (0..pool.len()).collect();
    ranked.sort_by(|&a, &b| {
        pool[b]
            .prefilter_score
            .total_cmp(&pool[a].prefilter_score)
            .then(pool[a].original_rank.cmp(&pool[b].original_rank))
    });
    let limit = limit.min(pool.len());
    let mut chosen = Vec::new();
    let mut seen = HashSet::new();
    // Reserve leading matches across routes. README routes use at most one third of reserved slots.
    let mut routes: Vec<String> = pool
        .iter()
        .flat_map(|c| c.route_ranks.keys().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    routes.sort();
    let mut readme_reserved = 0;
    for route in routes {
        if chosen.len() >= limit / 2 {
            break;
        }
        let readme = route.ends_with("[readme]");
        if readme && readme_reserved >= limit.div_ceil(3) {
            continue;
        }
        let best = ranked
            .iter()
            .copied()
            .filter(|&at| pool[at].route_ranks.contains_key(&route))
            .min_by_key(|&at| pool[at].route_ranks[&route]);
        if let Some(at) = best {
            if seen.insert(at) {
                chosen.push(at);
                if readme {
                    readme_reserved += 1;
                }
            }
        }
    }
    for at in ranked {
        if chosen.len() == limit {
            break;
        }
        if seen.insert(at) {
            chosen.push(at);
        }
    }
    chosen.sort_by(|&a, &b| pool[b].prefilter_score.total_cmp(&pool[a].prefilter_score));
    for &at in &chosen {
        pool[at].selected_for_ai = true;
    }
    chosen
}

pub fn excerpt(text: &str, query: &str, max_bytes: usize) -> String {
    let mut tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect();
    let chinese: Vec<char> = query
        .chars()
        .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
        .collect();
    tokens.extend(chinese.windows(2).map(|w| w.iter().collect()));
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut order: Vec<usize> = (0..paragraphs.len()).collect();
    order.sort_by_key(|&i| {
        std::cmp::Reverse(
            tokens
                .iter()
                .filter(|t| paragraphs[i].to_lowercase().contains(t.as_str()))
                .count(),
        )
    });
    if !order.contains(&0) {
        order.insert(0, 0);
    }
    let mut selected = Vec::new();
    let mut bytes = 0;
    // A short introduction keeps context even when no exact query term appears.
    if let Some(first) = paragraphs.first() {
        let s = truncate(first, max_bytes.min(220));
        bytes += s.len();
        selected.push(s);
    }
    for i in order {
        if i == 0 || bytes + 2 >= max_bytes {
            continue;
        }
        let s = truncate(paragraphs[i], max_bytes - bytes - 2);
        bytes += s.len() + 2;
        if !s.is_empty() {
            selected.push(s);
        }
    }
    selected.join("\n\n")
}
pub fn truncate(text: &str, bytes: usize) -> String {
    let mut end = bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].into()
}
