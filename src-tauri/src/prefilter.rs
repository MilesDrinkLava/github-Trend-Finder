//! Query-aware ranking with correlated-route suppression and diverse exploration.
use crate::intent::Mode;
use crate::research::{Candidate, Plan};
use std::collections::{HashMap, HashSet};

fn similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        0.0
    } else {
        a.intersection(b).count() as f64 / a.union(b).count() as f64
    }
}

pub fn topical(c: &Candidate, plan: &Plan, query: &str) -> f64 {
    let cores: Vec<_> = plan
        .criteria
        .iter()
        .filter(|c| c.mode == Mode::Core)
        .collect();
    let fields = [
        (c.repo.full_name.as_str(), 1.0),
        (c.repo.description.as_deref().unwrap_or(""), 0.9),
    ];
    let topics = c.repo.topics.join(" ");
    let score = |terms: &[String]| {
        fields
            .iter()
            .map(|(s, w)| crate::judgment::match_terms(s, terms) * w)
            .fold(
                crate::judgment::match_terms(&topics, terms) * 0.95,
                f64::max,
            )
    };
    if cores.is_empty() {
        return score(&[query.into()]);
    }
    cores
        .iter()
        .map(|c| score(&crate::judgment::criterion_terms(c)))
        .sum::<f64>()
        / cores.len() as f64
}

fn route_groups(pool: &[Candidate]) -> HashMap<String, usize> {
    let mut routes: Vec<String> = pool
        .iter()
        .flat_map(|c| c.route_ranks.keys().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    routes.sort();
    let mut groups: Vec<HashSet<u64>> = Vec::new();
    let mut assignment = HashMap::new();
    for route in routes {
        let heads: HashSet<u64> = pool
            .iter()
            .filter(|c| c.route_ranks.get(&route).is_some_and(|r| *r <= 20))
            .map(|c| c.repo.id)
            .collect();
        // Near-identical result sets are one source of evidence, regardless of query spelling.
        let group = groups
            .iter()
            .position(|g| {
                !heads.is_empty()
                    && heads.len().min(g.len()) >= 5
                    && heads.intersection(g).count() as f64 / heads.union(g).count().max(1) as f64
                        >= 0.65
            })
            .unwrap_or_else(|| {
                groups.push(heads);
                groups.len() - 1
            });
        assignment.insert(route, group);
    }
    assignment
}

fn diverse(pool: &[Candidate], eligible: &[usize], count: usize, seed: &[usize]) -> Vec<usize> {
    let descriptions: Vec<HashSet<String>> = pool
        .iter()
        .map(|c| {
            crate::intent::tokens(
                c.repo
                    .description
                    .as_deref()
                    .unwrap_or_else(|| c.repo.full_name.split('/').next_back().unwrap_or("")),
            )
        })
        .map(|v| v.into_iter().collect())
        .collect();
    let mut out = Vec::new();
    let mut used: HashSet<usize> = seed.iter().copied().collect();
    let mut redundancy: Vec<f64> = (0..pool.len())
        .map(|i| {
            seed.iter()
                .map(|&other| similarity(&descriptions[i], &descriptions[other]))
                .fold(0.0, f64::max)
        })
        .collect();
    while out.len() < count {
        let best = eligible
            .iter()
            .copied()
            .filter(|i| !used.contains(i))
            .max_by(|&a, &b| {
                let value = |at: usize| pool[at].prefilter_score - 0.22 * redundancy[at];
                value(a)
                    .total_cmp(&value(b))
                    .then(pool[b].original_rank.cmp(&pool[a].original_rank))
            });
        let Some(at) = best else {
            break;
        };
        used.insert(at);
        out.push(at);
        for &i in eligible {
            redundancy[i] = redundancy[i].max(similarity(&descriptions[i], &descriptions[at]));
        }
    }
    out
}

pub fn next_candidates(pool: &[Candidate], limit: usize) -> Vec<usize> {
    let eligible: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, c)| c.assessment.is_none())
        .map(|(i, _)| i)
        .collect();
    let seeds: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, c)| c.assessment.is_some())
        .map(|(i, _)| i)
        .collect();
    diverse(pool, &eligible, limit, &seeds)
}

pub fn choose_with_plan(
    pool: &mut [Candidate],
    limit: usize,
    query: &str,
    plan: &Plan,
) -> Vec<usize> {
    let groups = route_groups(pool);
    let group_count = groups.values().collect::<HashSet<_>>().len().clamp(1, 3) as f64;
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
        let lexical = topical(c, plan, query);
        let mut votes: HashMap<usize, f64> = HashMap::new();
        for (route, rank) in &c.route_ranks {
            let vote = 41.0 / (40.0 + *rank as f64)
                * if route.ends_with("[readme]") {
                    0.6
                } else {
                    1.0
                };
            let group = *groups.get(route).unwrap_or(&usize::MAX);
            votes
                .entry(group)
                .and_modify(|v| *v = v.max(vote))
                .or_insert(vote);
        }
        let mut values: Vec<f64> = votes.values().copied().collect();
        values.sort_by(|a, b| b.total_cmp(a));
        let rank = if values.is_empty() {
            41.0 / (41.0 + c.original_rank as f64)
        } else {
            values.iter().take(3).sum::<f64>() / group_count
        };
        c.prefilter_score = 0.6 * lexical + 0.4 * rank;
        c.prefilter_reasons = vec![
            format!(
                "核心任务词覆盖 {:.0}%（仅用于召回，不代表能力已验证）",
                lexical * 100.0
            ),
            format!(
                "{} 路检索归并为 {} 组去重排名信号",
                c.route_ranks.len(),
                votes.len()
            ),
        ];
        let name = c.repo.full_name.to_lowercase();
        if !wants_resources
            && (name.contains("awesome-")
                || name.contains("/awesome")
                || name.contains("-resources"))
        {
            c.prefilter_score *= 0.5;
            c.prefilter_reasons
                .push("汇总目录降权，原始候选仍保留".into());
        }
    }
    let limit = limit.min(pool.len());
    let probe = if limit >= 5 { limit.div_ceil(7) } else { 0 };
    let eligible: Vec<usize> = (0..pool.len()).collect();
    let mut chosen = diverse(pool, &eligible, limit - probe, &[]);
    let mut seen: HashSet<usize> = chosen.iter().copied().collect();
    let mut covered: HashSet<usize> = chosen
        .iter()
        .flat_map(|&i| {
            pool[i]
                .route_ranks
                .keys()
                .filter_map(|r| groups.get(r).copied())
        })
        .collect();
    for _ in 0..probe {
        let at = eligible
            .iter()
            .copied()
            .filter(|i| !seen.contains(i))
            .max_by(|&a, &b| {
                let novelty = |i: usize| {
                    pool[i]
                        .route_ranks
                        .keys()
                        .filter_map(|r| groups.get(r))
                        .any(|g| !covered.contains(g))
                };
                novelty(a).cmp(&novelty(b)).then_with(|| {
                    let head = |i: usize| {
                        pool[i]
                            .route_ranks
                            .values()
                            .min()
                            .copied()
                            .unwrap_or(pool[i].original_rank + 1)
                    };
                    head(b)
                        .cmp(&head(a))
                        .then(pool[a].prefilter_score.total_cmp(&pool[b].prefilter_score))
                })
            });
        if let Some(at) = at {
            chosen.push(at);
            seen.insert(at);
            covered.extend(
                pool[at]
                    .route_ranks
                    .keys()
                    .filter_map(|r| groups.get(r).copied()),
            );
            pool[at]
                .prefilter_reasons
                .push("探索名额：保留不同检索来源，交给模型核实".into());
        }
    }
    for &at in &chosen {
        pool[at].selected_for_ai = true;
    }
    chosen
}

/// Legacy caller compatibility: use actual route expressions as bilingual alternatives.
#[cfg(test)]
pub fn choose(pool: &mut [Candidate], limit: usize, query: &str) -> Vec<usize> {
    let mut plan = crate::intent::fallback(query);
    let mut terms: Vec<String> = pool
        .iter()
        .flat_map(|c| c.route_ranks.keys())
        .map(|r| r.rsplit_once(" [").map(|x| x.0).unwrap_or(r).to_string())
        .collect();
    terms.sort();
    terms.dedup();
    terms.truncate(12);
    plan.criteria[0].terms = terms;
    choose_with_plan(pool, limit, query, &plan)
}

#[cfg(test)]
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
