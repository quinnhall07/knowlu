use std::path::{Path, PathBuf};
use serde_json::json;
use knowlu::state::{resolve_vault, ConsoleState};
use knowlu::commands::{
    backup_now_inner, close_info_inner, console_ctx, create_task_inner, decide_inner,
    delete_note_inner, get_settings_inner, mark_seen_inner, note_inner, open_issue_inner,
    resolve_issue_inner, set_fields_inner, set_settings_inner, state_inner, sync_inner,
    ui_event_inner,
};
use knowlu::scheduler::{lock, LiveSlot, RunSummary, Scheduler};

/// Every journal record across `state/journal/*.jsonl`, parsed. Files are CRLF (translate on
/// read per the repo's line-ending rule) and one JSON object per line.
fn journal_records(v: &Path) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(v.join("state/journal")).unwrap().flatten() {
        let text = std::fs::read_to_string(e.path()).unwrap().replace("\r\n", "\n");
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            out.push(serde_json::from_str(line).unwrap());
        }
    }
    out
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qo-console-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(Path::new("../engine/tests/fixtures/vault-full"), &dir);
    dir
}
fn copy(from: &Path, to: &Path) { std::fs::create_dir_all(to).unwrap(); for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy(&p, &t); } else { std::fs::copy(&p, &t).unwrap(); } } }

#[test]
fn resolve_vault_prefers_the_flag_then_a_vault_shaped_cwd_then_refuses_by_name() {
    let v = scratch("resolve");
    assert_eq!(resolve_vault(Some(v.to_str().unwrap()), Path::new("C:\\nowhere")).unwrap(), v);
    assert_eq!(resolve_vault(None, &v).unwrap(), v);
    let err = resolve_vault(None, &std::env::temp_dir()).unwrap_err();
    assert!(err.contains("--vault") && err.contains("config/planning.yaml"), "{err}");
}

#[test]
fn state_returns_the_envelope_with_the_read_model_and_never_writes() {
    let v = scratch("state");
    let before = std::fs::read(v.join("tasks").join("ph-106-exam-1-prep.md")).unwrap();
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join("qo-console-appdata"));
    let env = state_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true);
    assert_eq!(env["state"]["schema"], 1);
    assert_eq!(env["state"]["verdict"]["active"], 5);
    assert!(env["state"]["topline"]["console_build"].is_string());
    assert_eq!(std::fs::read(v.join("tasks").join("ph-106-exam-1-prep.md")).unwrap(), before);
    let bad = state_inner(&cs, "tomorrow").unwrap();
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("tomorrow"));
}

/// D7 / §6: between Finish and the first `rank` a vault has no read model, and the window painted
/// nothing for about a minute — a white page with an error line in it. The envelope now carries a
/// block that says so, with the steps the slot has finished, and the page has a sentence for it.
#[test]
fn a_vault_with_no_read_model_yet_carries_the_first_run_block() {
    let v = scratch("firstrun");
    assert!(!v.join("state").join("today.md").exists(), "this vault has never been ranked");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-console-appdata-firstrun-{}", std::process::id())));
    // R-C1c-plan-1, the premise tested rather than assumed: the read model builds fine without
    // `state/today.md` (`surface::build_state` has no failure path), so `ok` is true and the BLOCK
    // — not a failed envelope — is what the page keys on.
    let env = state_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true);
    let sch = Scheduler::default();
    let fr = knowlu::commands::first_run_value(&cs, &sch).expect("a vault with no today.md carries it");
    assert_eq!(fr["running"], false);
    // No slot has run yet: an empty list, never an invented step.
    assert_eq!(fr["steps"], json!([]));

    *lock(&sch.last) = Some(RunSummary {
        started: "2026-09-22T19:14:37Z".into(),
        ended: "2026-09-22T19:15:38Z".into(),
        steps: vec![("coursework".into(), 0), ("judge (skipped: no entitlement)".into(), 0)],
        ok: true,
        engine_ok: true,
        late: false,
        reason: None,
        attempts: 1,
    });
    let fr = knowlu::commands::first_run_value(&cs, &sch).expect("still no read model");
    assert_eq!(fr["steps"][1][0], "judge (skipped: no entitlement)");
    assert_eq!(fr["steps"][1][1], 0);

    // …and the moment `rank` has written the day, the block is gone and the page paints normally.
    std::fs::create_dir_all(v.join("state")).unwrap();
    std::fs::write(v.join("state").join("today.md"), b"# Today\n").unwrap();
    assert!(knowlu::commands::first_run_value(&cs, &sch).is_none());
}

/// R-C1c-8: while a slot runs, the block carries what `Scheduler.live` has published (the steps
/// that have landed and the one in progress), never `Scheduler.last`, which belongs to an earlier
/// slot and is written only when a slot ends. With no slot running it is `last`'s steps, as before,
/// and nothing is in progress.
#[test]
fn the_first_run_block_reads_the_live_slot_while_one_runs_and_the_last_one_after() {
    let v = scratch("firstrun-live");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-console-appdata-firstrun-live-{}", std::process::id())));
    let sch = Scheduler::default();
    *lock(&sch.last) = Some(RunSummary {
        started: "2026-09-22T19:14:37Z".into(),
        ended: "2026-09-22T19:15:38Z".into(),
        steps: vec![("coursework".into(), 0), ("rank".into(), 1)],
        ok: false,
        engine_ok: false,
        late: false,
        reason: None,
        attempts: 1,
    });
    *lock(&sch.live) = LiveSlot { steps: vec![("coursework".into(), 0)], current: Some("judge".into()) };
    *lock(&sch.running) = true;
    let fr = knowlu::commands::first_run_value(&cs, &sch).expect("no read model yet");
    assert_eq!(fr["running"], true);
    assert_eq!(fr["current"], "judge", "the step in progress: {fr}");
    assert_eq!(fr["steps"], json!([["coursework", 0]]), "the live steps, not the last slot's: {fr}");

    *lock(&sch.running) = false;
    let fr = knowlu::commands::first_run_value(&cs, &sch).expect("still no read model");
    assert_eq!(fr["running"], false);
    assert_eq!(fr.get("current"), Some(&serde_json::Value::Null), "nothing is in progress: {fr}");
    assert_eq!(fr["steps"], json!([["coursework", 0], ["rank", 1]]), "the ended slot's steps: {fr}");
    let _ = std::fs::remove_dir_all(&v);
}

/// The C1c Task 5 live proof (2026-09-23) found no first-run line on screen, and the test above
/// only reaches `first_run_value`. This drives the whole envelope the `state` command hands the
/// page (`commands::state_envelope`, which the `#[tauri::command]` is one call to) over a vault made
/// the way the wizard makes one (`scaffold::create_vault`: config, the seed task, nothing ranked),
/// with a slot in flight as there was live. `attach_scheduler` runs after the block is attached;
/// this proves it leaves the block alone and still fills the topline.
#[test]
fn the_state_command_carries_the_first_run_block_on_a_wizard_made_vault() {
    let parent = std::env::temp_dir().join(format!("qo-console-firstrun-envelope-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let v = parent.join("Fall 2026");
    let plan = knowlu::scaffold::VaultPlan {
        profile_id: knowlu::profiles::id_for(&v),
        ics_url: None,
        personal_calendar: None,
        google_calendar: false,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into(), "18:00".into()],
        device: "M".into(),
        campus: "none".into(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        api_base: knowlu::account::api_base(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    };
    knowlu::scaffold::create_vault(&v, &plan).unwrap();
    assert!(!v.join("state").join("today.md").exists(), "a wizard-made vault has never been ranked");
    let cs = ConsoleState::open(v.clone(), parent.join("appdata"));
    let sch = Scheduler::default();
    *lock(&sch.running) = true;

    let env = knowlu::commands::state_envelope(&cs, &sch, "today");
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["first_run"]["running"], true, "{env}");
    assert_eq!(env["first_run"]["steps"], json!([]), "a slot in flight has recorded nothing yet");
    // R-C1c-8's shape: the step in progress rides on the block too, null until a step starts.
    assert_eq!(env["first_run"].get("current"), Some(&serde_json::Value::Null), "{env}");
    assert!(env["state"]["must_do"].is_object(), "the day is built and painted behind the block");
    assert!(env["state"]["topline"]["scheduler"].is_object(), "attach_scheduler still ran: {env}");

    // `rank` writes the day: the block stops coming, and nothing else about the envelope changes.
    std::fs::write(v.join("state").join("today.md"), b"# Today\n").unwrap();
    let env = knowlu::commands::state_envelope(&cs, &sch, "today");
    assert!(env.get("first_run").is_none(), "{env}");
    assert_eq!(env["ok"], true);
    let _ = std::fs::remove_dir_all(&parent);
}

#[test]
fn note_finds_a_note_by_id_and_mark_seen_moves_the_delta_window() {
    let v = scratch("note");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-console-appdata-{}", std::process::id())));
    let s = state_inner(&cs, "today").unwrap();
    let id = s["state"]["must_do"]["groups"][0]["rows"][0]["id"].as_str().unwrap().to_string();
    let n = note_inner(&cs, &id).unwrap();
    assert_eq!(n["ok"], true);
    assert_eq!(n["note"]["folder"], "tasks");
    let m = mark_seen_inner(&cs).unwrap();
    assert_eq!(m["ok"], true);
    let after = state_inner(&cs, "today").unwrap();
    assert_eq!(after["state"]["delta"]["since_kind"], "seen");
}

#[test]
fn settings_round_trip_and_the_profile_id_is_derived_not_named() {
    use knowlu::state::Settings;
    let d = std::env::temp_dir().join(format!("qo-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap();
    let p = d.join("settings.json");
    let (s, err) = Settings::load(&p, std::path::Path::new(r"C:\anywhere\vault"));
    assert!(err.is_none(), "a missing settings file is not an error: {err:?}");
    assert!(s.profile_id.starts_with("profile_") && s.profile_id.len() == "profile_".len() + 10, "{}", s.profile_id);
    assert!(s.backup_dir.is_none() && s.autostart && s.quit_at.is_none());
    let mut s2 = s.clone(); s2.backup_dir = Some(d.join("bk")); s2.save(&p).unwrap();
    let (s3, err2) = Settings::load(&p, std::path::Path::new(r"C:\anywhere\vault"));
    assert!(err2.is_none(), "{err2:?}");
    assert_eq!(s3, s2);
}

#[test]
fn a_corrupt_settings_file_reports_an_error_and_falls_back_to_defaults() {
    use knowlu::state::ConsoleState;
    let data = std::env::temp_dir().join(format!("qo-corrupt-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data); std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("settings.json"), "{not json").unwrap();
    let v = scratch("corrupt-settings");
    let cs = ConsoleState::open(v.clone(), data);
    let err = cs.settings_error.as_ref().expect("a corrupt settings.json must report an error");
    assert!(err.contains("settings.json"), "{err}");
    let s = cs.settings.lock().unwrap();
    assert_eq!(s.profile_id, knowlu_engine::ids::derived_id("profile", &v.to_string_lossy()));
    assert!(s.backup_dir.is_none() && s.autostart && s.quit_at.is_none());
}

/// Fix round 1, review M4. `last_error` alone leaves a lapsed subscription or a signed-out machine
/// invisible in a support report — a skip carries no `last_error`, and `HistoryStatus` (Task 10's
/// predecessor) had no skip state to have missed in the first place.
#[test]
fn diagnostics_text_names_a_skipped_sync_as_well_as_a_failed_one() {
    use knowlu::{state::ConsoleState, tray::diagnostics_text};
    let v = scratch("diag-skip");
    // Final review M3: a PID-keyed folder a reused PID would find again (`cea59a9`'s pattern) —
    // start clean and leave nothing behind.
    let data = std::env::temp_dir().join(format!("qo-diag-skip-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let cs = ConsoleState::open(v.clone(), data.clone());
    *cs.sync.lock().unwrap() = knowlu_engine::sync::SyncStatus {
        ok: false,
        at: Some(knowlu_engine::journal::now_ts(None)),
        lines: vec![],
        last_error: None,
        skipped: Some("no entitlement".to_string()),
    };
    let t = diagnostics_text(&cs);
    assert!(t.contains("sync skipped: no entitlement"), "a lapsed subscription must be visible in the diagnostics blob: {t}");
    drop(cs);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn diagnostics_text_carries_no_note_title() {
    use knowlu::{state::ConsoleState, tray::diagnostics_text};
    let v = scratch("diag");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-diag-data-{}", std::process::id())));

    // Plant an `end` run record whose summary carries a real note *filename* the way the engine
    // actually writes one (`src/cli.rs`'s `an_unreadable_note_is_counted_and_named_in_the_summary`:
    // `"... 1 unreadable: bad.md"`) — the fixture's own run records (none, in vault-full) can never
    // exercise this path, so the redaction guarantee has to be tested against a planted one.
    let mut record = serde_json::json!({
        "run_id": "local-diagnostics-redaction-test",
        "phase": "end",
        "runner": "local",
        "ts": knowlu_engine::journal::now_ts(None),
        "device": "test",
        "result": "WARN",
        "summary": "today.md refreshed (5 active; 1 unreadable: ph-106-exam-1-prep.md)",
        "git_after": serde_json::Value::Null,
        "steps": [],
        "journal_records": 0,
        "seq": 999999999i64
    }).as_object().unwrap().clone();
    knowlu_engine::runs::Runs::new(&v).append(&mut record).unwrap();

    let t = diagnostics_text(&cs);
    assert!(t.contains("Knowlu") && t.contains("build "), "{t}");
    assert!(!t.contains("PH 106"), "a task title leaked into diagnostics: {t}");
    assert!(t.contains("<note>"), "the planted run summary's note filename must be redacted: {t}");
    assert!(!t.contains("ph-106"), "a note filename leaked into diagnostics: {t}");
}

#[test]
fn set_fields_journals_a_quinn_dashboard_record_and_returns_fresh_state() {
    let v = scratch("set");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-set-data-{}", std::process::id())));
    let id = first_id(&cs);
    let mut f = serde_json::Map::new(); f.insert("importance".into(), json!(1)); f.insert("title".into(), json!("Exam 1: prep, part 2"));
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    let text = std::fs::read_to_string(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    assert!(text.contains("importance: 1") && (text.contains("title: 'Exam 1: prep, part 2'") || text.contains("title: \"Exam 1: prep, part 2\"")), "{text}");
    let journal = std::fs::read_dir(v.join("state/journal")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect::<String>();
    assert!(journal.contains("\"actor\": \"quinn\"") && journal.contains("\"via\": \"dashboard\""));
    assert_eq!(env["state"]["schema"], 1);
    assert!(knowlu_engine::journal::VIAS.contains(&"dashboard"));
}

#[test]
fn a_non_editable_field_is_refused_with_the_current_state_and_nothing_written() {
    let v = scratch("refuse");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-refuse-data-{}", std::process::id())));
    let id = first_id(&cs);
    let before = std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    let mut f = serde_json::Map::new(); f.insert("source_uid".into(), json!("x"));
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], false); assert!(env["error"].as_str().unwrap().contains("source_uid"));
    assert_eq!(env["state"]["schema"], 1, "a refusal still carries the current state");
    assert_eq!(before, std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap());
}

#[test]
fn progress_100_and_status_done_land_in_one_write() {
    let v = scratch("done");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-done-data-{}", std::process::id())));
    let id = first_id(&cs);
    let mut f = serde_json::Map::new(); f.insert("progress".into(), json!(100)); f.insert("status".into(), json!("done"));
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], true);
    assert!(env["state"]["closed_this_week"].as_array().unwrap().iter().any(|c| c["id"] == json!(id)), "one line in CLOSED THIS WEEK");
}

#[test]
fn create_task_defaults_and_slug_and_delete_archives() {
    let v = scratch("create");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-create-data-{}", std::process::id())));
    let mut f = serde_json::Map::new(); f.insert("title".into(), json!("Read chapter 3: waves")); f.insert("course".into(), json!("ph-106")); f.insert("due".into(), json!("2026-09-10"));
    let env = create_task_inner(&cs, "today", f).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    let path = v.join("tasks/read-chapter-3-waves.md");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("due: 2026-09-10T23:59") && text.contains("effort_hours: 1.0") && text.contains("importance: 3") && text.contains("status: active") && text.contains("created_by: quinn") && text.contains("id: task_"), "{text}");
    let id = text.lines().find_map(|l| l.strip_prefix("id: ")).unwrap().trim().to_string();
    let env = delete_note_inner(&cs, "today", &id).unwrap();
    assert_eq!(env["ok"], true);
    assert!(!path.exists() && v.join("archive/read-chapter-3-waves.md").exists(), "nothing is unlinked");
    let mut f = serde_json::Map::new(); f.insert("title".into(), json!("   "));
    assert_eq!(create_task_inner(&cs, "today", f).unwrap()["ok"], false, "title is required");
}

#[test]
fn approving_a_task_proposal_puts_the_task_in_the_returned_state() {
    let v = scratch("decide");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-decide-data-{}", std::process::id())));
    // `task-freshman-forum.md` carries a frozen `expires: 2026-09-30` — pin "today" the same way
    // `rejecting_and_snoozing_write_the_decision_fields` does, or this test starts failing from
    // 2026-10-01 when `read_approvals` drops the card the same way it dropped the amend card here.
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let s = state_inner(&cs, "decisions").unwrap();
    let card = s["state"]["decisions"]["cards"].as_array().unwrap().iter().find(|c| c["kind"] == json!("task")).expect("vault-full holds a pending task proposal");
    let approval_id = card["id"].as_str().unwrap().to_string();
    let before = s["state"]["verdict"]["active"].as_i64().unwrap();
    let env = decide_inner(&cs, "today", &approval_id, "approved", "looks right", None).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["decision"]["executed"].as_array().unwrap().len(), 1);
    assert_eq!(env["state"]["verdict"]["active"].as_i64().unwrap(), before + 1, "the approved task is in the ranked list NOW, not six hours later");
    assert!(!env["state"]["decisions"]["cards"].as_array().unwrap().iter().any(|c| c["id"] == json!(approval_id)));

    // R-T9: the decision (status) is Quinn's; execution (materializing the task) is the system
    // acting on that decision, not a fresh human judgement — else `write::write_literals`'s
    // judge-once freeze would treat every field of the new task as hand-set forever.
    let stem = env["decision"]["executed"][0].as_str().unwrap();
    let task_text = std::fs::read_to_string(v.join("tasks").join(format!("{stem}.md"))).unwrap();
    let task_id = task_text.lines().find_map(|l| l.strip_prefix("id: ")).unwrap().trim().to_string();
    let records = journal_records(&v);
    let create_rec = records.iter().find(|r| r["op"] == "create" && r["id"] == json!(task_id)).expect("a create record for the materialized task");
    assert_eq!(create_rec["actor"], "agent:approvals", "{create_rec}");
    assert_eq!(create_rec["via"], "dashboard", "{create_rec}");
    let status_rec = records.iter().find(|r| r["op"] == "set" && r["field"] == "status" && r["id"] == json!(approval_id)).expect("a set record for the approval's status field");
    assert_eq!(status_rec["actor"], "quinn", "{status_rec}");
}

#[test]
fn rejecting_and_snoozing_write_the_decision_fields() {
    let v = scratch("snooze");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-snooze-data-{}", std::process::id())));
    // `vault-full`'s two approvals carry frozen `expires:` dates (2026-09-20, 2026-09-30) — the
    // fixture is never regenerated (CLAUDE.md rule 2), so this test pins "today" to the fixture's
    // own reference date rather than racing the real clock past 2026-09-20 and losing a card
    // (`engine/src/surface.rs`'s `read_approvals` excludes a pending approval whose `expires` is
    // before today). 2026-08-28 is the date `engine/tests/fixtures/golden-today-full.md` and the
    // oracle tests' `PINNED_DATE` were generated against.
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let s = state_inner(&cs, "decisions").unwrap();
    let cards = s["state"]["decisions"]["cards"].as_array().unwrap().clone();
    let a = cards[0]["id"].as_str().unwrap(); let b = cards[1]["id"].as_str().unwrap();
    assert_eq!(decide_inner(&cs, "today", a, "rejected", "no", None).unwrap()["ok"], true);
    let env = decide_inner(&cs, "today", b, "snoozed", "later", Some("2099-01-01".into())).unwrap();
    assert_eq!(env["ok"], true);
    let text = std::fs::read_to_string(v.join("approvals").join(format!("{}.md", cards[1]["slug"].as_str().unwrap()))).unwrap();
    assert!(text.contains("status: snoozed") && text.contains("snooze_until: 2099-01-01") && text.contains("decision_note:"), "{text}");
    assert_eq!(decide_inner(&cs, "today", a, "maybe", "", None).unwrap()["ok"], false, "verdict is one of three words");
}

/// `ConsoleState::set_test_today` is the seam the test above relies on — proved here on its own
/// terms rather than by the side effect of another test passing. `amend-ph-106-due.md` carries a
/// frozen `expires: 2026-09-20` (never regenerated, CLAUDE.md rule 2); 2026-08-28 is before that
/// date and always will be, so pinning "today" there must keep the card in view regardless of what
/// day this suite actually runs on (`engine/src/surface.rs`'s `read_approvals` excludes a pending
/// approval once `expires` is before today).
#[test]
fn set_test_today_pins_what_the_read_model_treats_as_today() {
    let v = scratch("pin-today");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-pin-today-data-{}", std::process::id())));
    cs.set_test_today(Some("2026-08-28".parse().unwrap()));
    let s = state_inner(&cs, "decisions").unwrap();
    let cards = s["state"]["decisions"]["cards"].as_array().unwrap();
    assert!(cards.iter().any(|c| c["kind"] == json!("amend")), "pinning today to 2026-08-28 must keep the amend approval (expires 2026-09-20) in view: {cards:?}");
}

#[test]
fn issue_flag_needs_a_category_and_snapshots_the_object_and_info_closes() {
    let v = scratch("issue");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-issue-data-{}", std::process::id())));
    let id = first_id(&cs);
    assert_eq!(open_issue_inner(&cs, "today", &id, vec![], "x").unwrap()["ok"], false);
    let env = open_issue_inner(&cs, "today", &id, vec!["wrong-effort".into()], "12 h is too much").unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["state"]["issues_panel"]["open_count"], 1);
    let issue_id = env["state"]["issues_panel"]["rows"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(resolve_issue_inner(&cs, "today", &issue_id, "effort set to 6").unwrap()["state"]["issues_panel"]["open_count"], 0);

    // `vault-full/info/` holds only `.gitkeep` — plant an info item directly via the engine so
    // `close_info_inner` is actually exercised, not skipped (Task 9 review fix 2).
    knowlu_engine::info::open_info(
        &v,
        &knowlu_engine::info::NewInfo { title: "Library closes early Friday", kind: "notice", body: "", opened_by: "quinn", close_key: None, expires: None },
        &console_ctx(),
        None,
        None,
    ).unwrap();
    let s = state_inner(&cs, "today").unwrap();
    let before_count = s["state"]["nav_counts"]["good_to_know"]["count"].as_i64().unwrap();
    let info = s["state"]["good_to_know"].as_array().unwrap().first().expect("the planted info item").clone();
    let env = close_info_inner(&cs, "today", info["id"].as_str().unwrap()).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert!(env["state"]["good_to_know"].as_array().unwrap().iter().all(|i| i["id"] != info["id"]));
    assert_eq!(env["state"]["nav_counts"]["good_to_know"]["count"].as_i64().unwrap(), before_count - 1);
}

#[test]
fn sync_on_a_vault_with_no_account_is_calm_and_backup_needs_a_folder() {
    let v = scratch("sync");
    // Both folders are keyed by PID, and a persistent machine (the self-hosted CI runner, a dev
    // laptop) reuses PIDs: a folder left by an earlier run already holds a `backup_dir` in its
    // settings, and "backup needs a folder" then fails. Start clean and leave nothing behind.
    let data = std::env::temp_dir().join(format!("qo-sync-data-{}", std::process::id()));
    let bk = std::env::temp_dir().join(format!("qo-sync-bk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&bk);
    let cs = ConsoleState::open(v.clone(), data.clone());
    let env = sync_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true);
    // C3', Task 10: `topline.sync` is the engine's `SyncStatus`, not the old git-shaped
    // `HistoryStatus` — a vault with no `config/cloud.yaml` is a calm, named skip, never an error.
    assert_eq!(env["state"]["topline"]["sync"]["skipped"], "no account");
    assert!(env["state"]["topline"]["sync"]["last_error"].is_null());
    let env = backup_now_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], false); assert!(env["error"].as_str().unwrap().contains("backup folder"));
    let mut patch = serde_json::Map::new(); patch.insert("backup_dir".into(), json!(bk.to_string_lossy()));
    assert_eq!(set_settings_inner(&cs, patch).unwrap()["ok"], true);
    let env = backup_now_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["state"]["topline"]["backup"]["behind_days"], 0);
    assert!(bk.join(&cs.settings.lock().unwrap().profile_id).join("vault/tasks").is_dir());
    let mut bad = serde_json::Map::new(); bad.insert("profile_id".into(), json!("me"));
    assert_eq!(set_settings_inner(&cs, bad).unwrap()["ok"], false, "only backup_dir and autostart are settable");
    drop(cs);
    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&bk);
}

// R-T10 (Task 10 review): `serde_json::Map` iterates alphabetically, so "autostart" is applied
// before the refusal on "zzz_unknown" is reached — `set_settings_inner` must validate the whole
// patch before touching the live settings guard or the file on disk, else a bad patch that
// happens to fail on a later-sorting key still leaves a stray mutation behind.
#[test]
fn set_settings_validates_the_whole_patch_before_touching_anything() {
    let v = scratch("settings-validate");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-settings-validate-data-{}", std::process::id())));
    // Establish a real settings file first so "unchanged" below is a real byte comparison, not
    // just "still missing".
    let mut seed = serde_json::Map::new(); seed.insert("autostart".into(), json!(true));
    assert_eq!(set_settings_inner(&cs, seed).unwrap()["ok"], true);
    let before = std::fs::read(&cs.settings_path).unwrap();

    let mut patch = serde_json::Map::new();
    patch.insert("autostart".into(), json!(false));
    patch.insert("zzz_unknown".into(), json!(1));
    let env = set_settings_inner(&cs, patch).unwrap();
    assert_eq!(env["ok"], false, "{env}");
    assert!(env["error"].as_str().unwrap().contains("zzz_unknown"), "{env}");

    let g = get_settings_inner(&cs).unwrap();
    assert_eq!(g["settings"]["autostart"], true, "autostart must not have flipped: the whole patch failed");
    assert_eq!(std::fs::read(&cs.settings_path).unwrap(), before, "a refused patch must leave the settings file untouched");

    let mut bad_type = serde_json::Map::new(); bad_type.insert("backup_dir".into(), json!(5));
    let env2 = set_settings_inner(&cs, bad_type).unwrap();
    assert_eq!(env2["ok"], false, "{env2}");
    assert!(env2["error"].as_str().unwrap().contains("backup_dir"), "{env2}");
}

#[test]
fn ui_events_land_ids_only_and_refuse_text() {
    let v = scratch("uiev");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-uiev-data-{}", std::process::id())));
    let id = first_id(&cs);
    assert_eq!(ui_event_inner(&cs, "object_seen", "today", Some(id.clone()), Some("task".into()), Some(2100))["ok"], true);
    assert_eq!(ui_event_inner(&cs, "object_seen", "today", Some("Exam prep".into()), None, None)["ok"], false);
    assert_eq!(ui_event_inner(&cs, "typed", "today", None, None, None)["ok"], false);
    let files: Vec<_> = std::fs::read_dir(v.join("state/events-ui")).unwrap().flatten().collect();
    assert_eq!(files.len(), 1);
    let text = std::fs::read_to_string(files[0].path()).unwrap();
    assert_eq!(text.lines().count(), 1); assert!(text.contains(&cs.session) && !text.contains("Exam"));
}

// C3', Task 10: `vault_head` and `engine_newer` left with git (H9b) — they compared the vault's git
// HEAD against this build, and a vault has not been a git repository since cloud design §4.1. The
// two tests that exercised them (`engine_newer_compares_the_console_build_with_the_vaults_head_and_
// is_false_without_a_repo`, `engine_newer_is_true_only_when_a_present_head_differs_from_the_console_
// build`) go with them — `state::refresh_head` and `ConsoleState::head_sha` no longer exist.

/// The topline's `sync` key is the engine's own `SyncStatus`, copied through verbatim rather than
/// assembled field by field in `commands.rs` (console spec §3.1: nothing here computes) — every
/// field `SyncStatus` carries round-trips exactly, including one this test sets by hand so the
/// assertion is not just "whatever a scratch vault happens to produce" (the case the sync test above
/// already covers).
#[test]
fn topline_sync_is_the_engines_sync_status_verbatim() {
    let v = scratch("sync-verbatim");
    // Final review M3: start clean and leave nothing behind (`cea59a9`'s pattern).
    let data = std::env::temp_dir().join(format!("qo-sync-verbatim-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let cs = ConsoleState::open(v.clone(), data.clone());
    let status = knowlu_engine::sync::SyncStatus {
        ok: true,
        at: Some("2026-09-24T07:00:00.000Z".to_string()),
        lines: vec!["sync: 1 record(s) and 0 note(s) down; 1 applied, 0 card(s), 0 refused".to_string()],
        last_error: None,
        skipped: None,
    };
    *cs.sync.lock().unwrap() = status.clone();
    let s = state_inner(&cs, "today").unwrap();
    let expected = serde_json::to_value(&status).unwrap();
    assert_eq!(s["state"]["topline"]["sync"], expected, "the wire value must be exactly SyncStatus's own serialisation");
    assert!(s["state"]["topline"].get("vault_head").is_none(), "vault_head left with git");
    assert!(s["state"]["topline"].get("engine_newer").is_none(), "engine_newer left with git");
    assert!(s["state"]["topline"].get("auto_sync").is_none(), "auto_sync left with git");
    drop(cs);
    let _ = std::fs::remove_dir_all(&data);
}

// The brief's `first_id` reads `state["must_do"]["groups"][0]["rows"][0]["id"]` — the first
// Overdue row by wall-clock "now". That group's membership is wall-clock-dependent (must_do only
// holds items with slack <= 0), and every other fixture task's `due` in `vault-full` has since
// fallen behind "today", pushing `ph-106-exam-1-prep.md` (due 2026-09-25, still ahead) out of
// position zero and breaking the hardcoded `tasks/ph-106-exam-1-prep.md` assertions above. The
// tests care about *which specific note* gets written, not about ranking, so this resolves that
// same fixture note's id directly instead of depending on group order (Task 8 deviation).
fn first_id(cs: &ConsoleState) -> String {
    let text = std::fs::read_to_string(cs.vault.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    text.lines().find_map(|l| l.strip_prefix("id: ")).unwrap().trim().to_string()
}

/// B1 (console spec §8/§9): a console write and the engine's `sync::run_lines_with` must never
/// interleave — a pull can write a note it has never seen or file an amend card while a write is
/// single-line surgery on a note it has just read. `ConsoleState::vault_io` is the one lock both
/// take; `run_sync` holds it for the whole of the engine call, so holding it here from a test
/// thread stands in for a sync in flight and the write has to wait for it.
#[test]
fn a_write_waits_for_the_vault_lock_a_sync_would_be_holding() {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    let v = scratch("vaultio");
    let cs = Arc::new(ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-vaultio-data-{}", std::process::id()))));
    let id = first_id(&cs);

    let held = cs.vault_io.lock().unwrap();   // stands in for `run_sync`'s hold
    let writer = {
        let cs = Arc::clone(&cs);
        std::thread::spawn(move || {
            let started = Instant::now();
            let mut f = serde_json::Map::new();
            f.insert("importance".into(), json!(2));
            let env = set_fields_inner(&cs, "today", &id, f).unwrap();
            (started.elapsed(), env)
        })
    };
    std::thread::sleep(Duration::from_millis(200));
    assert!(!writer.is_finished(), "the write must not have landed while the vault lock was held");
    drop(held);

    let (waited, env) = writer.join().unwrap();
    assert_eq!(env["ok"], true, "…and it goes through the moment the lock is free: {env}");
    assert!(waited >= Duration::from_millis(150), "the write actually waited on the lock, not raced past it: {waited:?}");
    let text = std::fs::read_to_string(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    assert!(text.contains("importance: 2"), "{text}");

    // Plan 2 Task 5 (F11): a backup walks and copies the WHOLE working tree — the same tree a sync
    // rewrites — so `backup_now_inner` joins `vault_io` too. Before this it read the vault while a
    // rebase was mid-checkout and mirrored a mixture of two commits.
    let bdir = std::env::temp_dir().join(format!("qo-vaultio-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bdir);
    { let mut s = cs.settings.lock().unwrap(); s.backup_dir = Some(bdir.clone()); }
    let held = cs.vault_io.lock().unwrap();
    let backer = {
        let cs = Arc::clone(&cs);
        std::thread::spawn(move || {
            let started = Instant::now();
            let env = backup_now_inner(&cs, "today").unwrap();
            (started.elapsed(), env)
        })
    };
    std::thread::sleep(Duration::from_millis(200));
    assert!(!backer.is_finished(), "the backup must not have run while the vault lock was held");
    drop(held);

    let (waited, env) = backer.join().unwrap();
    assert_eq!(env["ok"], true, "…and it goes through the moment the lock is free: {env}");
    assert!(waited >= Duration::from_millis(150), "the backup actually waited on the lock, not raced past it: {waited:?}");
    let _ = std::fs::remove_dir_all(&bdir);
}

/// B8: `status` is what every filter, horizon and `closed_this_week` read. `literal_for` would
/// turn a JSON `null` into the literal `null`, which parses as an absent status and drops the note
/// out of every view at once — so it is refused by name, before any literal is built, and nothing
/// is written.
#[test]
fn a_null_status_is_refused_and_the_note_is_untouched() {
    let v = scratch("nullstatus");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-nullstatus-data-{}", std::process::id())));
    let id = first_id(&cs);
    let before = std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    let mut f = serde_json::Map::new();
    f.insert("status".into(), serde_json::Value::Null);
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], false, "{env}");
    assert_eq!(env["error"], "status cannot be empty");
    assert_eq!(env["state"]["schema"], 1, "a refusal still carries the current state");
    assert_eq!(before, std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap());
    // A real status still writes, so the refusal is about `null` and not about the field.
    let mut ok = serde_json::Map::new();
    ok.insert("status".into(), json!("done"));
    assert_eq!(set_fields_inner(&cs, "today", &id, ok).unwrap()["ok"], true);
}

/// The note behind `first_id`, read through the engine's one note reader — `None` if it cannot be.
fn fixture_title_and_status(v: &Path) -> Option<(String, String)> {
    let text = knowlu_engine::pystr::read_text(&v.join("tasks/ph-106-exam-1-prep.md")).ok()?;
    let (meta, _) = knowlu_engine::models::split_frontmatter(&text).ok()?;
    let get = |k: &str| knowlu_engine::yaml::get(&meta, k).and_then(knowlu_engine::yaml::text);
    Some((get("title")?, get("status")?))
}

/// A reviewer's probe: `title: "a---b"` broke the note — it wrote once and then neither read nor
/// took a second edit. A title with dashes is legitimate text and must round-trip; a line break
/// that would reach the note raw is refused by name with nothing written.
#[test]
fn a_title_with_dashes_or_a_line_break_never_breaks_the_note() {
    for (n, title) in ["a---b", "--- x", "line one\nline two"].into_iter().enumerate() {
        let v = scratch(&format!("dashes{n}"));
        let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-dashes{n}-data-{}", std::process::id())));
        let id = first_id(&cs);
        let mut f = serde_json::Map::new(); f.insert("title".into(), json!(title));
        let env = set_fields_inner(&cs, "today", &id, f).unwrap();
        assert_eq!(env["ok"], true, "{title:?}: {env}");
        assert_eq!(fixture_title_and_status(&v), Some((title.to_string(), "active".to_string())), "{title:?}");
        assert_eq!(note_inner(&cs, &id).unwrap()["ok"], true, "{title:?}: the note must still be found");
        // The second edit is the one the probe broke.
        let mut again = serde_json::Map::new(); again.insert("status".into(), json!("done"));
        let env = set_fields_inner(&cs, "today", &id, again).unwrap();
        assert_eq!(env["ok"], true, "{title:?}: second edit: {env}");
        assert_eq!(fixture_title_and_status(&v), Some((title.to_string(), "done".to_string())), "{title:?}");
    }
    // A raw line break (a verbatim field, or a separator `to_literal` leaves unescaped) is refused.
    for (n, (field, value)) in [("due", "2026-10-01\n---"), ("title", "a\u{2028}b")].into_iter().enumerate() {
        let v = scratch(&format!("breaks{n}"));
        let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-breaks{n}-data-{}", std::process::id())));
        let id = first_id(&cs);
        let before = std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
        let records = journal_records(&v).len();
        let mut f = serde_json::Map::new(); f.insert(field.into(), json!(value));
        let env = set_fields_inner(&cs, "today", &id, f).unwrap();
        assert_eq!(env["ok"], false, "{value:?}: {env}");
        assert!(env["error"].as_str().unwrap().contains("line break"), "{value:?}: {env}");
        assert_eq!(before, std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap(), "{value:?}");
        assert_eq!(journal_records(&v).len(), records, "{value:?}: no journal record");
        let mut ok = serde_json::Map::new(); ok.insert("status".into(), json!("done"));
        assert_eq!(set_fields_inner(&cs, "today", &id, ok).unwrap()["ok"], true, "{value:?}");
    }
}

/// Plan 4a Task 7: the settings panel's one write. `set_profile_name_in` is the testable core —
/// which registry, which vault, which name — so the rename is exercised without an `AppHandle`.
#[test]
fn set_profile_name_renames_the_registry_entry_and_refuses_an_empty_name() {
    let v = scratch("profname");
    let root = std::env::temp_dir().join(format!("knowlu-setname-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    knowlu::profiles::register(&root, "Before", &v).unwrap();
    let out = knowlu::commands::set_profile_name_in(&root, &v, "After").unwrap();
    assert_eq!(out["ok"], true);
    assert_eq!(knowlu::profiles::load(&root).unwrap()[0].name, "After");
    let bad = knowlu::commands::set_profile_name_in(&root, &v, "   ").unwrap();
    assert_eq!(bad["ok"], false);
    assert_eq!(knowlu::profiles::load(&root).unwrap()[0].name, "After", "a refusal changes nothing");
    let _ = std::fs::remove_dir_all(&root);
}

/// Review round 1, IMPORTANT 2: `profiles::load` separates absent from unreadable, and a registry
/// that is THERE and broken must never be written over. `set_profile_name_in` reaches that rule
/// through `register`'s own `?`; this is the proof that it does, and that the bytes survive.
#[test]
fn set_profile_name_refuses_an_unreadable_registry_and_leaves_the_file_byte_for_byte() {
    let v = scratch("profname-broken");
    let root = std::env::temp_dir().join(format!("knowlu-setname-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let reg = root.join("profiles.json");
    std::fs::write(&reg, b"{ this is not a registry").unwrap();
    let before = std::fs::read(&reg).unwrap();

    let out = knowlu::commands::set_profile_name_in(&root, &v, "After").unwrap();
    assert_eq!(out["ok"], false, "a rename over an unreadable registry is a refusal, not a rewrite");
    assert!(out["error"].as_str().unwrap().contains("is not a profile registry"), "{}", out["error"]);
    assert_eq!(std::fs::read(&reg).unwrap(), before, "the refused write changed the file");
    let _ = std::fs::remove_dir_all(&root);
}

/// Review round 1, IMPORTANT 1 + the DRY note: *Open this profile* and *Switch profile…* are one
/// spawn path that differs only in its arguments, and both end in the `--after-pid` handshake
/// (R-P4a-1). The spawn itself needs a window; the argv does not. (Belongs in
/// `app/tests/onboarding.rs` once that file exists.)
#[test]
fn a_relaunch_carries_its_own_arguments_and_always_the_after_pid_handshake() {
    use std::ffi::OsString;
    let open = knowlu::onboarding::relaunch_args([OsString::from("--vault"), OsString::from("C:\\v\\ada")], 4321);
    assert_eq!(open, vec![OsString::from("--vault"), OsString::from("C:\\v\\ada"), OsString::from("--after-pid"), OsString::from("4321")]);
    let switch = knowlu::onboarding::relaunch_args(["--pick"], 4321);
    assert_eq!(switch, vec![OsString::from("--pick"), OsString::from("--after-pid"), OsString::from("4321")]);
}

/// The settings row's four commands compute nothing (R9): `inference_status` marshals
/// `inference::status`, and the two installers refuse a bad path with the module's own words.
#[test]
fn the_inference_commands_marshal_and_refuse() {
    let root = std::env::temp_dir().join(format!("knowlu-cmd-inference-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let s = knowlu::inference::status(&root);
    let v = serde_json::to_value(&s).unwrap();
    assert!(v["runtime"].is_null() && v["model"].is_null() && v["model_bytes"] == 0);

    let notes = root.join("notes.txt");
    std::fs::write(&notes, b"x").unwrap();
    let err = knowlu::inference::install_model_from_file(&root, &notes, None).unwrap_err();
    assert!(err.contains(".gguf"), "{err}");

    // A model that installs, then is removed: the two ends of the row's lifecycle.
    let gguf = root.join("m.gguf");
    std::fs::write(&gguf, b"abc").unwrap();
    knowlu::inference::install_model_from_file(&root, &gguf, None).unwrap();
    assert!(knowlu::inference::status(&root).model.is_some());
    knowlu::inference::remove_model(&root).unwrap();
    assert!(knowlu::inference::status(&root).model.is_none());
    let _ = std::fs::remove_dir_all(&root);
}
