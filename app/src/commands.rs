//! One command per S2 §8 route. Marshal, call, envelope — nothing here computes (spec §3.1).
use serde_json::{json, Value};
use tauri::State;
use crate::scheduler::Scheduler;
use crate::state::ConsoleState;
use knowlu_engine::{journal::Journal, write::{self, WriteContext, WriteOpts}};
use knowlu_engine::yamlemit::{safe_dump_block, Node};

pub const CONSOLE_BUILD: Option<&str> = option_env!("KNOWLU_BUILD_SHA");

fn envelope(result: Result<Value, String>, key: &str) -> Value {
    match result { Ok(v) => json!({ "ok": true, "error": Value::Null, key: v }), Err(e) => json!({ "ok": false, "error": e, key: Value::Null }) }
}

/// "Today" for every read/write path in this file. Honours `ConsoleState::test_today` — the
/// test-only seam described there — when a test has set it; every real caller leaves that `None`
/// and gets the real clock exactly as before the seam existed. A pinned date carries noon local
/// time, matching nothing in particular except being safely clear of both a day's midnight edges.
fn now_in(cs: &ConsoleState) -> jiff::Zoned {
    let zone = knowlu_engine::cli::vault_zone(&cs.vault);
    match *cs.test_today.lock().unwrap() {
        Some(d) => d.at(12, 0, 0, 0).to_zoned(zone).expect("a pinned test date is always a valid zoned time"),
        None => jiff::Zoned::now().with_time_zone(zone),
    }
}

fn build_state_value(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    let view = knowlu_engine::surface::View::parse(view).ok_or_else(|| format!("unknown view {view:?}"))?;
    let now = now_in(cs);
    let state = knowlu_engine::surface::build_state(&cs.vault, view, now.date(), &now, cs.seen_at().as_deref());
    let mut v = serde_json::to_value(&state).map_err(|e| e.to_string())?;
    // `vault_head` and `engine_newer` went with git (C3′, Task 10): they compared the VAULT's git
    // HEAD against this build, and a vault has not been a git repository since §4.1. The console's
    // own build stays — the diagnostics blob and the issue report both name it.
    v["topline"]["console_build"] = json!(CONSOLE_BUILD);
    v["topline"]["seen_at"] = json!(cs.seen_at());
    // Sync/backup/startup-missed come from the caches Task 10's commands fill —
    // copied here verbatim, never recomputed (spec §3.1: nothing in `commands.rs` computes).
    v["topline"]["sync"] = serde_json::to_value(&*cs.sync.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())?;
    v["topline"]["backup"] = serde_json::to_value(&*cs.backup.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())?;
    v["topline"]["startup_missed"] = json!(cs.startup_missed.load(std::sync::atomic::Ordering::SeqCst));
    Ok(v)
}

/// Copies live scheduler status into an already-built envelope's `state.topline` (Knowlu plan 1,
/// Task 12). Kept separate from `build_state_value` so every `*_inner` function — exercised
/// directly by `tests/commands.rs` against a bare `ConsoleState`, with no `Scheduler` in play —
/// keeps its existing signature; only the `#[tauri::command]` wrappers, which always run inside
/// Tauri and always have `State<'_, Scheduler>`, attach it. A no-op when the envelope carries no
/// `state` key (a refused command still returns one — `mutate` always rebuilds it — so this only
/// ever skips truly state-less envelopes); **Task 15's page must tolerate a missing
/// `topline.scheduler`/`topline.last_slot` rather than assume they are always present**, since a
/// state-less envelope carries neither.
///
/// `pub` (review item 6) so `tests/scheduler.rs` can exercise it directly against a scratch
/// `ConsoleState` + `Scheduler` without going through Tauri's IPC. Reads `Scheduler.mode_device`
/// (cached, refreshed every 60 s by the housekeeping thread) rather than re-parsing
/// `config/runners.yaml` itself — this runs on every poll, across ten commands (review item 5).
pub fn attach_scheduler(env: &mut Value, sch: &Scheduler) -> Result<(), String> {
    if env["state"].is_null() {
        return Ok(());
    }
    // Final fix wave C1: both mutexes go through `scheduler::lock`, the module's poison-tolerant
    // helper. They were `.map_err(|_| "lock")?`, and every caller discards this function's error
    // with `let _ =` — so one panic under a slot silently stripped `last_slot` and `scheduler`
    // from every poll's envelope for the life of the process, with nothing said anywhere. Neither
    // value carries an invariant a panic could break (an Option<RunSummary>, a (mode, device_ok)
    // pair), which is exactly why the scheduler recovers them rather than propagating.
    let last = crate::scheduler::lock(&sch.last).clone();
    env["state"]["topline"]["last_slot"] = serde_json::to_value(last).map_err(|e| e.to_string())?;
    let (mode, device_ok) = *crate::scheduler::lock(&sch.mode_device);
    env["state"]["topline"]["scheduler"] = json!({
        "mode": match mode { knowlu_engine::schedule::SchedulerMode::App => "app", knowlu_engine::schedule::SchedulerMode::Script => "script" },
        "paused": sch.paused.load(std::sync::atomic::Ordering::SeqCst),
        "device_ok": device_ok,
    });
    Ok(())
}

/// D7 / §6: what the window has to paint while the vault has no read model yet — the minute between
/// Finish and the first `rank`, which was a white page with an engine error in it.
///
/// **Ruling R-C1c-plan-1:** attached whenever the vault has never been through a whole slot
/// (`ingest::is_first_run` — the absence of `state/today.md`, the same predicate
/// `scheduler::needs_first_run` and the engine's own first-run rules read), and never on a failed
/// read model: `surface::build_state` has no failure path, so a wizard-made vault answers `ok` with
/// a nearly empty state, and a page that waited for a failure would paint that empty day and call it
/// the first look. The page paints the block while the key is there and drops it when it stops
/// coming.
///
/// The block is `{ running, current, steps: [[name, code], …] }`, all from the live `Scheduler`
/// (R-C1c-8). While a slot runs, `steps` and `current` are `Scheduler.live`: each step as it lands
/// and the one doing its work now, so the list fills during the slot. `Scheduler.last` is written
/// only when a slot ends, and on a first run that succeeds `rank` has written the day by then, so
/// reading it mid-slot listed nothing. With no slot running, `steps` is `last`'s (a first slot that
/// ended without a day) and `current` is null.
pub fn first_run_value(cs: &ConsoleState, sch: &Scheduler) -> Option<Value> {
    if !knowlu_engine::ingest::is_first_run(&cs.vault) {
        return None;
    }
    // `live` is read under the `running` guard (the lock order `Scheduler.live` names), so the flag
    // and the list are one reading: no slot can start or end between them.
    let running = crate::scheduler::lock(&sch.running);
    if *running {
        let live = crate::scheduler::lock(&sch.live).clone();
        return Some(json!({ "running": true, "current": live.current, "steps": live.steps }));
    }
    drop(running);
    let steps = crate::scheduler::lock(&sch.last).as_ref().map(|s| s.steps.clone()).unwrap_or_default();
    Some(json!({ "running": false, "current": Value::Null, "steps": steps }))
}

pub fn state_inner(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    Ok(envelope(build_state_value(cs, view), "state"))
}

pub fn note_inner(cs: &ConsoleState, id: &str) -> Result<Value, String> {
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let now = now_in(cs);
    let mut journal = knowlu_engine::journal::Journal::new(&cs.vault);
    let detail = knowlu_engine::surface::note_detail(&cs.vault, id, now.date(), &mut journal).ok_or_else(|| format!("no note with id {id}"));
    Ok(envelope(detail.and_then(|d| serde_json::to_value(d).map_err(|e| e.to_string())), "note"))
}

pub fn mark_seen_inner(cs: &ConsoleState) -> Result<Value, String> {
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let stamp = knowlu_engine::journal::now_ts(None);
    std::fs::write(&cs.seen_path, &stamp).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "error": Value::Null, "seen_at": stamp }))
}

/// Fire-and-forget interaction logging (spec §7.5): never takes `cs.lock` (it must not queue
/// behind a mutating command) and never returns state — the page's `ev()` wrapper ignores
/// failures outright. `knowlu_engine::uievents::record` is what refuses free text; this just
/// marshals the refusal into `{ok, error}` instead of a `Result`.
pub fn ui_event_inner(cs: &ConsoleState, action: &str, view: &str, object_id: Option<String>, object_kind: Option<String>, ms: Option<i64>) -> Value {
    let ev = knowlu_engine::uievents::UiEvent { session: &cs.session, view, action, object_id: object_id.as_deref(), object_kind: object_kind.as_deref(), ms };
    match knowlu_engine::uievents::record(&cs.vault, &ev, None) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Frontmatter fields the console may edit. Anything else (`id`, `source_uid`, `judgment`, …) is
/// refused rather than silently ignored — the page and the write path agree on this list.
pub const EDITABLE: [&str; 12] = ["title", "course", "due", "effort_hours", "importance", "importance_reason", "status", "progress", "slice_hours", "domain", "rank_override", "effort_confidence"];
/// Fields whose literal must go through `write::to_literal` so a colon or quote in free text
/// survives as YAML; every other editable field is passed through as the literal the page sent.
pub const QUOTED: [&str; 5] = ["title", "course", "importance_reason", "domain", "effort_confidence"];

/// The one `WriteContext` constructor in `app/` — every console-originated write is `quinn` via
/// `dashboard` (`journal::VIAS` carries it since Task 7).
pub fn console_ctx() -> WriteContext {
    debug_assert!(knowlu_engine::journal::VIAS.contains(&"dashboard"));
    WriteContext::new("quinn", "dashboard")
}

/// The context for a `decide`d proposal's in-process execution (R-T9, Task 9 review). The
/// DECISION (writing `status`/`decision_note`/`snooze_until`) is Quinn's and keeps
/// `console_ctx()`; EXECUTING that decision — `process_approvals` materializing a task, applying
/// an amendment, expanding a digest — is the system acting on Quinn's already-made decision,
/// exactly as `cli::run`'s scheduled pass does it (actor `agent:approvals`), so it must not be
/// journaled as a fresh human judgement: `write::write_literals`'s judge-once freeze
/// (`journal::human_set`) reads a `create`/`set` record's `actor` field, and an `actor: quinn`
/// materialize record would wrongly freeze every field of the new task (effort_hours, importance,
/// course, due, …) against future agent re-judgement. `via` stays `dashboard` so provenance still
/// shows the write happened from the console, not the scheduled runner.
pub fn executor_ctx() -> WriteContext {
    debug_assert!(knowlu_engine::journal::VIAS.contains(&"dashboard"));
    WriteContext::new("agent:approvals", "dashboard")
}

/// Takes the console lock, runs `f` against a fresh journal, records a write on success, and
/// always rebuilds `state` for `view` — a refusal carries the current state too, never `null`,
/// so the page never has to guess what changed.
fn mutate(cs: &ConsoleState, view: &str, f: impl FnOnce(&mut Journal) -> Result<(), String>) -> Result<Value, String> {
    // `vault_io` FIRST, then `cs.lock` — always that order, never the reverse (console spec §8/§9;
    // see `ConsoleState::vault_io`). A write is single-line surgery on a note this call has just
    // read; the engine's `sync` can write a pulled note or file an amend card under it. Serialising
    // the two here means the `sync` command's own in-process call and every write queue against
    // each other (the slot's own sync step is a CHILD PROCESS and takes no `vault_io` at all — see
    // `ConsoleState::vault_io`, review M1), while `cs.lock` alone still guards the read polls so a
    // `state` poll never waits behind a sync's own network call.
    let _io = cs.vault_io.lock().map_err(|_| "vault lock poisoned".to_string())?;
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let mut journal = Journal::new(&cs.vault);
    let result = f(&mut journal);
    if result.is_ok() { cs.note_write(); }
    let state = build_state_value(cs, view)?;
    Ok(match result { Ok(()) => json!({ "ok": true, "error": Value::Null, "state": state }), Err(e) => json!({ "ok": false, "error": e, "state": state }) })
}

/// `QUOTED` fields are re-emitted through `write::to_literal` so free text with a colon or quote
/// survives; every other editable field is passed through as the literal the page already sent
/// (a `Value::String` verbatim, `Null` as `"null"`, numbers/bools via `to_string`).
fn literal_for(field: &str, v: &Value) -> Result<String, String> {
    if QUOTED.contains(&field) {
        let s = v.as_str().ok_or_else(|| format!("{field} must be a string"))?;
        return Ok(write::to_literal(&serde_yaml_ng::Value::String(s.to_string())));
    }
    Ok(match v { Value::String(s) => s.clone(), Value::Null => "null".into(), other => other.to_string() })
}

pub fn set_fields_inner(cs: &ConsoleState, view: &str, id: &str, fields: serde_json::Map<String, Value>) -> Result<Value, String> {
    mutate(cs, view, |journal| {
        let mut literals = Vec::new();
        for (k, v) in &fields {
            if !EDITABLE.contains(&k.as_str()) { return Err(format!("{k} is not editable")); }
            // `status` is what every filter, every horizon and `closed_this_week` read; `literal_for`
            // would happily turn a `Null` into the literal `null`, which parses as an absent status
            // and drops the note out of every view at once. The page already treats an emptied
            // field as a cancel — this is the server-side half of that rule, refused by name rather
            // than written and regretted (final fix wave, B8).
            if k == "status" && v.is_null() { return Err("status cannot be empty".to_string()); }
            literals.push((k.clone(), literal_for(k, v)?));
        }
        let res = write::write_literals(&cs.vault, id, &literals, &console_ctx(), journal, &WriteOpts::default()).map_err(|e| e.to_string())?;
        if let Some((name, why)) = res.skipped.iter().next() { return Err(format!("{name}: {why}")); }
        Ok(())
    })
}

/// Lowercase, non-alphanumerics collapsed to a single `-`, trimmed, capped at 60 chars — the
/// engine mints the note's `id`, so this only needs to produce a readable filename.
fn slugify(title: &str) -> String {
    let mut s: String = title.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    while s.contains("--") { s = s.replace("--", "-"); }
    s.trim_matches('-').chars().take(60).collect::<String>().trim_matches('-').to_string()
}

pub fn create_task_inner(cs: &ConsoleState, view: &str, fields: serde_json::Map<String, Value>) -> Result<Value, String> {
    mutate(cs, view, |journal| {
        let title = fields.get("title").and_then(|v| v.as_str()).map(str::trim).filter(|t| !t.is_empty()).ok_or("title is required")?;
        let course = fields.get("course").and_then(|v| v.as_str()).map(str::trim).filter(|c| !c.is_empty());
        let due = fields.get("due").and_then(|v| v.as_str()).map(|d| if d.len() == 10 { format!("{d}T23:59") } else { d.to_string() });
        let effort = fields.get("effort_hours").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let front = Node::map(vec![
            ("title", Node::text(title)),
            ("course", Node::opt_text(course)),
            ("domain", Node::text("school")),
            ("due", due.as_deref().map(Node::text).unwrap_or(Node::Null)),
            ("effort_hours", Node::Float(effort)),
            ("effort_source", Node::text("quinn")),
            ("importance", Node::Int(3)),
            ("status", Node::text("active")),
            ("progress", Node::Int(0)),
            ("created_by", Node::text("quinn")),
        ]);
        let text = format!("---\n{}---\n\n", safe_dump_block(&front));
        let mut slug = slugify(title); if slug.is_empty() { slug = "task".into(); }
        let mut rel = format!("tasks/{slug}.md"); let mut n = 2;
        while cs.vault.join(&rel).exists() { rel = format!("tasks/{slug}-{n}.md"); n += 1; }
        write::create(&cs.vault, &rel, &text, &console_ctx(), journal, None).map(|_| ()).map_err(|e| e.to_string())
    })
}

pub fn delete_note_inner(cs: &ConsoleState, view: &str, id: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| write::delete(&cs.vault, id, &console_ctx(), journal).map(|_| ()).map_err(|e| e.to_string()))
}

/// Approve/reject/snooze one approval, then run `process_approvals` in the same call so an
/// approved task shows up in `state` NOW — the console never waits for the next scheduled run.
/// `decision` summarises what that pass did, marshalled to strings only (spec's envelope shape).
pub fn decide_inner(cs: &ConsoleState, view: &str, id: &str, verdict: &str, note: &str, snooze_until: Option<String>) -> Result<Value, String> {
    let mut decision = Value::Null;
    let env = mutate(cs, view, |journal| {
        if !["approved", "rejected", "snoozed"].contains(&verdict) { return Err(format!("verdict must be approved, rejected or snoozed, not {verdict:?}")); }
        let snooze = match (verdict, snooze_until.as_deref()) { ("snoozed", Some(d)) if d.len() == 10 => d.to_string(), ("snoozed", _) => return Err("snoozed needs snooze_until as YYYY-MM-DD".into()), _ => "null".into() };
        let literals = vec![
            ("status".to_string(), verdict.to_string()),
            ("decision_note".to_string(), write::to_literal(&serde_yaml_ng::Value::String(note.to_string()))),
            ("snooze_until".to_string(), snooze),
        ];
        write::write_literals(&cs.vault, id, &literals, &console_ctx(), journal, &WriteOpts::default()).map_err(|e| e.to_string())?;
        let now = now_in(cs);
        let r = knowlu_engine::approvals::process_approvals(&cs.vault, now.date(), now.datetime(), &executor_ctx(), journal);
        decision = json!({ "executed": r.executed, "expired": r.expired, "woken": r.woken, "rejected": r.rejected, "warnings": r.warnings });
        Ok(())
    })?;
    let mut env = env; env["decision"] = decision; Ok(env)
}

pub fn close_info_inner(cs: &ConsoleState, view: &str, id: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| knowlu_engine::info::close_info(&cs.vault, None, Some(id), "quinn", &console_ctx(), Some(journal), None).map(|_| ()).map_err(|e| e.to_string()))
}

pub fn open_issue_inner(cs: &ConsoleState, view: &str, target: &str, categories: Vec<String>, text: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| {
        if categories.is_empty() { return Err("pick at least one category".into()); }
        let text = text.trim();
        knowlu_engine::issues::open_issue(&cs.vault, target, &categories, (!text.is_empty()).then_some(text), &console_ctx(), Some(journal), None, None).map(|_| ()).map_err(|e| e.to_string())
    })
}

pub fn resolve_issue_inner(cs: &ConsoleState, view: &str, id: &str, resolution: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| knowlu_engine::issues::address_issue(&cs.vault, id, resolution, None, &console_ctx(), Some(journal), None).map(|_| ()).map_err(|e| e.to_string()))
}

/// Runs the engine's sync OUTSIDE `cs.lock` — a pull can take seconds and must not block a
/// concurrent `state` poll — then takes the lock only to rebuild `state`. `ok` is
/// `last_error.is_none()`, not whether anything actually moved.
pub fn sync_inner(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    let out = crate::state::run_sync(cs);
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let state = build_state_value(cs, view)?;
    Ok(json!({ "ok": out.last_error.is_none(), "error": out.last_error, "state": state }))
}

/// Runs `backup::tick` OUTSIDE `cs.lock` for the same reason as `sync_inner`. A missing backup
/// folder is `Err` from `run_backup` before the engine is ever called; a folder that exists but
/// the tick itself fails against comes back `Ok` with `last_error` set.
pub fn backup_now_inner(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    // F11 (console spec §8/§9): a backup walks and copies the WHOLE working tree, which is exactly
    // what the engine's `sync` can write under it — mirror a tree mid-pull and the copy is a
    // mixture of two states. `vault_io` FIRST and scoped to the engine call, then `cs.lock` for the
    // state rebuild: the one ordering, never the reverse (see `ConsoleState::vault_io`).
    // Poison-tolerant like `run_sync`'s own hold — a backup is a read, and a writer that panicked
    // elsewhere must not turn every later backup into an error the user cannot clear without
    // restarting.
    let r = {
        let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());
        crate::state::run_backup(cs, jiff::Timestamp::now())
    };
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let state = build_state_value(cs, view)?;
    Ok(match r {
        Ok(st) if st.last_error.is_none() => json!({ "ok": true, "error": Value::Null, "state": state }),
        Ok(st) => json!({ "ok": false, "error": st.last_error, "state": state }),
        Err(e) => json!({ "ok": false, "error": e, "state": state }),
    })
}

pub fn get_settings_inner(cs: &ConsoleState) -> Result<Value, String> {
    Ok(json!({ "ok": true, "error": Value::Null, "settings": serde_json::to_value(&*cs.settings.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())? }))
}

/// Only `backup_dir` (string path; `null`/empty string clears it) and `autostart` (bool) may be
/// set here — anything else is refused by name, never silently ignored. Toggling the OS
/// autostart entry needs the `AppHandle`, which only the `#[tauri::command]` twin has, so it
/// happens there, not here.
///
/// R-T10: `serde_json::Map` iterates keys alphabetically, so a patch like `{"autostart": true,
/// "unknown": 1}` would hit `autostart` before the refusal on `unknown` — mutating the live
/// guard on a call that overall fails. The whole patch is validated into a clone first; the live
/// guard is touched only after every key is accepted AND `save` has succeeded, so a refused
/// patch leaves both the in-memory settings and the file on disk exactly as they were.
pub fn set_settings_inner(cs: &ConsoleState, patch: serde_json::Map<String, Value>) -> Result<Value, String> {
    let mut s = cs.settings.lock().map_err(|_| "lock")?;
    let mut next = s.clone();
    for (k, v) in &patch {
        match k.as_str() {
            "backup_dir" => match v {
                Value::Null => next.backup_dir = None,
                Value::String(p) => next.backup_dir = (!p.is_empty()).then(|| std::path::PathBuf::from(p)),
                _ => return Ok(json!({ "ok": false, "error": "backup_dir must be a path string or null", "settings": Value::Null })),
            },
            "autostart" => match v.as_bool() {
                Some(b) => next.autostart = b,
                None => return Ok(json!({ "ok": false, "error": "autostart must be true or false", "settings": Value::Null })),
            },
            other => return Ok(json!({ "ok": false, "error": format!("{other} is not a setting you can change here"), "settings": Value::Null })),
        }
    }
    next.save(&cs.settings_path)?;
    *s = next;
    Ok(json!({ "ok": true, "error": Value::Null, "settings": serde_json::to_value(&*s).map_err(|e| e.to_string())? }))
}

// Every MUTATING command carries `(async)` (final fix wave, B2). These are synchronous functions,
// and Tauri 2 runs a plain `#[tauri::command]` on the webview's own thread — so a write that waits
// on `vault_io` behind a sync's own network call, or on `process_approvals`, freezes the window
// itself. `(async)` moves them onto Tauri's pool instead; the page's `invoke(...).then(...)` shape
// is unchanged, and so are the `*_inner` functions the tests call. The four read commands
// (`state`, `note`, `mark_seen`, `ui_event`) and `get_settings` stay on the main thread: they take
// only `cs.lock`, they never wait on the network, and keeping them there keeps a poll cheap.
#[tauri::command] pub fn state(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String) -> Value { state_envelope(&cs, &sch, &view) }

/// The whole of what `state` answers, `Scheduler` and all, without Tauri's `State` wrappers — so a
/// test can read the exact envelope the page reads (the C1c Task 5 live-proof investigation).
pub fn state_envelope(cs: &ConsoleState, sch: &Scheduler, view: &str) -> Value {
    let mut env = state_inner(cs, view).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null }));
    if let Some(fr) = first_run_value(cs, sch) {
        // §6: a vault with no read model yet answers `ok: true` and no state, rather than an error
        // line a student can do nothing about — the page has a sentence for exactly this minute.
        // Any other failure, on a vault that has been ranked, keeps today's `ok: false`.
        if env["ok"] != true { env = json!({ "ok": true, "error": Value::Null, "state": Value::Null }); }
        env["first_run"] = fr;
    }
    let _ = attach_scheduler(&mut env, sch);
    env
}
#[tauri::command] pub fn note(cs: State<'_, ConsoleState>, id: String) -> Value { note_inner(&cs, &id).unwrap_or_else(|e| json!({ "ok": false, "error": e, "note": Value::Null })) }
#[tauri::command] pub fn mark_seen(cs: State<'_, ConsoleState>) -> Value { mark_seen_inner(&cs).unwrap_or_else(|e| json!({ "ok": false, "error": e })) }
#[tauri::command] pub fn ui_event(cs: State<'_, ConsoleState>, action: String, view: String, object_id: Option<String>, object_kind: Option<String>, ms: Option<i64>) -> Value { ui_event_inner(&cs, &action, &view, object_id, object_kind, ms) }
#[tauri::command(async)] pub fn set_fields(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, id: String, fields: serde_json::Map<String, Value>) -> Value { let mut env = set_fields_inner(&cs, &view, &id, fields).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn create_task(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, fields: serde_json::Map<String, Value>) -> Value { let mut env = create_task_inner(&cs, &view, fields).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn delete_note(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, id: String) -> Value { let mut env = delete_note_inner(&cs, &view, &id).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn decide(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, id: String, verdict: String, note: String, snooze_until: Option<String>) -> Value { let mut env = decide_inner(&cs, &view, &id, &verdict, &note, snooze_until).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn close_info(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, id: String) -> Value { let mut env = close_info_inner(&cs, &view, &id).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn open_issue(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, target: String, categories: Vec<String>, text: String) -> Value { let mut env = open_issue_inner(&cs, &view, &target, categories, &text).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn resolve_issue(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, id: String, resolution: String) -> Value { let mut env = resolve_issue_inner(&cs, &view, &id, &resolution).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn sync(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String) -> Value { let mut env = sync_inner(&cs, &view).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command(async)] pub fn backup_now(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String) -> Value { let mut env = backup_now_inner(&cs, &view).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
#[tauri::command] pub fn get_settings(cs: State<'_, ConsoleState>) -> Value { get_settings_inner(&cs).unwrap_or_else(|e| json!({ "ok": false, "error": e, "settings": Value::Null })) }
/// The only command with an `AppHandle` parameter: toggling `autostart` has to reach the OS
/// autostart entry (`tauri_plugin_autostart`), which needs the handle that `*_inner` never has.
/// The setting is saved first (`set_settings_inner`'s usual refusal rules apply), then the plugin
/// is told to match it — a plugin failure does not undo the saved setting or fail the command;
/// the settings file stays the source of truth for what Knowlu will do at next launch.
#[tauri::command(async)]
pub fn set_settings(app: tauri::AppHandle, cs: State<'_, ConsoleState>, patch: serde_json::Map<String, Value>) -> Value {
    let had_autostart = patch.contains_key("autostart");
    match set_settings_inner(&cs, patch) {
        Ok(env) => {
            if had_autostart && env["ok"] == true {
                use tauri_plugin_autostart::ManagerExt;
                let al = app.autolaunch();
                let enable = cs.settings.lock().map(|s| s.autostart).unwrap_or(true);
                let _ = if enable { al.enable() } else { al.disable() };
            }
            env
        }
        Err(e) => json!({ "ok": false, "error": e, "settings": Value::Null }),
    }
}

// ---- Plan 4a Task 7: what the settings overlay needs. R-P4a-4 — the panel is an overlay IN the
// page, never a `View` name: a view name would reach `surface::View::parse` on every poll and be
// refused. Nothing here computes either (spec §3.1); these marshal the registry, the clipboard and
// the relaunch.

/// The testable core: which registry, which vault, which name. The `#[tauri::command]` twin only
/// supplies the root and the vault from the running state.
pub fn set_profile_name_in(root: &std::path::Path, vault: &std::path::Path, name: &str) -> Result<Value, String> {
    let name = name.trim();
    if name.is_empty() { return Ok(json!({ "ok": false, "error": "a profile needs a name", "profile": Value::Null })); }
    match crate::profiles::register(root, name, vault) {
        Ok(p) => Ok(json!({ "ok": true, "error": Value::Null, "profile": serde_json::to_value(p).map_err(|e| e.to_string())? })),
        Err(e) => Ok(json!({ "ok": false, "error": e, "profile": Value::Null })),
    }
}

/// Refuses on an unreadable registry rather than writing one (review round 1, IMPORTANT 2): the
/// `?` inside `profiles::register`'s own `load` is what does it, and the `Err` arm below carries
/// its message out verbatim. Rewriting a registry that could not be read would drop every other
/// profile on the machine — the same rule Task 2 gave `register`, reached through it.
#[tauri::command(async)]
pub fn set_profile_name(cs: State<'_, ConsoleState>, name: String) -> Value {
    let Some(root) = crate::state::app_data_root() else { return json!({ "ok": false, "error": "no app data root", "profile": Value::Null }) };
    set_profile_name_in(&root, &cs.vault, &name).unwrap_or_else(|e| json!({ "ok": false, "error": e, "profile": Value::Null }))
}

/// The tray's *Copy diagnostics*, reachable from the settings panel too — the same text, through
/// the same clipboard call, so there is one diagnostics blob and not two (spec §4). The text is
/// built in Rust and never passes through the page.
#[tauri::command(async)]
pub fn copy_diagnostics(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let text = crate::tray::diagnostics_text(&cs);
    match app.clipboard().write_text(text) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// The vault-path row's Copy button. Separate from `copy_diagnostics` on purpose: this one copies
/// what the page already shows, that one copies text the page never sees.
#[tauri::command(async)]
pub fn copy_text(app: tauri::AppHandle, text: String) -> Value {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    match app.clipboard().write_text(text) { Ok(()) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e.to_string() }) }
}

/// What the panel needs that `Settings` deliberately does not carry: the vault path is `--vault`,
/// not a setting; the version is the binary's; and the profile NAME lives in `profiles.json`, not
/// in `settings.json` (S4) — so the panel reads all three from here and never guesses.
///
/// `profiles::load` separates ABSENT (`Ok(empty)` — every first launch) from UNREADABLE (`Err`),
/// and an unreadable registry must not quietly become "the vault folder's name is your profile
/// name" (review round 1, IMPORTANT 2). The row still renders — the panel is how you reach
/// diagnostics, so it has to open even when the registry is broken — but it renders the FALLBACK
/// and says so in `registry_error`, which the page shows beside it. `ok` stays true: nothing failed
/// that stops the panel working.
#[tauri::command]
pub fn settings_context(cs: State<'_, ConsoleState>) -> Value {
    let id = cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default();
    let (name, registry_error) = match crate::state::app_data_root().map(|r| crate::profiles::load(&r)) {
        Some(Ok(ps)) => (ps.into_iter().find(|p| p.id == id).map(|p| p.name), Value::Null),
        Some(Err(e)) => (None, json!(e)),
        None => (None, json!("no app data root — the profile name cannot be read")),
    };
    // Knowlu plan 3a Task 10: the wizard's offer, read ONCE. Cleared here rather than by a second
    // command, so the prompt cannot be shown twice by a page that polled twice.
    let offer = crate::onboarding::offer_marker(&cs.data_dir);
    let offer_inference = offer.is_file();
    if offer_inference { let _ = std::fs::remove_file(&offer); }
    json!({
        "ok": true, "error": Value::Null,
        "vault": cs.vault.to_string_lossy(), "version": env!("CARGO_PKG_VERSION"),
        "profile_name": name.unwrap_or_else(|| crate::profiles::default_name(&cs.vault)),
        "registry_error": registry_error,
        "offer_inference": offer_inference,
    })
}

/// *Switch profile…* (R-P4a-15): relaunch into the picker and quit. `--pick` forces the picker even
/// with one profile registered, and `relaunch` adds the `--after-pid` handshake that keeps
/// `tauri-plugin-single-instance` from killing the window that is arriving instead of the one that
/// is leaving.
///
/// **A switch is a QUIT, and takes the whole quit path** (review round 1, IMPORTANT 1): stamp
/// `settings.quit_at`, save, then `quit_flush` — push on close and back up on quit, capped at 10 s,
/// with the quit log line written from its `then` (F10). It is `tray.rs`'s `"quit"` arm, verbatim,
/// with a spawn in the middle. Skipping it lost the same work Quit protects AND left `quit_at`
/// unstamped, which is what `startup_missed` counts from — so every switch would have under-counted
/// the slots missed while the app was down.
///
/// The spawn goes BEFORE the flush so the child is already waiting on our pid (bounded at 10 s in
/// `wait_for_pid_gone`) while the flush runs. Both caps are 10 s: a flush that runs to the cap can
/// outlast the child's patience, and then the single-instance plugin closes whichever window loses
/// — the documented fallback, and better than a switch that silently discards a push.
#[tauri::command(async)]
pub fn switch_profile(cs: State<'_, ConsoleState>) -> Value {
    {
        let mut s = cs.settings.lock().unwrap_or_else(|e| e.into_inner());
        s.quit_at = Some(knowlu_engine::journal::now_ts(None));
        let _ = s.save(&cs.settings_path);
    }
    if let Err(e) = crate::onboarding::relaunch(["--pick"]) {
        return json!({ "ok": false, "error": e });
    }
    let logs = cs.data_dir.join("logs");
    crate::state::quit_flush(&cs, std::time::Duration::from_secs(10), move |q| {
        let _ = std::fs::create_dir_all(&logs);
        let stamp = knowlu_engine::journal::now_ts(None).replace(':', "");
        let _ = std::fs::write(logs.join(format!("quit-{stamp}.txt")), format!("synced={} backed_up={} timed_out={} switch=1\n", q.synced, q.backed_up, q.timed_out));
        std::process::exit(0);
    });
    json!({ "ok": true, "error": Value::Null })
}

/// The directory a staged bundle lands in: `updates\` under the app data root — **one folder for
/// the whole install, not one per profile** (`profiles::updates_dir`; Knowlu spec §6, "one bundle,
/// one version"), since an update is the same bundle whichever profile is open.
///
/// **No temp fallback** (fix round 1, m4). `main.rs` does fall back to `%TEMP%\knowlu` for the app
/// data root when `LOCALAPPDATA` is unset, and that is right for a settings file — losing settings is
/// an inconvenience. It is *not* right for an installer that will be run with this user's privileges:
/// the system temp directory is writable by every account on the machine, so staging there would put
/// the one file whose integrity the whole disk-hop argument rests on (see `updates::install_staged`)
/// somewhere anyone can replace it. With nowhere private to stage, Knowlu declines to stage at all
/// and says why; everything else about the app keeps working.
///
/// `pub(crate)` so the housekeeping tick in `scheduler.rs` calls THIS, rather than keeping a second
/// copy of the same two lines that could drift from it (final review, I3).
pub(crate) fn updates_dir() -> Result<std::path::PathBuf, String> {
    crate::state::app_data_root()
        .map(|r| crate::profiles::updates_dir(&r))
        .ok_or_else(|| "LOCALAPPDATA is not set, so there is no private folder to stage an update in".to_string())
}

/// The install-wide app-data root, where `runtime\` and `models\` live.
///
/// **One folder for the whole install, like `updates\` and unlike `judgments\`** — a llama.cpp build
/// and a `.gguf` are immutable artefacts identical for every profile, and a friend with two vaults
/// must not download two gigabytes twice. The judgment LOG is per profile, because two vaults are
/// two different sets of judgments (spec §5.4).
///
/// No temp fallback, for the same reason `updates_dir` has none (m4): a runtime is an executable
/// this app will run with the user's own privileges, and the system temp directory is writable by
/// every account on the machine. With nowhere private to put it, Knowlu declines and says why.
pub(crate) fn inference_root() -> Result<std::path::PathBuf, String> {
    crate::state::app_data_root()
        .ok_or_else(|| "LOCALAPPDATA is not set, so there is nowhere private to keep a model".to_string())
}

/// `{ok, error, version, staged, held, last_check, last_error, action_error}` — the page renders it;
/// nothing here decides anything but the mid-run gate, which lives in `updates::update_offer`.
///
/// **`async fn`, deliberately** (B4/R-P4a-12): the body must `.await` the plugin's own async check,
/// and `tauri::async_runtime::block_on` inside a command PANICS, because a command is already
/// running on that runtime. `#[tauri::command]` on an `async fn` IS the async form — `(async)` is
/// only for a synchronous body that needs moving off the webview thread. The two `_blocking` twins
/// in `updates` are for the tray arm and the housekeeping tick, which are plain std threads.
#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle, sch: State<'_, Scheduler>, up: State<'_, crate::updates::Updates>) -> Result<Value, ()> {
    // *Check now* is an explicit action, so it clears the previous action's outcome and then
    // records its own (fix round 1, IMPORTANT 2). It is also the reader's only dismissal for a tray
    // failure — which is exactly why the CLEAR lives here and not in `record_check`: the daily
    // housekeeping check must leave a tray failure standing.
    crate::updates::clear_action_error(&up);
    let result = match updates_dir() {
        // m4: nowhere private to stage is a failed check, recorded the ordinary quiet way.
        Err(e) => Err(e),
        // `up` goes in so the single-flight latch is taken inside `check_and_stage` rather than
        // here: the housekeeping tick's check and this one must not both be filling one bundle path
        // (R-P4a-24).
        Ok(dir) => crate::updates::check_and_stage(&app, &up, &dir).await,
    };
    let err = result.as_ref().err().cloned();
    // m2: a single-flight refusal is NOT a check. Nothing was fetched, so stamping `last_check`
    // would date a check that never happened — it goes to the action field instead, where the next
    // explicit action clears it.
    if err.as_deref() == Some(crate::updates::ALREADY_CHECKING) {
        crate::updates::note_error(&up, crate::updates::ALREADY_CHECKING);
    } else {
        crate::updates::record_check(&up, result, &knowlu_engine::journal::now_ts(None));
    }
    let running = *crate::scheduler::lock(&sch.running);
    let offer = crate::updates::update_offer(running, crate::scheduler::lock(&up.staged).as_ref());
    if let Some(item) = crate::scheduler::lock(&up.item).as_ref() { let _ = item.set_enabled(offer.is_some()); }
    Ok(json!({
        "ok": err.is_none(), "error": err,
        "version": env!("CARGO_PKG_VERSION"),
        "staged": offer,
        // The bundle the RUN is holding back (R-P4a-24). `staged` is the offer and is gated;
        // `held` is the fact, so the settings row can say what is really on disk instead of
        // "up to date". Exactly one of the two is ever set.
        "held": crate::updates::stage_note(running, crate::scheduler::lock(&up.staged).as_ref()),
        "last_check": crate::scheduler::lock(&up.last_check).clone(),
        "last_error": crate::scheduler::lock(&up.last_error).clone(),
        // The last ACTION's outcome (fix round 1, IMPORTANT 2). Separate from `last_error` because
        // `record_check` rewrites that one on every check, tray click or not — which is how the
        // tray's refusal used to reach nobody.
        "action_error": crate::scheduler::lock(&up.last_action_error).clone(),
    }))
}

/// *Restart to update.* Refused outright while a slot is running (R9), and it **takes** the slot
/// flag for the whole install so a slot cannot start under it (S11) — the caller is told why in
/// both cases.
///
/// `async fn` for the same reason as `check_for_updates` (R-P4a-12): the install body awaits the
/// plugin, and `block_on` here would panic.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, sch: State<'_, Scheduler>, up: State<'_, crate::updates::Updates>) -> Result<Value, ()> {
    // An install attempt supersedes the last action's outcome, whichever door it came in by — so a
    // tray refusal recorded a minute ago is cleared by trying again here (fix round 1, IMPORTANT 2).
    crate::updates::clear_action_error(&up);
    let staged = crate::scheduler::lock(&up.staged).clone();
    let Some(s) = staged else { return Ok(json!({ "ok": false, "error": "nothing staged" })) };
    let Some(_hold) = crate::updates::hold_for_install(&sch) else {
        crate::updates::note_error(&up, crate::updates::SLOT_RUNNING);
        return Ok(json!({ "ok": false, "error": crate::updates::SLOT_RUNNING }));
    };
    Ok(match crate::updates::install_staged(&app, &s).await {
        // Reached only where `Update::install` returns rather than exiting the process; the clear
        // above already holds, and this says so for the platforms that relaunch by hand.
        Ok(()) => { crate::updates::clear_action_error(&up); json!({ "ok": true, "error": Value::Null }) }
        Err(e) => { crate::updates::note_error(&up, &e); json!({ "ok": false, "error": e }) }
    })
}

// ---- Knowlu plan 3a Task 10: local judgment. Every one of these is reached from a click and from
// nowhere else (spec §5.3: "never automatic"); nothing here computes (spec §3.1) — `inference` does.

/// `{ok, error, runtime, model, model_bytes, root}` — what the settings row renders.
#[tauri::command]
pub fn inference_status() -> Value {
    match inference_root() {
        Err(e) => json!({ "ok": false, "error": e, "runtime": Value::Null, "model": Value::Null, "model_bytes": 0 }),
        Ok(root) => {
            let s = crate::inference::status(&root);
            json!({
                "ok": true, "error": Value::Null,
                "runtime": s.runtime.map(|p| p.to_string_lossy().to_string()),
                "model": s.model.map(|p| p.to_string_lossy().to_string()),
                "model_bytes": s.model_bytes,
                "root": root.to_string_lossy(),
            })
        }
    }
}

/// One place both install commands turn `kind` into a `Half` and a missing root into an envelope,
/// so neither of them grows an `else` arm of its own (M7).
fn half_and_root(kind: &str) -> Result<(crate::inference::Half, std::path::PathBuf), Value> {
    let half = crate::inference::Half::parse(kind)
        .ok_or_else(|| json!({ "ok": false, "error": format!("{kind} is not something Knowlu installs") }))?;
    let root = inference_root().map_err(|e| json!({ "ok": false, "error": e }))?;
    Ok((half, root))
}

fn installed(done: Result<std::path::PathBuf, String>) -> Value {
    match done {
        Ok(p) => json!({ "ok": true, "error": Value::Null, "path": p.to_string_lossy() }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// *Install from a file…*, for either half. **The working path until the site exists** (the
/// manifest endpoint is open item §11), and the better one for a friend on metered wifi or a campus
/// network that blocks the host: they download once, by hand, and point at it.
///
/// **It is not the unverified path** (R-P3a-2): `inference::install_from_file` sends a runtime
/// through `install_runtime_from_zip`, which checks the file's SHA-256 against the compiled-in
/// `SUPPORTED_RUNTIMES` before anything is written. A model is taken on the user's own authority,
/// for the reason the module doc gives.
///
/// `(async)` because a two-gigabyte copy and a SHA-256 over it must not run on the webview thread.
#[tauri::command(async)]
pub fn install_inference_file(kind: String, path: String) -> Value {
    let (half, root) = match half_and_root(&kind) { Ok(v) => v, Err(e) => return e };
    installed(crate::inference::install_from_file(&root, half, std::path::Path::new(&path)))
}

/// *Download*, for either half. Reached only from the row's button — no tick and no slot calls it
/// (D8) — and it marshals only: the manifest read, the capped download and the install are one
/// function in `inference`, because chaining three network-and-disk steps is computation and
/// `commands.rs` computes nothing (M7).
#[tauri::command(async)]
pub fn install_inference_download(kind: String) -> Value {
    let (half, root) = match half_and_root(&kind) { Ok(v) => v, Err(e) => return e };
    installed(crate::inference::install_from_manifest(&root, half, crate::inference::MANIFEST_URL))
}

/// Free the gigabytes. The runtime stays: it is small, and re-downloading it is the annoying half.
#[tauri::command(async)]
pub fn remove_inference_model() -> Value {
    let root = match inference_root() { Ok(r) => r, Err(e) => return json!({ "ok": false, "error": e }) };
    match crate::inference::remove_model(&root) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}
