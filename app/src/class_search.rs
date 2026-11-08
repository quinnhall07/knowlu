//! The class-search fetch (replan §6.2, §6.8, C4): a school's public Banner class search, read with
//! plain `ureq`, no window, no student cookie. Redirects are never followed (a redirect is `closed`),
//! and no answer is retried: the first refusal ends the run.
use std::time::{Duration, Instant};

use knowlu_engine::registrar::SectionRef;
use serde_json::{json, Value};

use crate::scaffold::ClassSearch;

/// At least this long between two requests (§6.8).
pub const GAP: Duration = Duration::from_secs(1);
/// Distinct `(subject, number)` queries a run may make; past it the rest are `Paused`.
pub const MAX_QUERIES: usize = 20;
/// Seconds a run may last before it starts another query; past it the rest are `Paused`. The cap is
/// checked only before each query, and each request is allowed 30 s (`agent`), so a run's real bound
/// is 60 s plus one query's requests: up to 30 s each, and a reset and three pages are four of them.
/// A run past the cap pauses the queries it has not started; it never cuts one short.
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

/// The real clock and sleep: time since it was built, and `thread::sleep`.
pub struct RealPace {
    start: Instant,
}

impl RealPace {
    pub fn new() -> RealPace {
        RealPace { start: Instant::now() }
    }
}

impl Pace for RealPace {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }
    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
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
    pub fn new(subject: &str, number: &str) -> Option<Query> {
        if subject.is_empty() || number.is_empty() {
            return None;
        }
        Some(Query { subject: subject.to_string(), number: number.to_string() })
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn number(&self) -> &str {
        &self.number
    }
}

/// The wanted sections grouped by distinct `(subject, number)`, in first-seen order, each with its
/// section codes in the order given. A section with no subject or no number makes no query, and nor
/// does one of another term than `term` (§6.4 `other_term`).
pub fn queries(wanted: &[SectionRef], term: &str) -> Vec<(Query, Vec<String>)> {
    let mut out: Vec<(Query, Vec<String>)> = Vec::new();
    for w in wanted.iter().filter(|w| w.term == term) {
        let Some(q) = Query::new(&w.subject, &w.number) else { continue };
        match out.iter_mut().find(|(have, _)| *have == q) {
            Some((_, sections)) => sections.push(w.section.clone()),
            None => out.push((q, vec![w.section.clone()])),
        }
    }
    out
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
        match self {
            Status::Answered => "answered",
            Status::Closed => "closed",
            Status::Paused => "paused",
            Status::FetchFailed => "fetch_failed",
        }
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
        let queries: Vec<Value> = self.queries.iter().map(|q| json!({
            "subject": q.subject,
            "number": q.number,
            "sections": q.sections,
            "status": q.status.as_str(),
            "body": q.body,
        })).collect();
        json!({ "school": self.school, "term": self.term, "queries": queries })
    }
}

/// `Knowlu/<version> (+https://knowlu.com)`.
pub fn user_agent() -> String {
    format!("Knowlu/{} (+https://knowlu.com)", env!("CARGO_PKG_VERSION"))
}

/// `https://<host>`, the base a real run uses.
pub fn https_base(cs: &ClassSearch) -> String {
    format!("https://{}", cs.host)
}

/// What ends a run. Each is a named outcome for the rest of the queries, never an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// A 429, a 403 or a 5xx: the school asked us to stop.
    Paused,
    /// A redirect, a sign-in, an answer that is not class-search JSON, or no rows after a good term POST.
    Closed,
    /// No answer at all, or one that cannot be read.
    Failed,
}

impl Stop {
    fn status(self) -> Status {
        match self {
            Stop::Paused => Status::Paused,
            Stop::Closed => Status::Closed,
            Stop::Failed => Status::FetchFailed,
        }
    }
}

/// Percent-encoding for one form or query value: everything but the unreserved set.
fn enc(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The `name=value` pairs of a set of `Set-Cookie` lines, joined as a `Cookie` header; `None` when
/// there is no pair. Attributes are dropped, a later pair of the same name replaces an earlier one,
/// and an empty value (a deletion) is not kept. It lives on the stack of one run, never in a file.
fn cookie_header(set_cookies: &[String]) -> Option<String> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in set_cookies {
        let first = line.split(';').next().unwrap_or("").trim();
        let Some((name, value)) = first.split_once('=') else { continue };
        let (name, value) = (name.trim(), value.trim());
        if name.is_empty() || value.is_empty() {
            continue;
        }
        pairs.retain(|(n, _)| n != name);
        pairs.push((name.to_string(), value.to_string()));
    }
    if pairs.is_empty() {
        return None;
    }
    Some(pairs.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join("; "))
}

/// A fresh agent for one request. The engine's `ureq` line turns the cookie jar on for the whole
/// build, and a jar belongs to its agent, so one agent per request has nothing stored to send: the one
/// anonymous cookie is carried by hand and no other goes out. Redirects are never followed.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .user_agent(user_agent())
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

/// One run's connection to the school: plain `ureq`, no jar, the one anonymous cookie carried by hand.
struct Session<'a> {
    base: String,
    prefix: &'static str,
    pace: &'a mut dyn Pace,
    /// The clock when the last request ended.
    last: Option<Duration>,
    cookie: Option<String>,
    /// Some query of this run has returned rows, so the cookie is known to be good.
    proven: bool,
}

impl Session<'_> {
    /// One request, after the gap. `Ok` is a 2xx, with its `Set-Cookie` lines and its body.
    fn call(&mut self, post: bool, target: &str, form: Option<&str>) -> Result<(Vec<String>, String), Stop> {
        if let Some(last) = self.last {
            let since = self.pace.now().saturating_sub(last);
            if since < GAP {
                self.pace.sleep(GAP - since);
            }
        }
        let url = format!("{}{}", self.base, target);
        let agent = agent();
        let sent = if post {
            let mut req = agent.post(&url);
            if let Some(c) = &self.cookie {
                req = req.header("cookie", c);
            }
            match form {
                Some(f) => req.header("content-type", "application/x-www-form-urlencoded").send(f.to_string()),
                None => req.send_empty(),
            }
        } else {
            let mut req = agent.get(&url);
            if let Some(c) = &self.cookie {
                req = req.header("cookie", c);
            }
            req.call()
        };
        self.last = Some(self.pace.now());
        let mut resp = sent.map_err(|_| Stop::Failed)?;
        match resp.status().as_u16() {
            200..=299 => {}
            300..=399 | 401 => return Err(Stop::Closed),
            429 | 403 | 500..=599 => return Err(Stop::Paused),
            _ => return Err(Stop::Failed),
        }
        let cookies = resp.headers().get_all("set-cookie").iter()
            .filter_map(|v| v.to_str().ok()).map(str::to_string).collect();
        let body = resp.body_mut().with_config().limit(1 << 22).read_to_string().map_err(|_| Stop::Failed)?;
        self.last = Some(self.pace.now());
        Ok((cookies, body))
    }

    /// The term POST, once; keeps the answer's cookie.
    fn open(&mut self, term: &str) -> Result<(), Stop> {
        let target = format!("{}term/search?mode=search", self.prefix);
        let (cookies, body) = self.call(true, &target, Some(&format!("term={}", enc(term))))?;
        if serde_json::from_str::<Value>(&body).is_err() {
            return Err(Stop::Closed);
        }
        self.cookie = cookie_header(&cookies);
        Ok(())
    }

    /// One page of one search: the answer's text and its parsed form. An answer that is not
    /// `{data: [...]}` is `Closed`, and so is a first page with no rows until a query of this run has
    /// returned rows (R0b: a search with no accepted cookie answers 200 with none; once one has, the
    /// cookie is proven and an empty answer is the school's "no such course").
    fn page(&mut self, q: &Query, term: &str, offset: usize) -> Result<(String, Value), Stop> {
        let target = format!(
            "{}searchResults/searchResults?txt_term={}&txt_subject={}&txt_courseNumber={}&pageOffset={offset}&pageMaxSize={PAGE_SIZE}",
            self.prefix, enc(term), enc(q.subject()), enc(q.number()),
        );
        let (_, text) = self.call(false, &target, None)?;
        let answer: Value = serde_json::from_str(&text).map_err(|_| Stop::Closed)?;
        let empty = answer.get("data").and_then(Value::as_array).ok_or(Stop::Closed)?.is_empty();
        if empty && offset == 0 && !self.proven {
            return Err(Stop::Closed);
        }
        Ok((text, answer))
    }

    /// The reset, then the course's pages (at most `MAX_PAGES`), read as one answer.
    fn course(&mut self, q: &Query, term: &str, sections: &[String]) -> Result<(Status, String), Stop> {
        self.call(true, &format!("{}classSearch/resetDataForm", self.prefix), None)?;
        let (raw, mut answer) = self.page(q, term, 0)?;
        let total = answer.get("totalCount").and_then(Value::as_u64).map_or(0, |n| n as usize);
        let mut rows: Vec<Value> = answer["data"].as_array().cloned().unwrap_or_default();
        self.proven |= !rows.is_empty();
        let mut pages = 1;
        while rows.len() < total && pages < MAX_PAGES {
            let (_, next) = self.page(q, term, pages * PAGE_SIZE)?;
            let more = next["data"].as_array().cloned().unwrap_or_default();
            if more.is_empty() {
                break;
            }
            rows.extend(more);
            pages += 1;
        }
        let field = |r: &Value, key: &str| r.get(key).and_then(Value::as_str).map(str::to_string);
        let seen = |section: &String| rows.iter().any(|r| {
            field(r, "term").as_deref() == Some(term)
                && field(r, "subject").as_deref() == Some(q.subject())
                && field(r, "courseNumber").as_deref() == Some(q.number())
                && field(r, "sequenceNumber").as_deref() == Some(section.as_str())
        });
        // Rows left unread matter only when a wanted section could be among them.
        let status = if rows.len() < total && !sections.iter().all(seen) { Status::FetchFailed } else { Status::Answered };
        let body = if pages == 1 {
            raw
        } else {
            answer["data"] = Value::Array(rows);
            answer.to_string()
        };
        Ok((status, body))
    }
}

/// One run: the term POST once, then reset and search per distinct query, serially. `base` is the
/// scheme and host (`https_base(cs)` in the app, a loopback fake in tests). Never fails: every
/// outcome is a `Status` in the bundle.
pub fn fetch(base: &str, cs: &ClassSearch, term: &str, wanted: &[SectionRef], pace: &mut dyn Pace) -> Bundle {
    let groups = queries(wanted, term);
    let mut bundle = Bundle { school: cs.school.to_string(), term: term.to_string(), queries: Vec::new() };
    if groups.is_empty() {
        return bundle;
    }
    let started = pace.now();
    let mut session = Session { base: base.trim_end_matches('/').to_string(), prefix: cs.prefix, pace, last: None, cookie: None, proven: false };
    // The first thing that ends the run; every query from then on ends the same way.
    let mut stopped: Option<Stop> = session.open(term).err();
    for (i, (q, sections)) in groups.into_iter().enumerate() {
        let late = i >= MAX_QUERIES || session.pace.now().saturating_sub(started).as_secs() >= MAX_SECONDS;
        let (status, body) = match stopped {
            Some(stop) => (stop.status(), String::new()),
            None if late => (Status::Paused, String::new()),
            None => match session.course(&q, term, &sections) {
                Ok(done) => done,
                Err(stop) => {
                    stopped = Some(stop);
                    (stop.status(), String::new())
                }
            },
        };
        bundle.queries.push(QueryResult { subject: q.subject, number: q.number, sections, status, body });
    }
    bundle
}

