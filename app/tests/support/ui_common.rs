//! The new page's shared pin helpers (v0.4.0 note §8, N4; plan §3, T5). Every `ui_*.rs` pin file
//! includes this module with `#[path = "support/ui_common.rs"] mod ui_common;` (the
//! `support/slot_rig.rs` precedent), so a helper one file never calls raises no warning.
//!
//! It holds the one `ROOT` constant (the swap, T23, changes it to `static` and nothing else), the
//! file readers, the JS comment and string strippers, the `main.rs` command-list parser and the
//! untrusted-text and markup scanners the shell's pins and the page tasks' pins share.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The page's root, relative to the app crate. T23 (the swap) moved the new page here from `static/next`.
pub const ROOT: &str = "static";

/// The app crate's own folder (`app/`), whatever the test's working directory.
pub fn app_dir() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }

/// The new page's root as a path.
pub fn root() -> PathBuf { app_dir().join(ROOT) }

/// A file under `ROOT`, by its `/`-separated path; panics with the path when it cannot be read.
pub fn read(rel: &str) -> String {
    let p = root().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// A file under `ROOT`, or `None` when it is not there.
pub fn try_read(rel: &str) -> Option<String> { fs::read_to_string(root().join(rel)).ok() }

/// A file in the app crate (`app/<rel>`), such as `src/main.rs` or `tauri.conf.json`.
pub fn read_app(rel: &str) -> String {
    let p = app_dir().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, base, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Every file under `ROOT`, as sorted `/`-separated paths relative to it.
pub fn shipped_files() -> Vec<String> {
    let mut out = Vec::new();
    walk(&root(), &root(), &mut out);
    out
}

/// Every shipped file whose extension is one of `exts` (no dot), with its text.
pub fn shipped_with(exts: &[&str]) -> Vec<(String, String)> {
    shipped_files()
        .into_iter()
        .filter(|f| f.rsplit('.').next().is_some_and(|e| exts.contains(&e.to_ascii_lowercase().as_str())))
        .map(|f| {
            let t = read(&f);
            (f, t)
        })
        .collect()
}

/// Every shipped script (`.js`), with its text. "Every shipped script" pins scan this, so a later
/// task's files join them on their own.
pub fn scripts() -> Vec<(String, String)> { shipped_with(&["js"]) }

/// Every shipped stylesheet (`.css`), with its text.
pub fn styles() -> Vec<(String, String)> { shipped_with(&["css"]) }

/// The one document.
pub fn index_html() -> String { read("index.html") }

/// Every page file plan §4 names, with the task that owns it. T5 creates each as a one-line
/// placeholder whose whole content is `/* TNN owns */`; the page task replaces only its own.
pub const PLACEHOLDERS: [(&str, &str); 37] = [
    ("js/pages/home.js", "T7"),
    ("css/home.css", "T7"),
    ("js/pages/home_aside.js", "T8"),
    ("css/home_aside.css", "T8"),
    ("js/pages/lockin.js", "T9"),
    ("css/lockin.css", "T9"),
    ("js/pages/tasks.js", "T10"),
    ("css/tasks.css", "T10"),
    ("js/pages/calendar_data.js", "T11"),
    ("js/pages/calendar.js", "T11"),
    ("css/calendar.css", "T11"),
    ("js/pages/classes.js", "T13"),
    ("js/pages/class.js", "T13"),
    ("css/classes.css", "T13"),
    ("js/carried/details.js", "T14"),
    ("css/details.css", "T14"),
    ("js/carried/deck.js", "T15"),
    ("css/deck.css", "T15"),
    ("js/pages/settings.js", "T16"),
    ("css/settings.css", "T16"),
    ("js/carried/sources.js", "T17"),
    ("js/carried/mail.js", "T17"),
    ("css/mail.css", "T17"),
    ("js/carried/week.js", "T18"),
    ("js/carried/weeksetup.js", "T18"),
    ("css/week.css", "T18"),
    ("js/carried/status.js", "T19"),
    ("js/carried/update.js", "T19"),
    ("css/status.css", "T19"),
    ("js/carried/account.js", "T20"),
    ("js/carried/reconsent.js", "T20"),
    ("js/carried/report.js", "T20"),
    ("css/sheets.css", "T20"),
    ("js/wizard.js", "T21"),
    ("js/picker.js", "T21"),
    ("css/wizard.css", "T21"),
    ("css/picker.css", "T21"),
];

/// The shell's own files (T5): what a pin about the launch, boot or poll path reads.
pub const SHELL_SCRIPTS: [&str; 7] = ["js/prefs.js", "js/fmt.js", "js/kit.js", "js/data.js", "js/router.js", "js/shell.js", "js/boot.js"];

/// True when `text` is a placeholder: its whole content, trimmed, is `/* TNN owns */`.
pub fn is_placeholder(text: &str) -> bool {
    let t = text.trim();
    t.strip_prefix("/* T").and_then(|r| r.strip_suffix(" owns */")).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit() || b.is_ascii_lowercase()))
}

// ---- JS text -------------------------------------------------------------------------------------

/// Blank out JS string literals (single, double, template; with escapes) and comments. A string
/// becomes one space, so what is left is the code's shape.
pub fn strip_strings_and_comments(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let (mut out, mut i) = (String::new(), 0);
    while i < c.len() {
        let ch = c[i];
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' { i += 1; }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&'/')) {
                if c[i] == '\n' { out.push('\n'); }
                i += 1;
            }
            i += 2;
        } else if ch == '"' || ch == '\'' || ch == '`' {
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' { i += 1; }
                if c.get(i) == Some(&'\n') && ch != '`' { break; }
                i += 1;
            }
            i += 1;
            out.push(' ');
        } else {
            out.push(ch);
            i += 1;
        }
    }
    out
}

/// Remove JS (and CSS) comments, keeping string literals and line breaks, so a pin reads the code
/// and the strings it sends and never a word in prose.
pub fn strip_comments(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let (mut out, mut i) = (String::new(), 0);
    while i < c.len() {
        let ch = c[i];
        if ch == '/' && c.get(i + 1) == Some(&'/') && !(i > 0 && c[i - 1] == ':') {
            while i < c.len() && c[i] != '\n' { i += 1; }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&'/')) {
                if c[i] == '\n' { out.push('\n'); }
                i += 1;
            }
            i += 2;
        } else if ch == '"' || ch == '\'' || ch == '`' {
            out.push(ch);
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' {
                    out.push(c[i]);
                    i += 1;
                    if i >= c.len() { break; }
                }
                if c[i] == '\n' && ch != '`' { break; }
                out.push(c[i]);
                i += 1;
            }
            if i < c.len() && c[i] == ch { out.push(ch); i += 1; }
        } else {
            out.push(ch);
            i += 1;
        }
    }
    out
}

/// Remove CSS comments only (CSS has no `//` comments, and `url(//x)` must survive).
pub fn strip_css_comments(src: &str) -> String {
    let mut out = String::new();
    let mut rest = src;
    while let Some(at) = rest.find("/*") {
        out.push_str(&rest[..at]);
        match rest[at + 2..].find("*/") {
            Some(end) => rest = &rest[at + 2 + end + 2..],
            None => { rest = ""; }
        }
    }
    out.push_str(rest);
    out
}

/// True when, outside strings and comments, the word `import` is followed by whitespace or `(`.
pub fn has_import_syntax(src: &str) -> bool {
    let code = strip_strings_and_comments(src);
    let b = code.as_bytes();
    code.match_indices("import").any(|(i, _)| {
        let before_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'$' || b[i - 1] == b'.');
        matches!(b.get(i + 6).copied(), Some(b' ' | b'\t' | b'\n' | b'\r' | b'(')) && before_ok
    })
}

/// Every string literal in `src` (comments stripped first), its contents without the quotes.
pub fn string_literals(src: &str) -> Vec<String> {
    let code = strip_comments(src);
    let c: Vec<char> = code.chars().collect();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < c.len() {
        let ch = c[i];
        if ch == '"' || ch == '\'' || ch == '`' {
            let mut s = String::new();
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' && i + 1 < c.len() { s.push(c[i + 1]); i += 2; continue; }
                if c[i] == '\n' && ch != '`' { break; }
                s.push(c[i]);
                i += 1;
            }
            i += 1;
            out.push(s);
        } else {
            i += 1;
        }
    }
    out
}

/// The body of the first function whose text starts with `head` (e.g. `"function poll("`), from
/// `head` to its matching closing brace, strings and comments counted out of the brace match.
pub fn fn_body<'a>(js: &'a str, head: &str) -> &'a str {
    let start = js.find(head).unwrap_or_else(|| panic!("no {head}"));
    let open = start + js[start..].find('{').unwrap_or_else(|| panic!("{head} has no body"));
    // Walk the original text by brace depth, skipping strings and comments by hand.
    let b: Vec<char> = js[open..].chars().collect();
    let (mut depth, mut i, mut bytes) = (0i32, 0usize, 0usize);
    while i < b.len() {
        let ch = b[i];
        if ch == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' { bytes += b[i].len_utf8(); i += 1; }
            continue;
        }
        if ch == '/' && b.get(i + 1) == Some(&'*') {
            bytes += 2; i += 2;
            while i < b.len() && !(b[i] == '*' && b.get(i + 1) == Some(&'/')) { bytes += b[i].len_utf8(); i += 1; }
            bytes += 2; i += 2;
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '`' {
            bytes += 1; i += 1;
            while i < b.len() && b[i] != ch {
                if b[i] == '\\' { bytes += 1; i += 1; }
                if i < b.len() { bytes += b[i].len_utf8(); i += 1; }
            }
            bytes += 1; i += 1;
            continue;
        }
        if ch == '{' { depth += 1; }
        if ch == '}' {
            depth -= 1;
            if depth == 0 { return &js[start..open + bytes + 1]; }
        }
        bytes += ch.len_utf8();
        i += 1;
    }
    panic!("{head}: unbalanced braces")
}

/// Every command a script sends: the first argument of `.call("<name>"` (the one wrapper,
/// `KnowluData.call`) and of `invoke("<name>"`, when it is a quoted snake_case name.
pub fn commands_sent(js: &str) -> Vec<String> {
    let code = strip_comments(js);
    let mut out = Vec::new();
    for pat in [".call(\"", ".call('", "invoke(\"", "invoke('"] {
        for (i, _) in code.match_indices(pat) {
            let rest = &code[i + pat.len()..];
            let name: String = rest.chars().take_while(|c| c.is_ascii_lowercase() || *c == '_').collect();
            let quote = pat.chars().last().unwrap();
            if !name.is_empty() && rest[name.len()..].starts_with(quote) { out.push(name); }
        }
    }
    out
}

// ---- main.rs -------------------------------------------------------------------------------------

/// Every `generate_handler![…]` list in `main.rs`, each as its trimmed command paths.
pub fn handler_lists() -> Vec<Vec<String>> {
    const OPEN: &str = "generate_handler![";
    let src = read_app("src/main.rs");
    let mut lists = Vec::new();
    let mut rest = src.as_str();
    while let Some(at) = rest.find(OPEN) {
        let body = &rest[at + OPEN.len()..];
        let end = body.find(']').expect("a generate_handler! list is closed with ]");
        lists.push(body[..end].split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect());
        rest = &body[end..];
    }
    lists
}

fn list_with(lists: &[Vec<String>], marker: &str) -> Vec<String> {
    let found: Vec<&Vec<String>> = lists.iter().filter(|l| l.iter().any(|n| n == marker)).collect();
    assert_eq!(found.len(), 1, "exactly one generate_handler! list names {marker}");
    found[0].iter().map(|p| p.rsplit("::").next().unwrap_or(p).to_string()).collect()
}

/// `run_shell`'s commands (the wizard and the picker), as bare names.
pub fn shell_commands() -> Vec<String> { list_with(&handler_lists(), "onboarding::finish_onboarding") }

/// `run_console`'s commands (the console), as bare names.
pub fn console_commands() -> Vec<String> { list_with(&handler_lists(), "commands::state") }

/// Every `#[tauri::command]` in `app/src` whose signature takes `view: String`, by name: the
/// commands that answer with a fresh `state` for the page's view.
pub fn commands_taking_a_view() -> Vec<String> {
    let mut out = Vec::new();
    let mut files = Vec::new();
    walk(&app_dir().join("src"), &app_dir().join("src"), &mut files);
    for f in files.into_iter().filter(|f| f.ends_with(".rs")) {
        let rust = fs::read_to_string(app_dir().join("src").join(&f)).unwrap_or_default();
        for block in rust.split("#[tauri::command").skip(1) {
            let Some(close) = block.find(']') else { continue };
            let after = block[close + 1..].trim_start();
            let Some(sig) = after.strip_prefix("pub fn ").or_else(|| after.strip_prefix("pub async fn ")) else { continue };
            let name = sig.split('(').next().unwrap_or("").to_string();
            let Some(args) = sig.split_once('(').map(|(_, a)| a) else { continue };
            let args = args.split(')').next().unwrap_or("");
            if args.split(',').any(|a| a.trim().starts_with("view:")) { out.push(name); }
        }
    }
    out.sort();
    out.dedup();
    out
}

// ---- untrusted text and markup -------------------------------------------------------------------

/// Every occurrence of `.field` (a property read, not `Array.from`) in `js`, with its line.
pub fn field_reads<'a>(js: &'a str, fields: &[&str]) -> Vec<(usize, &'a str, String)> {
    let mut out = Vec::new();
    for (n, line) in js.lines().enumerate() {
        let b = line.as_bytes();
        for f in fields {
            let pat = format!(".{f}");
            let mut from = 0;
            while let Some(i) = line[from..].find(&pat) {
                let at = from + i;
                let after = b.get(at + pat.len()).copied().unwrap_or(b' ');
                let ident_after = after.is_ascii_alphanumeric() || after == b'_';
                let on_ident = at > 0 && (b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
                if !ident_after && on_ident && !line[..at].ends_with("Array") {
                    out.push((n + 1, line, line[..at + pat.len()].to_string()));
                }
                from = at + pat.len();
            }
        }
    }
    out
}

/// The fields of a mail-derived card or row (email spec D21): they reach the DOM only through `h()`.
pub const MAIL_FIELDS: [&str; 4] = ["from", "mail_date", "evidence", "sender"];

/// One read of an untrusted field is safe when it sits directly inside `h(`, is the fallback
/// `h(c.why || ...)`, or is only a test (`!c.x`, or `c.x ? ...`). `upto` is the line up to the end of
/// the read; `line` is the whole line.
pub fn read_is_safe(line: &str, upto: &str) -> bool {
    let var_len = upto.len() - upto.rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).map_or(0, |i| i + 1);
    let pre = &upto[..upto.len() - var_len];
    let post = &line[upto.len()..];
    let in_h = |p: &str| p.strip_suffix("h(").is_some_and(|b| !b.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.'));
    in_h(pre) || pre.strip_suffix("c.why || ").is_some_and(in_h) || pre.ends_with('!') || post.starts_with(" ? ")
}

/// True when the line calls `ev(` as a function of its own (not `prev(` or `.ev(`'s neighbours).
pub fn is_ev_call(line: &str) -> bool {
    let mut from = 0;
    while let Some(i) = line[from..].find("ev(") {
        let at = from + i;
        if at == 0 || !(line.as_bytes()[at - 1].is_ascii_alphanumeric() || line.as_bytes()[at - 1] == b'_') { return true; }
        from = at + 3;
    }
    false
}

/// Split `expr` at its top-level `+` signs (outside strings and brackets), each part trimmed.
pub fn plus_operands(expr: &str) -> Vec<String> {
    let c: Vec<char> = expr.chars().collect();
    let (mut parts, mut cur, mut depth, mut i) = (Vec::new(), String::new(), 0i32, 0usize);
    while i < c.len() {
        let ch = c[i];
        if ch == '"' || ch == '\'' || ch == '`' {
            cur.push(ch);
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' && i + 1 < c.len() { cur.push(c[i]); i += 1; }
                cur.push(c[i]);
                i += 1;
            }
            if i < c.len() { cur.push(ch); }
            i += 1;
            continue;
        }
        if ch == '(' || ch == '[' || ch == '{' { depth += 1; }
        if ch == ')' || ch == ']' || ch == '}' { depth -= 1; }
        if ch == '+' && depth == 0 {
            parts.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
        i += 1;
    }
    parts.push(cur.trim().to_string());
    parts
}

fn is_string_literal(s: &str) -> bool {
    let s = s.trim();
    s.len() >= 2 && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
}

fn is_ident(s: &str) -> bool { !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.') && !s.starts_with(|c: char| c.is_ascii_digit()) }

/// An ALL_CAPS identifier: a module constant (`WC_HTML`, `KnowluX.EMPTY` is not one).
fn is_constant(s: &str) -> bool { is_ident(s) && !s.contains('.') && s.chars().any(|c| c.is_ascii_uppercase()) && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') }

/// A name that says it holds markup built by this rule: it ends in `Html` or `html` (`rowHtml`,
/// `KnowluKit.markHtml`), so the builder itself is held to the rule wherever it assigns.
fn names_markup(s: &str) -> bool { s.ends_with("Html") || s.ends_with("html") || s.ends_with("HTML") }

/// True when `s` is one call `name(...)` whose parentheses close at its end.
fn whole_call<'a>(s: &'a str) -> Option<(&'a str, &'a str)> {
    let s = s.trim();
    let open = s.find('(')?;
    let name = &s[..open];
    if !is_ident(name) || !s.ends_with(')') { return None; }
    let inner = &s[open + 1..s.len() - 1];
    let mut depth = 0i32;
    for ch in strip_strings_and_comments(inner).chars() {
        if ch == '(' { depth += 1; }
        if ch == ')' { depth -= 1; if depth < 0 { return None; } }
    }
    (depth == 0).then_some((name, inner))
}

/// `op` without parentheses that wrap the whole of it (`(a + b)` → `a + b`; `h(a)` stays).
fn unwrap_parens(op: &str) -> &str {
    let mut op = op.trim();
    while op.starts_with('(') && op.ends_with(')') {
        let inner = &op[1..op.len() - 1];
        let mut depth = 0i32;
        let mut closes_early = false;
        for ch in strip_strings_and_comments(inner).chars() {
            if ch == '(' { depth += 1; }
            if ch == ')' { depth -= 1; if depth < 0 { closes_early = true; break; } }
        }
        if closes_early || depth != 0 { break; }
        op = inner.trim();
    }
    op
}

/// A whole expression is safe when every top-level `+` operand is (`operand_is_safe`).
pub fn expr_is_safe(expr: &str) -> bool { plus_operands(expr).iter().all(|o| operand_is_safe(o)) }

/// One operand of a markup concatenation is safe (D5) when it is
/// - a string literal, a module constant (`ALL_CAPS`), or a call to `h(…)`;
/// - `KnowluKit.owed()`, the update-owed attributes (plan §3), which kit.js builds by this rule;
/// - a name or a call that says it holds markup built by this same rule (`rowHtml`, `markHtml(r)`);
/// - a conditional whose two arms are safe (`x ? h(a) : ""`);
/// - an array of those joined: `list.join(…)` where `list` says markup, `xs.map(rowHtml).join(…)`,
///   or `xs.map(function (r) { return <safe>; }).join(…)`.
pub fn operand_is_safe(op: &str) -> bool {
    let op = unwrap_parens(op);
    if op.is_empty() || is_string_literal(op) || is_constant(op) || op == "KnowluKit.owed()" { return true; }
    if is_ident(op) && names_markup(op) { return true; }
    if let Some((name, _)) = whole_call(op) {
        if name == "h" || names_markup(name) { return true; }
    }
    let shape = strip_strings_and_comments(op);
    if shape.contains(".join(") {
        // `.join(` found in the code's shape; take its last occurrence in `op` itself and require
        // it to close the operand.
        if let Some(j) = op.rfind(".join(") {
            let arg = &op[j + 6..];
            if arg.ends_with(')') && is_string_literal(&arg[..arg.len() - 1]) {
                let base = op[..j].trim();
                if is_ident(base) && names_markup(base) { return true; }
                if let Some(m) = base.find(".map(") {
                    let cb = base[m + 5..].strip_suffix(')').unwrap_or("").trim();
                    if is_ident(cb) && names_markup(cb) { return true; }
                    if cb.starts_with("function") {
                        if let (Some(r), Some(end)) = (cb.find("return "), cb.rfind(';')) {
                            if r + 7 <= end && expr_is_safe(&cb[r + 7..end]) && cb[..r].matches("return ").count() == 0 { return true; }
                        }
                    }
                }
            }
        }
    }
    if shape.contains('?') && shape.contains(':') {
        if let Some((_, arms)) = split_ternary(op) {
            return arms.iter().all(|a| expr_is_safe(a));
        }
    }
    false
}

/// `test ? a : b` at the top level, as (test, [a, b]); `None` when `op` is not one.
fn split_ternary(op: &str) -> Option<(String, Vec<String>)> {
    let c: Vec<char> = op.chars().collect();
    let (mut depth, mut i, mut q, mut colon) = (0i32, 0usize, None, None);
    while i < c.len() {
        let ch = c[i];
        if ch == '"' || ch == '\'' {
            i += 1;
            while i < c.len() && c[i] != ch { if c[i] == '\\' { i += 1; } i += 1; }
            i += 1;
            continue;
        }
        if ch == '(' || ch == '[' || ch == '{' { depth += 1; }
        if ch == ')' || ch == ']' || ch == '}' { depth -= 1; }
        if depth == 0 && ch == '?' && q.is_none() { q = Some(i); }
        if depth == 0 && ch == ':' && q.is_some() && colon.is_none() { colon = Some(i); }
        i += 1;
    }
    let (q, colon) = (q?, colon?);
    let s: String = c[..q].iter().collect();
    let a: String = c[q + 1..colon].iter().collect();
    let b: String = c[colon + 1..].iter().collect();
    Some((s.trim().to_string(), vec![a.trim().to_string(), b.trim().to_string()]))
}

/// The markup sinks D5 governs, as the text that starts each one's right-hand side.
pub const HTML_SINKS: [&str; 4] = [".innerHTML = ", ".outerHTML = ", ".innerHTML += ", "insertAdjacentHTML("];

/// Every place in `js` (comments stripped) where markup is written or built that breaks D5's rule,
/// as `(line number, line)`:
/// - a sink (`innerHTML =`, `outerHTML =`, `insertAdjacentHTML(`) whose right-hand side is not made
///   only of safe operands (`operand_is_safe`);
/// - an assignment or `return` whose value builds markup (a string literal holding `<` and a
///   letter or `/`) from an operand that is not safe;
/// - an assignment to a name that says it holds markup (`…Html =`) from an operand that is not safe;
/// - a template literal with a `${` substitution, which no shipped script uses.
pub fn markup_violations(js: &str) -> Vec<(usize, String)> {
    let code = strip_comments(js);
    let mut bad = Vec::new();
    for (n, raw) in code.lines().enumerate() {
        let line = raw.trim();
        if line.contains('`') && line.contains("${") { bad.push((n + 1, line.to_string())); continue; }
        let mut rhs: Vec<String> = Vec::new();
        for sink in HTML_SINKS {
            if let Some(at) = line.find(sink) {
                let mut r = line[at + sink.len()..].to_string();
                if sink == "insertAdjacentHTML(" {
                    // insertAdjacentHTML("beforeend", <markup>)
                    r = r.split_once(',').map(|(_, m)| m.to_string()).unwrap_or_default();
                    r = r.trim_end_matches(';').trim_end_matches(')').to_string();
                }
                rhs.push(r);
            }
        }
        let markup_lit = string_literals(line).iter().any(|s| s.char_indices().any(|(i, ch)| ch == '<' && s[i + 1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/')));
        if rhs.is_empty() && markup_lit {
            if let Some(eq) = assignment_rhs(line) {
                rhs.push(eq);
            } else if let Some(r) = line.strip_prefix("return ") {
                rhs.push(r.to_string());
            } else {
                // A continuation line of a concatenation (`'<b>' + h(x) +`): the line itself.
                rhs.push(line.trim_start_matches('+').trim_end_matches(';').trim_end_matches(',').trim_end_matches('+').to_string());
            }
        }
        if rhs.is_empty() {
            if let Some((lhs, r)) = line.split_once(" = ").or_else(|| line.split_once(" += ")) {
                let name = lhs.trim().trim_start_matches("var ").trim_start_matches("let ").trim_start_matches("const ").trim();
                if is_ident(name) && names_markup(name) { rhs.push(r.to_string()); }
            }
        }
        for r in rhs {
            let r = r.trim().trim_end_matches(';').trim();
            if r.is_empty() { continue; }
            if !plus_operands(r).iter().all(|o| operand_is_safe(o)) { bad.push((n + 1, line.to_string())); }
        }
    }
    bad
}

/// The right-hand side of a plain assignment on this line (`a = b`, `var a = b`, `a += b`), not a
/// comparison; `None` when the line has none.
fn assignment_rhs(line: &str) -> Option<String> {
    let shape = strip_strings_and_comments(line);
    let b = shape.as_bytes();
    for (i, _) in shape.match_indices('=') {
        let prev = if i > 0 { b[i - 1] } else { b' ' };
        let next = b.get(i + 1).copied().unwrap_or(b' ');
        if next == b'=' || matches!(prev, b'=' | b'!' | b'<' | b'>') { continue; }
        // Map the position in the shape back to the line: count the code before it.
        let lhs_shape = &shape[..i];
        if lhs_shape.contains('(') && !lhs_shape.trim_start().starts_with("var ") { return None; }
        let lhs_len = original_len_for(line, i);
        return Some(line[lhs_len + 1..].to_string());
    }
    None
}

/// The byte length in `line` that strips to `shape_len` chars of `strip_strings_and_comments(line)`.
fn original_len_for(line: &str, shape_len: usize) -> usize {
    for (i, _) in line.char_indices().chain(std::iter::once((line.len(), ' '))) {
        if strip_strings_and_comments(&line[..i]).len() == shape_len && line[i..].starts_with('=') { return i; }
    }
    line.len()
}
