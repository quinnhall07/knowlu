//! Backup (Knowlu spec §4): a live **mirror** of the vault's folders under a folder the user
//! owns, copied on content change with temp-and-rename, **never deleting** — a source that
//! disappears moves to the mirror's own `archive/` — plus one dated zip **snapshot** a day,
//! `SNAPSHOTS_KEPT` retained. Failure is a status, never a panic; the console paints it amber.
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
pub const BACKUP_FOLDERS: [&str; 9] = [
    ids::NOTE_FOLDERS[0], ids::NOTE_FOLDERS[1], ids::NOTE_FOLDERS[2],
    ids::NOTE_FOLDERS[3], ids::NOTE_FOLDERS[4], ids::NOTE_FOLDERS[5],
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

fn place(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?; }
    let tmp = dst.with_file_name(format!("{}{TMP_MARK}{}", dst.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
    std::fs::copy(src, &tmp).map_err(|e| format!("copy {}: {e}", src.display()))?;
    std::fs::rename(&tmp, dst).map_err(|e| { let _ = std::fs::remove_file(&tmp); format!("rename {}: {e}", dst.display()) })
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
            wanted.insert(rel.clone());
            if same_bytes(&src, &dst) { rep.unchanged += 1; continue; }
            match place(&src, &dst) { Ok(()) => rep.copied += 1, Err(e) => rep.errors.push(e) }
        }
    }
    // 3. archive what the vault no longer has (never under the mirror's own archive/)
    let mut mirrored = Vec::new(); walk(&root, &mut mirrored);
    for m in mirrored {
        let rel = m.strip_prefix(&root).map_err(|e| e.to_string())?.to_path_buf();
        if rel.starts_with("archive") || wanted.contains(&rel) { continue; }
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
                let rel = src.strip_prefix(vault).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                zip.start_file(rel, opts).map_err(|e| e.to_string())?;
                zip.write_all(&std::fs::read(&src).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
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
