//! Issue reports: the payload, the scrub, the preview (Knowlu C1, Task 16).
//!
//! **The text the user reads is the payload.** Not a summary of it, not a friendly rendering of a
//! structure sent separately — the exact bytes. That is what makes the preview screen the consent
//! the legal note (§9) and Google's Limited Use both want: a person agreed to a human reading *this*.
//! It is redacted before they see it, and they may edit it before they send it.
//!
//! `tray::diagnostics_text` is a different thing and stays as it is: it goes to the clipboard, it is
//! three lines long, and nobody transmits it.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::state::ConsoleState;

/// `scrub` is the Rust twin of the cloud's `cloud/supabase/functions/_shared/scrub.ts`, transcribed
/// rule for rule (ruling R-C1-45). **Six classes, in the cloud's own order** — email, then any-scheme
/// URL, then the Windows account name, then a note's `.md` filename, then a named credential, then a
/// bare 20+ character token — each pass a hand-written scanner rather than a `regex::Regex`: `regex`
/// is a dependency of `knowlu-engine`, not of this crate (`app/Cargo.toml` carries no direct `regex`
/// line), and adding one is not this task's to do.
///
/// Order is load-bearing exactly as the cloud's own comment says: URLs before the bare-token rule,
/// or a URL's path reads as a token and the sentence loses its shape; the Windows account name and a
/// note's filename both go before the bare-token rule for the same reason. The credential rule goes
/// **before** the bare-token rule too — `key=sk_live_…` and `Authorization: Bearer <anything>` are a
/// credential shape regardless of the value's own length, and claiming the whole `keyword[:=]value`
/// span (with the cloud's `$1=<secret>` replacement, keeping the keyword's own spelling) is what lets
/// a short bearer token be redacted even though it would never clear the bare-token rule's 20-char
/// floor on its own.
pub fn scrub(text: &str) -> String {
    let s = scrub_email(text);
    let s = scrub_url(&s);
    let s = scrub_winuser(&s);
    let s = scrub_note(&s);
    let s = scrub_credential(&s);
    scrub_token(&s)
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g` — no `\b`, so a match can start
/// anywhere; the local part is whatever local-charset run sits immediately before an `@`, and the
/// domain is the rightmost `.`-plus-2-or-more-letters tail inside the domain-charset run after it
/// (the same backtrack a real match would settle on).
fn scrub_email(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let is_local = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-');
    let is_domain = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-';
    let mut out = String::new();
    let mut i = 0usize;
    let mut last = 0usize;
    while i < cs.len() {
        if cs[i] == '@' {
            let mut ls = i;
            while ls > last && is_local(cs[ls - 1]) {
                ls -= 1;
            }
            if ls < i {
                let mut de = i + 1;
                while de < cs.len() && is_domain(cs[de]) {
                    de += 1;
                }
                let mut k = de;
                let mut matched: Option<usize> = None;
                while k > i + 2 {
                    k -= 1;
                    if cs[k] == '.' {
                        let mut le = k + 1;
                        while le < de && cs[le].is_ascii_alphabetic() {
                            le += 1;
                        }
                        if le - (k + 1) >= 2 {
                            matched = Some(le);
                            break;
                        }
                    }
                }
                if let Some(end) = matched {
                    out.push_str(&cs[last..ls].iter().collect::<String>());
                    out.push_str("<email>");
                    last = end;
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&cs[last..].iter().collect::<String>());
    out
}

/// `URL_RE = /\b[a-z][a-z0-9+.-]*:\/\/\S+/gi` — any scheme, `webcal://` included (fix round 1, item
/// 1 on the cloud side): a letter, then scheme characters, then a literal `://`, then every
/// non-whitespace character to the end of the token.
fn scrub_url(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let is_scheme_cont = |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-');
    let mut out = String::new();
    let mut i = 0usize;
    let mut last = 0usize;
    while i + 2 < cs.len() {
        if cs[i] == ':' && cs[i + 1] == '/' && cs[i + 2] == '/' {
            let mut s = i;
            while s > 0 && is_scheme_cont(cs[s - 1]) {
                s -= 1;
            }
            let boundary_ok = s == 0 || !is_word(cs[s - 1]);
            if s < i && cs[s].is_ascii_alphabetic() && boundary_ok {
                let mut e = i + 3;
                while e < cs.len() && !cs[e].is_whitespace() {
                    e += 1;
                }
                out.push_str(&cs[last..s].iter().collect::<String>());
                out.push_str("<url>");
                last = e;
                i = e;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&cs[last..].iter().collect::<String>());
    out
}

/// `WINUSER = /([:\\\/]Users[\\\/])[^\\\/"']+/gi` — fix round 1, item 2 on the cloud side: the
/// excluded set is only `\`, `/`, `"` and `'`, **not whitespace**, so a two-word account name is
/// consumed whole rather than truncated at its first word. The captured prefix (`:\Users\` or
/// `/Users/`, case-insensitively) is kept; only the name itself becomes `<user>`.
fn scrub_winuser(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let lower: Vec<char> = text.to_ascii_lowercase().chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    let mut last = 0usize;
    while i < cs.len() {
        if matches!(lower[i], ':' | '\\' | '/') && i + 6 < cs.len() {
            let seg: String = lower[i + 1..i + 6].iter().collect();
            if seg == "users" && matches!(lower[i + 6], '\\' | '/') {
                let mut e = i + 7;
                while e < cs.len() && !matches!(cs[e], '\\' | '/' | '"' | '\'') {
                    e += 1;
                }
                if e > i + 7 {
                    out.push_str(&cs[last..i + 7].iter().collect::<String>());
                    out.push_str("<user>");
                    last = e;
                    i = e;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&cs[last..].iter().collect::<String>());
    out
}

/// `NOTE = /\b[\w.-]+\.md\b/g` — a note's filename, before the bare-token rule: a filename is the
/// more specific fact, and dots stay in the token class so a JWT is one token, not three.
fn scrub_note(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let is_class = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-';
    let mut out = String::new();
    let mut i = 0usize;
    let mut last = 0usize;
    while i < cs.len() {
        if is_class(cs[i]) && (i == 0 || !is_class(cs[i - 1])) {
            let rs = i;
            let mut re = i;
            while re < cs.len() && is_class(cs[re]) {
                re += 1;
            }
            if re >= rs + 4 {
                let mut found: Option<usize> = None;
                let mut p = re as isize - 3;
                while p >= rs as isize + 1 {
                    let pu = p as usize;
                    if cs[pu] == '.' && cs[pu + 1] == 'm' && cs[pu + 2] == 'd' {
                        let boundary_ok = pu + 3 >= cs.len() || !is_word(cs[pu + 3]);
                        if boundary_ok {
                            found = Some(pu);
                            break;
                        }
                    }
                    p -= 1;
                }
                if let Some(p) = found {
                    out.push_str(&cs[last..rs].iter().collect::<String>());
                    out.push_str("<note>");
                    last = p + 3;
                    i = p + 3;
                    continue;
                }
            }
            i = re;
            continue;
        }
        i += 1;
    }
    out.push_str(&cs[last..].iter().collect::<String>());
    out
}

/// `CREDENTIAL = /\b(password|passwd|pwd|token|secret|api[_-]?key|key|authorization|bearer)\s*[:=]\s*(?:bearer\s+)?\S+/gi`,
/// replaced as `$1=<secret>` — a named credential is a secret whatever its length, so this goes
/// before the bare-token rule and claims the whole span: the keyword, the separator, an optional
/// leading `Bearer `, and the value up to the next whitespace. `bearer` is itself a keyword, so
/// `Authorization: Bearer <anything>` is claimed whole and the scheme name never stands next to an
/// unredacted value.
fn scrub_credential(text: &str) -> String {
    const KEYWORDS: [&str; 11] = [
        "password", "passwd", "pwd", "token", "secret", "api_key", "api-key", "apikey", "key", "authorization",
        "bearer",
    ];
    let cs: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    let mut last = 0usize;
    while i < cs.len() {
        if i == 0 || !is_word(cs[i - 1]) {
            let mut matched: Option<(usize, String)> = None;
            for kw in KEYWORDS.iter() {
                let klen = kw.chars().count();
                if i + klen <= cs.len() {
                    let seg: String = cs[i..i + klen].iter().collect();
                    if seg.eq_ignore_ascii_case(kw) {
                        matched = Some((i + klen, seg));
                        break;
                    }
                }
            }
            if let Some((kend, kw_text)) = matched {
                let mut j = kend;
                while j < cs.len() && cs[j].is_whitespace() {
                    j += 1;
                }
                if j < cs.len() && (cs[j] == ':' || cs[j] == '=') {
                    j += 1;
                    while j < cs.len() && cs[j].is_whitespace() {
                        j += 1;
                    }
                    const BEARER_LEN: usize = 6;
                    if j + BEARER_LEN <= cs.len() {
                        let seg: String = cs[j..j + BEARER_LEN].iter().collect();
                        if seg.eq_ignore_ascii_case("bearer") {
                            let ws_start = j + BEARER_LEN;
                            let mut k = ws_start;
                            while k < cs.len() && cs[k].is_whitespace() {
                                k += 1;
                            }
                            if k > ws_start {
                                j = k;
                            }
                        }
                    }
                    let val_start = j;
                    let mut e = j;
                    while e < cs.len() && !cs[e].is_whitespace() {
                        e += 1;
                    }
                    if e > val_start {
                        out.push_str(&cs[last..i].iter().collect::<String>());
                        out.push_str(&kw_text);
                        out.push_str("=<secret>");
                        last = e;
                        i = e;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out.push_str(&cs[last..].iter().collect::<String>());
    out
}

/// `TOKEN = /\b[A-Za-z0-9_.-]{20,}\b/g` — a bare run of 20 or more token characters, the class's
/// leading and trailing `.`/`-` trimmed off (they are not `\w`, so a `\b` can't land on them) before
/// the length floor is checked.
fn scrub_token(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let is_class = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-';
    let mut out = String::new();
    let mut i = 0usize;
    let mut last = 0usize;
    while i < cs.len() {
        if is_class(cs[i]) && (i == 0 || !is_class(cs[i - 1])) {
            let rs = i;
            let mut re = i;
            while re < cs.len() && is_class(cs[re]) {
                re += 1;
            }
            let ts = (rs..re).find(|&k| is_word(cs[k]));
            if let Some(ts) = ts {
                let te = (rs..re).rev().find(|&k| is_word(cs[k])).map(|k| k + 1).unwrap_or(ts);
                if te - ts >= 20 {
                    out.push_str(&cs[last..ts].iter().collect::<String>());
                    out.push_str("<token>");
                    last = te;
                    i = te;
                    continue;
                }
            }
            i = re;
            continue;
        }
        i += 1;
    }
    out.push_str(&cs[last..].iter().collect::<String>());
    out
}

/// How many notes are in each folder. **Counts, never names** (spec §6): a filename is a slugified
/// title, and a unique course schedule is not depersonalised by dropping a name off the front of it.
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

fn os_build() -> String {
    // No new dependency for one line: the same value `ver` reports, read from the environment Windows
    // already exports. Absent on a machine that has neither, which is honest.
    std::env::var("OS").unwrap_or_else(|_| "unknown".to_string())
}

/// Everything the report carries, as the text the user is about to read. Spec §6: the last 200 lines
/// of the logs, the two build shas, the OS, the profile id, the vault's shape as counts, and the
/// **name** of the view the user was looking at — never that view's contents, and never its keys
/// either: a view's key set is a fact about the read model, not about this install, and the name is
/// the only part a reader of the report can act on (ruling R-C1-5).
pub fn preview_text(cs: &ConsoleState, view: &str) -> String {
    let mut out = String::new();
    out.push_str("--- Knowlu issue report ---\n");
    out.push_str(&format!(
        "app {} build {}\n",
        env!("CARGO_PKG_VERSION"),
        crate::commands::CONSOLE_BUILD.unwrap_or("unknown")
    ));
    out.push_str(&format!("os {}\n", os_build()));
    out.push_str(&format!("profile {}\n", cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default()));
    out.push_str(&format!("view {view}\n"));
    let shape: Vec<String> = vault_shape(&cs.vault).into_iter().map(|(k, n)| format!("{k}={n}")).collect();
    out.push_str(&format!("vault {}\n", shape.join(" ")));
    if let Some(e) = &cs.settings_error {
        out.push_str(&format!("settings error: {}\n", scrub(e)));
    }
    if let Some(e) = &crate::scheduler::lock(&cs.history).last_error {
        out.push_str(&format!("sync error: {}\n", scrub(e)));
    }
    if let Some(e) = &crate::scheduler::lock(&cs.backup).last_error {
        out.push_str(&format!("backup error: {}\n", scrub(e)));
    }
    let runs = knowlu_engine::surface::runs_panel(&cs.vault, jiff::Timestamp::now());
    for r in runs.recent.iter().take(3) {
        out.push_str(&scrub(&format!("run {} {} {} {}", r.runner, r.started.clone().unwrap_or_default(), r.result, r.summary)));
        out.push('\n');
    }
    out.push_str("--- the last 200 log lines ---\n");
    for l in log_tail(&cs.data_dir, 200) {
        out.push_str(&l);
        out.push('\n');
    }
    out.push_str("--- what went wrong (write here) ---\n\n");
    out
}

/// One `POST /issues`, compact JSON, never `send_json` — R-C1-27, the same reason
/// `account::post_json` gives: `app/Cargo.toml` does not carry ureq's `json` feature, and
/// `send_json` pretty-prints besides, which would put a newline inside a body a test (and a real
/// server) reads as one line.
pub fn send_at(api_base: &str, token: &str, text: &str, profile_id: &str) -> Result<String, String> {
    crate::account::check_api_base(api_base)?;
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
    if !(200..300).contains(&status) {
        return Err(format!("the report was not accepted ({status})"));
    }
    let v: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
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
