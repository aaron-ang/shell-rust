use crate::history::History;
use crate::job::Jobs;

#[derive(Clone)]
pub struct Shell {
    pub history: History,
    pub jobs: Jobs,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            history: History::open(),
            jobs: Jobs::new(),
        }
    }
}
