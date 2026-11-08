//! The class-search fetch (replan §6.2, §6.8, C4): a school's public Banner class search, read with
//! plain `ureq`, no window, no student cookie. **Stub**: C4's tests come first; the body is `todo!()`.
use std::time::Duration;

use knowlu_engine::registrar::SectionRef;
use serde_json::Value;

use crate::scaffold::ClassSearch;

/// At least this long between two requests (§6.8).
pub const GAP: Duration = Duration::from_secs(1);
/// Distinct `(subject, number)` queries a run may make; past it the rest are `Paused`.
pub const MAX_QUERIES: usize = 20;
/// Seconds a run may last before it starts another query; past it the rest are `Paused`.
pub const MAX_SECONDS: u64 = 60;
/// Rows asked for per page (`pageMaxSize`).
pub const PAGE_SIZE: usize = 50;
/// Pages followed for one course.
pub const MAX_PAGES: usize = 3;

/// The clock and the sleep a run uses, injected so tests never wait.
pub trait Pace {
    /// Time since an arbitrary fixed start; never goes back.
    fn now(&self) -> Duration;
    fn sleep(&mut self, d: Duration);
}

/// One search's key. Private fields and no `Default`: it cannot be built without a subject and a
/// number, so the whole term can never be asked for (§6.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    subject: String,
    number: String,
}

impl Query {
    /// `None` when the subject or the number is empty.
    pub fn new(_subject: &str, _number: &str) -> Option<Query> {
        todo!("C4")
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn number(&self) -> &str {
        &self.number
    }
}

/// The wanted sections grouped by distinct `(subject, number)`, in first-seen order, each with its
/// section codes in the order given. A section with no subject or no number makes no query.
pub fn queries(_wanted: &[SectionRef]) -> Vec<(Query, Vec<String>)> {
    todo!("C4")
}

/// How one query ended. Fixed words in the bundle: `answered`, `closed`, `paused`, `fetch_failed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Answered,
    Closed,
    Paused,
    FetchFailed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        todo!("C4")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub subject: String,
    pub number: String,
    pub sections: Vec<String>,
    pub status: Status,
    /// The raw answer text (all pages merged into one `{success, totalCount, data}` answer); empty
    /// unless `Answered` or `FetchFailed` after a partial read.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    pub school: String,
    pub term: String,
    pub queries: Vec<QueryResult>,
}

impl Bundle {
    /// `{school, term, queries: [{subject, number, sections, status, body}]}`, `body` a string.
    pub fn to_json(&self) -> Value {
        todo!("C4")
    }
}

/// `Knowlu/<version> (+https://knowlu.com)`.
pub fn user_agent() -> String {
    todo!("C4")
}

/// `https://<host>`, the base a real run uses.
pub fn https_base(_cs: &ClassSearch) -> String {
    todo!("C4")
}

/// One run: the term POST once, then reset and search per distinct query, serially. `base` is the
/// scheme and host (`https_base(cs)` in the app, a loopback fake in tests). Never fails: every
/// outcome is a `Status` in the bundle.
pub fn fetch(_base: &str, _cs: &ClassSearch, _term: &str, _wanted: &[SectionRef], _pace: &mut dyn Pace) -> Bundle {
    todo!("C4")
}
