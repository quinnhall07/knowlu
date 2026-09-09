use knowlu::updates::{record_check, update_offer, Staged, Updates};
use std::path::PathBuf;

fn staged(v: &str) -> Staged { Staged { version: v.to_string(), path: PathBuf::from("C:\\stage.bundle") } }

/// R9 (Knowlu spec decision 16): **never mid-run.** A staged update while a slot is running is not
/// offered — not dimmed, not queued behind a dialog: not offered at all, until the slot ends.
#[test]
fn a_staged_update_is_never_offered_while_a_slot_is_running() {
    let s = staged("0.2.0");
    assert_eq!(update_offer(false, Some(&s)).as_deref(), Some("0.2.0"));
    assert_eq!(update_offer(true, Some(&s)), None, "a slot is running — no offer");
    assert_eq!(update_offer(false, None), None, "nothing staged, nothing offered");
    assert_eq!(update_offer(true, None), None);
}

/// An unreachable endpoint is the NORMAL state until the site exists: one quiet record, no dialog,
/// no offer, and the next check is not blocked by it.
#[test]
fn an_unreachable_endpoint_is_recorded_quietly_and_offers_nothing() {
    let u = Updates::default();
    record_check(&u, Err("error sending request for url".to_string()), "2026-09-05T12:00:00Z");
    assert!(u.staged.lock().unwrap().is_none());
    assert_eq!(u.last_check.lock().unwrap().as_deref(), Some("2026-09-05T12:00:00Z"));
    assert!(u.last_error.lock().unwrap().as_deref().unwrap().contains("error sending request"));
    assert_eq!(update_offer(false, u.staged.lock().unwrap().as_ref()), None);

    // A later success clears the error and stages the version.
    record_check(&u, Ok(Some(staged("0.2.0"))), "2026-09-06T12:00:00Z");
    assert!(u.last_error.lock().unwrap().is_none(), "a good check clears the last error");
    assert_eq!(update_offer(false, u.staged.lock().unwrap().as_ref()).as_deref(), Some("0.2.0"));

    // "No update available" is a success that stages nothing and must not un-stage silently…
    record_check(&u, Ok(None), "2026-09-07T12:00:00Z");
    assert!(u.staged.lock().unwrap().is_none(), "…except that a manifest without it means it is gone");
}

/// S11: an install is exactly as exclusive as a slot. `hold_for_install` takes `Scheduler.running`
/// for the whole install, so a tick that comes due mid-install finds the flag set and stands down —
/// and the hold is released on drop, including on the error path.
#[test]
fn an_install_holds_the_slot_flag_and_gives_it_back() {
    use knowlu::scheduler::{lock, Scheduler};
    use knowlu::updates::hold_for_install;
    let sch = Scheduler::default();
    assert!(!*lock(&sch.running));
    {
        let hold = hold_for_install(&sch).expect("a free scheduler grants the hold");
        assert!(*lock(&sch.running), "a slot cannot start while an install is running");
        assert!(hold_for_install(&sch).is_none(), "and neither can a second install");
        assert_eq!(update_offer(*lock(&sch.running), Some(&staged("0.2.0"))), None, "nor is a second offer made");
        drop(hold);
    }
    assert!(!*lock(&sch.running), "the flag is given back");
}

/// R-P4a-24, single flight. *Check now* and the daily housekeeping check can be asked for at the
/// same moment, and both download into ONE `updates\` folder toward ONE bundle path. The second is
/// refused outright — not queued, not started beside the first — and the latch comes back on drop
/// so the refusal is never sticky. No network: this is the decision, held on its own.
#[test]
fn a_second_check_is_refused_while_one_is_in_flight() {
    use knowlu::updates::begin_check;
    let u = Updates::default();
    {
        let flight = begin_check(&u).expect("a fresh Updates grants the first check");
        assert!(begin_check(&u).is_none(), "a second check while one is downloading is refused");
        assert!(begin_check(&u).is_none(), "and stays refused for as long as the first holds it");
        drop(flight);
    }
    assert!(begin_check(&u).is_some(), "the latch is given back, so tomorrow's check is not blocked");
}

/// R-P4a-24, temp-then-rename. The bundle path must never hold a partial file: `install_staged`
/// hands whatever is at that path straight to the installer, and a truncated NSIS package is a
/// failed update at the worst possible moment. The partial lives at a `.part<pid>` name and is gone
/// by the time the real path exists — including when the real path was already occupied.
#[test]
fn a_staged_bundle_is_written_through_a_temp_name_and_renamed() {
    use knowlu::updates::stage_bytes;
    let dir = std::env::temp_dir().join(format!("knowlu-stage-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("knowlu-0.2.0.bundle");

    stage_bytes(&path, b"first").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"first");
    // A second stage of the same version REPLACES it rather than failing or appending: one bundle,
    // one version (R9).
    stage_bytes(&path, b"second-and-longer").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second-and-longer");

    // Nothing but the bundle is left in the folder — no `.part` from either write.
    let left: Vec<String> = std::fs::read_dir(&dir).unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["knowlu-0.2.0.bundle".to_string()], "a partial was left behind: {left:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// R-P4a-24, the observable. `update_offer` withholds the OFFER while a slot runs; `stage_note` is
/// what stops the settings row saying "up to date" over a bundle that is already downloaded and
/// verified. Exactly one of the two is ever `Some`.
#[test]
fn a_bundle_held_back_by_a_run_is_still_reported_as_staged() {
    use knowlu::updates::stage_note;
    let s = staged("0.2.0");
    assert_eq!(stage_note(true, Some(&s)).as_deref(), Some("0.2.0"), "held by the run, and said so");
    assert_eq!(update_offer(true, Some(&s)), None, "…while the offer itself stays away (R9)");
    assert_eq!(stage_note(false, Some(&s)), None, "no run holding it: the OFFER is the report");
    assert_eq!(update_offer(false, Some(&s)).as_deref(), Some("0.2.0"));
    assert_eq!(stage_note(true, None), None, "nothing staged is nothing held");
    assert_eq!(stage_note(false, None), None);
}

/// R-P4a-24 as fix round 1 corrected it (IMPORTANT 2). The tray's *Restart to update* arm has no
/// envelope to answer in, so its refusal is written to state — and the first round wrote it to
/// `last_error`, which `record_check` rewrites on **every** check. `bootConsole` runs one, so the
/// message reached nobody. It now lives in `last_action_error`, which `record_check` never touches,
/// and this test pins the SURVIVAL that the first round's version wrongly pinned as clearing.
#[test]
fn an_action_failure_survives_every_check_until_the_next_action() {
    use knowlu::updates::{clear_action_error, note_error, SLOT_RUNNING};
    let u = Updates::default();
    record_check(&u, Ok(Some(staged("0.2.0"))), "2026-09-06T12:00:00Z");

    // The tray click is refused: the action field alone moves.
    note_error(&u, SLOT_RUNNING);
    assert_eq!(u.last_action_error.lock().unwrap().as_deref(), Some(SLOT_RUNNING));
    assert!(u.last_error.lock().unwrap().is_none(), "an action failure is not the check's error");
    assert_eq!(u.last_check.lock().unwrap().as_deref(), Some("2026-09-06T12:00:00Z"), "no check happened");
    assert_eq!(u.staged.lock().unwrap().as_ref().map(|s| s.version.clone()), Some("0.2.0".to_string()),
               "a refused install does not un-stage the bundle");

    // THE POINT: a later check — the daily one, or the one every boot runs — does not wipe it.
    record_check(&u, Ok(Some(staged("0.2.0"))), "2026-09-07T12:00:00Z");
    assert_eq!(u.last_action_error.lock().unwrap().as_deref(), Some(SLOT_RUNNING),
               "a successful check must not silently dismiss the tray's failure");
    // …and a failed check does not wipe it either, nor does it borrow it.
    record_check(&u, Err("error sending request for url".to_string()), "2026-09-08T12:00:00Z");
    assert_eq!(u.last_action_error.lock().unwrap().as_deref(), Some(SLOT_RUNNING));
    assert_eq!(u.last_error.lock().unwrap().as_deref(), Some("error sending request for url"));

    // Only the next explicit action clears it — `check_for_updates`, `install_update` and the tray
    // arm all call this on the way in.
    clear_action_error(&u);
    assert!(u.last_action_error.lock().unwrap().is_none());
    assert_eq!(u.last_error.lock().unwrap().as_deref(), Some("error sending request for url"),
               "clearing the action says nothing about the check");
}

/// Fix round 1, IMPORTANT 3: a check must not re-download a version that is already on disk. Every
/// daily check used to fetch the whole installer again, so a staged update left un-installed for a
/// week cost a week of downloads to arrive at the identical bytes. Both halves have to hold — the
/// recorded `Staged` names this version AND the file is still there.
#[test]
fn a_version_already_on_disk_is_not_downloaded_again() {
    use knowlu::updates::already_staged;
    let dir = std::env::temp_dir().join(format!("knowlu-already-staged-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("knowlu-0.2.0.bundle");
    let s = Staged { version: "0.2.0".to_string(), path: path.clone() };

    assert_eq!(already_staged(None, "0.2.0"), None, "nothing staged: download it");
    assert_eq!(already_staged(Some(&s), "0.2.0"), None, "recorded but the file is gone: download it");

    std::fs::write(&path, b"bundle").unwrap();
    let hit = already_staged(Some(&s), "0.2.0").expect("this exact version is on disk");
    assert_eq!(hit.version, "0.2.0");
    assert_eq!(hit.path, path, "the caller gets back the Staged it should return, not a rebuilt one");
    assert_eq!(already_staged(Some(&s), "0.3.0"), None, "a newer release is a different bundle");

    let _ = std::fs::remove_dir_all(&dir);
}
