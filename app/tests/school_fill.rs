//! P1t, the fill's decisions and the window script's pins (onboarding replan §7.5 and §8 P1t; makes P2a
//! and P2b pass). The decisions run over a fake page (`FakePage`) and the recording file seam in
//! `support/school_fakes.rs`; nothing opens a window, reads a credential or leaves the machine.
//! Invented usernames, passwords and hosts only (`sso.example.test`).
#![cfg(windows)]
use knowlu::school_fill::{
    run_fill, script, script_failure, Button, Field, FillOutcome, FillPage, PageShape, Rejection, FIELD_WAIT,
    SUBMIT_SETTLE,
};
use knowlu::school_password::SchoolSecret;
use std::time::Duration;

#[path = "support/school_fakes.rs"]
mod school_fakes;
use school_fakes::{at, count, Fake, Log};

const HOSTS: &[&str] = &["sso.example.test"];
const WRONG: &str = "That password is not correct";
const USER: &str = "student-one";
const PW: &str = "hunter2-synthetic-pw";
const LOGIN: &str = "https://sso.example.test/login";
const NEXT: &str = "https://sso.example.test/login/password";

// ---- the fake page ---------------------------------------------------------------------------------------

/// One page of the attempt. `me_ok`: `users/me` answers 200 once the attempt is on it.
#[derive(Clone)]
struct Step {
    shape: Option<PageShape>,
    me_ok: bool,
    /// (Default true.) The page's own script takes over the submit (from the form or from `document`) and sends by script.
    script_sends: bool,
    /// Replaces the shape when the field wait runs (a target that changes before the submit).
    after_wait: Option<PageShape>,
    /// Replaces the visible text when the settle wait runs.
    text_after_settle: Option<String>,
}

fn step(shape: PageShape) -> Step {
    Step { shape: Some(shape), me_ok: false, script_sends: true, after_wait: None, text_after_settle: None }
}

fn signed_in() -> Step {
    Step { me_ok: true, ..step(PageShape { url: "https://lms.example.test/ultra/".into(), ..Default::default() }) }
}

struct FakePage {
    log: Log,
    steps: Vec<Step>,
    at: usize,
    armed: bool,
    typed: Vec<(Field, String)>,
    /// Every URL the page was asked to load, with any query a native GET would carry.
    urls: Vec<String>,
}

impl FakePage {
    fn new(log: &Log, steps: Vec<Step>) -> FakePage {
        FakePage { log: log.clone(), steps, at: 0, armed: false, typed: Vec::new(), urls: Vec::new() }
    }
    fn cur(&mut self) -> &mut Step {
        let i = self.at.min(self.steps.len() - 1);
        &mut self.steps[i]
    }
}

impl FillPage for FakePage {
    fn shape(&mut self) -> Option<PageShape> {
        self.log.borrow_mut().push("shape".into());
        self.cur().shape.clone()
    }
    fn set_field(&mut self, field: Field, value: &str) -> bool {
        self.log.borrow_mut().push(format!("set:{field:?}"));
        self.typed.push((field, value.to_string()));
        true
    }
    fn wait(&mut self, d: Duration) {
        self.log.borrow_mut().push(format!("wait:{}", d.as_millis()));
        if let Some(s) = self.cur().after_wait.take() {
            self.cur().shape = Some(s);
        }
    }
    fn arm_backstop(&mut self) -> bool {
        self.log.borrow_mut().push("arm".into());
        self.armed = true;
        true
    }
    fn press(&mut self, button: usize) -> bool {
        self.log.borrow_mut().push(format!("press:{button}"));
        let (shape, sends) = (self.cur().shape.clone().expect("a pressed page has a shape"), self.cur().script_sends);
        let method = shape.buttons[button].formmethod.clone().or(shape.form_method.clone());
        let native_get = method.as_deref().map_or(true, |m| m.eq_ignore_ascii_case("get"));
        if sends || !native_get {
            self.at += 1; // the page's handler sent it, or a POST navigated
        } else if !self.armed {
            // The browser's own GET of the form: every field lands in the URL.
            let q: Vec<String> = self.typed.iter().map(|(f, v)| format!("{f:?}={v}")).collect();
            self.urls.push(format!("{}?{}", shape.url, q.join("&")));
        }
        true
    }
    fn await_signed_in(&mut self, within: Duration) -> bool {
        assert_eq!(within, SUBMIT_SETTLE, "the wait after a submit is SUBMIT_SETTLE");
        self.log.borrow_mut().push("settle".into());
        if let Some(text) = self.cur().text_after_settle.take() {
            if let Some(s) = self.cur().shape.as_mut() {
                s.visible_text = text;
            }
        }
        self.cur().me_ok
    }
}

// ---- builders ---------------------------------------------------------------------------------------------

/// The founder's page shape: one form, no `method`, an empty `action`, a show-password control placed
/// first, then the form's own submit. A script submit.
fn script_page(url: &str, fields: &[Field]) -> PageShape {
    PageShape {
        url: url.into(),
        fields: fields.to_vec(),
        form_action: Some(String::new()),
        form_method: None,
        buttons: vec![Button::default(), Button { submit: true, ..Default::default() }],
        ..Default::default()
    }
}

fn nav_page(url: &str, fields: &[Field], action: &str, method: &str) -> PageShape {
    PageShape { form_action: Some(action.into()), form_method: Some(method.into()), ..script_page(url, fields) }
}

fn with_text(mut s: PageShape, text: &str) -> PageShape {
    s.visible_text = text.into();
    s
}

fn secret() -> SchoolSecret {
    SchoolSecret::new(USER, PW)
}

struct Run {
    out: FillOutcome,
    fake: Fake,
    page: FakePage,
}

/// One attempt over `steps`, with the marks and the page logging into one shared log.
fn go(steps: Vec<Step>, wrong: &str) -> Run {
    let fake = Fake::new("profile_fill");
    let mut page = FakePage::new(&fake.log, steps);
    let out = run_fill(&mut page, &fake, HOSTS, wrong, &secret());
    Run { out, fake, page }
}

/// A one-page attempt that then signs in: used where only the decision to fill matters.
fn one(shape: PageShape) -> Run {
    go(vec![step(shape), signed_in()], WRONG)
}

fn filled_nothing(r: &Run) {
    assert_eq!(count(&r.fake.log, "set:"), 0, "{:?}", r.fake.log.borrow());
    assert_eq!(count(&r.fake.log, "press:"), 0, "{:?}", r.fake.log.borrow());
    assert_eq!(count(&r.fake.log, "write_in_flight"), 0, "nothing was sent, so no mark");
}

fn filled(r: &Run) {
    assert_eq!(r.out, FillOutcome::SignedIn, "{:?}", r.fake.log.borrow());
    assert_eq!(count(&r.fake.log, "press:"), 1);
}

use Field::{Password, Username};

// ---- where: host, scheme, redirect --------------------------------------------------------------------------

#[test]
fn a_listed_https_page_fills_and_signs_in() {
    let r = one(script_page(LOGIN, &[Password]));
    filled(&r);
    assert_eq!(r.page.typed, vec![(Password, PW.to_string())]);
}

#[test]
fn a_host_outside_sign_in_hosts_fills_nothing() {
    for url in [
        "https://evil.example.test/login",
        "https://sso.example.test.evil.test/login",
        "https://evil.test/sso.example.test",
        "https://lms.example.test/login",
    ] {
        let r = one(script_page(url, &[Password]));
        assert_eq!(r.out, FillOutcome::CannotRead, "{url}");
        filled_nothing(&r);
    }
}

#[test]
fn an_http_page_fills_nothing() {
    let r = one(script_page("http://sso.example.test/login", &[Password]));
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
}

#[test]
fn a_redirect_off_the_list_mid_attempt_fills_no_password() {
    let r = go(
        vec![step(script_page(LOGIN, &[Username])), step(script_page("https://evil.example.test/p", &[Password])), signed_in()],
        WRONG,
    );
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed), "the mark was already set");
    assert_eq!(r.page.typed, vec![(Username, USER.to_string())], "no password filled on the foreign host");
    assert_eq!(count(&r.fake.log, "press:"), 1);
}

#[test]
fn an_unknown_page_before_the_first_submit_is_cannot_read_with_nothing_sent() {
    for shape in [None, Some(with_text(script_page(LOGIN, &[]), "Welcome back")), Some(PageShape { buttons: vec![], ..script_page(LOGIN, &[Password]) })] {
        let r = go(vec![Step { shape, ..step(PageShape::default()) }], WRONG);
        assert_eq!(r.out, FillOutcome::CannotRead);
        filled_nothing(&r);
    }
}

// ---- where the form sends it ----------------------------------------------------------------------------------

#[test]
fn a_navigation_submit_to_a_listed_https_host_by_post_fills() {
    filled(&one(nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "post")));
    filled(&one(nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "POST")));
}

#[test]
fn the_controls_formaction_wins_over_the_forms_action() {
    let mut good = nav_page(LOGIN, &[Password], "https://evil.example.test/x", "post");
    good.buttons[1].formaction = Some("https://sso.example.test/submit".into());
    filled(&one(good));
    let mut bad = nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "post");
    bad.buttons[1].formaction = Some("https://evil.example.test/x".into());
    let r = one(bad);
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
}

#[test]
fn the_controls_formmethod_wins_over_the_forms_method() {
    let mut good = nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "get");
    good.buttons[1].formmethod = Some("post".into());
    filled(&one(good));
    let mut bad = nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "post");
    bad.buttons[1].formmethod = Some("get".into());
    let r = one(bad);
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
}

#[test]
fn an_off_list_http_or_get_target_from_either_source_fills_nothing() {
    let cases = [
        nav_page(LOGIN, &[Password], "https://evil.example.test/x", "post"),
        nav_page(LOGIN, &[Password], "http://sso.example.test/submit", "post"),
        nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "get"),
        nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "GET"),
    ];
    for shape in cases {
        let r = one(shape);
        assert_eq!(r.out, FillOutcome::CannotRead);
        filled_nothing(&r);
    }
    let mut formaction_http = nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "post");
    formaction_http.buttons[1].formaction = Some("http://sso.example.test/submit".into());
    filled_nothing(&one(formaction_http));
}

#[test]
fn a_form_with_a_non_empty_action_and_no_method_is_a_get_and_fills_nothing() {
    let mut s = script_page(LOGIN, &[Password]);
    s.form_action = Some("https://sso.example.test/submit".into());
    let r = one(s);
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
}

#[test]
fn a_script_submit_shaped_like_the_founders_page_fills_on_a_listed_host_only() {
    let ok = go(vec![Step { script_sends: true, ..step(script_page(LOGIN, &[Password])) }, signed_in()], WRONG);
    filled(&ok);
    assert!(at(&ok.fake.log, "arm").is_some(), "a script submit is backstopped");
    let r = one(script_page("https://evil.example.test/login", &[Password]));
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
}

#[test]
fn a_control_that_adds_formmethod_get_makes_a_script_submit_form_a_navigation_and_fills_nothing() {
    let mut s = script_page(LOGIN, &[Password]);
    s.buttons[1].formmethod = Some("get".into());
    let r = one(s);
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
}

#[test]
fn a_script_submit_the_page_does_not_intercept_leaks_no_field_into_any_url() {
    // The capture-phase listener prevents the default: no navigation, no field in a URL. With no
    // users/me 200 within SUBMIT_SETTLE the attempt is Rejected(Unconfirmed), the mark set.
    let r = go(vec![Step { script_sends: false, ..step(script_page(LOGIN, &[Password])) }], WRONG);
    assert!(r.page.urls.is_empty(), "a field reached a URL: {:?}", r.page.urls);
    assert!(at(&r.fake.log, "arm").unwrap() < at(&r.fake.log, "press:").unwrap(), "armed before the press");
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed));
    assert!(r.fake.in_flight.get());
}

#[test]
fn a_page_that_intercepts_from_a_handler_on_document_still_sends_and_signs_in() {
    let r = go(vec![Step { script_sends: true, ..step(script_page(LOGIN, &[Password])) }, signed_in()], WRONG);
    filled(&r);
    assert!(r.page.urls.is_empty(), "{:?}", r.page.urls);
}

#[test]
fn a_navigation_submit_is_not_backstopped() {
    let r = one(nav_page(LOGIN, &[Password], "https://sso.example.test/submit", "post"));
    filled(&r);
    assert_eq!(count(&r.fake.log, "arm"), 0, "the listener is for a script submit");
}

#[test]
fn a_navigation_submit_with_an_empty_action_resolves_to_the_pages_own_url() {
    let ok = one(nav_page(LOGIN, &[Password], "", "post"));
    filled(&ok);
    let r = one(nav_page("https://evil.example.test/login", &[Password], "", "post"));
    assert_eq!(r.out, FillOutcome::CannotRead);
    filled_nothing(&r);
    let http = one(nav_page("http://sso.example.test/login", &[Password], "", "post"));
    filled_nothing(&http);
    let get = one(nav_page(LOGIN, &[Password], "", "get"));
    filled_nothing(&get);
}

#[test]
fn the_password_steps_new_form_off_list_after_an_on_list_username_step_fills_no_password() {
    let mut pw_page = script_page(NEXT, &[Password]);
    pw_page.form_action = Some("https://evil.example.test/p".into());
    pw_page.form_method = Some("post".into());
    let r = go(vec![step(script_page(LOGIN, &[Username])), step(pw_page), signed_in()], WRONG);
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed), "the mark is already set");
    assert_eq!(r.page.typed, vec![(Username, USER.to_string())]);
    assert_eq!(count(&r.fake.log, "press:"), 1);
}

#[test]
fn a_target_or_host_that_changes_between_the_last_field_and_the_submit_sends_nothing() {
    for moved in [
        script_page("https://evil.example.test/login", &[Password]),
        nav_page(LOGIN, &[Password], "https://evil.example.test/x", "post"),
        script_page("http://sso.example.test/login", &[Password]),
    ] {
        let r = go(vec![Step { after_wait: Some(moved), ..step(script_page(LOGIN, &[Password])) }, signed_in()], WRONG);
        assert_eq!(r.out, FillOutcome::CannotRead);
        assert_eq!(count(&r.fake.log, "press:"), 0, "{:?}", r.fake.log.borrow());
        assert_eq!(count(&r.fake.log, "write_in_flight"), 0, "nothing was sent, so no mark");
    }
}

#[test]
fn a_change_before_the_passwords_submit_sends_nothing_and_is_unconfirmed() {
    let moved = script_page("https://evil.example.test/p", &[Password]);
    let r = go(
        vec![step(script_page(LOGIN, &[Username])), Step { after_wait: Some(moved), ..step(script_page(NEXT, &[Password])) }, signed_in()],
        WRONG,
    );
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed));
    assert_eq!(count(&r.fake.log, "press:"), 1, "only the username step was pressed");
}

// ---- the steps ------------------------------------------------------------------------------------------------

#[test]
fn the_username_step_then_the_password_step() {
    let r = go(
        vec![
            Step { script_sends: true, ..step(script_page(LOGIN, &[Username])) },
            Step { script_sends: true, ..step(script_page(NEXT, &[Password])) },
            signed_in(),
        ],
        WRONG,
    );
    assert_eq!(r.out, FillOutcome::SignedIn, "{:?}", r.fake.log.borrow());
    assert_eq!(r.page.typed, vec![(Username, USER.to_string()), (Password, PW.to_string())]);
    assert_eq!(count(&r.fake.log, "press:"), 2);
}

#[test]
fn the_password_only_page_is_one_step() {
    let r = one(script_page(LOGIN, &[Password]));
    filled(&r);
    assert_eq!(r.page.typed, vec![(Password, PW.to_string())]);
}

#[test]
fn a_field_seen_twice_ends_the_attempt() {
    let r = go(
        vec![
            Step { script_sends: true, ..step(script_page(LOGIN, &[Username])) },
            step(script_page(LOGIN, &[Username])),
            signed_in(),
        ],
        WRONG,
    );
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed));
    assert_eq!(r.page.typed.len(), 1, "the username is filled once: {:?}", r.page.typed);
}

#[test]
fn the_submit_pressed_is_the_forms_own_never_a_show_password_control_placed_first() {
    let r = one(script_page(LOGIN, &[Password]));
    filled(&r);
    assert!(at(&r.fake.log, "press:1").is_some(), "button 0 is the show-password control: {:?}", r.fake.log.borrow());
    assert_eq!(count(&r.fake.log, "press:0"), 0);
}

#[test]
fn the_wait_comes_before_the_submit() {
    let r = one(script_page(LOGIN, &[Password]));
    filled(&r);
    let log = &r.fake.log;
    let w = at(log, &format!("wait:{}", FIELD_WAIT.as_millis())).expect("the field wait");
    assert!(at(log, "set:").unwrap() < w && w < at(log, "press:").unwrap(), "{:?}", log.borrow());
    assert_eq!(FIELD_WAIT, Duration::from_millis(1500));
}

// ---- the in-flight mark ---------------------------------------------------------------------------------------

#[test]
fn the_mark_is_written_then_flushed_before_the_username_steps_submit() {
    let r = go(
        vec![
            Step { script_sends: true, ..step(script_page(LOGIN, &[Username])) },
            Step { script_sends: true, ..step(script_page(NEXT, &[Password])) },
            signed_in(),
        ],
        WRONG,
    );
    let log = &r.fake.log;
    let (w, f, p) = (at(log, "write_in_flight").unwrap(), at(log, "flush_in_flight").unwrap(), at(log, "press:").unwrap());
    assert!(w < f && f < p, "write, flush, then the first press: {:?}", log.borrow());
    assert_eq!(count(log, "write_in_flight"), 1, "written once per attempt, not again at the password step");
}

#[test]
fn an_unknown_page_after_the_username_step_is_unconfirmed_and_no_password_is_filled() {
    let r = go(vec![Step { script_sends: true, ..step(script_page(LOGIN, &[Username])) }, step(with_text(script_page(NEXT, &[]), "Accept the new terms"))], WRONG);
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed));
    assert_eq!(r.page.typed, vec![(Username, USER.to_string())]);
    assert!(r.fake.in_flight.get());
}

#[test]
fn a_mark_that_cannot_be_written_or_flushed_submits_nothing_and_is_cannot_read() {
    for failing in ["write_in_flight", "flush_in_flight"] {
        let fake = Fake::new("profile_fill").failing(&[failing]);
        let mut page = FakePage::new(&fake.log, vec![step(script_page(LOGIN, &[Password])), signed_in()]);
        let out = run_fill(&mut page, &fake, HOSTS, WRONG, &secret());
        assert_eq!(out, FillOutcome::CannotRead, "{failing}");
        assert_eq!(count(&fake.log, "press:"), 0, "{failing}: nothing submitted");
    }
}

#[test]
fn only_users_me_200_in_the_attempt_clears_the_mark() {
    let ok = one(script_page(LOGIN, &[Password]));
    assert_eq!(ok.out, FillOutcome::SignedIn);
    assert!(!ok.fake.in_flight.get(), "cleared by the 200");
    assert!(at(&ok.fake.log, "clear_in_flight").unwrap() > at(&ok.fake.log, "settle").unwrap());
    let bad = go(vec![step(script_page(LOGIN, &[Password]))], WRONG);
    assert!(bad.fake.in_flight.get(), "any other ending leaves it set");
    assert_eq!(count(&bad.fake.log, "clear_in_flight"), 0);
}

#[test]
fn a_lost_page_a_timeout_a_5xx_and_a_dropped_connection_each_give_unconfirmed_with_the_mark_set() {
    // Lost: the page's script is gone after the submit.
    let lost = go(vec![step(script_page(LOGIN, &[Password])), Step { shape: None, ..step(PageShape::default()) }], WRONG);
    // Timeout, 5xx and a dropped connection all look the same from the page: no 200, nothing known.
    let quiet = go(vec![step(script_page(LOGIN, &[Password])), step(with_text(script_page("https://lms.example.test/err", &[]), "Service Unavailable"))], WRONG);
    for r in [lost, quiet] {
        assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed), "{:?}", r.fake.log.borrow());
        assert!(r.fake.in_flight.get());
    }
}

// ---- after the password's submit ----------------------------------------------------------------------------------

fn after_password(text: &str, wrong: &str) -> FillOutcome {
    let mut pw = script_page(LOGIN, &[Password]);
    pw.visible_text = String::new();
    let landing = with_text(script_page(NEXT, &[Password]), text);
    go(vec![Step { script_sends: true, ..step(pw) }, step(landing)], wrong).out
}

#[test]
fn the_rows_wrong_password_text_after_the_submit_is_rejected_wrong() {
    assert_eq!(after_password(&format!("Sign in. {WRONG}. Try again."), WRONG), FillOutcome::Rejected(Rejection::Wrong));
}

#[test]
fn every_other_ending_after_the_submit_is_unconfirmed() {
    for text in [
        "Password",                       // the password asked again with no message
        "Something went wrong",           // another error text
        "Your password has expired",      // a "password expired" page
        "Enter the code we texted you",   // an MFA page
        "Accept the new terms to go on",  // an unknown page
    ] {
        assert_eq!(after_password(text, WRONG), FillOutcome::Rejected(Rejection::Unconfirmed), "{text}");
    }
}

#[test]
fn with_the_rows_text_empty_a_wrong_password_page_is_unconfirmed() {
    assert_eq!(after_password(WRONG, ""), FillOutcome::Rejected(Rejection::Unconfirmed));
}

#[test]
fn only_visible_text_is_matched_never_a_hidden_element_a_script_a_template_or_an_attribute() {
    let mut mfa = with_text(script_page(NEXT, &[Password]), "Enter the code we texted you");
    mfa.hidden_text = format!("<div hidden>{WRONG}</div><script>var m='{WRONG}'</script><template>{WRONG}</template>");
    let r = go(vec![Step { script_sends: true, ..step(script_page(LOGIN, &[Password])) }, step(mfa)], WRONG);
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed));
}

#[test]
fn the_text_is_matched_after_the_settle_never_before() {
    let mut landing = with_text(script_page(NEXT, &[Password]), WRONG); // what the page shows before the settle
    landing.hidden_text.clear();
    let r = go(
        vec![
            Step { script_sends: true, ..step(script_page(LOGIN, &[Password])) },
            Step { text_after_settle: Some("Enter the code we texted you".into()), ..step(landing) },
        ],
        WRONG,
    );
    assert_eq!(r.out, FillOutcome::Rejected(Rejection::Unconfirmed), "{:?}", r.fake.log.borrow());
    let log = &r.fake.log;
    let last_settle = log.borrow().iter().rposition(|e| e == "settle").unwrap();
    assert!(log.borrow()[last_settle..].iter().any(|e| e == "shape"), "the page is read after the settle: {:?}", log.borrow());
}

// ---- the value's splice and the window script's pins --------------------------------------------------------------

const SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/school_fill.rs");

fn source() -> String {
    std::fs::read_to_string(SRC).expect("school_fill.rs")
}

/// The window script's fixed body: the raw string `BODY`, with quotes normalised to `"`.
fn body() -> String {
    let src = source();
    let from = src.find("BODY: &str = r#\"").expect("the fixed script body is a raw string named BODY") + "BODY: &str = r#\"".len();
    let end = src[from..].find("\"#;").expect("end of BODY");
    src[from..from + end].replace('\'', "\"")
}

fn code_lines(src: &str) -> Vec<&str> {
    src.lines().filter(|l| !l.trim_start().starts_with("//")).collect()
}

#[test]
fn the_script_is_a_fixed_body_then_one_serde_json_argument() {
    let s = script(&secret());
    let i = s.rfind("})(").expect("the splice `})(`");
    assert!(s.ends_with(')'));
    let arg = &s[i + 3..s.len() - 1];
    let v: serde_json::Value = serde_json::from_str(arg).expect("the argument is JSON");
    assert_eq!(serde_json::to_string(&v).unwrap(), arg, "its bytes are exactly serde_json's");
    assert!(arg.contains(&serde_json::to_string(PW).unwrap()), "the value is in it, once escaped");
    let body_part = &s[..i];
    assert!(!body_part.contains(PW) && !body_part.contains(USER), "the value is nowhere in the fixed body");
}

#[test]
fn the_value_is_spliced_by_serde_json_alone() {
    let src = source();
    let code = code_lines(&src);
    let uses = code.iter().filter(|l| l.contains("serde_json::to_string(")).count();
    assert_eq!(uses, 1, "serde_json::to_string builds the one argument, in one place");
    for l in &code {
        if l.contains("push_str(") || l.contains("format!(") {
            for v in ["password", "secret", "username"] {
                assert!(!l.to_lowercase().contains(v), "this line may append the value: {l}");
            }
        }
    }
}

#[test]
fn a_hostile_value_round_trips_through_the_argument() {
    let nasty = "a'b\"c\\d\ne\u{2028}f</script>g\u{1F600}h";
    let s = script(&SchoolSecret::new("u", nasty));
    let arg = &s[s.rfind("})(").unwrap() + 3..s.len() - 1];
    let v: serde_json::Value = serde_json::from_str(arg).unwrap();
    let found = v.as_object().expect("an object").values().any(|x| x.as_str() == Some(nasty));
    assert!(found, "{arg}");
}

#[test]
fn a_script_that_fails_yields_fixed_words_with_no_part_of_the_value_or_the_script() {
    let words = script_failure(&format!("ReferenceError: {PW} is not defined at {USER}"));
    assert!(!words.is_empty());
    assert!(!words.contains(PW) && !words.contains(USER) && !words.contains("ReferenceError"));
    assert_eq!(words, script_failure("something else entirely"), "fixed, not derived from the error");
}

#[test]
fn the_script_picks_the_submit_from_the_forms_own_submit_controls_and_never_the_first_button() {
    let b = body();
    assert!(b.contains("[type=submit]") || b.contains("[type=\"submit\"]"), "the form's own type=submit controls");
    for banned in ["querySelector(\"button\")", "querySelector(\"button,", "buttons[0]", "getElementsByTagName(\"button\")[0]"] {
        assert!(!b.contains(banned), "{banned}: never the first button");
    }
}

#[test]
fn the_script_reads_formaction_and_formmethod_from_the_control_and_action_and_method_from_the_form() {
    let b = body();
    for needle in ["formaction", "formmethod", "action", "method"] {
        assert!(b.contains(needle), "{needle}");
    }
}

#[test]
fn the_script_fires_input_keyup_and_change_then_leaves_the_box_in_that_order() {
    let b = body();
    let pos = |n: &str| b.find(n).unwrap_or_else(|| panic!("{n} missing"));
    let order = [pos("\"input\""), pos("\"keyup\""), pos("\"change\""), pos("blur")];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
}

#[test]
fn the_script_waits_before_it_presses() {
    let b = body();
    let wait = b.find("setTimeout").expect("a wait");
    let press = [b.find(".click()"), b.find("requestSubmit(")].into_iter().flatten().min().expect("a press");
    assert!(wait < press);
}

#[test]
fn the_script_reads_page_text_by_a_visible_text_property_only() {
    let b = body();
    assert!(b.contains("innerText"));
    assert!(!b.contains("textContent") && !b.contains("innerHTML") && !b.contains("outerHTML"));
}

#[test]
fn the_script_presses_by_click_or_request_submit_and_never_has_a_submit_member() {
    let b = body();
    assert!(b.contains(".click()") || b.contains("requestSubmit("));
    assert!(!b.contains(".submit"), "`.submit(`, `.submit` and `prototype.submit.call` are all out");
    assert!(!b.contains("[\"submit\"]"), "[\"submit\"] is out too");
    // The event name and requestSubmit(control) stay allowed.
    assert!(b.matches("\"submit\"").count() >= b.matches("addEventListener(\"submit\"").count());
}

#[test]
fn the_backstop_is_a_capture_listener_on_window_that_always_prevents_default_added_before_the_press() {
    let b = body();
    let add = b.find("window.addEventListener(\"submit\"").expect("a submit listener on window");
    let tail = &b[add..];
    let prevent = tail.find("preventDefault()").expect("it calls preventDefault");
    for cond in ["if", "?", "&&", "||"] {
        assert!(!tail[..prevent].contains(cond), "preventDefault is unconditional (found {cond})");
    }
    let after = &tail[prevent..tail.len().min(prevent + 120)];
    assert!(after.contains("true") || tail[..prevent].contains("capture"), "capture phase");
    let press = [b.find(".click()"), b.find("requestSubmit(")].into_iter().flatten().min().expect("a press");
    assert!(add < press, "added just before the press");
}
