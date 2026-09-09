//! The starvation report is a PowerShell script, so this test drives it the way a person does and
//! asserts on what it prints. Read-only by construction: the fixture it runs against is a temp
//! directory this test builds and deletes.
use std::path::{Path, PathBuf};

fn vault(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qo-starve-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("state").join("journal")).unwrap();
    std::fs::create_dir_all(d.join("tasks")).unwrap();
    d
}

/// m6 (final fix wave): the script buckets by `[datetime]::UtcNow.Date` now, because the journal's
/// own `ts` is UTC (`journal::now_ts`) — so this fixture's notion of "today" has to be UTC too, or
/// the two disagree by a day for roughly a third of the clock (verified live: 2026-09-07 19:55 CDT
/// is already 2026-09-08 UTC). `jiff::Zoned::now().date()` is the machine's LOCAL date and would be
/// exactly that stale assumption.
fn today() -> jiff::civil::Date {
    jiff::Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC).date()
}

fn line(ts: &str, actor: &str, field: &str) -> String {
    format!(
        "{{\"actor\": \"{actor}\", \"device\": \"d\", \"evidence\": null, \"field\": \"{field}\", \"id\": \"task_0123456789\", \"new\": 2, \"old\": 1, \"op\": \"set\", \"path\": \"tasks/t.md\", \"run_id\": null, \"seq\": 1, \"ts\": \"{ts}\", \"via\": \"cloud-routine\"}}"
    )
}

fn run(vault: &Path, days: &str) -> (i32, String) {
    // m7 (final fix wave): a relative path passes only when `cargo test` runs from the repo root —
    // `env!("CARGO_MANIFEST_DIR")` is this crate's root at compile time, so the script is found no
    // matter the process's current directory when the test binary runs. The script lives one
    // level up, in the workspace root's `scripts/`.
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("scripts").join("starvation.ps1");
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
            "-File", &script.to_string_lossy(),
            "-Vault", &vault.to_string_lossy(),
            "-Days", days,
        ])
        .output()
        .expect("powershell");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

/// A day on which BOTH producers wrote is the state during the overlap; a day on which only the app
/// wrote is what "starved" looks like; the verdict counts back from today.
#[test]
fn the_report_separates_the_two_producers_and_counts_the_starved_days() {
    let v = vault("both");
    let today = today();
    let day = |n: i64| today.checked_sub(jiff::Span::new().days(n)).unwrap().to_string();
    std::fs::write(
        v.join("state").join("journal").join(format!("{}.jsonl", day(9))),
        format!("{}\n{}\n", line(&format!("{}T13:00:00Z", day(9)), "agent:routine.enrich", "importance"),
                             line(&format!("{}T18:00:00Z", day(9)), "agent:knowlu.enrich", "importance")),
    ).unwrap();
    std::fs::write(
        v.join("state").join("journal").join(format!("{}.jsonl", day(1))),
        format!("{}\n", line(&format!("{}T18:00:00Z", day(1)), "agent:knowlu.enrich", "effort_hours")),
    ).unwrap();

    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    // **Not `contains("routine")` / `contains("knowlu")`** — the table's own header line carries
    // both words whatever the journal holds, so those asserted nothing (M4). Assert the ROWS.
    let row = |day: String, r: &str, k: &str| {
        let want = format!("{day}   {r:>7}  {k:>6}");
        assert!(out.contains(&want), "expected row {want:?} in:\n{out}");
    };
    row(day(9), "1", "1");
    row(day(1), "0", "1");
    row(day(0), "0", "0");
    // The most recent routine write was nine days ago, so the verdict counts from there.
    assert!(out.contains("9 day(s) ago"), "the verdict must count from the last routine write: {out}");
    // `contains("starved")` is satisfied by NOT STARVED (M4). The verdict token carries its colon.
    assert!(out.contains("STARVED: seven or more days"), "{out}");
    assert!(!out.contains("NOT STARVED"), "{out}");
    let _ = std::fs::remove_dir_all(&v);
}

/// A routine write today is the opposite verdict, and it must be unambiguous — turning the routine
/// off while it is still doing work is the mistake this report exists to prevent.
#[test]
fn a_routine_write_today_says_do_not_turn_it_off_yet() {
    let v = vault("busy");
    let today = today().to_string();
    std::fs::write(
        v.join("state").join("journal").join(format!("{today}.jsonl")),
        format!("{}\n", line(&format!("{today}T13:00:00Z"), "agent:routine.enrich", "course")),
    ).unwrap();
    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("NOT STARVED"), "{out}");
    assert!(!out.contains("STARVED: seven"), "{out}");
    assert!(out.contains("0 day(s) ago"), "a write today is zero days ago: {out}");
    let _ = std::fs::remove_dir_all(&v);
}

/// The queue itself is part of the picture: nothing flagged means neither producer has anything to
/// do, which is not the same as the routine having been beaten to it.
#[test]
fn the_report_counts_what_is_still_flagged() {
    let v = vault("queue");
    std::fs::write(
        v.join("tasks").join("a.md"),
        "---\ntitle: A\nstatus: active\nneeds_enrichment: true\nid: task_0000000001\n---\n\nB.\n",
    ).unwrap();
    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 task(s) still flagged needs_enrichment: true"), "{out}");
    assert!(out.contains("the routine has never enriched anything"), "an empty journal is its own verdict: {out}");
    let _ = std::fs::remove_dir_all(&v);
}

/// A vault with no journal at all is a report, not a crash.
#[test]
fn an_empty_vault_reports_rather_than_failing() {
    let v = vault("empty");
    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    assert!(!out.trim().is_empty());
    let _ = std::fs::remove_dir_all(&v);
}
