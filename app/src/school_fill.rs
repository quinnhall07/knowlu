//! P2: the fill's decisions and its window script (replan §7.5, §8 P2a and P2b). **Stubs for P1t.**
#![cfg(windows)]
use crate::school_password::{SchoolMarks, SchoolSecret};
use std::time::Duration;

/// The wait after a field is set, before the submit.
pub const FIELD_WAIT: Duration = Duration::from_millis(1500);
/// The wait after each credential-carrying submit (T3b sets it; until then it is `SETTLE`).
pub const SUBMIT_SETTLE: Duration = crate::grades::SETTLE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Username,
    Password,
}

/// One button of the page's form. `formaction` is resolved (absolute) when present.
#[derive(Debug, Clone, Default)]
pub struct Button {
    pub submit: bool,
    pub formaction: Option<String>,
    pub formmethod: Option<String>,
}

/// What the page shows now. `form_action` is the form's resolved `action` (`None` or empty: none).
#[derive(Debug, Clone, Default)]
pub struct PageShape {
    pub url: String,
    pub fields: Vec<Field>,
    pub form_action: Option<String>,
    pub form_method: Option<String>,
    pub buttons: Vec<Button>,
    /// Visible text only (`innerText`).
    pub visible_text: String,
    /// What the page carries but does not show (hidden elements, script, templates, attributes). Never matched.
    pub hidden_text: String,
}

/// The page seam. The window implements it over `eval`; tests over a fake.
pub trait FillPage {
    /// `None`: the page's script is gone (`Lost`) or unreadable.
    fn shape(&mut self) -> Option<PageShape>;
    /// Focus, set, fire input, keyup and change, leave the box.
    fn set_field(&mut self, field: Field, value: &str) -> bool;
    fn wait(&mut self, d: Duration);
    /// Adds the capture-phase `submit` listener on `window` that always prevents the default.
    fn arm_backstop(&mut self) -> bool;
    /// Presses `buttons[button]` by `click()` or `requestSubmit`.
    fn press(&mut self, button: usize) -> bool;
    /// Waits up to `within` for `users/me` to answer 200.
    fn await_signed_in(&mut self, within: Duration) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Wrong,
    Unconfirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillOutcome {
    SignedIn,
    CannotRead,
    Rejected(Rejection),
}

/// One attempt: every page, in order. `hosts` is the row's `sign_in_hosts`; `wrong_text` its
/// `wrong_password_text`. Writes and flushes the in-flight mark through `marks` before the first
/// credential-carrying submit, and clears it on `SignedIn`.
pub fn run_fill(
    _page: &mut dyn FillPage,
    _marks: &dyn SchoolMarks,
    _hosts: &[&str],
    _wrong_text: &str,
    _secret: &SchoolSecret,
) -> FillOutcome {
    todo!("P2a")
}

/// The window script for one step: a fixed body, `})(`, `serde_json::to_string` of the value, `)`.
pub fn script(_secret: &SchoolSecret) -> String {
    todo!("P2b")
}

/// The fixed words for a script that failed; holds no part of the error, the value or the script.
pub fn script_failure(_raw_error: &str) -> &'static str {
    todo!("P2b")
}
