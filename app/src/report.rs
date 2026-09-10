//! Issue reports: the payload, the scrub, the preview (Knowlu C1, Task 16).
//!
//! **The text the user reads is the payload.** Not a summary of it, not a friendly rendering of a
//! structure sent separately — the exact bytes. That is what makes the preview screen the consent
//! the legal note (§9) and Google's Limited Use both want: a person agreed to a human reading *this*.
//! It is redacted before they see it, and they may edit it before they send it.
//!
//! `tray::diagnostics_text` is a different thing and stays as it is: it goes to the clipboard, it is
//! three lines long, and nobody transmits it.
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tauri::State;

use crate::state::ConsoleState;

/// `EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g` — the cloud's pattern, verbatim. No
/// `\b`, no case-insensitivity: both character classes already spell out both cases.
static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());

/// `URL_RE = /\b[a-z][a-z0-9+.-]*:\/\/\S+/gi` — any scheme, `webcal://` included (fix round 1, item
/// 1 on the cloud side). `(?-u:\b)` matches JavaScript's ASCII-only `\b` (this pattern carries no
/// `u` flag on the cloud side either) rather than Rust's Unicode-aware default. `\S` stays at Rust's
/// Unicode default rather than `(?-u:\S)`: the string-mode `Regex` refuses to compile a *negated*
/// ASCII class (`(?-u:\S)`, `(?-u:\W)`, `(?-u:\D)`) — a lone UTF-8 continuation byte would match it,
/// which is not a valid match boundary in `&str` — so an ASCII-restricted `\S` would need
/// `regex::bytes::Regex` and a byte-slice API throughout this module. The gap this leaves is narrow
/// and the wrong direction to worry about: Rust's Unicode `\S` treats *more* characters as
/// whitespace than ASCII does (the extra ones are Unicode space separators, which JavaScript's own
/// `\s` — never ASCII-only either — already treats as whitespace too), so it can only stop a match
/// slightly earlier than the cloud does on an input with Unicode whitespace in it, never leave more
/// text unredacted.
static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(?-u:\b)[a-z][a-z0-9+.-]*://\S+").unwrap());

/// `WINUSER = /([:\\\/]Users[\\\/])[^\\\/"']+/gi` — fix round 1, item 2 on the cloud side: the
/// excluded set is only `\`, `/`, `"` and `'`, **not whitespace**, so a two-word account name is
/// consumed whole rather than truncated at its first word.
static WINUSER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)([:\\/]Users[\\/])[^\\/"']+"#).unwrap());

/// `NOTE = /\b[\w.-]+\.md\b/g` — a note's filename, before the bare-token rule: a filename is the
/// more specific fact, and dots stay in the token class so a JWT is one token, not three.
static NOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\b)(?-u:[\w.-])+\.md(?-u:\b)").unwrap());

/// `CREDENTIAL = /\b(password|passwd|pwd|token|secret|api[_-]?key|key|authorization|bearer)\s*[:=]\s*(?:bearer\s+)?\S+/gi`
/// — a named credential is a secret whatever its length, so this goes before the bare-token rule
/// and claims the whole span: the keyword, the separator, an optional leading `Bearer `, and the
/// value up to the next whitespace. `bearer` is itself a keyword, so `Authorization: Bearer
/// <anything>` is claimed whole and the scheme name never stands next to an unredacted value.
/// `\S` here is Rust's Unicode default too, for the same reason `URL_RE`'s is: `(?-u:\S)` is a
/// negated ASCII class and the string-mode `Regex` refuses to compile one.
static CREDENTIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:\b)(password|passwd|pwd|token|secret|api[_-]?key|key|authorization|bearer)(?-u:\s)*[:=](?-u:\s)*(?:bearer(?-u:\s)+)?\S+",
    )
    .unwrap()
});

/// `TOKEN = /\b[A-Za-z0-9_.-]{20,}\b/g` — a bare run of 20 or more token characters.
static TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\b)[A-Za-z0-9_.-]{20,}(?-u:\b)").unwrap());

/// `scrub` is the Rust twin of the cloud's `cloud/supabase/functions/_shared/scrub.ts`, transcribed
/// pattern for pattern (ruling R-C1-46, tightening R-C1-45): **six regexes, in the cloud's own
/// order** — email, then any-scheme URL, then the Windows account name, then a note's `.md`
/// filename, then a named credential, then a bare 20+ character token, each a literal `regex` crate
/// transcription of the corresponding line in `scrub.ts`, not a hand-written scanner.
///
/// **Why the switch:** a hand-written scanner is an attempt to reproduce a backtracking regex
/// engine's leftmost-first semantics by hand, and round 1's scanners got it wrong on real inputs —
/// `-https://…` left the URL on the wire because the scanner's single backtrack step gave up where
/// the `+`/`.`/`-` scheme-continuation characters aren't `\w`, so a `\b` (and the earliest valid
/// match) sits one character later than the scanner ever looked. The `regex` crate's default
/// `Regex` type matches with the same leftmost-first priority a backtracking engine produces (Thompson
/// NFA simulation with alternation priority, not actual backtracking, but the same *result*), so a
/// faithful pattern transcription reproduces the cloud's output rather than approximating it.
/// `regex = "1.13.1"` is the same version `engine/Cargo.toml` already pins and was already compiled
/// into this workspace's one `target/` — see `app/Cargo.toml`'s comment on the line.
///
/// Order is load-bearing exactly as the cloud's own comment says: URLs before the bare-token rule,
/// or a URL's path reads as a token and the sentence loses its shape; the Windows account name and a
/// note's filename both go before the bare-token rule for the same reason; the credential rule goes
/// before the bare-token rule too, so a short bearer token is still claimed even though it would
/// never clear the bare-token rule's 20-character floor on its own.
pub fn scrub(text: &str) -> String {
    let s = EMAIL.replace_all(text, "<email>");
    let s = URL_RE.replace_all(&s, "<url>");
    let s = WINUSER.replace_all(&s, "${1}<user>");
    let s = NOTE.replace_all(&s, "<note>");
    let s = CREDENTIAL.replace_all(&s, "${1}=<secret>");
    let s = TOKEN.replace_all(&s, "<token>");
    s.into_owned()
}

/// How many notes are in each folder. **Counts, never names** (spec §6): a filename is a slugified
/// title, and a unique course schedule is not depersonalised by dropping a name off the front of it.
/// A decision, not an oversight (M4): the extension check matches a directory literally named
/// `x.md` the same as a file — nobody makes one, and treating them alike keeps this one expression.
pub fn vault_shape(vault: &Path) -> Vec<(String, usize)> {
    ["tasks", "approvals", "archive", "courses", "info", "issues"]
        .iter()
        .map(|d| {
            let n = std::fs::read_dir(vault.join(d))
                .map(|rd| rd.flatten().filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false)).count())
                .unwrap_or(0);
            ((*d).to_string(), n)
        })
        .collect()
}

/// The last `lines` lines across the newest slot logs, oldest first, each scrubbed. The engine's own
/// stdout is in these files — that is the point of them — and it names notes and feeds.
pub fn log_tail(data_dir: &Path, lines: usize) -> Vec<String> {
    let dir = data_dir.join("logs");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("slot-"))
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (t, e.path())))
        .collect();
    files.sort();
    let mut all: Vec<String> = Vec::new();
    for (_, p) in files {
        if let Ok(text) = std::fs::read_to_string(&p) {
            for l in text.lines() {
                all.push(scrub(l));
            }
        }
    }
    if all.len() > lines {
        all.drain(..all.len() - lines);
    }
    all
}

/// The cloud's own cap on the `body` field (`cloud/supabase/functions/issues/handler.ts:5`) —
/// mirrored by hand, not imported: a JS constant and a Rust one cannot share a definition across
/// the language boundary this repo draws (`tauri` never enters the engine, and nothing in `app/`
/// reaches into `cloud/` either). If the server's cap ever moves, this one has to move with it.
const MAX_BODY: usize = 8192;

/// Left over, under [`MAX_BODY`], for the sentence the student is about to type in the "write here"
/// section. `preview_text` trims the auto-generated part of the report — log lines first, then run
/// rows — so what is left of the budget after assembling everything else is at least this much,
/// and a short "it broke when I clicked X" never bumps into the cap on its own.
const STUDENT_MARGIN: usize = 512;

/// The real Windows build, `major.minor.build` (M2, fix round 1): `%OS%` is `Windows_NT` on every
/// Windows machine ever — a constant that identifies no build — so this reads the actual version
/// through `RtlGetVersion`, which (unlike `GetVersionExW`) is not subject to the application
/// manifest's compatibility lie. `windows` is already a direct dependency of this crate
/// (`credentials.rs`'s credential store, `main.rs`'s message boxes, `profiles.rs`'s process wait);
/// this uses two more of its own feature flags (`app/Cargo.toml`), not a new crate. Falls back to
/// `%OS%` only if the call itself fails, which nothing on record ever does on real Windows.
fn os_build() -> String {
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
    let mut info =
        OSVERSIONINFOW { dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32, ..Default::default() };
    let status = unsafe { RtlGetVersion(&mut info) };
    if status.0 >= 0 {
        format!("{}.{}.{}", info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber)
    } else {
        std::env::var("OS").unwrap_or_else(|_| "unknown".to_string())
    }
}

/// The fixed pieces of `preview_text` assembled back into one string: the header (never trimmed —
/// it is a handful of short lines and cutting it would hide the facts a reader needs most), the run
/// rows still standing, a marker naming how many of the oldest log lines were dropped to fit (only
/// present once something was), the log lines still standing, and the trailer. `.len()` here counts
/// UTF-8 bytes, which is always at least the cloud's UTF-16 `.length` for the same text — a
/// conservative estimate that only ever trims a little more than the server strictly requires,
/// never a little less.
fn assemble_preview(header: &str, run_lines: &[String], log_lines: &[String], dropped_logs: usize, trailer: &str) -> String {
    let mut out = String::from(header);
    for r in run_lines {
        out.push_str(r);
        out.push('\n');
    }
    out.push_str("--- the last 200 log lines ---\n");
    if dropped_logs > 0 {
        out.push_str(&format!("… {dropped_logs} earlier log lines dropped to fit the report size limit\n"));
    }
    for l in log_lines {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(trailer);
    out
}

/// Everything the report carries, as the text the user is about to read. Spec §6: the last 200 lines
/// of the logs, the two build shas, the OS, the profile id, the vault's shape as counts, and the
/// **name** of the view the user was looking at — never that view's contents, and never its keys
/// either: a view's key set is a fact about the read model, not about this install, and the name is
/// the only part a reader of the report can act on (ruling R-C1-5).
///
/// **Bounded against the cloud's [`MAX_BODY`] (I2, fix round 1):** two hundred log lines, on a busy
/// install, routinely add up to more than the server accepts — and every report would then be
/// refused with nothing more than a status code (see [`send_at`]'s I3 fix) instead of ever reaching
/// a human. So this trims: the oldest log lines first (the newest is what a reader wants), then —
/// only if that alone is not enough — the run rows, leaving a one-line marker naming what was
/// dropped, so what the student reads is still exactly what fits and exactly what is sent.
pub fn preview_text(cs: &ConsoleState, view: &str) -> String {
    let mut header = String::new();
    header.push_str("--- Knowlu issue report ---\n");
    header.push_str(&format!(
        "app {} build {}\n",
        env!("CARGO_PKG_VERSION"),
        crate::commands::CONSOLE_BUILD.unwrap_or("unknown")
    ));
    header.push_str(&format!("os {}\n", os_build()));
    header.push_str(&format!("profile {}\n", cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default()));
    header.push_str(&format!("view {view}\n"));
    let shape: Vec<String> = vault_shape(&cs.vault).into_iter().map(|(k, n)| format!("{k}={n}")).collect();
    header.push_str(&format!("vault {}\n", shape.join(" ")));
    if let Some(e) = &cs.settings_error {
        header.push_str(&format!("settings error: {}\n", scrub(e)));
    }
    if let Some(e) = &crate::scheduler::lock(&cs.history).last_error {
        header.push_str(&format!("sync error: {}\n", scrub(e)));
    }
    if let Some(e) = &crate::scheduler::lock(&cs.backup).last_error {
        header.push_str(&format!("backup error: {}\n", scrub(e)));
    }

    let runs = knowlu_engine::surface::runs_panel(&cs.vault, jiff::Timestamp::now());
    let mut run_lines: Vec<String> = runs
        .recent
        .iter()
        .take(3)
        // `engine_build` on the wire (`send_at`) is this same console build's own sha (M3): one repo,
        // one sha per CI build, so the value is not wrong — only the field name says something the
        // console side doesn't. The run rows here are a different fact (the engine's own run history)
        // and unrelated to that field.
        .map(|r| scrub(&format!("run {} {} {} {}", r.runner, r.started.clone().unwrap_or_default(), r.result, r.summary)))
        .collect();
    let mut log_lines = log_tail(&cs.data_dir, 200);
    let trailer = "--- what went wrong (write here) ---\n\n";
    let budget = MAX_BODY.saturating_sub(STUDENT_MARGIN);

    let mut dropped_logs = 0usize;
    while assemble_preview(&header, &run_lines, &log_lines, dropped_logs, trailer).len() > budget && !log_lines.is_empty() {
        log_lines.remove(0);
        dropped_logs += 1;
    }
    while assemble_preview(&header, &run_lines, &log_lines, dropped_logs, trailer).len() > budget && !run_lines.is_empty() {
        run_lines.remove(0);
    }
    assemble_preview(&header, &run_lines, &log_lines, dropped_logs, trailer)
}

/// One `POST /issues`, compact JSON, never `send_json` — R-C1-27, the same reason
/// `account::post_json` gives: `app/Cargo.toml` does not carry ureq's `json` feature, and
/// `send_json` pretty-prints besides, which would put a newline inside a body a test (and a real
/// server) reads as one line.
///
/// **Refuses an over-[`MAX_BODY`] `text` before it ever reaches the network (I2, fix round 1).**
/// `preview_text` already trims what it builds to fit, but the student may still edit the textarea
/// back over the line — and the one thing this task exists to prevent is sending bytes the student
/// never read, so the fix belongs here, as a refusal, and not as a second silent truncation.
pub fn send_at(api_base: &str, token: &str, text: &str, profile_id: &str) -> Result<String, String> {
    crate::account::check_api_base(api_base)?;
    if text.trim().len() > MAX_BODY {
        return Err(format!("the report is longer than {MAX_BODY} characters — please shorten it before sending"));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let body = json!({
        "body": text,
        "app_version": env!("CARGO_PKG_VERSION"),
        "engine_build": crate::commands::CONSOLE_BUILD,
        "os_build": os_build(),
        "profile_id": profile_id,
    });
    let payload = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let mut res = agent
        .post(&format!("{}/issues", api_base.trim_end_matches('/')))
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {token}"))
        .send(payload)
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let reply = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        // I3, fix round 1: the cloud's own sentence, whichever one this reply carried
        // (`{"error":"<one sentence>"}`, `_shared/http.ts`) — never a bare status code on a panel,
        // the same rule `account.rs`'s `provider_error` states for GoTrue.
        let said = v.get("error").and_then(|x| x.as_str()).filter(|s| !s.is_empty());
        return Err(match said {
            Some(s) => format!("the report was not accepted: {s}"),
            None => format!("the report was not accepted ({status})"),
        });
    }
    Ok(v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string())
}

/// The preview screen's text. Everything is already redacted; the page shows it in a textarea the
/// user can edit, and what they press Send on is what arrives.
#[tauri::command(async)]
pub fn report_preview(cs: State<'_, ConsoleState>, view: String) -> Value {
    json!({ "ok": true, "error": Value::Null, "text": preview_text(&cs, &view) })
}

/// Send exactly what the page had. **The text is not rebuilt here** — rebuilding it would send
/// something the user never read, which is the one thing the preview exists to prevent.
#[tauri::command(async)]
pub fn report_send(cs: State<'_, ConsoleState>, text: String) -> Value {
    let profile_id = cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default();
    let out = (|| -> Result<String, String> {
        let cfg = crate::account::cloud_config(&cs.vault)?;
        let auth = crate::account::auth_base(&cfg.api_base)?;
        let token = crate::account::valid_access_token_at(
            &auth,
            &cfg.anon_key,
            &cfg.session_credential_target,
            jiff::Timestamp::now().as_second(),
        )?;
        send_at(&cfg.api_base, &token, &text, &profile_id)
    })();
    match out {
        Ok(id) => json!({ "ok": true, "error": Value::Null, "id": id }),
        Err(e) => json!({ "ok": false, "error": e, "id": Value::Null }),
    }
}
