//! The email lane's app half (spec `2026-09-30-email-forwarding-design.md`). Nothing here opens a
//! window or reaches a network.
//!
//! **T9.1: the kept profiles and `mail.json`** (spec test 32's session half, §5.4, D15). One kept
//! WebView2 profile per mailbox, `<data_dir>\mail-session-<address_id>`, and one throwaway
//! `<data_dir>\mail-setup` when the saved sign-in is off; both behind `is_session_dir`, the grades
//! rule. `mail.json` sits beside `settings.json`, never in it; its `turned_off` only grows (§13 Q11).
//! Every address id here is synthetic.
use knowlu::mail_session::{
    forget, forget_all, is_session_dir, session_dir, setup_dir, start_up_sweep, turn_off, MailState,
};
use serde_json::json;
use std::path::{Path, PathBuf};

const A: &str = "0b0e5b2c-1111-4aaa-8bbb-000000000001";
const B: &str = "0b0e5b2c-2222-4aaa-8bbb-000000000002";

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-mail-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A synthetic kept profile: a cookie file two folders down, as WebView2 lays one out.
fn profile(d: &Path, name: &str) {
    let net = d.join(name).join("Default").join("Network");
    std::fs::create_dir_all(&net).unwrap();
    std::fs::write(net.join("Cookies"), "synthetic").unwrap();
}

#[test]
fn is_session_dir_accepts_only_mail_session_ids_and_mail_setup_under_data_dir() {
    let d = temp("guard");
    assert_eq!(session_dir(&d, A), Some(d.join(format!("mail-session-{A}"))));
    assert_eq!(setup_dir(&d), d.join("mail-setup"));
    assert!(is_session_dir(&d, &session_dir(&d, A).unwrap()));
    assert!(is_session_dir(&d, &setup_dir(&d)));
    let long = "a".repeat(65);
    for bad in ["", "..", "../x", "a/b", "a\\b", "A-UPPER", "a.b", "a b", "x:y", long.as_str()] {
        assert_eq!(session_dir(&d, bad), None, "{bad:?} is not an address id");
    }
    let other = temp("guard-other");
    for wrong in [
        d.clone(), d.parent().unwrap().to_path_buf(), d.join("mail-session-"), d.join("mail-session"),
        d.join(format!("MAIL-SESSION-{A}")), d.join("MAIL-SETUP"), d.join("mail-setup-old"), d.join("lms-session"),
        d.join(format!("mail-session-{A}")).join("Default"), d.join("mail-setup").join("..").join("mail-setup"),
        d.join("x").join("..").join(format!("mail-session-{A}")), other.join(format!("mail-session-{A}")),
        other.join("mail-setup"),
    ] {
        assert!(!is_session_dir(&d, &wrong), "{}", wrong.display());
    }
    let rel = Path::new("relative");
    assert!(!is_session_dir(rel, &rel.join("mail-setup")), "a relative profile is refused");
    let up = d.join("..").join(d.file_name().unwrap());
    assert!(!is_session_dir(&up, &up.join("mail-setup")), "a data_dir with .. is refused");
}

#[test]
fn forget_deletes_only_that_mailboxs_profile() {
    let d = temp("forget");
    profile(&d, &format!("mail-session-{A}"));
    profile(&d, &format!("mail-session-{B}"));
    profile(&d, "mail-setup");
    profile(&d, "lms-session");
    std::fs::write(d.join("settings.json"), "{}").unwrap();
    assert_eq!(forget(&d, A), Ok(true));
    assert!(!d.join(format!("mail-session-{A}")).exists());
    for kept in [format!("mail-session-{B}"), "mail-setup".into(), "lms-session".into(), "settings.json".into()] {
        assert!(d.join(&kept).exists(), "{kept} survives forget");
    }
    assert_eq!(forget(&d, A), Ok(true), "nothing to forget is done, not a failure");
    assert!(forget(&d, "../lms-session").is_err(), "a bad id is refused");
    assert!(d.join("lms-session").exists());
    assert!(forget(Path::new("relative"), A).is_err(), "a relative profile is refused");
    assert!(MailState::load(&d).unwrap().pending_deletes.is_empty());
}

/// Holds a file inside `dir` open with no sharing, so Windows refuses to delete it.
#[cfg(windows)]
fn lock(dir: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new().read(true).share_mode(0).open(dir.join("Default").join("Network").join("Cookies")).unwrap()
}

#[cfg(windows)]
#[test]
fn a_failed_delete_is_recorded_and_retried_at_start_up() {
    let d = temp("retry");
    let name = format!("mail-session-{A}");
    profile(&d, &name);
    let held = lock(&d.join(&name));
    assert_eq!(forget(&d, A), Ok(false), "a held profile is not gone, and is recorded");
    assert_eq!(MailState::load(&d).unwrap().pending_deletes, vec![name.clone()]);
    assert_eq!(start_up_sweep(&d), Ok(vec![name.clone()]), "still held: still pending");
    assert_eq!(MailState::load(&d).unwrap().pending_deletes, vec![name.clone()]);
    drop(held);
    assert_eq!(start_up_sweep(&d), Ok(vec![]), "released: the retry deletes it");
    assert!(!d.join(&name).exists());
    assert!(MailState::load(&d).unwrap().pending_deletes.is_empty());
}

/// D15: a Settings write made from an older read (here, before sign-out's delete failed) never
/// drops the pending delete, and never switches the saved sign-in back.
#[cfg(windows)]
#[test]
fn a_settings_write_from_an_older_read_keeps_a_pending_delete() {
    let d = temp("stale");
    let name = format!("mail-session-{A}");
    profile(&d, &name);
    let older = MailState::load(&d).unwrap();
    let held = lock(&d.join(&name));
    assert_eq!(forget_all(&d), Ok(false), "a held profile is recorded");
    MailState::update(&d, |s| {
        s.keep_session = !older.keep_session;
        true
    })
    .unwrap();
    let s = MailState::load(&d).unwrap();
    assert_eq!(s.pending_deletes, vec![name.clone()], "the failed delete is still named");
    assert!(!s.keep_session);
    drop(held);
    assert_eq!(start_up_sweep(&d), Ok(vec![]));
}

/// `mail.json` has one write path, `MailState::update`, which reloads under the lock: no public
/// call can write a stale copy over `pending_deletes` or `keep_session`.
#[test]
fn mail_json_is_written_only_through_update() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("mail_session.rs")).unwrap();
    assert!(!src.contains("pub fn save"), "a public save writes whatever copy its caller holds");
}

/// D15, §5.4: an unreadable `mail.json` is a named state, never "nothing pending". A delete that
/// fails and cannot be recorded is an error, and the sweep says it cannot read the file.
#[cfg(windows)]
#[test]
fn an_unreadable_mail_json_is_named_never_read_as_clean() {
    let d = temp("corrupt");
    let name = format!("mail-session-{A}");
    profile(&d, &name);
    std::fs::write(d.join("mail.json"), "{").unwrap();
    let held = lock(&d.join(&name));
    assert!(forget(&d, A).is_err(), "held and unrecordable: an error, never a silent false");
    assert!(forget_all(&d).is_err());
    assert!(d.join(&name).exists());
    assert!(start_up_sweep(&d).is_err(), "an unreadable mail.json is never a clean sweep");
    drop(held);
    assert_eq!(forget(&d, A), Ok(true), "the delete itself stands");
    assert!(start_up_sweep(&d).is_err(), "still unreadable: still named");
    assert_eq!(std::fs::read_to_string(d.join("mail.json")).unwrap(), "{", "never overwritten");
}

#[test]
fn the_start_up_sweep_removes_a_leftover_mail_setup() {
    let d = temp("sweep");
    profile(&d, "mail-setup");
    profile(&d, &format!("mail-session-{A}"));
    profile(&d, "lms-session");
    assert_eq!(start_up_sweep(&d), Ok(vec![]));
    assert!(!d.join("mail-setup").exists());
    assert!(d.join(format!("mail-session-{A}")).exists(), "a kept profile is kept");
    assert!(d.join("lms-session").exists());
    assert_eq!(start_up_sweep(&d), Ok(vec![]), "nothing left is done");
}

#[test]
fn forget_all_removes_every_mail_profile_and_nothing_else() {
    let d = temp("forget-all");
    for name in [format!("mail-session-{A}"), format!("mail-session-{B}"), "mail-setup".into()] {
        profile(&d, &name);
    }
    let kept = ["lms-session", "mail-setup-old", "mail-sessions", "mail-session-Not.An.Id", "lu-session"];
    for name in kept {
        profile(&d, name);
    }
    for file in ["settings.json", "grades.json", "account.json"] {
        std::fs::write(d.join(file), "{}").unwrap();
    }
    turn_off(&d, A).unwrap();
    assert_eq!(forget_all(&d), Ok(true));
    for gone in [format!("mail-session-{A}"), format!("mail-session-{B}"), "mail-setup".into()] {
        assert!(!d.join(&gone).exists(), "{gone} is removed");
    }
    for name in kept.iter().copied().chain(["settings.json", "grades.json", "account.json", "mail.json"]) {
        assert!(d.join(name).exists(), "{name} survives forget_all");
    }
    assert_eq!(MailState::load(&d).unwrap().turned_off, vec![A.to_string()], "turned_off survives");
    assert!(forget_all(Path::new("relative")).is_err());
}

#[test]
fn turned_off_only_grows() {
    let d = temp("turned-off");
    turn_off(&d, A).unwrap();
    turn_off(&d, B).unwrap();
    turn_off(&d, A).unwrap();
    assert_eq!(MailState::load(&d).unwrap().turned_off, vec![A.to_string(), B.to_string()]);
    // Every write reloads under the lock, so a Settings change made from an older read drops neither.
    MailState::update(&d, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    let s = MailState::load(&d).unwrap();
    assert_eq!(s.turned_off, vec![A.to_string(), B.to_string()]);
    assert!(!s.keep_session, "the rest of the save lands");
    assert!(s.is_turned_off(A) && s.is_turned_off(B) && !s.is_turned_off("0b0e5b2c"));
    assert!(turn_off(&d, "../x").is_err(), "a bad id is refused, never written");
    // An unreadable mail.json is never read as "nothing turned off", and never overwritten.
    std::fs::write(d.join("mail.json"), "{").unwrap();
    assert!(MailState::load(&d).is_err());
    assert!(turn_off(&d, A).is_err());
    assert!(MailState::update(&d, |s| {
        *s = MailState::default();
        true
    })
    .is_err());
    assert_eq!(std::fs::read_to_string(d.join("mail.json")).unwrap(), "{");
}

#[test]
fn mail_json_is_its_own_file_and_settings_json_is_untouched() {
    let d = temp("json");
    std::fs::write(d.join("settings.json"), "SETTINGS, UNTOUCHED").unwrap();
    let fresh = MailState::load(&d).unwrap();
    assert_eq!(fresh, MailState::default(), "missing is the default");
    assert!(fresh.keep_session, "the saved sign-in is on by default (D15)");
    MailState::update(&d, |s| s.last_reconcile.insert(A.into(), "2026-10-02T12:00:00Z".into()).is_none()).unwrap();
    turn_off(&d, B).unwrap();
    let text = std::fs::read_to_string(d.join("mail.json")).unwrap();
    let want = json!({
        "keep_session": true, "last_reconcile": { A: "2026-10-02T12:00:00Z" }, "pending_deletes": [], "turned_off": [B],
    });
    assert_eq!(text, knowlu_engine::ledger::dumps_value(&want));
    assert_eq!(std::fs::read_to_string(d.join("settings.json")).unwrap(), "SETTINGS, UNTOUCHED");
    // A file written before a field existed reads that field as its default.
    std::fs::write(d.join("mail.json"), r#"{"turned_off": []}"#).unwrap();
    assert_eq!(MailState::load(&d).unwrap(), MailState::default());
    for bad in ["", "[]", "null", r#"{"keep_session": "yes"}"#, r#"{"turned_off": "x"}"#] {
        std::fs::write(d.join("mail.json"), bad).unwrap();
        assert!(MailState::load(&d).is_err(), "{bad:?}");
    }
}
