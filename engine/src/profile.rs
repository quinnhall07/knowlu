//! The student's profile (M2 spec §6.3, §7.6): `profile/preferences.md` and `profile/interests.md`
//! read into plain data and written through `write`, never parsed and re-dumped.
//!
//! Off the contract list, and it holds no vault-byte guard of its own: the block-list refusal is
//! `write::write_one_line_literals`'s and the body edit is `write::set_body`'s, so a mistake here
//! refuses an edit or mis-cleans a list but cannot orphan list lines. The records these writes make
//! stay on this computer (D13, `sync::build_push`).

use std::path::Path;

use serde::Serialize;
use serde_yaml_ng::Value;

use crate::events::{load_interests, Interests};
use crate::journal::Journal;
use crate::pystr;
use crate::write::{self, WriteContext, WriteError, WriteOpts};

/// The preferences note, vault-relative and POSIX, as `set_body` takes it as a target.
const PREFERENCES: &str = "profile/preferences.md";
/// The interests note, vault-relative and POSIX, as `write_one_line_literals` takes it as a target.
const INTERESTS: &str = "profile/interests.md";

/// `profile/preferences.md` as Knowlu creates it (CF1): empty, so the body the student's first save
/// writes is the whole file and no frontmatter is invented (spec §6.3, D12).
pub const PREFERENCES_TEXT: &str = "";
/// `profile/interests.md` as Knowlu creates it, byte-equal to spec §6.3: four empty flow lists.
pub const INTERESTS_TEXT: &str = "---\nstrong: []\nmild: []\nnever: []\nclubs: []\n---\n";

/// The four interest keys, in the order `set_interests` writes them.
const KEYS: [&str; 4] = ["strong", "mild", "never", "clubs"];

/// What the settings page shows of the student's profile. Plain data; reading it writes nothing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Profile {
    /// The preferences body as the drawer shows it (`split_frontmatter`'s), the `expected` a save
    /// sends back. Empty when the file is absent or unreadable.
    pub preferences: String,
    pub strong: Vec<String>,
    pub mild: Vec<String>,
    pub never: Vec<String>,
    pub clubs: Vec<String>,
    /// False when any of the four lists continues past its own line (a block list): the page
    /// shows them read-only and the edit would be refused by name (D10).
    pub interests_editable: bool,
    /// An unreadable file is a warning here, never an error.
    pub warnings: Vec<String>,
}

/// Read `profile/preferences.md` and `profile/interests.md` from `vault`. The lists come from
/// `events::load_interests`, so the app and the event filter read the same lists, and editability
/// from `write::value_spans_lines`, the one detector.
pub fn read(vault: &Path) -> Profile {
    let mut warnings = Vec::new();
    let prefs_path = vault.join(PREFERENCES);
    let preferences = if prefs_path.exists() {
        match pystr::read_text(&prefs_path).map(|t| crate::models::split_frontmatter(&t)) {
            Ok(Ok((_, body))) => body,
            Ok(Err(err)) => {
                warnings.push(format!("preferences unreadable: {err:?}"));
                String::new()
            }
            Err(err) => {
                warnings.push(format!("preferences unreadable: {err}"));
                String::new()
            }
        }
    } else {
        String::new()
    };

    let interests_path = vault.join(INTERESTS);
    let (interests, more) = load_interests(&interests_path);
    warnings.extend(more);
    let interests_editable = if interests_path.exists() {
        match pystr::read_text(&interests_path) {
            Ok(text) => !KEYS.iter().any(|key| write::value_spans_lines(&text, key)),
            Err(_) => false,
        }
    } else {
        true
    };
    let Interests { strong, mild, never, clubs } = interests;
    Profile { preferences, strong, mild, never, clubs, interests_editable, warnings }
}

/// Create `profile/<name>.md` through `write::create_profile_file` when it is absent. A file
/// already there (a dangling link included) is left alone.
fn ensure(vault: &Path, name: &str, text: &str, ctx: &WriteContext, journal: &mut Journal) -> Result<(), WriteError> {
    if vault.join("profile").join(format!("{name}.md")).symlink_metadata().is_ok() {
        return Ok(());
    }
    write::create_profile_file(vault, name, text, ctx, journal).map(|_| ())
}

/// Replace the preferences text: create the file when absent, then `write::set_body` with
/// `expected`, the body the page last read. `Ok(false)` is a no-op; a stale `expected` is
/// `WriteError::Conflict`. No text enters the journal record.
pub fn set_preferences(
    vault: &Path,
    expected: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<bool, WriteError> {
    ensure(vault, "preferences", PREFERENCES_TEXT, ctx, journal)?;
    write::set_body(vault, PREFERENCES, expected, text, ctx, journal)
}

/// One list, cleaned: items trimmed, empty items and exact duplicates dropped (the first kept). An
/// item the one-line surgery cannot carry is refused by name, `key` first.
fn clean(key: &str, items: &[String]) -> Result<Vec<String>, WriteError> {
    let mut out: Vec<String> = Vec::new();
    for item in items.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Some(why) = write::single_line_problem(item) {
            return Err(WriteError::Io(format!("{key}: {item:?} {why}")));
        }
        if !out.iter().any(|seen| seen == item) {
            out.push(item.to_string());
        }
    }
    Ok(out)
}

/// Write the four lists as one-line flow lists through `write::write_one_line_literals`, which
/// refuses a block list by name before any record. Every item is checked before the file is created
/// or anything is journalled. The literals come from `write::to_literal`, so the one emitter writes
/// them. Last writer wins: only the student writes these lists.
pub fn set_interests(
    vault: &Path,
    lists: &Interests,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<(), WriteError> {
    let cleaned = [
        clean(KEYS[0], &lists.strong)?,
        clean(KEYS[1], &lists.mild)?,
        clean(KEYS[2], &lists.never)?,
        clean(KEYS[3], &lists.clubs)?,
    ];
    ensure(vault, "interests", INTERESTS_TEXT, ctx, journal)?;
    let literals: Vec<(String, String)> = KEYS
        .iter()
        .zip(cleaned)
        .map(|(key, items)| {
            let list = Value::Sequence(items.into_iter().map(Value::String).collect());
            (key.to_string(), write::to_literal(&list))
        })
        .collect();
    write::write_one_line_literals(vault, INTERESTS, &literals, ctx, journal, &WriteOpts::default())
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Interests;
    use crate::journal::Journal;
    use crate::write::{WriteContext, WriteError};
    use std::path::{Path, PathBuf};

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("qo-profile-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("state").join("journal")).unwrap();
        crate::journal::create_actor_file(&dir, crate::journal::HUMAN_ACTOR).unwrap();
        dir
    }

    fn copy_tree(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap().flatten() {
            let target = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    /// A private scratch copy of `vault-full`; the fixture itself is only ever read.
    fn fixture_copy() -> PathBuf {
        let dir = scratch("fixture");
        let _ = std::fs::remove_dir_all(&dir);
        copy_tree(Path::new("tests/fixtures/vault-full"), &dir);
        dir
    }

    /// The vault's own human context: the token is read from the vault, never written as a literal.
    fn ctx(v: &Path) -> WriteContext {
        WriteContext::new(crate::journal::read_human_actor(v).unwrap(), "dashboard")
    }

    fn fingerprint(v: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() { walk(&path, out) } else { out.push((path.clone(), std::fs::read(&path).unwrap())) }
            }
        }
        let mut out = Vec::new();
        walk(v, &mut out);
        out.sort();
        out
    }

    fn lists(strong: &[&str], mild: &[&str], never: &[&str], clubs: &[&str]) -> Interests {
        let own = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Interests { strong: own(strong), mild: own(mild), never: own(never), clubs: own(clubs) }
    }

    fn records(v: &Path) -> Vec<crate::ledger::Record> {
        Journal::new(v).read(None, None)
    }

    #[test]
    fn profile_read_of_an_absent_profile_is_empty_and_editable() {
        let v = scratch("absent");
        let before = fingerprint(&v);
        let p = read(&v);
        assert_eq!(p.preferences, "");
        assert_eq!((p.strong, p.mild, p.never, p.clubs), (vec![], vec![], vec![], vec![]));
        assert!(p.interests_editable, "an absent file is created in flow style, so it is editable");
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        assert_eq!(fingerprint(&v), before, "read writes nothing");
    }

    #[test]
    fn profile_read_of_the_fixture_reads_the_lists_and_marks_them_not_editable() {
        let v = fixture_copy();
        let before = fingerprint(&v);
        let p = read(&v);
        assert_eq!(
            p.strong,
            ["undergraduate research opportunities", "CS, AI, and engineering talks or workshops", "career fairs, co-op and internship info sessions"]
        );
        assert_eq!(p.mild, ["free food", "entrepreneurship and startup events"]);
        assert_eq!(p.never, ["greek life recruitment", "alumni and donor events"]);
        assert_eq!(p.clubs, ["Artificial Intelligence Club"]);
        assert!(!p.interests_editable, "the fixture writes block lists");
        assert_eq!(p.preferences, "", "the fixture has no preferences file");
        assert_eq!(fingerprint(&v), before, "read writes nothing");
    }

    #[test]
    fn set_preferences_creates_then_edits() {
        let v = scratch("prefs");
        let (ctx, mut j) = (ctx(&v), Journal::new(&v));
        let first = "Mornings are best.\nNo work after 9pm.";
        assert_eq!(set_preferences(&v, "", first, &ctx, &mut j), Ok(true));
        let path = v.join("profile").join("preferences.md");
        let text = crate::pystr::read_text(&path).unwrap();
        assert!(!text.starts_with("---"), "no frontmatter is added: {text:?}");
        assert_eq!(read(&v).preferences, "Mornings are best.\nNo work after 9pm.\n");
        let recs = records(&v);
        assert_eq!(recs.len(), 2, "{recs:?}");
        assert_eq!((recs[0]["op"].as_str(), recs[0]["path"].as_str()), (Some("create"), Some("profile/preferences.md")));
        assert_eq!((recs[1]["op"].as_str(), recs[1]["path"].as_str()), (Some("set_body"), Some("profile/preferences.md")));
        assert!(recs.iter().all(|r| !crate::ledger::dumps_value(&serde_json::Value::Object(r.clone())).contains("Mornings")), "no text enters a record");
        assert!(recs.iter().all(|r| r["actor"] == ctx.actor.as_str()));

        let shown = read(&v).preferences;
        assert_eq!(set_preferences(&v, &shown, "Evenings are best.", &ctx, &mut j), Ok(true));
        assert_eq!(read(&v).preferences, "Evenings are best.\n");
        assert_eq!(set_preferences(&v, "Evenings are best.\n", "Evenings are best.", &ctx, &mut j), Ok(false), "a no-op");

        // A stale `expected` is a named Conflict, and nothing moves.
        let before = fingerprint(&v);
        let err = set_preferences(&v, "Mornings are best.\n", "Lost.", &ctx, &mut j).unwrap_err();
        assert_eq!(err, WriteError::Conflict("profile/preferences.md".to_string()));
        assert_eq!(fingerprint(&v), before);
    }

    #[test]
    fn set_interests_writes_one_line_lists_that_load_interests_reads() {
        let v = scratch("interests");
        let (ctx, mut j) = (ctx(&v), Journal::new(&v));
        let want = lists(
            &["  undergraduate research ", "AI talks", "", "AI talks", "   "],
            &["free food"],
            &[],
            &["AI Club", "Chess, club", "AI Club"],
        );
        set_interests(&v, &want, &ctx, &mut j).unwrap();
        let path = v.join("profile").join("interests.md");
        let text = crate::pystr::read_text(&path).unwrap();
        assert!(text.contains("\nstrong: [undergraduate research, AI talks]\n"), "{text:?}");
        assert!(text.contains("\nnever: []\n"), "{text:?}");
        let (got, warnings) = crate::events::load_interests(&path);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(got, lists(&["undergraduate research", "AI talks"], &["free food"], &[], &["AI Club", "Chess, club"]));
        let p = read(&v);
        assert_eq!((p.strong, p.clubs), (got.strong, got.clubs));
        assert!(p.interests_editable);
        // The `create` record, then one `set` per key that changed (never did not).
        let recs = records(&v);
        let ops: Vec<_> = recs.iter().map(|r| (r["op"].as_str().unwrap(), r["field"].as_str())).collect();
        assert_eq!(ops, [("create", None), ("set", Some("strong")), ("set", Some("mild")), ("set", Some("clubs"))]);

        // A body the student wrote is never touched.
        let mut with_body = text.clone();
        with_body.push_str("\nMy own notes.\n");
        crate::pystr::write_text(&path, &with_body).unwrap();
        set_interests(&v, &lists(&["research"], &["free food"], &[], &[]), &ctx, &mut j).unwrap();
        let after = crate::pystr::read_text(&path).unwrap();
        assert!(after.ends_with("---\n\nMy own notes.\n"), "{after:?}");
        assert!(after.contains("\nstrong: [research]\n") && after.contains("\nclubs: []\n"), "{after:?}");
    }

    #[test]
    fn set_interests_refuses_an_item_that_is_not_one_line_by_name_before_any_record() {
        for (item, key) in [("a\nb", "mild"), ("---", "clubs"), ("two\r\nlines", "never")] {
            let v = scratch("badtext");
            let (ctx, mut j) = (ctx(&v), Journal::new(&v));
            let before = fingerprint(&v);
            let mut want = lists(&["fine"], &[], &[], &[]);
            match key {
                "mild" => want.mild.push(item.to_string()),
                "never" => want.never.push(item.to_string()),
                _ => want.clubs.push(item.to_string()),
            }
            let err = set_interests(&v, &want, &ctx, &mut j).unwrap_err();
            assert!(err.to_string().contains(key), "{item:?}: the error names the key: {err}");
            assert_eq!(fingerprint(&v), before, "{item:?}: not even the file's create record");
        }
    }

    #[test]
    fn set_interests_refuses_a_block_list_before_any_record() {
        let v = fixture_copy();
        let (ctx, mut j) = (ctx(&v), Journal::new(&v));
        let before = fingerprint(&v);
        let err = set_interests(&v, &lists(&["new"], &[], &[], &[]), &ctx, &mut j).unwrap_err();
        assert_eq!(err, WriteError::MultiLine("strong".to_string()));
        assert_eq!(fingerprint(&v), before, "the journal and the file are unchanged");
    }
}
