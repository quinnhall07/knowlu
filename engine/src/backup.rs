//! Backup (Knowlu spec §4): a live **mirror** of the vault's folders under a folder the user
//! owns, copied on content change with temp-and-rename, **never deleting** — a source that
//! disappears moves to the mirror's own `archive/` — plus one dated zip **snapshot** a day,
//! `SNAPSHOTS_KEPT` retained. Failure is a status, never a panic; the console paints it amber.
//! No backup holds class-search data (replan §6.12, C2e): the series file is copied filtered, a
//! leftover `state/calendar-series.json*` beside it is never copied, and one an older mirror took
//! is the one file the mirror deletes rather than archives.
//! The console is the only caller. Credentials are never a rule here: the vault does not hold
//! them (the keychain does); `config/` is copied as the user's own data to the user's own storage.
use std::io::Write;
use std::path::{Path, PathBuf};
use serde_json::json;

use crate::ids;

/// Every note folder (`ids::NOTE_FOLDERS` — the single source of truth for what a note lives
/// under) plus the three non-note folders a backup also owns: `state` (generated, but the
/// user's own history of it), `config` (the user's own settings, never credentials — those are
/// the keychain's), and `profile`. Derived by index rather than duplicated so the two arrays
/// cannot drift out of sync by hand; `note_folders_is_a_subset_of_backup_folders` below still
/// checks it at runtime in case a future edit to either array quietly breaks the derivation's
/// assumptions (e.g. reordering one without the other).
pub const BACKUP_FOLDERS: [&str; 11] = [
    ids::NOTE_FOLDERS[0], ids::NOTE_FOLDERS[1], ids::NOTE_FOLDERS[2],
    ids::NOTE_FOLDERS[3], ids::NOTE_FOLDERS[4], ids::NOTE_FOLDERS[5],
    ids::NOTE_FOLDERS[6], ids::NOTE_FOLDERS[7],
    "state", "config", "profile",
];
pub const SNAPSHOTS_KEPT: usize = 30;
const TMP_MARK: &str = ".tmp-";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MirrorReport { pub copied: usize, pub unchanged: usize, pub archived: usize, pub errors: Vec<String> }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BackupStatus { pub last_ok: Option<String>, pub behind_days: Option<i64>, pub last_error: Option<String>, pub target_reachable: bool }

fn profile_root(target: &Path, profile_id: &str) -> PathBuf { target.join(profile_id) }

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() { let p = e.path(); if p.is_dir() { walk(&p, out) } else { out.push(p) } }
    }
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(ma), Ok(mb)) if ma.len() == mb.len() => std::fs::read(a).ok() == std::fs::read(b).ok(),
        _ => false,
    }
}

/// What a backup does with one vault file (replan §6.12, C2e), for `mirror` and `snapshot` alike.
enum Take {
    /// Copied as today, byte for byte.
    Whole,
    /// The series file: the bytes [`crate::commitments::series_for_backup`] gives, with no
    /// class-search data in them.
    Filtered(Vec<u8>),
    /// The series file when those bytes could not be had: skipped this run but still wanted, so
    /// `mirror` never archives the older copy.
    Skipped,
    /// A leftover `state/calendar-series.json*` beside the file (`write_state_file`'s
    /// `.tmp<pid>`, which `TMP_MARK` does not catch): never copied, never wanted.
    Never,
}

/// The one per-path rule (§6.12); `rel` is vault-relative.
fn take(vault: &Path, rel: &Path) -> Take {
    if rel == Path::new(crate::commitments::SERIES_FILE) {
        return crate::commitments::series_for_backup(vault).map_or(Take::Skipped, Take::Filtered);
    }
    if is_series_leftover(rel) { Take::Never } else { Take::Whole }
}

/// `state/<name>` where `name` starts `calendar-series.json` and is not the file itself.
fn is_series_leftover(rel: &Path) -> bool {
    rel.parent() == Some(Path::new("state"))
        && rel != Path::new(crate::commitments::SERIES_FILE)
        && rel.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("calendar-series.json"))
}

/// Temp beside `dst`, filled by `fill`, then renamed over it.
fn place_with(dst: &Path, fill: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
    if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?; }
    let tmp = dst.with_file_name(format!("{}{TMP_MARK}{}", dst.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
    fill(&tmp).inspect_err(|_| { let _ = std::fs::remove_file(&tmp); })?;
    std::fs::rename(&tmp, dst).map_err(|e| { let _ = std::fs::remove_file(&tmp); format!("rename {}: {e}", dst.display()) })
}

fn place(src: &Path, dst: &Path) -> Result<(), String> {
    place_with(dst, |tmp| std::fs::copy(src, tmp).map(|_| ()).map_err(|e| format!("copy {}: {e}", src.display())))
}

fn place_bytes(bytes: &[u8], dst: &Path) -> Result<(), String> {
    place_with(dst, |tmp| std::fs::write(tmp, bytes).map_err(|e| format!("write {}: {e}", dst.display())))
}

fn free_name(dir: &Path, name: &str) -> PathBuf {
    let mut p = dir.join(name);
    let (stem, ext) = match name.rsplit_once('.') { Some((s, e)) => (s.to_string(), format!(".{e}")), None => (name.to_string(), String::new()) };
    let mut n = 2;
    while p.exists() { p = dir.join(format!("{stem}-{n}{ext}")); n += 1; }
    p
}

pub fn mirror(vault: &Path, target: &Path, profile_id: &str) -> Result<MirrorReport, String> {
    let root = profile_root(target, profile_id).join("vault");
    std::fs::create_dir_all(&root).map_err(|e| format!("backup target {}: {e}", root.display()))?;
    let mut rep = MirrorReport::default();
    // 1. sweep leftover temps from a dead process
    let mut existing = Vec::new(); walk(&root, &mut existing);
    for p in &existing { if p.file_name().map(|n| n.to_string_lossy().contains(TMP_MARK)).unwrap_or(false) { let _ = std::fs::remove_file(p); } }
    // 2. copy on content change
    let mut wanted = std::collections::BTreeSet::new();
    for folder in BACKUP_FOLDERS {
        let src_dir = vault.join(folder);
        if !src_dir.is_dir() { continue; }
        let mut files = Vec::new(); walk(&src_dir, &mut files);
        for src in files {
            let rel = src.strip_prefix(vault).map_err(|e| e.to_string())?.to_path_buf();
            let dst = root.join(&rel);
            let placed = match take(vault, &rel) {
                Take::Never => continue,
                Take::Skipped => { wanted.insert(rel); continue; }
                // Compared filtered-to-destination, so a second tick copies nothing (§6.12).
                Take::Filtered(bytes) if std::fs::read(&dst).is_ok_and(|d| d == bytes) => None,
                Take::Filtered(bytes) => Some(place_bytes(&bytes, &dst)),
                Take::Whole if same_bytes(&src, &dst) => None,
                Take::Whole => Some(place(&src, &dst)),
            };
            wanted.insert(rel);
            match placed { None => rep.unchanged += 1, Some(Ok(())) => rep.copied += 1, Some(Err(e)) => rep.errors.push(e) }
        }
    }
    // 3. archive what the vault no longer has (never under the mirror's own archive/)
    let mut mirrored = Vec::new(); walk(&root, &mut mirrored);
    for m in mirrored {
        let rel = m.strip_prefix(&root).map_err(|e| e.to_string())?.to_path_buf();
        if rel.starts_with("archive") || wanted.contains(&rel) { continue; }
        // A leftover series copy an older mirror took can hold class-search data: deleted, never
        // archived, where it would outlive the purge (C2e, ruling 1).
        if is_series_leftover(&rel) {
            if let Err(e) = std::fs::remove_file(&m) { rep.errors.push(format!("remove {}: {e}", m.display())); }
            continue;
        }
        let dst_dir = root.join("archive").join(rel.parent().unwrap_or(Path::new("")));
        if std::fs::create_dir_all(&dst_dir).is_ok() {
            let dst = free_name(&dst_dir, &rel.file_name().unwrap_or_default().to_string_lossy());
            match std::fs::rename(&m, &dst) { Ok(()) => rep.archived += 1, Err(e) => rep.errors.push(format!("archive {}: {e}", m.display())) }
        }
    }
    Ok(rep)
}

/// Remove any leftover `*.tmp-<pid>` file directly under `dir` — a crash between `File::create`
/// and the final `rename` in either `mirror`'s `place()` or `snapshot()` leaves one of these
/// behind, and unlike `mirror` (which walks its own tree on every call), `snapshot`'s temp file
/// lives beside a `.zip` whose extension-based filter (`prune_snapshots`) would never match a
/// name like `2026-09-04.zip.tmp-1234` — so it has to be swept here instead, non-recursively,
/// on every call, before anything else touches the directory.
fn sweep_stray_temps(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() && p.file_name().map(|n| n.to_string_lossy().contains(TMP_MARK)).unwrap_or(false) {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

pub fn snapshot(vault: &Path, target: &Path, profile_id: &str, today: jiff::civil::Date) -> Result<PathBuf, String> {
    let dir = profile_root(target, profile_id).join("snapshots");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    sweep_stray_temps(&dir);
    let out = dir.join(format!("{today}.zip"));
    if out.exists() { return Ok(out); }
    let tmp = dir.join(format!("{today}.zip{TMP_MARK}{}", std::process::id()));
    {
        let file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for folder in BACKUP_FOLDERS {
            let src_dir = vault.join(folder);
            if !src_dir.is_dir() { continue; }
            let mut files = Vec::new(); walk(&src_dir, &mut files);
            for src in files {
                let rel = src.strip_prefix(vault).map_err(|e| e.to_string())?;
                let bytes = match take(vault, rel) {
                    Take::Never | Take::Skipped => continue,
                    Take::Filtered(bytes) => bytes,
                    Take::Whole => std::fs::read(&src).map_err(|e| e.to_string())?,
                };
                zip.start_file(rel.to_string_lossy().replace('\\', "/"), opts).map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, &out).map_err(|e| { let _ = std::fs::remove_file(&tmp); e.to_string() })?;
    Ok(out)
}

pub fn prune_snapshots(target: &Path, profile_id: &str, keep: usize) -> Result<Vec<PathBuf>, String> {
    let dir = profile_root(target, profile_id).join("snapshots");
    let mut zips: Vec<PathBuf> = std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "zip").unwrap_or(false)).collect();
    zips.sort();
    let mut removed = Vec::new();
    while zips.len() > keep { let p = zips.remove(0); std::fs::remove_file(&p).map_err(|e| e.to_string())?; removed.push(p); }
    Ok(removed)
}

fn status_path(target: &Path, profile_id: &str) -> PathBuf { profile_root(target, profile_id).join("status.json") }

pub fn status(target: &Path, profile_id: &str, now: jiff::Timestamp) -> BackupStatus {
    let reachable = target.is_dir();
    let doc = std::fs::read_to_string(status_path(target, profile_id)).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let last_ok = doc.as_ref().and_then(|d| d.get("last_ok")).and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_error = doc.as_ref().and_then(|d| d.get("last_error")).and_then(|v| v.as_str()).map(|s| s.to_string());
    let behind_days = last_ok.as_deref().and_then(|s| s.parse::<jiff::Timestamp>().ok()).map(|t| (now.as_second() - t.as_second()) / 86_400);
    BackupStatus { last_ok, behind_days, last_error, target_reachable: reachable }
}

pub fn tick(vault: &Path, target: &Path, profile_id: &str, now: jiff::Timestamp, today: jiff::civil::Date) -> BackupStatus {
    let stamp = crate::journal::now_ts(Some(now));
    let result = mirror(vault, target, profile_id)
        .and_then(|r| if r.errors.is_empty() { Ok(()) } else { Err(r.errors.join("; ")) })
        .and_then(|_| snapshot(vault, target, profile_id, today).map(|_| ()))
        .and_then(|_| prune_snapshots(target, profile_id, SNAPSHOTS_KEPT).map(|_| ()));
    let prior = status(target, profile_id, now);
    let (last_ok, last_error) = match result { Ok(()) => (Some(stamp), None), Err(e) => (prior.last_ok, Some(e)) };
    let doc = json!({ "last_ok": last_ok, "last_error": last_error });
    let reachable = target.is_dir();
    if reachable {
        if let Some(p) = status_path(target, profile_id).parent() { let _ = std::fs::create_dir_all(p); }
        let _ = std::fs::write(status_path(target, profile_id), crate::ledger::dumps_value(&doc));
    }
    // Built in-memory rather than re-read from disk: when `target` is unreachable nothing was
    // ever written, and a disk re-read would silently discard `last_error` (an unreachable
    // target must never look like a target with no history — see the amber-status test).
    let behind_days = last_ok.as_deref().and_then(|s| s.parse::<jiff::Timestamp>().ok()).map(|t| (now.as_second() - t.as_second()) / 86_400);
    BackupStatus { last_ok, behind_days, last_error, target_reachable: reachable }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `std::process::id()` alone is identical for every test in this binary — the default
    // multi-threaded test runner then races two tests over the same directory. Fold in the
    // thread id too (stable for the lifetime of one test function) so each test gets its own.
    fn unique_dir_suffix() -> String { format!("{}-{:?}", std::process::id(), std::thread::current().id()) }
    fn fixture() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-backup-src-{}", unique_dir_suffix()));
        let _ = std::fs::remove_dir_all(&d);
        copy_tree(std::path::Path::new("tests/fixtures/vault-full"), &d);
        d
    }
    fn target() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-backup-dst-{}", unique_dir_suffix()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy_tree(&p, &t) } else { std::fs::copy(&p, &t).unwrap(); } }
    }
    fn tree_bytes(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut out = std::collections::BTreeMap::new();
        fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() { let p = e.path(); if p.is_dir() { walk(root, &p, out) } else { out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read(&p).unwrap()); } }
        }
        walk(root, root, &mut out); out
    }

    #[test]
    fn the_mirror_is_byte_identical_for_every_backup_folder_and_a_second_pass_copies_nothing() {
        let (v, t) = (fixture(), target());
        let r = mirror(&v, &t, "p1").unwrap();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let m = t.join("p1").join("vault");
        for f in BACKUP_FOLDERS { if v.join(f).is_dir() { assert_eq!(tree_bytes(&v.join(f)), tree_bytes(&m.join(f)), "{f}"); } }
        let again = mirror(&v, &t, "p1").unwrap();
        assert_eq!(again.copied, 0); assert!(again.unchanged > 0);
    }

    #[test]
    fn a_removed_note_moves_to_the_mirrors_archive_and_nothing_is_ever_deleted() {
        let (v, t) = (fixture(), target());
        mirror(&v, &t, "p1").unwrap();
        let victim = std::fs::read_dir(v.join("tasks")).unwrap().flatten().next().unwrap().path();
        let name = victim.file_name().unwrap().to_owned();
        std::fs::remove_file(&victim).unwrap();
        let r = mirror(&v, &t, "p1").unwrap();
        assert_eq!(r.archived, 1);
        assert!(!t.join("p1/vault/tasks").join(&name).exists());
        assert!(t.join("p1/vault/archive/tasks").join(&name).exists(), "the mirror keeps its own archive");
    }

    #[test]
    fn a_crash_between_temp_and_rename_never_leaves_a_half_note_at_the_final_path() {
        let (v, t) = (fixture(), target());
        mirror(&v, &t, "p1").unwrap();
        let m = t.join("p1/vault/tasks");
        std::fs::write(m.join("stray.md.tmp-999"), "half").unwrap();   // a leftover temp from a dead process
        let r = mirror(&v, &t, "p1").unwrap();
        assert!(r.errors.is_empty());
        assert!(!m.join("stray.md.tmp-999").exists(), "leftover temps are swept");
        assert!(!m.join("stray.md").exists());
    }

    #[test]
    fn snapshots_are_one_per_day_and_pruned_to_the_newest_thirty() {
        let (v, t) = (fixture(), target());
        let today: jiff::civil::Date = "2026-09-04".parse().unwrap();
        let z = snapshot(&v, &t, "p1", today).unwrap();
        assert!(z.ends_with("snapshots/2026-09-04.zip") || z.ends_with("snapshots\\2026-09-04.zip"));
        let size = std::fs::metadata(&z).unwrap().len();
        assert_eq!(snapshot(&v, &t, "p1", today).unwrap(), z, "idempotent per day");
        assert_eq!(std::fs::metadata(&z).unwrap().len(), size);
        for i in 1..=35u32 { std::fs::write(t.join("p1/snapshots").join(format!("2026-07-{:02}.zip", i.min(31))), b"x").unwrap(); }
        let removed = prune_snapshots(&t, "p1", SNAPSHOTS_KEPT).unwrap();
        let left = std::fs::read_dir(t.join("p1/snapshots")).unwrap().count();
        assert_eq!(left, SNAPSHOTS_KEPT);
        assert!(removed.iter().all(|p| p.file_name().unwrap().to_string_lossy().as_ref() < "2026-07-03"));
    }

    #[test]
    fn an_unreachable_target_is_amber_in_status_never_an_error_out_of_tick() {
        let v = fixture();
        let gone = std::path::Path::new(r"Q:\no-such-drive\knowlu-backup");
        let now: jiff::Timestamp = "2026-09-04T18:00:00Z".parse().unwrap();
        let s = tick(&v, gone, "p1", now, "2026-09-04".parse().unwrap());
        assert!(!s.target_reachable); assert!(s.last_error.is_some()); assert!(s.last_ok.is_none());
    }

    #[test]
    fn status_reports_days_behind_from_the_status_file() {
        let (v, t) = (fixture(), target());
        let then: jiff::Timestamp = "2026-09-01T18:00:00Z".parse().unwrap();
        tick(&v, &t, "p1", then, "2026-09-01".parse().unwrap());
        let s = status(&t, "p1", "2026-09-04T18:00:00Z".parse().unwrap());
        assert_eq!(s.behind_days, Some(3));
        assert!(s.target_reachable);
    }

    #[test]
    fn note_folders_is_a_subset_of_backup_folders() {
        for f in crate::ids::NOTE_FOLDERS {
            assert!(BACKUP_FOLDERS.contains(&f), "{f} (from ids::NOTE_FOLDERS) missing from BACKUP_FOLDERS");
        }
    }

    #[test]
    fn backup_folders_is_every_note_folder_in_order_then_the_three_it_also_owns() {
        // Grades spec §7: `grades/` is backed up like every other note folder. Stated against
        // `NOTE_FOLDERS` itself rather than a count, so the array's length and order are pinned
        // whichever folder is added next.
        let n = crate::ids::NOTE_FOLDERS.len();
        assert_eq!(BACKUP_FOLDERS.len(), n + 3, "{BACKUP_FOLDERS:?}");
        assert_eq!(BACKUP_FOLDERS[..n], crate::ids::NOTE_FOLDERS[..], "{BACKUP_FOLDERS:?}");
        assert_eq!(BACKUP_FOLDERS[n..], ["state", "config", "profile"], "{BACKUP_FOLDERS:?}");
        assert!(BACKUP_FOLDERS.contains(&"grades"), "{BACKUP_FOLDERS:?}");
    }

    #[test]
    fn backup_folders_has_eleven_entries_with_commitments_then_grades_before_state() {
        // The commitment model's pin, widened by the grades merge (M1 plan, "Merging with
        // `p1-commitments`"): `…, info, commitments, grades, state, config, profile`.
        assert_eq!(BACKUP_FOLDERS.len(), 11);
        assert_eq!(BACKUP_FOLDERS[6], "commitments");
        assert_eq!(BACKUP_FOLDERS[7], "grades");
        assert_eq!(BACKUP_FOLDERS[8], "state");
    }

    #[test]
    fn a_stray_snapshot_temp_file_is_swept_and_todays_real_snapshot_still_gets_written() {
        let (v, t) = (fixture(), target());
        let dir = t.join("p1/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        let stray = dir.join("2026-09-04.zip.tmp-999");
        std::fs::write(&stray, "half").unwrap(); // a leftover temp from a dead process
        let today: jiff::civil::Date = "2026-09-04".parse().unwrap();
        let z = snapshot(&v, &t, "p1", today).unwrap();
        assert!(!stray.exists(), "leftover snapshot temps are swept");
        assert!(z.exists());
        assert!(std::fs::metadata(&z).unwrap().len() > 0);
    }

    #[test]
    fn a_stray_snapshot_temp_file_is_swept_by_tick_too() {
        let (v, t) = (fixture(), target());
        let dir = t.join("p1/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        let stray = dir.join("2026-09-04.zip.tmp-999");
        std::fs::write(&stray, "half").unwrap();
        let now: jiff::Timestamp = "2026-09-04T18:00:00Z".parse().unwrap();
        let s = tick(&v, &t, "p1", now, "2026-09-04".parse().unwrap());
        assert!(!stray.exists(), "leftover snapshot temps are swept via tick");
        assert!(s.target_reachable);
        assert!(s.last_error.is_none(), "{:?}", s.last_error);
        assert!(dir.join("2026-09-04.zip").exists());
    }
}

#[cfg(test)]
mod class_search_backup_tests {
    //! C2e (onboarding replan §6.12, RQ6, RQ14): a backup never holds class-search data, in the
    //! mirror or in a snapshot zip, and holds everything else exactly as before. Each vault is
    //! built here from invented data; no fixture is read. `restore_vault_in` (`app/`) copies a
    //! mirror and filters nothing, so what a restore can bring back is what the mirror holds.
    use super::*;
    use std::collections::BTreeMap;
    use std::io::Read;

    const SERIES: &str = "state/calendar-series.json";
    const TMP: &str = "state/calendar-series.json.tmp4242";
    const REG_CAL: &str = "registrar:ua:202640";

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    impl std::ops::Deref for Scratch {
        type Target = PathBuf;
        fn deref(&self) -> &PathBuf { &self.0 }
    }

    fn scratch(name: &str, what: &str) -> Scratch {
        let d = std::env::temp_dir().join(format!("knowlu-c2e-{}-{name}-{what}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }

    fn series(uid: &str, cal: &str) -> serde_json::Value {
        json!({
            "calendar": cal, "event_type": null, "first": "2026-08-24", "has_master": true, "instances": [],
            "last_seen": "2026-09-10", "meets": [{"days": ["mon", "wed"], "end": "12:50", "start": "12:00"}],
            "rdate": false, "rule": {"count": null, "freq": "WEEKLY", "interval": 1, "until": null},
            "source_uid": uid, "title": "Invented", "unsupported": false, "until": null, "where": null,
        })
    }

    fn registrar() -> serde_json::Value {
        json!({"ua:202640": {"applied": [{"day": "2026-09-10", "field": "meets", "note": "invented"}], "ended": [],
            "first_fetch": "2026-09-10", "kept": [], "last_fetch": "2026-09-10", "status": [], "unread": []}})
    }

    /// A series file with a registrar series and a Google one, plus (when `class_search`) a class-search
    /// series, its calendar, an `ended` key and the no-time block.
    fn series_text(class_search: bool) -> String {
        let mut calendars = json!({REG_CAL: "2026-09-10", "google:invented": "2026-09-10"});
        // In `file_bytes`' order, `(source_uid, calendar)`, as every engine-written file is: the filtered
        // copy is re-emitted canonically, so a hand-ordered `want` would differ in order alone.
        let mut all = vec![series("gcal-series:cs100", "google:invented"), series("registrar:ua:202640-40003", REG_CAL)];
        let mut ended = json!({});
        let mut v = json!({});
        if class_search {
            calendars["classsearch:ua:202640"] = json!("2026-09-10");
            all.push(series("classsearch:ua:202640-40001", "classsearch:ua:202640"));
            ended = json!({"classsearch:ua:202640-40009": {"calendar": "classsearch:ua:202640", "dropped": "2026-09-10", "last_instance": null, "until": null}});
            v["class_search"] = json!({"classsearch:ua:202640": {"no_time": ["cs-100"]}});
        }
        v["calendars"] = calendars; v["ended"] = ended; v["series"] = json!(all); v["registrar"] = registrar();
        format!("{}\n", serde_json::to_string(&v).unwrap())
    }

    /// A vault of invented notes, a plan file and `series` as the series file.
    fn vault(name: &str, series: &str) -> Scratch {
        let v = scratch(name, "vault");
        for (rel, text) in [
            ("tasks/essay.md", "---\nid: t1\ntitle: \"Invented essay\"\n---\n"),
            ("courses/cs-100.md", "---\ntitle: \"CS 100\"\nslug: cs-100\n---\n"),
            ("config/ingest.yaml", "feeds: []\n"),
            ("state/plan.json", "{\"day\": \"2026-09-10\"}\n"),
            (SERIES, series),
        ] {
            let p = v.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        v
    }

    fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut files = Vec::new(); walk(root, &mut files);
        files.into_iter().map(|p| (p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read(&p).unwrap())).collect()
    }

    fn zip_entries(zip_path: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut z = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
        (0..z.len()).map(|i| {
            let mut e = z.by_index(i).unwrap();
            let mut b = Vec::new(); e.read_to_end(&mut b).unwrap();
            (e.name().to_string(), b)
        }).collect()
    }

    fn has(bytes: &[u8], needle: &str) -> bool { bytes.windows(needle.len()).any(|w| w == needle.as_bytes()) }

    fn assert_clean(what: &str, files: &BTreeMap<String, Vec<u8>>) {
        for (name, bytes) in files {
            assert!(!has(bytes, "classsearch") && !has(bytes, "class_search"), "{what}: {name} holds class-search text");
        }
    }

    fn today() -> jiff::civil::Date { "2026-09-10".parse().unwrap() }

    fn json_of(bytes: &[u8]) -> serde_json::Value { serde_json::from_slice(bytes).unwrap() }

    /// The mirror and the zip hold no `classsearch` text and no `class_search` key, with a class-search series,
    /// calendar, `ended` key, a no-time block and a leftover temp copy all present. The registrar block
    /// and the Google series survive, equal. Red now: both copy the file whole.
    #[test]
    fn a_backup_holds_no_class_search_text_in_the_mirror_or_the_zip_and_keeps_the_registrar_and_google() {
        let v = vault("none", &series_text(true));
        std::fs::write(v.join(TMP), series_text(true)).unwrap();
        let t = scratch("none", "target");
        mirror(&v, &t, "p1").unwrap();
        let zip = snapshot(&v, &t, "p1", today()).unwrap();
        let (m, z) = (tree(&t.join("p1/vault")), zip_entries(&zip));
        assert_clean("mirror", &m); assert_clean("zip", &z);
        let want = json_of(series_text(false).as_bytes());
        for (what, got) in [("mirror", json_of(&m[SERIES])), ("zip", json_of(&z[SERIES]))] {
            assert_eq!(got["registrar"], want["registrar"], "{what}");
            assert_eq!(got["series"], want["series"], "{what}: registrar and Google series");
            assert_eq!(got["calendars"], want["calendars"], "{what}");
        }
    }

    /// A leftover `calendar-series.json.tmp<pid>` (`TMP_MARK` is `.tmp-`, so it is not swept) reaches neither
    /// the mirror nor the zip. Red now.
    #[test]
    fn a_leftover_series_temp_copy_reaches_neither_the_mirror_nor_the_zip() {
        let v = vault("tmp", &series_text(false));
        std::fs::write(v.join(TMP), series_text(true)).unwrap();
        let t = scratch("tmp", "target");
        mirror(&v, &t, "p1").unwrap();
        let zip = snapshot(&v, &t, "p1", today()).unwrap();
        assert!(!t.join("p1/vault").join(TMP).exists(), "mirror");
        assert!(!zip_entries(&zip).contains_key(TMP), "zip");
        assert!(!t.join("p1/vault/archive").exists() || tree(&t.join("p1/vault/archive")).is_empty(), "nothing archived");
    }

    /// Every other file is copied exactly as today: notes, config, state files, byte for byte, in both.
    /// Passes against the stub and must keep passing.
    #[test]
    fn every_other_file_is_copied_byte_for_byte() {
        let v = vault("rest", &series_text(true));
        std::fs::write(v.join(TMP), "x").unwrap();
        let t = scratch("rest", "target");
        mirror(&v, &t, "p1").unwrap();
        let zip = snapshot(&v, &t, "p1", today()).unwrap();
        let mut want = tree(&v); want.remove(SERIES); want.remove(TMP);
        for (what, mut got) in [("mirror", tree(&t.join("p1/vault"))), ("zip", zip_entries(&zip))] {
            got.remove(SERIES);
            got.remove(TMP);
            assert_eq!(got, want, "{what}");
        }
    }

    /// A file with no class-search data, hand-written non-canonical with an extra key and odd whitespace, is
    /// copied byte for byte into both, and a second tick copies nothing.
    /// Passes against the stub and must keep passing.
    #[test]
    fn a_series_file_with_no_class_search_data_is_copied_byte_for_byte_and_a_second_tick_copies_nothing() {
        let raw = "{ \"series\" :[ ],\n\t\"calendars\":{ },  \"ended\" : {}, \"extra_key\" : [1,\n 2]\n}\n\n";
        let v = vault("exact", raw);
        let t = scratch("exact", "target");
        mirror(&v, &t, "p1").unwrap();
        let zip = snapshot(&v, &t, "p1", today()).unwrap();
        assert_eq!(std::fs::read(t.join("p1/vault").join(SERIES)).unwrap(), raw.as_bytes());
        assert_eq!(zip_entries(&zip)[SERIES], raw.as_bytes());
        let again = mirror(&v, &t, "p1").unwrap();
        assert_eq!((again.copied, again.archived), (0, 0));
    }

    /// With class-search data present, a second tick with nothing changed copies 0 files, archives none and
    /// reports every file, the series file included, `unchanged`. Red now: the stub-era copy is unfiltered, so
    /// the first assertion fails; once only the copy is filtered, `same_bytes` would recopy it every tick.
    #[test]
    fn a_second_tick_with_class_search_data_present_copies_nothing_and_reports_the_path_unchanged() {
        let v = vault("second", &series_text(true));
        std::fs::write(v.join(TMP), series_text(true)).unwrap();
        let t = scratch("second", "target");
        let first = mirror(&v, &t, "p1").unwrap();
        assert!(first.errors.is_empty(), "{:?}", first.errors);
        assert_clean("mirror", &tree(&t.join("p1/vault")));
        let wanted = tree(&v).keys().filter(|k| *k != TMP).count();
        assert_eq!(first.copied, wanted, "the first tick copies every wanted file");
        let before = tree(&t.join("p1/vault"));
        let again = mirror(&v, &t, "p1").unwrap();
        assert_eq!((again.copied, again.archived, again.unchanged), (0, 0, wanted), "{again:?}");
        assert_eq!(tree(&t.join("p1/vault")), before);
    }

    /// A malformed file holding class-search text is skipped for that run: the older mirror copy stays, nothing
    /// is archived, and the zip has no entry for it. Red now: the file is copied whole.
    #[test]
    fn a_malformed_series_file_with_class_search_text_is_skipped_and_the_older_mirror_copy_stays() {
        let v = vault("bad", &series_text(false));
        let t = scratch("bad", "target");
        mirror(&v, &t, "p1").unwrap();
        let older = std::fs::read(t.join("p1/vault").join(SERIES)).unwrap();
        std::fs::write(v.join(SERIES), "{\"calendars\": {\"classsearch:ua:202640\": ").unwrap();
        let r = mirror(&v, &t, "p1").unwrap();
        assert_eq!(r.archived, 0, "still counted as wanted, so never archived: {r:?}");
        assert_eq!(std::fs::read(t.join("p1/vault").join(SERIES)).unwrap(), older, "the older copy stays");
        assert!(!t.join("p1/vault/archive").exists() || tree(&t.join("p1/vault/archive")).is_empty());
        let zip = snapshot(&v, &t, "p1", today()).unwrap();
        let z = zip_entries(&zip);
        assert!(!z.contains_key(SERIES), "skipped in the zip");
        assert_clean("zip", &z);
        assert!(z.contains_key("tasks/essay.md"), "the rest of the zip is whole");
    }

    /// A malformed file with none of that text is copied as today, in both.
    /// Passes against the stub and must keep passing.
    #[test]
    fn a_malformed_series_file_with_no_class_search_text_is_copied_as_today() {
        let raw = "{\"calendars\": {\"google:invented\": ";
        let v = vault("bad-clean", raw);
        let t = scratch("bad-clean", "target");
        mirror(&v, &t, "p1").unwrap();
        let zip = snapshot(&v, &t, "p1", today()).unwrap();
        assert_eq!(std::fs::read(t.join("p1/vault").join(SERIES)).unwrap(), raw.as_bytes());
        assert_eq!(zip_entries(&zip)[SERIES], raw.as_bytes());
    }

    /// An older mirror (written before C2e) holds the series file and a temp copy with class-search text.
    /// `restore_vault_in` copies a mirror whole, so the next run must scrub it: the series copy is
    /// overwritten with the filtered bytes, and the old temp copy is not moved into `archive/`, where it
    /// would survive. Red now: both stay as they were (the temp one in `archive/`).
    #[test]
    fn a_run_over_an_older_mirror_that_holds_class_search_data_leaves_none_of_it_anywhere_in_the_mirror() {
        let v = vault("older", &series_text(true));
        let t = scratch("older", "target");
        let m = t.join("p1/vault");
        for rel in [SERIES, TMP] {
            std::fs::create_dir_all(m.join(rel).parent().unwrap()).unwrap();
            std::fs::write(m.join(rel), series_text(true)).unwrap();
        }
        mirror(&v, &t, "p1").unwrap();
        assert_clean("mirror (archive included)", &tree(&m));
        assert!(!m.join(TMP).exists());
    }

    /// Determinism: the same vault gives the same filtered bytes, mirror tree and zip entries on every run,
    /// and a second target gets the same bytes as the first. Red now.
    #[test]
    fn the_filtered_backup_is_the_same_bytes_on_every_run() {
        let v = vault("det", &series_text(true));
        std::fs::write(v.join(TMP), series_text(true)).unwrap();
        let (a, b) = (scratch("det", "a"), scratch("det", "b"));
        mirror(&v, &a, "p1").unwrap(); mirror(&v, &b, "p1").unwrap();
        let (za, zb) = (snapshot(&v, &a, "p1", today()).unwrap(), snapshot(&v, &b, "p1", today()).unwrap());
        let (ta, tb) = (tree(&a.join("p1/vault")), tree(&b.join("p1/vault")));
        assert_clean("mirror a", &ta);
        assert_eq!(ta, tb);
        let (ea, eb) = (zip_entries(&za), zip_entries(&zb));
        assert_clean("zip a", &ea);
        assert_eq!(ea, eb);
        assert_eq!(ta[SERIES], ea[SERIES], "mirror and zip hold the same filtered bytes");
    }
}
