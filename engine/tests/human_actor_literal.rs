//! Ruling 11 (plan T6): **the literal guard.** The human token is one constant
//! (`journal::HUMAN_ACTOR`) and its legacy spelling one other (`journal::LEGACY_HUMAN_ACTOR`); this
//! fails on the exact literal `"quinn"` anywhere else in non-test code — `engine/src`, `app/src`,
//! `app/static`'s scripts and `cloud/supabase` — naming `file:line` for each. `quinn-ops`
//! (`profiles::migrate_flat_layout`) is a different string and stays legal. Like `workflows.rs`, it
//! reads the workspace from `CARGO_MANIFEST_DIR/..`.

use std::path::{Path, PathBuf};

const LEGACY: &str = "quinn";
const THE_CONSTANT: &str = "pub const LEGACY_HUMAN_ACTOR: &str = \"quinn\";";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every file under `dir` that `keep` accepts, recursively, in path order.
fn files(dir: &Path, keep: &dyn Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files(&path, keep));
        } else if keep(&path) {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn rel(path: &Path) -> String {
    path.strip_prefix(root()).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// The 1-based lines of every string literal whose whole content is exactly `quinn`, in Rust source,
/// **outside comments and outside any item under `#[cfg(test)]`**.
///
/// A small lexer rather than a line filter, so a brace or quote inside a string, a char, a raw string
/// or a comment can never end a skipped item early (test modules are full of `"{…}"`). The skipped
/// item is the one after the attribute: to its matching `}`, or to its `;` if it has no body.
fn rust_hits(src: &str) -> Vec<usize> {
    let c: Vec<char> = src.chars().collect();
    let at = |i: usize, pat: &str| pat.chars().enumerate().all(|(k, p)| c.get(i + k) == Some(&p));
    let newlines = |from: usize, to: usize| c[from..to.min(c.len())].iter().filter(|x| **x == '\n').count();
    let (mut i, mut line, mut hits) = (0usize, 1usize, Vec::new());
    let mut skip: Option<(i64, bool)> = None; // (brace depth, body seen)
    while i < c.len() {
        if at(i, "//") {
            while i < c.len() && c[i] != '\n' { i += 1; }
            continue;
        }
        if at(i, "/*") {
            let (start, mut depth) = (i, 0i64);
            while i < c.len() {
                if at(i, "/*") { depth += 1; i += 2; } else if at(i, "*/") { depth -= 1; i += 2; if depth == 0 { break; } } else { i += 1; }
            }
            line += newlines(start, i);
            continue;
        }
        let ident = |k: usize| c[k].is_alphanumeric() || c[k] == '_';
        // `r"…"`, `r#"…"#` and the byte form `br"…"`, but never the tail of an identifier.
        let raw_prefix = i == 0 || !ident(i - 1) || (c[i - 1] == 'b' && (i == 1 || !ident(i - 2)));
        if c[i] == 'r' && raw_prefix {
            let hashes = c[i + 1..].iter().take_while(|x| **x == '#').count();
            if c.get(i + 1 + hashes) == Some(&'"') {
                let open = i + 2 + hashes;
                let close: String = std::iter::once('"').chain(std::iter::repeat_n('#', hashes)).collect();
                let mut j = open;
                while j < c.len() && !at(j, &close) { j += 1; }
                if skip.is_none() && c[open..j.min(c.len())].iter().collect::<String>() == LEGACY { hits.push(line); }
                line += newlines(i, j);
                i = j + close.len();
                continue;
            }
        }
        if c[i] == '"' {
            let mut j = i + 1;
            while j < c.len() && c[j] != '"' { j += if c[j] == '\\' { 2 } else { 1 }; }
            if skip.is_none() && c[i + 1..j.min(c.len())].iter().collect::<String>() == LEGACY { hits.push(line); }
            line += newlines(i, j);
            i = j + 1;
            continue;
        }
        if c[i] == '\'' {
            // A char literal (`'{'`, `'\''`, `'\u{7d}'`) is skipped whole; a lifetime is one tick.
            if c.get(i + 1) == Some(&'\\') {
                let mut j = i + 3;
                while j < c.len() && c[j] != '\'' { j += 1; }
                i = j + 1;
            } else if c.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }
        if skip.is_none() && at(i, "#[cfg(test)]") {
            skip = Some((0, false));
            i += "#[cfg(test)]".len();
            continue;
        }
        if let Some((depth, seen)) = skip.as_mut() {
            match c[i] {
                '{' => { *depth += 1; *seen = true; }
                '}' => { *depth -= 1; if *seen && *depth == 0 { skip = None; } }
                ';' if !*seen && *depth == 0 => skip = None,
                _ => {}
            }
        }
        if c[i] == '\n' { line += 1; }
        i += 1;
    }
    hits
}

/// The 1-based lines holding `"quinn"` or `'quinn'` in a script, a migration or a config file.
fn text_hits(src: &str) -> Vec<usize> {
    let (dq, sq) = (format!("\"{LEGACY}\""), format!("'{LEGACY}'"));
    src.lines().enumerate().filter(|(_, l)| l.contains(&dq) || l.contains(&sq)).map(|(n, _)| n + 1).collect()
}

fn read(path: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap()).to_string()
}

/// `file:line: text` for every hit in the scanned tree, before the one allowance.
fn all_hits() -> Vec<String> {
    let mut out = Vec::new();
    let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
    for dir in ["engine/src", "app/src"] {
        for f in files(&root().join(dir), &rs) {
            let text = read(&f);
            let lines: Vec<&str> = text.lines().collect();
            out.extend(rust_hits(&text).into_iter().map(|n| format!("{}:{n}: {}", rel(&f), lines.get(n - 1).unwrap_or(&"").trim())));
        }
    }
    let js = |p: &Path| p.extension().is_some_and(|e| e == "js");
    let cloud = |p: &Path| !p.to_string_lossy().ends_with("_test.ts");
    for f in files(&root().join("app/static"), &js).into_iter().chain(files(&root().join("cloud/supabase"), &cloud)) {
        let text = read(&f);
        let lines: Vec<&str> = text.lines().collect();
        out.extend(text_hits(&text).into_iter().map(|n| format!("{}:{n}: {}", rel(&f), lines[n - 1].trim())));
    }
    out
}

fn is_the_constant(hit: &str) -> bool {
    hit.starts_with("engine/src/journal.rs:") && hit.ends_with(THE_CONSTANT)
}

#[test]
fn no_quinn_literal_outside_the_legacy_constant() {
    let stray: Vec<String> = all_hits().into_iter().filter(|h| !is_the_constant(h)).collect();
    assert!(
        stray.is_empty(),
        "the literal \"quinn\" outside journal::LEGACY_HUMAN_ACTOR (ruling 11; use journal::HUMAN_ACTOR, \
         journal::is_human or the vault's own token):\n{}",
        stray.join("\n")
    );
}

#[test]
fn the_scanner_is_not_vacuous() {
    let none: Vec<usize> = Vec::new();
    // A test module's literal is not counted; a plain function's is; `quinn-ops` never is.
    assert_eq!(rust_hits("#[cfg(test)]\nmod tests {\n    fn f() { let _ = \"quinn\"; }\n}\n"), none);
    assert_eq!(rust_hits("fn f() -> &'static str {\n    \"quinn\"\n}\n"), vec![2]);
    assert_eq!(rust_hits("fn f() { let _ = \"quinn-ops\"; let _ = \"set by quinn\"; }\n"), none);
    // Comments never count; a body-less test item ends at its `;` and scanning resumes after it.
    assert_eq!(rust_hits("// \"quinn\"\n/// \"quinn\"\n/* \"quinn\" */\n#[cfg(test)]\nstatic X: &str = \"quinn\";\nfn g() { h(\"quinn\") }\n"), vec![6]);
    // Braces inside chars, strings and raw strings do not end a skipped test module early.
    let tricky = "#[cfg(test)]\nmod t {\n    fn a() { let _ = '}'; let _ = \"}}\"; let _ = r#\"}\"#; let _: &'static str = \"\\\"}\"; }\n    fn b() { let _ = \"quinn\"; }\n}\nfn c() { let _ = \"quinn\"; }\n";
    assert_eq!(rust_hits(tricky), vec![6]);
    // A raw string holding exactly the token counts too.
    assert_eq!(rust_hits("fn d() { e(r\"quinn\") }\n"), vec![1]);
    // Scripts and the cloud: either quote.
    assert_eq!(text_hits("const a = 'quinn';\nconst b = \"quinn\";\nconst c = 'quinn-ops';\n"), vec![1, 2]);
}

#[test]
fn the_legacy_constant_is_there_exactly_once() {
    let journal = read(&root().join("engine/src/journal.rs"));
    let lines: Vec<&str> = journal.lines().collect();
    let in_journal: Vec<&str> = rust_hits(&journal).into_iter().map(|n| lines[n - 1].trim()).collect();
    assert_eq!(in_journal, [THE_CONSTANT], "journal.rs's only non-test literal is the legacy constant");
    assert_eq!(all_hits().iter().filter(|h| is_the_constant(h)).count(), 1);
    let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
    let definitions = ["engine/src", "app/src"]
        .iter()
        .flat_map(|d| files(&root().join(d), &rs))
        .map(|f| read(&f).matches("const LEGACY_HUMAN_ACTOR").count())
        .sum::<usize>();
    assert_eq!(definitions, 1, "one legacy constant, never renamed away or duplicated");
}
