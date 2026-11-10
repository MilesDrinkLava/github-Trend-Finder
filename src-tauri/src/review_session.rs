//! One live search pool. Reserve an operation before awaiting to prevent double billing.
use crate::research::ResearchResult;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Sessions {
    session: String,
    operation: u64,
    busy: bool,
    result: Option<ResearchResult>,
}
impl Sessions {
    pub fn start_search(&mut self, session: &str, operation: u64) {
        self.session = session.into();
        self.operation = operation;
        self.busy = true;
        self.result = None;
    }
    pub fn ready(&self, session: &str) -> Result<ResearchResult, String> {
        if self.session != session {
            return Err("这轮搜索已失效，请使用当前搜索结果".into());
        }
        if self.busy {
            return Err("当前分析尚未结束，请稍候，避免重复调用".into());
        }
        self.result
            .clone()
            .ok_or_else(|| "还没有可继续分析的 GitHub 候选".into())
    }
    pub fn reserve(&mut self, session: &str, operation: u64) -> Result<(), String> {
        self.ready(session)?;
        self.operation = operation;
        self.busy = true;
        Ok(())
    }
    pub fn finish(&mut self, operation: u64, result: Option<ResearchResult>) {
        if operation == self.operation {
            if result.is_some() {
                self.result = result;
            }
            self.busy = false;
        }
    }
}

pub struct Operation {
    pub sessions: Arc<Mutex<Sessions>>,
    pub id: u64,
}
impl Operation {
    pub fn finish(&self, result: Option<ResearchResult>) {
        if let Ok(mut state) = self.sessions.lock() {
            state.finish(self.id, result);
        }
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.finish(None);
    }
}
