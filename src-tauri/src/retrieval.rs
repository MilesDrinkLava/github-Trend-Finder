//! Fair first-page coverage, then spend requests where they add useful new candidates.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Route {
    pub query: String,
    pub readme: bool,
    pub page_size: usize,
    pub pages: usize,
    pub returned: usize,
    pub new_count: usize,
    pub useful_new: usize,
    pub stale_pages: usize,
    pub exhausted: bool,
}
impl Route {
    pub fn label(&self) -> String {
        format!(
            "{} [{}]",
            self.query,
            if self.readme {
                "readme"
            } else {
                "name,description"
            }
        )
    }
    pub fn observe(
        &mut self,
        returned: usize,
        new_count: usize,
        useful_new: usize,
        total: usize,
        incomplete: bool,
    ) {
        self.pages += 1;
        self.returned += returned;
        self.new_count += new_count;
        self.useful_new += useful_new;
        self.stale_pages = if new_count == 0 {
            self.stale_pages + 1
        } else {
            0
        };
        self.exhausted = (!incomplete
            && (returned < self.page_size || self.pages * self.page_size >= total.min(1000)))
            || self.pages >= 10
            || self.stale_pages >= 2;
    }
}

pub fn build(queries: &[String], target: usize) -> Vec<Route> {
    let direct_size = (target * 4 / 5).div_ceil(queries.len().max(1)).clamp(1, 50);
    let mut routes: Vec<Route> = queries
        .iter()
        .map(|q| Route {
            query: q.clone(),
            readme: false,
            page_size: direct_size,
            pages: 0,
            returned: 0,
            new_count: 0,
            useful_new: 0,
            stale_pages: 0,
            exhausted: false,
        })
        .collect();
    // Search both language ecosystems in README. Prefer short queries over the original long sentence.
    for chinese in [true, false] {
        if let Some(q) = queries
            .iter()
            .filter(|q| crate::research::has_chinese(q) == chinese)
            .min_by_key(|q| q.chars().count())
        {
            routes.push(Route {
                query: q.clone(),
                readme: true,
                page_size: (target / 10).clamp(1, 50),
                pages: 0,
                returned: 0,
                new_count: 0,
                useful_new: 0,
                stale_pages: 0,
                exhausted: false,
            });
        }
    }
    routes
}

pub fn next(routes: &[Route]) -> Option<usize> {
    if let Some((i, _)) = routes
        .iter()
        .enumerate()
        .find(|(_, r)| r.pages == 0 && !r.exhausted)
    {
        return Some(i);
    }
    routes
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.exhausted)
        .max_by(|(ia, a), (ib, b)| {
            let value = |r: &Route| {
                let novelty = (r.new_count as f64 + 1.0) / (r.returned as f64 + 2.0);
                let topical = (r.useful_new as f64 + 1.0) / (r.new_count as f64 + 2.0);
                let scope = if r.readme { 0.7 } else { 1.0 };
                scope * (0.35 + 0.65 * topical) * novelty / (r.pages as f64).sqrt()
                    + 0.12 / (r.pages as f64 + 1.0)
            };
            value(a).total_cmp(&value(b)).then(ib.cmp(ia))
        })
        .map(|(i, _)| i)
}
