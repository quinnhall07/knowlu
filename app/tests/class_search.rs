//! C4 (replan §6.2, §6.8, §8): the class-search fetch, on a fake Banner bound to `127.0.0.1:0` that
//! serves itself. No egress, no name resolution. Hosts are invented. The sleep and the clock are
//! injected (`FakePace`), so nothing here waits, and a request's time is the fake clock's, never the
//! wall's.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use knowlu::class_search::{self, fetch, queries, Bundle, Pace, Query, QueryResult, Status};
use knowlu::scaffold::{ClassSearch, CAMPUSES};
use knowlu_engine::registrar::{SectionRef, SCHOOLS};
use serde_json::{json, Value};

const PREFIX: &str = "/StudentRegistrationSsb/ssb/";
const TERM: &str = "202640";
const COOKIE: &str = "JSESSIONID=sess-invented-1";

fn cs() -> ClassSearch {
    ClassSearch { school: "ua", host: "registrar.invented.test", prefix: PREFIX, checked: "2026-11-08" }
}

fn want(subject: &str, number: &str, section: &str) -> SectionRef {
    SectionRef { term: TERM.into(), subject: subject.into(), number: number.into(), section: section.into() }
}

// ---- the fake clock -------------------------------------------------------------------------

type Clock = Arc<Mutex<Duration>>;

struct FakePace(Clock);
impl Pace for FakePace {
    fn now(&self) -> Duration {
        *self.0.lock().unwrap()
    }
    fn sleep(&mut self, d: Duration) {
        *self.0.lock().unwrap() += d;
    }
}

// ---- the fake Banner ------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Req {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: String,
    /// The fake clock when it arrived.
    at: Duration,
}

impl Req {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }
    fn query(&self) -> Vec<(String, String)> {
        let q = self.target.split_once('?').map_or("", |(_, q)| q);
        q.split('&').filter(|p| !p.is_empty()).map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (k.to_string(), v.to_string())
        }).collect()
    }
    fn arg(&self, key: &str) -> String {
        self.query().into_iter().find(|(k, _)| k == key).map(|(_, v)| v).unwrap_or_default()
    }
    /// `POST term/search`, `POST classSearch/resetDataForm`, `GET searchResults/searchResults`.
    fn label(&self) -> String {
        format!("{} {}", self.method, self.path().strip_prefix(PREFIX).unwrap_or(self.path()))
    }
}

struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Resp {
    fn ok(body: impl Into<String>) -> Resp {
        Resp { status: 200, headers: vec![("content-type".into(), "application/json".into())], body: body.into() }
    }
    fn status(code: u16) -> Resp {
        Resp { status: code, headers: vec![], body: "refused".into() }
    }
}

struct Fake {
    base: String,
    seen: Arc<Mutex<Vec<Req>>>,
    stop: Arc<AtomicBool>,
    addr: std::net::SocketAddr,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn read_request(s: &mut TcpStream) -> Option<(String, String, Vec<(String, String)>, String)> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if s.read(&mut byte).ok()? == 0 {
            return None;
        }
        buf.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, target) = (first.next()?.to_string(), first.next()?.to_string());
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    let len: usize = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; len];
    s.read_exact(&mut body).ok()?;
    Some((method, target, headers, String::from_utf8_lossy(&body).to_string()))
}

impl Fake {
    /// `cost` is the fake time each request takes; `handler` gets the request and its index.
    fn start(clock: Clock, cost: Duration, handler: impl Fn(&Req, usize) -> Resp + Send + 'static) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (seen, stop) = (Arc::new(Mutex::new(Vec::new())), Arc::new(AtomicBool::new(false)));
        let (seen2, stop2) = (seen.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if stop2.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut s) = conn else { continue };
                let Some((method, target, headers, body)) = read_request(&mut s) else { continue };
                let at = *clock.lock().unwrap();
                let req = Req { method, target, headers, body, at };
                let index = seen2.lock().unwrap().len();
                seen2.lock().unwrap().push(req.clone());
                *clock.lock().unwrap() += cost;
                let r = handler(&req, index);
                let mut out = format!("HTTP/1.1 {} X\r\ncontent-length: {}\r\nconnection: close\r\n", r.status, r.body.len());
                for (k, v) in &r.headers {
                    out.push_str(&format!("{k}: {v}\r\n"));
                }
                out.push_str("\r\n");
                let _ = s.write_all(out.as_bytes());
                let _ = s.write_all(r.body.as_bytes());
            }
        });
        Fake { base: format!("http://{addr}"), seen, stop, addr, handle: Some(handle) }
    }
    fn requests(&self) -> Vec<Req> {
        self.seen.lock().unwrap().clone()
    }
    fn labels(&self) -> Vec<String> {
        self.requests().iter().map(Req::label).collect()
    }
}

/// Rows `total` long for the asked course, sliced by the asked page.
fn answer(req: &Req, total: usize) -> String {
    let (term, subject, number) = (req.arg("txt_term"), req.arg("txt_subject"), req.arg("txt_courseNumber"));
    let offset: usize = req.arg("pageOffset").parse().unwrap_or(0);
    let size: usize = req.arg("pageMaxSize").parse().unwrap_or(total);
    let rows: Vec<Value> = (offset..total.min(offset + size)).map(|i| json!({
        "term": term, "subject": subject, "courseNumber": number,
        "sequenceNumber": format!("{:03}", i + 1), "courseReferenceNumber": format!("{}", 10000 + i),
        "meetingsFaculty": [],
    })).collect();
    json!({ "success": true, "totalCount": total, "data": rows }).to_string()
}

/// A school that behaves: the term POST issues one cookie, and a search without it answers 200 with
/// no rows (R0b), as the real one does. `total_for` is the row count of a course.
fn banner(total_for: impl Fn(&str, &str) -> usize + Send + 'static) -> impl Fn(&Req, usize) -> Resp + Send + 'static {
    move |req, _| match req.label().as_str() {
        "POST term/search" => {
            let mut r = Resp::ok(json!({ "fwdURL": "/StudentRegistrationSsb/ssb/classSearch/classSearch" }).to_string());
            r.headers.push(("set-cookie".into(), format!("{COOKIE}; Path=/StudentRegistrationSsb; HttpOnly")));
            r
        }
        "POST classSearch/resetDataForm" => Resp::ok("{}"),
        "GET searchResults/searchResults" => {
            if req.header("cookie") != Some(COOKIE) {
                return Resp::ok(json!({ "success": true, "totalCount": 0, "data": [] }).to_string());
            }
            Resp::ok(answer(req, total_for(&req.arg("txt_subject"), &req.arg("txt_courseNumber"))))
        }
        _ => Resp::status(404),
    }
}

fn two_each() -> impl Fn(&str, &str) -> usize + Send + 'static {
    |_, _| 2
}

/// A run against `fake`; the pace's clock is the fake's.
fn run(fake: &Fake, clock: &Clock, wanted: &[SectionRef]) -> Bundle {
    fetch(&fake.base, &cs(), TERM, wanted, &mut FakePace(clock.clone()))
}

fn fresh() -> Clock {
    Arc::new(Mutex::new(Duration::ZERO))
}

fn statuses(b: &Bundle) -> Vec<Status> {
    b.queries.iter().map(|q| q.status).collect()
}

fn courses(n: usize) -> Vec<SectionRef> {
    (0..n).map(|i| want(&format!("C{i:02}"), "100", "001")).collect()
}

// ---- order and shape ------------------------------------------------------------------------

#[test]
fn one_term_post_then_a_reset_and_a_search_per_course_in_order() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    let b = run(&fake, &clock, &[want("XYZ", "100", "001"), want("ABC", "210", "002")]);
    assert_eq!(fake.labels(), [
        "POST term/search", "POST classSearch/resetDataForm", "GET searchResults/searchResults",
        "POST classSearch/resetDataForm", "GET searchResults/searchResults",
    ]);
    assert_eq!(statuses(&b), [Status::Answered, Status::Answered]);
    let term_post = &fake.requests()[0];
    assert_eq!(term_post.target, format!("{PREFIX}term/search?mode=search"));
    assert!(term_post.body.contains(&format!("term={TERM}")), "{}", term_post.body);
}

#[test]
fn two_sections_of_one_course_share_one_query() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    let b = run(&fake, &clock, &[want("XYZ", "100", "001"), want("ABC", "210", "001"), want("XYZ", "100", "002")]);
    assert_eq!(fake.labels().iter().filter(|l| l.starts_with("GET")).count(), 2, "{:?}", fake.labels());
    let got: Vec<(&str, &str, &[String])> = b.queries.iter().map(|q| (q.subject.as_str(), q.number.as_str(), q.sections.as_slice())).collect();
    assert_eq!(got, [("XYZ", "100", &["001".to_string(), "002".to_string()][..]), ("ABC", "210", &["001".to_string()][..])]);
}

#[test]
fn no_wanted_section_makes_no_request_at_all() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    let b = run(&fake, &clock, &[]);
    assert!(fake.requests().is_empty() && b.queries.is_empty());
}

#[test]
fn every_search_names_the_term_a_subject_a_number_and_a_page_of_fifty() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    run(&fake, &clock, &[want("XYZ", "100", "001"), want("ABC", "210L", "001")]);
    let searches: Vec<Req> = fake.requests().into_iter().filter(|r| r.method == "GET").collect();
    assert_eq!(searches.len(), 2);
    for (s, (subject, number)) in searches.iter().zip([("XYZ", "100"), ("ABC", "210L")]) {
        assert_eq!((s.arg("txt_term"), s.arg("txt_subject"), s.arg("txt_courseNumber")), (TERM.into(), subject.into(), number.into()));
        assert_eq!((s.arg("pageOffset").as_str(), s.arg("pageMaxSize").as_str()), ("0", "50"));
    }
    assert_eq!(class_search::PAGE_SIZE, 50);
}

#[test]
fn the_cookie_comes_from_the_term_answer_and_only_that_one_is_sent() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    let b = run(&fake, &clock, &[want("XYZ", "100", "001"), want("ABC", "210", "001")]);
    let reqs = fake.requests();
    assert_eq!(reqs[0].header("cookie"), None, "the term POST goes out with no cookie");
    for r in &reqs[1..] {
        assert_eq!(r.header("cookie"), Some(COOKIE), "{}", r.label());
    }
    for r in &reqs {
        for h in ["authorization", "proxy-authorization"] {
            assert_eq!(r.header(h), None, "{} sent {h}", r.label());
        }
    }
    // Rows came back, so the cookie was accepted: no cookie would have been an empty 200.
    assert_eq!(statuses(&b), [Status::Answered, Status::Answered]);
}

#[test]
fn the_user_agent_names_knowlu_its_version_and_the_site() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    run(&fake, &clock, &[want("XYZ", "100", "001")]);
    let reqs = fake.requests();
    assert!(!reqs.is_empty());
    for r in &reqs {
        let ua = r.header("user-agent").unwrap_or("");
        assert!(ua.contains("Knowlu") && ua.contains(env!("CARGO_PKG_VERSION")) && ua.contains("knowlu.com"), "{ua:?}");
    }
    assert_eq!(class_search::user_agent(), reqs[0].header("user-agent").unwrap());
}

#[test]
fn the_base_a_real_run_uses_is_https_on_the_curated_host() {
    assert_eq!(class_search::https_base(&cs()), "https://registrar.invented.test");
}

// ---- politeness -----------------------------------------------------------------------------

#[test]
fn at_least_one_second_passes_between_any_two_requests_on_the_injected_clock() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(|_, _| 120));
    run(&fake, &clock, &[want("XYZ", "100", "001"), want("ABC", "210", "001")]);
    let at: Vec<Duration> = fake.requests().iter().map(|r| r.at).collect();
    assert!(at.len() >= 7, "a term POST, two resets, and three pages for the first course: {at:?}");
    for pair in at.windows(2) {
        assert!(pair[1] - pair[0] >= Duration::from_secs(1), "{pair:?} of {at:?}");
    }
    assert_eq!(class_search::GAP, Duration::from_secs(1));
}

#[test]
fn the_caps_are_twenty_queries_and_sixty_seconds() {
    assert_eq!((class_search::MAX_QUERIES, class_search::MAX_SECONDS, class_search::MAX_PAGES), (20, 60, 3));
}

#[test]
fn past_twenty_distinct_queries_the_rest_are_paused_and_never_asked() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    let b = run(&fake, &clock, &courses(22));
    let mut want_states = vec![Status::Answered; 20];
    want_states.extend([Status::Paused, Status::Paused]);
    assert_eq!(statuses(&b), want_states);
    assert_eq!(fake.requests().len(), 1 + 20 * 2, "the term POST, then a reset and a search for twenty courses");
    assert!(b.queries[20..].iter().all(|q| q.body.is_empty()));
}

#[test]
fn a_run_past_sixty_seconds_pauses_the_courses_it_has_not_started() {
    let clock = fresh();
    // Each request takes 20 s on the fake clock: the term POST, then a whole query, is past 60 s.
    let fake = Fake::start(clock.clone(), Duration::from_secs(20), banner(two_each()));
    let b = run(&fake, &clock, &courses(3));
    assert_eq!(statuses(&b), [Status::Answered, Status::Paused, Status::Paused]);
    assert_eq!(fake.requests().len(), 3, "{:?}", fake.labels());
}

// ---- refusals, closed, failures -------------------------------------------------------------

/// `banner`, but the handler `bad` takes over for the search of the second course.
fn bad_second_search(bad: impl Fn() -> Resp + Send + 'static) -> impl Fn(&Req, usize) -> Resp + Send + 'static {
    let good = banner(two_each());
    move |req, i| {
        if req.label() == "GET searchResults/searchResults" && req.arg("txt_subject") == "C01" { bad() } else { good(req, i) }
    }
}

#[test]
fn a_429_a_403_or_a_5xx_stops_the_run_and_nothing_is_retried() {
    for code in [429u16, 403, 500, 503] {
        let clock = fresh();
        let fake = Fake::start(clock.clone(), Duration::ZERO, bad_second_search(move || Resp::status(code)));
        let b = run(&fake, &clock, &courses(4));
        assert_eq!(statuses(&b), [Status::Answered, Status::Paused, Status::Paused, Status::Paused], "{code}");
        assert_eq!(fake.labels().len(), 1 + 2 + 2, "{code}: no retry and no later course: {:?}", fake.labels());
        assert!(b.queries[1..].iter().all(|q| q.body.is_empty()), "{code}");
    }
}

#[test]
fn a_refusal_of_the_term_post_pauses_every_course_with_no_other_request() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, |_, _| Resp::status(429));
    let b = run(&fake, &clock, &courses(3));
    assert_eq!(statuses(&b), [Status::Paused; 3]);
    assert_eq!(fake.labels(), ["POST term/search"]);
}

#[test]
fn an_answer_that_is_not_json_is_closed_and_ends_the_run() {
    let clock = fresh();
    let html = || Resp { status: 200, headers: vec![("content-type".into(), "text/html".into())], body: "<html><title>Sign in</title></html>".into() };
    let fake = Fake::start(clock.clone(), Duration::ZERO, bad_second_search(html));
    let b = run(&fake, &clock, &courses(3));
    assert_eq!(statuses(&b), [Status::Answered, Status::Closed, Status::Closed]);
    assert_eq!(fake.labels().len(), 1 + 2 + 2, "{:?}", fake.labels());
}

#[test]
fn a_redirect_to_a_sign_in_is_closed() {
    let clock = fresh();
    let good = banner(two_each());
    let fake = Fake::start(clock.clone(), Duration::ZERO, move |req, i| {
        if req.label() == "GET searchResults/searchResults" {
            return Resp { status: 302, headers: vec![("location".into(), "/idp/login".into())], body: String::new() };
        }
        if req.path() == "/idp/login" {
            return Resp::ok("<html>Sign in</html>");
        }
        good(req, i)
    });
    let b = run(&fake, &clock, &courses(2));
    assert_eq!(statuses(&b), [Status::Closed, Status::Closed]);
}

#[test]
fn no_rows_after_a_good_term_post_is_closed_never_not_found() {
    let clock = fresh();
    let good = banner(two_each());
    let fake = Fake::start(clock.clone(), Duration::ZERO, move |req, i| {
        if req.label() == "GET searchResults/searchResults" {
            return Resp::ok(json!({ "success": true, "totalCount": 0, "data": [] }).to_string());
        }
        good(req, i)
    });
    let b = run(&fake, &clock, &courses(3));
    assert_eq!(statuses(&b), [Status::Closed; 3]);
    assert_eq!(fake.labels().len(), 3, "the run ends at the first empty answer: {:?}", fake.labels());
}

#[test]
fn an_unreachable_host_is_fetch_failed_for_every_course() {
    let dead = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", l.local_addr().unwrap())
    };
    let b = fetch(&dead, &cs(), TERM, &courses(2), &mut FakePace(fresh()));
    assert_eq!(statuses(&b), [Status::FetchFailed; 2]);
}

// ---- paging ---------------------------------------------------------------------------------

fn pages_of(total: usize) -> (Vec<String>, Bundle) {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(move |_, _| total));
    let b = run(&fake, &clock, &[want("XYZ", "100", "001")]);
    let offsets = fake.requests().iter().filter(|r| r.method == "GET").map(|r| r.arg("pageOffset")).collect();
    (offsets, b)
}

#[test]
fn a_course_is_paged_by_total_count_and_the_pages_read_as_one_answer() {
    assert_eq!(pages_of(50).0, ["0"]);
    assert_eq!(pages_of(51).0, ["0", "50"]);
    let (offsets, b) = pages_of(120);
    assert_eq!(offsets, ["0", "50", "100"]);
    assert_eq!(b.queries[0].status, Status::Answered);
    let merged: Value = serde_json::from_str(&b.queries[0].body).unwrap();
    assert_eq!(merged["totalCount"], 120);
    let rows = merged["data"].as_array().unwrap();
    assert_eq!(rows.len(), 120);
    assert_eq!((rows[0]["sequenceNumber"].as_str(), rows[119]["sequenceNumber"].as_str()), (Some("001"), Some("120")));
}

#[test]
fn paging_stops_at_three_pages_and_a_course_with_unread_rows_is_fetch_failed() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(|subject, _| if subject == "C00" { 400 } else { 2 }));
    // C00's wanted section, 300, sits past the 150 rows the three pages read.
    let b = run(&fake, &clock, &[want("C00", "100", "300"), want("C01", "100", "001")]);
    assert_eq!(statuses(&b), [Status::FetchFailed, Status::Answered]);
    let offsets: Vec<String> = fake.requests().iter().filter(|r| r.method == "GET").map(|r| r.arg("pageOffset")).collect();
    assert_eq!(offsets, ["0", "50", "100", "0"], "three pages for the long course, one for the next");
    let partial: Value = serde_json::from_str(&b.queries[0].body).unwrap();
    assert_eq!(partial["data"].as_array().unwrap().len(), 150, "the partial read is kept");
}

#[test]
fn a_long_course_whose_wanted_sections_were_all_on_the_pages_read_is_answered() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(|_, _| 400));
    // 001 is on the first page and 150 on the third: every wanted section was read, though 250 rows were not.
    let b = run(&fake, &clock, &[want("XYZ", "100", "001"), want("XYZ", "100", "150")]);
    assert_eq!(statuses(&b), [Status::Answered]);
    let offsets: Vec<String> = fake.requests().iter().filter(|r| r.method == "GET").map(|r| r.arg("pageOffset")).collect();
    assert_eq!(offsets, ["0", "50", "100"]);
    let merged: Value = serde_json::from_str(&b.queries[0].body).unwrap();
    assert_eq!(merged["data"].as_array().unwrap().len(), 150);
}

// ---- the query type, the bundle, the source's promises ---------------------------------------

#[test]
fn a_query_cannot_be_built_without_a_subject_and_a_number() {
    assert!(Query::new("", "100").is_none() && Query::new("XYZ", "").is_none() && Query::new("", "").is_none());
    let q = Query::new("XYZ", "100").unwrap();
    assert_eq!((q.subject(), q.number()), ("XYZ", "100"));
    let got = queries(&[want("", "100", "001"), want("XYZ", "", "001"), want("XYZ", "100", "001")]);
    assert_eq!(got.len(), 1, "a section with no subject or number makes no query: {got:?}");
    // The type itself: private fields and no `Default`, so no path builds an empty one (§6.8).
    let src = include_str!("../src/class_search.rs");
    let body = src.split("pub struct Query {").nth(1).and_then(|s| s.split('}').next()).expect("pub struct Query");
    assert!(!body.contains("pub "), "Query's fields stay private: {body}");
    assert!(!src.contains("Default for Query") && !src.contains("derive(Default"), "no Default on Query");
}

#[test]
fn the_bundle_is_school_term_and_queries_of_five_fixed_keys_and_no_name() {
    let clock = fresh();
    let fake = Fake::start(clock.clone(), Duration::ZERO, banner(two_each()));
    let b = run(&fake, &clock, &[want("XYZ", "100", "001"), want("XYZ", "100", "002")]);
    let j = b.to_json();
    let keys = |v: &Value| -> Vec<String> { let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect(); k.sort(); k };
    assert_eq!(keys(&j), ["queries", "school", "term"]);
    assert_eq!((j["school"].as_str(), j["term"].as_str()), (Some("ua"), Some(TERM)));
    let q = &j["queries"][0];
    assert_eq!(keys(q), ["body", "number", "sections", "status", "subject"]);
    assert_eq!((q["subject"].as_str(), q["number"].as_str(), q["status"].as_str()), (Some("XYZ"), Some("100"), Some("answered")));
    assert_eq!(q["sections"], json!(["001", "002"]));
    let body = q["body"].as_str().expect("body is the raw answer as text");
    let answered: Value = serde_json::from_str(body).unwrap();
    assert_eq!(answered["totalCount"], 2);
}

#[test]
fn the_status_words_are_fixed() {
    let one = |status| Bundle {
        school: "ua".into(), term: TERM.into(),
        queries: vec![QueryResult { subject: "XYZ".into(), number: "100".into(), sections: vec!["001".into()], status, body: String::new() }],
    };
    let words: Vec<String> = [Status::Answered, Status::Closed, Status::Paused, Status::FetchFailed]
        .into_iter().map(|s| one(s).to_json()["queries"][0]["status"].as_str().unwrap().to_string()).collect();
    assert_eq!(words, ["answered", "closed", "paused", "fetch_failed"]);
}

#[test]
fn the_reader_never_touches_a_window_a_profile_a_credential_or_a_cookie_file() {
    let src = include_str!("../src/class_search.rs");
    // Code only: the doc comments may name what the module does not do.
    let code: String = src.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    for banned in [
        "tauri", "WebviewWindow", "cookies_for_url", "lms_link", "profiles", "credentials", "wincred",
        "CookieStore", "cookie_store", "CookieJar", "std::fs", "File::", "write(", "ureq::config", "features",
    ] {
        assert!(!code.contains(banned), "class_search.rs must not mention `{banned}`");
    }
}

// ---- §6.10: the row's structure ---------------------------------------------------------------

/// What a curated row's class search must satisfy offline (§6.10). No test reaches the school.
fn structure_errors(cs: &ClassSearch, lms_sign_in: Option<&str>) -> Vec<String> {
    let mut e = Vec::new();
    if !SCHOOLS.iter().any(|s| s.key == cs.school) {
        e.push(format!("school `{}` is not a SCHOOLS key", cs.school));
    }
    if cs.checked.len() != 10 || jiff::civil::Date::strptime("%Y-%m-%d", cs.checked).is_err() {
        e.push(format!("checked `{}` is not a YYYY-MM-DD date", cs.checked));
    }
    if cs.host.is_empty() || cs.host.contains("://") || cs.host.contains('/') {
        e.push(format!("host `{}` carries a scheme or a path", cs.host));
    }
    if !(cs.prefix.starts_with('/') && cs.prefix.ends_with('/')) {
        e.push(format!("prefix `{}` must start and end with `/`", cs.prefix));
    }
    if lms_sign_in.is_some_and(|p| !p.starts_with('/')) {
        e.push(format!("lms_sign_in `{}` must start with `/`", lms_sign_in.unwrap()));
    }
    e
}

#[test]
fn every_curated_row_that_sets_class_search_is_structurally_sound() {
    for row in CAMPUSES.iter() {
        if let Some(c) = &row.class_search {
            assert_eq!(structure_errors(c, row.lms_sign_in), Vec::<String>::new(), "{}", row.label);
        } else if let Some(p) = row.lms_sign_in {
            assert!(p.starts_with('/'), "{}: lms_sign_in `{p}`", row.label);
        }
    }
}

#[test]
fn the_row_checks_catch_each_fault_and_pass_a_sound_row() {
    assert!(structure_errors(&cs(), Some("/auth/login")).is_empty());
    let fault = |c: ClassSearch| structure_errors(&c, None).len();
    assert_eq!(fault(ClassSearch { school: "nowhere", ..cs() }), 1);
    assert_eq!(fault(ClassSearch { checked: "2026-13-40", ..cs() }), 1);
    assert_eq!(fault(ClassSearch { checked: "11/08/2026", ..cs() }), 1);
    assert_eq!(fault(ClassSearch { host: "https://registrar.invented.test", ..cs() }), 1);
    assert_eq!(fault(ClassSearch { host: "registrar.invented.test/ssb", ..cs() }), 1);
    assert_eq!(fault(ClassSearch { prefix: "StudentRegistrationSsb/ssb/", ..cs() }), 1);
    assert_eq!(fault(ClassSearch { prefix: "/StudentRegistrationSsb/ssb", ..cs() }), 1);
    assert_eq!(structure_errors(&cs(), Some("auth/login")).len(), 1);
}
