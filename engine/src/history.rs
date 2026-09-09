//! History (Knowlu spec §4; console spec §8; S2 §10): git as a **transport**, driven only by the
//! console. Commit by name, never `-A`; only-ahead means push, never rebase; behind means
//! `pull --rebase` then push; a conflict goes through `reconcile` (Task 6); offline is amber and
//! the commit stands. The journal is the history — when the remote goes, so does this module's
//! reason to run, and a vault that is not a repository is a normal state, not an error.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use crate::journal::Journal;
use crate::write::WriteContext;

pub const STAGED_PATHS: [&str; 7] = ["tasks", "approvals", "archive", "courses", "issues", "info", "state"];
const LOCK_STALE_SECS: i64 = 120;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct HistoryStatus {
    pub is_repo: bool, pub has_remote: bool, pub ahead: i64, pub behind: i64, pub dirty: bool,
    pub last_error: Option<String>, pub conflicted: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncOutcome { pub status: HistoryStatus, pub committed: bool, pub pulled: bool, pub pushed: bool }

/// Every git call in this module is capped: a hanging credential prompt that
/// `GIT_TERMINAL_PROMPT=0` could not stop, or a remote that never answers, must be killed and
/// reported rather than wedging the sync thread forever (plan 2 Task 4, F12).
pub const GIT_TIMEOUT: Duration = Duration::from_secs(60);

pub fn git(vault: &Path, args: &[&str]) -> Result<String, String> { git_with("git", vault, args, GIT_TIMEOUT) }

/// The one process spawn in this module. `program` is a seam for the timeout test; production is
/// always `"git"`. `GIT_TERMINAL_PROMPT=0` on every invocation (final fix wave, A6): a tray app
/// that syncs every few minutes must never pop a credential dialog behind its own window. Stdout
/// and stderr are drained on their own threads (a chatty `fetch` must not fill a pipe); the child
/// is polled with a ramping sleep — 2 ms doubling to a 100 ms ceiling, clamped to whatever is left
/// of `timeout` — so a fast call (`status()` alone is five of these) never pays a flat 100 ms tax,
/// and past `timeout` the whole process tree is killed (R-P2-8, Task 4 fix round 1) so the drain
/// threads reach EOF and are joined rather than leaked.
pub fn git_with(program: &str, vault: &Path, args: &[&str], timeout: Duration) -> Result<String, String> {
    use std::io::Read;
    use crate::childproc::NoConsole;
    let mut child = Command::new(program).no_console().current_dir(vault).env("GIT_TERMINAL_PROMPT", "0")
        .args(["-c", "commit.gpgsign=false", "-c", "core.editor=true"]).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("git: {e}"))?;
    let mut out_pipe = child.stdout.take().unwrap();
    let mut err_pipe = child.stderr.take().unwrap();
    let out_t = std::thread::spawn(move || { let mut s = Vec::new(); let _ = out_pipe.read_to_end(&mut s); s });
    let err_t = std::thread::spawn(move || { let mut s = Vec::new(); let _ = err_pipe.read_to_end(&mut s); s });
    let sub = args.first().copied().unwrap_or("");
    let started = std::time::Instant::now();
    let mut poll = Duration::from_millis(2);
    let exit = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {}
            Err(e) => {
                kill_tree(&mut child);
                let _ = child.wait();
                let _ = out_t.join();
                let _ = err_t.join();
                return Err(format!("git {sub}: {e}"));
            }
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            // R-P2-8: kill the whole process tree, not just the immediate child — a killed
            // process can leave a grandchild alive holding the inherited pipe handle open (a
            // `.cmd` wrapper's `ping`, a credential helper git shelled out to), which would
            // otherwise leak that process plus both blocked reader threads on every timeout.
            kill_tree(&mut child);
            let _ = child.wait();
            let _ = out_t.join();
            let _ = err_t.join();
            return Err(format!("git {sub}: timed out after {} s", timeout.as_secs()));
        }
        std::thread::sleep(poll.min(timeout - elapsed));
        poll = (poll * 2).min(Duration::from_millis(100));
    };
    let stdout = String::from_utf8_lossy(&out_t.join().unwrap_or_default()).trim().to_string();
    let stderr = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).trim().to_string();
    if exit.success() { Ok(stdout) } else { Err(stderr) }
}

/// Kills `child` and, on Windows, its whole descendant tree via `taskkill /T /F` — a plain
/// `Child::kill` signals only the immediate process, which leaves a wrapper's own children (a
/// `.cmd` shim's `ping`, a credential helper) running and holding the piped stdout/stderr open,
/// so the drain threads in `git_with` would never see EOF (R-P2-8, Task 4 fix round 1). Errors
/// from `taskkill` are ignored: the immediate `child.kill()` that follows is the fallback if the
/// tree kill did not apply (the child already exited, `taskkill` itself is missing, …).
#[cfg(windows)]
fn kill_tree(child: &mut std::process::Child) {
    use crate::childproc::NoConsole;
    let _ = Command::new("taskkill").no_console().args(["/T", "/F", "/PID", &child.id().to_string()])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    let _ = child.kill();
}
#[cfg(not(windows))]
fn kill_tree(child: &mut std::process::Child) { let _ = child.kill(); }

fn counts(vault: &Path) -> Result<(i64, i64), String> {
    let s = git(vault, &["rev-list", "--left-right", "--count", "HEAD...origin/main"])?;
    let mut it = s.split_whitespace().map(|n| n.parse::<i64>().unwrap_or(0));
    Ok((it.next().unwrap_or(0), it.next().unwrap_or(0)))
}

pub fn status(vault: &Path) -> HistoryStatus {
    let mut st = HistoryStatus::default();
    if git(vault, &["rev-parse", "--is-inside-work-tree"]).map(|s| s == "true").unwrap_or(false) { st.is_repo = true; } else { return st; }
    st.has_remote = git(vault, &["remote", "get-url", "origin"]).is_ok();
    // The same seven folders `commit_by_name` stages — one list, never a second hand-written
    // copy that can drift out of step with it (final fix wave, A5).
    let mut dirty: Vec<&str> = vec!["status", "--porcelain", "--"];
    dirty.extend_from_slice(&STAGED_PATHS);
    st.dirty = !git(vault, &dirty).unwrap_or_default().is_empty();
    if st.has_remote { if let Ok((a, b)) = counts(vault) { st.ahead = a; st.behind = b; } }
    st.conflicted = git(vault, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default().lines().map(|s| s.to_string()).collect();
    st
}

/// Two paths under `state/` must never ride along in a commit: `.sync.lock` (this process's own
/// liveness file, held for the whole of the sync that is doing the staging) and `events-ui/`
/// (interaction telemetry, opt-in via `commit_ui_events`). This repo's `.gitignore` covers both —
/// but a scratch vault, a friend's vault and every test repo here have no such file, so the
/// exclusion belongs in the pathspec rather than in an assumption (final fix wave, A2).
///
/// **But a negative pathspec cannot be used where `.gitignore` ALSO covers the path.** git
/// (2.55) treats `:!state/events-ui` as *explicitly naming* that path, and `git add` refuses to
/// name an ignored one: `fatal: The following paths are ignored by one of your .gitignore
/// files`, exit 1. So on the real vault — and only there — every sync died in `stage`,
/// leaving the user's edits staged and never committed. `stage` therefore appends an exclusion
/// only where git is not already excluding it, which is the same set either way.
///
/// Nothing caught this: `repo_pair` copies `.gitattributes` but not `.gitignore`, and
/// `scripts/scratch-vault.ps1` copies neither, so neither the tests nor the go-live rehearsal
/// vault could reproduce it. Found on the live vault at go-live, 2026-09-09.
const NEVER_STAGED: [&str; 2] = ["state/.sync.lock", "state/events-ui"];

/// `git check-ignore -q` exits 0 when the path is ignored, 1 when it is not.
fn already_ignored(vault: &Path, path: &str) -> bool {
    git(vault, &["check-ignore", "-q", "--", path]).is_ok()
}

/// `git add --` over whichever of `paths` exist, with `NEVER_STAGED` appended. A no-op — and not
/// a git invocation at all — when none of them exist, since `git add` with only negative
/// pathspecs is an error.
fn stage(vault: &Path, paths: &[&str]) -> Result<(), String> {
    let mut args: Vec<String> = vec!["add".into(), "--".into()];
    args.extend(paths.iter().filter(|p| vault.join(p).exists()).map(|p| (*p).to_string()));
    if args.len() == 2 { return Ok(()); }
    for path in NEVER_STAGED {
        if !already_ignored(vault, path) {
            args.push(format!(":!{path}"));
        }
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    git(vault, &args).map(|_| ())
}

/// True while git has a rebase half-applied: the `rebase-merge` (merge backend) or `rebase-apply`
/// (am backend) directory exists under the repo's git dir.
fn rebase_in_progress(vault: &Path) -> bool {
    ["rebase-merge", "rebase-apply"].iter().any(|d| {
        git(vault, &["rev-parse", "--git-path", d]).map(|p| vault.join(p).exists()).unwrap_or(false)
    })
}

/// `status(vault).conflicted` — from `git diff --name-only --diff-filter=U` — lists an unmerged
/// path (`UU`, `AA`, `DU`, …). This — and only this — is what `resolve_conflicts` can act on. A
/// rebase directory with no unmerged path is a rebase that stopped for another reason and is
/// aborted, not resolved (plan 2 Task 4, F12).
fn unmerged_paths_exist(vault: &Path) -> bool { !status(vault).conflicted.is_empty() }
fn conflicts_pending(vault: &Path) -> bool { unmerged_paths_exist(vault) }

pub fn commit_by_name(vault: &Path, message: &str) -> Result<bool, String> {
    stage(vault, &STAGED_PATHS)?;
    if crate::uievents::commit_opt_in(vault) && vault.join("state/events-ui").exists() { git(vault, &["add", "-f", "--", "state/events-ui"])?; }
    if git(vault, &["diff", "--cached", "--quiet"]).is_ok() { return Ok(false); }
    git(vault, &["commit", "-q", "-m", message])?;
    Ok(true)
}

pub fn sync(vault: &Path, ctx: &WriteContext, journal: &mut Journal, edits: usize) -> SyncOutcome {
    let mut out = SyncOutcome::default();
    let _lock = match SyncLock::acquire(vault) { Ok(l) => l, Err(e) => { out.status = status(vault); out.status.last_error = Some(e); return out; } };
    out.status = status(vault);
    if !out.status.is_repo { return out; }
    match commit_by_name(vault, &format!("surface: {edits} edit{}", if edits == 1 { "" } else { "s" })) {
        Ok(c) => out.committed = c,
        Err(e) => { out.status.last_error = Some(e); return out; }
    }
    if !out.status.has_remote { out.status = status(vault); return out; }
    // `push HEAD:main` publishes whatever is checked out as the remote's main, and a rebase onto
    // origin/main is exactly as unsafe — both assume `HEAD` is main. A vault parked on a side
    // branch (a bisect, a hand-made experiment, a half-finished checkout) must have neither happen
    // to it: refuse before the fetch, name the branch, leave the commit local (A3; plan 2 R-P2-1).
    let branch = git(vault, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|_| "unknown".into());
    if branch != "main" {
        out.status = status(vault);
        out.status.last_error = Some(format!("not on main ({branch}) — sync refused"));
        return out;
    }
    if let Err(e) = git(vault, &["fetch", "-q", "origin"]) { out.status = status(vault); out.status.last_error = Some(e); return out; }
    let (ahead, behind) = match counts(vault) { Ok(c) => c, Err(e) => { out.status.last_error = Some(e); return out; } };
    if behind > 0 {
        match git(vault, &["rebase", "-q", "origin/main"]) {
            Ok(_) => out.pulled = true,
            // A rebase fails for plenty of reasons that are not a conflict — a dirty working tree
            // outside `STAGED_PATHS`, a missing upstream, a refusing hook. Every one of those used
            // to fall into `resolve_conflicts`, which finds no `U` entries, returns `Ok(vec![])`
            // and so read as a successful pull: the code pushed anyway, was rejected
            // non-fast-forward, and showed the user push stderr for a rebase that never happened.
            // Now the rebase's own first line is the error, nothing is pushed, and any half-started
            // rebase is aborted so the working tree is left exactly as the user had it (A1).
            Err(e) if !conflicts_pending(vault) => {
                if rebase_in_progress(vault) { let _ = git(vault, &["rebase", "--abort"]); }
                out.status = status(vault);
                out.status.last_error = Some(format!("rebase: {}", e.lines().next().unwrap_or("failed").trim()));
                return out;
            }
            Err(e) => {
                let settled = resolve_conflicts(vault, ctx, journal);
                match settled {
                    Ok(_) => out.pulled = true,
                    Err(conflict) => {
                        let names = status(vault).conflicted;
                        let _ = git(vault, &["rebase", "--abort"]);
                        out.status = status(vault);
                        out.status.conflicted = names;
                        out.status.last_error = Some(format!("conflict: {conflict}; auto-sync stopped ({e})"));
                        return out;
                    }
                }
            }
        }
    }
    if ahead > 0 || out.committed || out.pulled {
        match git(vault, &["push", "-q", "origin", "HEAD:main"]) { Ok(_) => out.pushed = true, Err(e) => { out.status = status(vault); out.status.last_error = Some(e); return out; } }
    }
    out.status = status(vault);
    out
}

fn is_note_path(p: &str) -> bool {
    p.ends_with(".md") && crate::ids::NOTE_FOLDERS.iter().any(|f| p.starts_with(&format!("{f}/")))
}

/// Journal `ts` strings compare lexically (UTC, fixed width). `%cI` from git is `+00:00`-suffixed
/// local time; normalise to the journal's `YYYY-MM-DDTHH:MM:SS.000Z`.
fn git_time_to_ts(iso: &str) -> String {
    iso.parse::<jiff::Timestamp>().map(|t| crate::journal::now_ts(Some(t))).unwrap_or_else(|_| iso.to_string())
}

/// Settle every conflicted note through `reconcile`: upstream wins the file on disk, this
/// device's replayed journal records are reconciled against upstream's, winners are re-applied
/// through `write` (journal first, note second), and `supersede` records are appended for
/// whichever side loses. Ledgers (`state/**`) are `merge=union` and are never expected here — a
/// conflicted ledger is an `Err`, not something this path resolves.
/// The last commit both sides share, from the pre-rebase HEAD git stamps as `ORIG_HEAD` when a
/// rebase starts. **No fallback** (R-T6, final fix wave A4): the old `merge-base HEAD origin/main`
/// second try collapses mid-rebase to origin/main's own tip, which dates every one of this
/// device's un-synced records as older than the boundary and drops them from `local` with no
/// supersede record — silently losing this device's edit, which the spec forbids. A rebase in
/// progress with no `ORIG_HEAD` is a state this module cannot reconcile, so it says so instead.
fn conflict_base(vault: &Path) -> Result<String, String> {
    git(vault, &["merge-base", "ORIG_HEAD", "origin/main"])
        .map_err(|e| format!("rebase in progress without ORIG_HEAD ({e})"))
}

pub fn resolve_conflicts(vault: &Path, ctx: &WriteContext, journal: &mut Journal) -> Result<Vec<String>, String> {
    let mut settled = Vec::new();
    loop {
        let conflicted: Vec<String> = git(vault, &["diff", "--name-only", "--diff-filter=U"])?.lines().map(|s| s.to_string()).collect();
        if conflicted.is_empty() { break; }
        if let Some(bad) = conflicted.iter().find(|p| !is_note_path(p)) {
            return Err(format!("{bad} conflicted and is not a note (ledgers are merge=union)"));
        }
        let this_device = crate::journal::device_name();
        // The boundary between "old" and "contender" records is the last commit BOTH sides
        // share — the merge base — not the fetched tip's time. `ORIG_HEAD` is the pre-rebase
        // HEAD git stamps when the rebase starts; every record this device wrote after its own
        // last successful sync is newer than that shared ancestor, whatever the other device did
        // since. Using the fetched tip's time instead would filter out this device's own older
        // (but still un-synced) records whenever the other device pushed after this device's edit
        // but before this device's sync — silently dropping this device's change with no
        // supersede record, which the spec forbids.
        let base = conflict_base(vault)?;
        let base_ts = git_time_to_ts(&git(vault, &["log", "-1", "--format=%cI", &base])?);
        for path in &conflicted {
            // During a rebase, `--ours` is the branch being rebased onto: upstream wins the file.
            git(vault, &["checkout", "--ours", "--", path])?;
            let upstream_meta = crate::ids::read_meta(&vault.join(path)).ok_or_else(|| format!("{path}: unreadable after taking upstream"))?;
            let note_id = upstream_meta.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let upstream_mtime = git_time_to_ts(&git(vault, &["log", "-1", "--format=%cI", "origin/main", "--", path])?);
            journal.invalidate();
            let all = note_id.as_deref().map(|id| journal.records_for(id, None)).unwrap_or_default();
            let ts_of = |r: &crate::ledger::Record| r.get("ts").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let dev_of = |r: &crate::ledger::Record| r.get("device").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let local: Vec<_> = all.iter().filter(|r| dev_of(r) == this_device && ts_of(r) > base_ts).cloned().collect();
            let upstream: Vec<_> = all.iter().filter(|r| dev_of(r) != this_device && ts_of(r) > base_ts).cloned().collect();
            let res = crate::reconcile::resolve(&upstream_meta, &upstream, &local, &upstream_mtime, note_id.as_deref(), path, &ctx.via);
            if !res.apply.is_empty() {
                let changes: Vec<(String, serde_yaml_ng::Value)> = res.apply.iter().map(|(k, v)| (k.clone(), crate::yaml::from_json(v))).collect();
                crate::write::write(vault, path, &changes, ctx, journal, &Default::default()).map_err(|e| format!("{path}: re-apply: {e}"))?;
            }
            for mut rec in res.supersede { journal.append(&mut rec).map_err(|e| format!("{path}: supersede: {e:?}"))?; }
            git(vault, &["add", "--", path])?;
            stage(vault, &["state"])?;   // the journal + supersede records the re-apply just wrote
            settled.push(path.clone());
        }
        match git(vault, &["-c", "core.editor=true", "rebase", "--continue"]) {
            Ok(_) => {}
            Err(e) if e.contains("CONFLICT") || status(vault).conflicted.len() > 0 => continue,
            Err(e) => return Err(format!("rebase --continue: {e}")),
        }
    }
    Ok(settled)
}

pub struct SyncLock { path: PathBuf }

impl SyncLock {
    pub fn acquire(vault: &Path) -> Result<SyncLock, String> {
        let path = vault.join("state").join(".sync.lock");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let ts = text.split_whitespace().nth(1).and_then(|t| t.parse::<jiff::Timestamp>().ok());
            let fresh = ts.map(|t| jiff::Timestamp::now().as_second() - t.as_second() < LOCK_STALE_SECS).unwrap_or(false);
            if fresh { return Err(format!("another sync holds {}", path.display())); }
        }
        if let Some(p) = path.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, format!("{} {}", std::process::id(), crate::journal::now_ts(None))).map_err(|e| e.to_string())?;
        Ok(SyncLock { path })
    }
}
impl Drop for SyncLock { fn drop(&mut self) { let _ = std::fs::remove_file(&self.path); } }

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn git_env() { std::env::set_var("GIT_AUTHOR_NAME", "test"); std::env::set_var("GIT_AUTHOR_EMAIL", "t@localhost"); std::env::set_var("GIT_COMMITTER_NAME", "test"); std::env::set_var("GIT_COMMITTER_EMAIL", "t@localhost"); }
    fn copy_tree(from: &Path, to: &Path) { std::fs::create_dir_all(to).unwrap(); for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy_tree(&p, &t) } else { std::fs::copy(&p, &t).unwrap(); } } }
    const VAULT_GITATTRIBUTES: &str = "state/runner-log.md merge=union\nstate/today.md merge=union\nstate/ingest-seen.md merge=union\nstate/calendar.md merge=union\nstate/events.md merge=union\nstate/events-seen.md merge=union\nstate/journal/** merge=union\nstate/runs/** merge=union\n";
    /// A bare remote + a working vault cloned from the fixture, on branch `main`.
    pub(crate) fn repo_pair(tag: &str) -> (PathBuf, PathBuf) {
        git_env();
        let base = std::env::temp_dir().join(format!("qo-history-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&base);
        let bare = base.join("remote.git"); let work = base.join("vault");
        std::fs::create_dir_all(&bare).unwrap();
        git(&bare, &["init", "--bare", "--initial-branch=main"]).unwrap();
        copy_tree(Path::new("tests/fixtures/vault-full"), &work);
        // The fixture vault has no `.gitattributes` of its own; without a vault's `merge=union`
        // attributes the ledgers under `state/journal/**` would show as real add/add conflicts in
        // these tests instead of auto-merging, which does not happen against a real vault. These
        // are the lines the original vault carried (this crate's root is not a vault, so they are
        // spelled here rather than copied).
        std::fs::write(work.join(".gitattributes"), VAULT_GITATTRIBUTES).unwrap();
        git(&work, &["init", "--initial-branch=main"]).unwrap();
        git(&work, &["add", "."]).unwrap();                      // test setup only — the module never uses -A or `.`
        git(&work, &["commit", "-q", "-m", "fixture"]).unwrap();
        git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]).unwrap();
        git(&work, &["push", "-q", "-u", "origin", "main"]).unwrap();
        (bare, work)
    }
    fn edit(work: &Path, field: &str, value: &str) {
        let mut j = crate::journal::Journal::new(work);
        let ctx = crate::write::WriteContext::new("quinn", "dashboard");
        crate::write::write_literals(work, "tasks/ph-106-exam-1-prep.md", &[(field.into(), value.into())], &ctx, &mut j, &Default::default()).unwrap();
    }
    /// Writes a minimal note at `path` in `other` (an existing clone), commits and pushes it —
    /// the shape plan 2's brief names `push_a_commit_from`, used by tests that need the remote to
    /// have moved on without this device's involvement.
    fn push_a_commit_from(other: &Path, path: &str, message: &str) {
        let full = other.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, "---\ntype: task\nid: task_0000000099\nstatus: open\ntitle: x\n---\n").unwrap();
        let folder = path.split('/').next().unwrap();
        git(other, &["add", folder]).unwrap();
        git(other, &["commit", "-q", "-m", message]).unwrap();
        git(other, &["push", "-q"]).unwrap();
    }
    fn remote_tip(bare: &Path) -> String { git(bare, &["rev-parse", "main"]).unwrap() }
    fn tip_of(work: &Path) -> String { git(work, &["rev-parse", "HEAD"]).unwrap() }

    #[test]
    fn a_plain_folder_is_not_a_repo_and_that_is_not_an_error() {
        let d = std::env::temp_dir().join(format!("qo-history-plain-{}-{:?}", std::process::id(), std::thread::current().id()));
        std::fs::create_dir_all(&d).unwrap();
        let s = status(&d);
        assert!(!s.is_repo && !s.has_remote && s.last_error.is_none(), "{s:?}");
    }

    #[test]
    fn commit_by_name_stages_only_the_named_paths() {
        let (_bare, work) = repo_pair("names");
        edit(&work, "importance", "4");
        std::fs::write(work.join("scratch.txt"), "not staged").unwrap();
        assert!(commit_by_name(&work, "surface: 1 edit").unwrap());
        let shown = git(&work, &["show", "--stat", "--format=", "HEAD"]).unwrap();
        assert!(shown.contains("tasks/ph-106-exam-1-prep.md") && shown.contains("state/journal/"), "{shown}");
        assert!(!shown.contains("scratch.txt"));
        assert!(!commit_by_name(&work, "surface: 0 edits").unwrap(), "nothing staged, no commit");
    }

    /// The bug that stopped the live vault committing at go-live (2026-09-09). git treats a
    /// negative pathspec as naming the path, and `git add` refuses to name an ignored one, so
    /// `git add -- state :!state/events-ui` dies with exit 1 on any vault whose `.gitignore`
    /// covers it. Every sync failed at `stage`, and the user's edits sat staged and
    /// uncommitted with only the sync line to say so.
    ///
    /// `repo_pair` copies `.gitattributes` but not `.gitignore`, so this test writes one. That
    /// omission is the whole reason the bug reached a live vault: the rehearsal vault
    /// (`scripts/scratch-vault.ps1`) copies neither file, so it could not reproduce it either.
    #[test]
    fn staging_survives_a_gitignore_that_already_covers_the_never_staged_paths() {
        let (_bare, work) = repo_pair("ignored");
        std::fs::write(work.join(".gitignore"), "/state/events-ui/\n/state/.sync.lock\n").unwrap();
        std::fs::create_dir_all(work.join("state").join("events-ui")).unwrap();
        // Any filename does: the ignore rule under test is the whole directory. Deliberately
        // NOT a ledger name -- `ledger::tests::only_this_module_opens_ledger_files` forbids
        // spelling one outside ledger.rs, and it caught this very line on the first full run.
        std::fs::write(work.join("state").join("events-ui").join("planted"), "x\n").unwrap();
        std::fs::write(work.join("state").join(".sync.lock"), "held\n").unwrap();
        edit(&work, "importance", "4");
        assert!(
            commit_by_name(&work, "surface: 1 edit").expect("stage must not die on an ignored exclusion"),
            "the edit must be committed"
        );
        let shown = git(&work, &["show", "--stat", "--format=", "HEAD"]).unwrap();
        assert!(shown.contains("tasks/ph-106-exam-1-prep.md"), "{shown}");
        assert!(!shown.contains("events-ui"), "telemetry must never ride along: {shown}");
        assert!(!shown.contains("sync.lock"), "the liveness file must never ride along: {shown}");
        let _ = std::fs::remove_dir_all(work.parent().unwrap());
    }

    #[test]
    fn only_ahead_pushes_and_never_rebases() {
        let (bare, work) = repo_pair("ahead");
        edit(&work, "importance", "2");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed && out.pushed && !out.pulled, "{out:?}");
        assert_eq!(git(&bare, &["rev-parse", "main"]).unwrap(), git(&work, &["rev-parse", "HEAD"]).unwrap());
        assert_eq!(out.status.ahead, 0);
    }

    #[test]
    fn behind_without_overlap_pulls_with_rebase_then_pushes() {
        let (bare, work) = repo_pair("behind");
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        std::fs::write(other.join("info").join("from-other.md"), "---\ntype: info\nid: info_0000000001\nstatus: open\ntitle: x\n---\n").unwrap();
        git(&other, &["add", "info"]).unwrap(); git(&other, &["commit", "-q", "-m", "other"]).unwrap(); git(&other, &["push", "-q"]).unwrap();
        edit(&work, "importance", "1");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.pulled && out.pushed, "{out:?}");
        assert!(work.join("info/from-other.md").exists());
        assert!(out.status.conflicted.is_empty());
    }

    #[test]
    fn a_dead_remote_is_amber_and_the_commit_stands() {
        let (bare, work) = repo_pair("dead");
        std::fs::remove_dir_all(&bare).unwrap();
        edit(&work, "importance", "3");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed && !out.pushed);
        assert!(out.status.last_error.is_some());
        assert_eq!(out.status.ahead, 1, "the edit is safe locally and waits");
    }

    #[test]
    fn the_lock_is_exclusive_and_stale_after_two_minutes() {
        let (_b, work) = repo_pair("lock");
        let l = SyncLock::acquire(&work).unwrap();
        assert!(SyncLock::acquire(&work).is_err());
        drop(l);
        std::fs::write(work.join("state/.sync.lock"), "99999 2000-01-01T00:00:00.000Z").unwrap();
        assert!(SyncLock::acquire(&work).is_ok(), "a stale lock is taken over");
    }

    #[test]
    fn a_same_field_conflict_is_settled_by_reconcile_and_leaves_a_supersede_record() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let (bare, work) = repo_pair("conflict");
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        // The other device edits `due` first and pushes; this device edits `due` later, then syncs.
        std::env::set_var("KNOWLU_DEVICE", "other-box");
        edit(&other, "due", "2026-10-01T09:00");
        git(&other, &["add", "tasks", "state"]).unwrap(); git(&other, &["commit", "-q", "-m", "other"]).unwrap(); git(&other, &["push", "-q"]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::env::set_var("KNOWLU_DEVICE", "this-box");
        edit(&work, "due", "2026-10-02T09:00");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.status.conflicted.is_empty(), "{out:?}");
        assert!(out.pushed);
        let meta = crate::ids::read_meta(&work.join("tasks/ph-106-exam-1-prep.md")).unwrap();
        assert_eq!(crate::approvals::plain(meta.get("due").unwrap()), "2026-10-02T09:00", "the later record wins");
        let mut j = crate::journal::Journal::new(&work);
        let id = crate::approvals::plain(meta.get("id").unwrap());
        assert!(j.records_for(&id, Some("due")).iter().any(|r| r.get("op").and_then(|o| o.as_str()) == Some("supersede")), "the loser is superseded, never silently dropped");
        std::env::remove_var("KNOWLU_DEVICE");
    }

    /// R-T6: the conflict boundary is the merge base, not the fetched tip's time. When THIS
    /// device edits first (but has not yet synced) and the OTHER device edits and pushes later,
    /// this device's own record still has a `ts` older than the other device's push — using the
    /// fetched tip as the boundary would filter this device's own record out of `local` entirely,
    /// silently dropping its edit with no supersede record. Using the merge base (the last commit
    /// both sides share, which predates both edits) keeps it in.
    #[test]
    fn a_conflict_where_this_device_edited_first_still_reconciles_and_supersedes() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let (bare, work) = repo_pair("conflict-reverse");
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        // This device edits `due` first (not yet synced); the other device edits later and pushes.
        std::env::set_var("KNOWLU_DEVICE", "this-box");
        edit(&work, "due", "2026-10-01T09:00");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::env::set_var("KNOWLU_DEVICE", "other-box");
        edit(&other, "due", "2026-10-02T09:00");
        git(&other, &["add", "tasks", "state"]).unwrap(); git(&other, &["commit", "-q", "-m", "other"]).unwrap(); git(&other, &["push", "-q"]).unwrap();
        std::env::set_var("KNOWLU_DEVICE", "this-box");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.status.conflicted.is_empty(), "{out:?}");
        assert!(out.pushed);
        let meta = crate::ids::read_meta(&work.join("tasks/ph-106-exam-1-prep.md")).unwrap();
        assert_eq!(crate::approvals::plain(meta.get("due").unwrap()), "2026-10-02T09:00", "the later record (the other device's) wins even though this device edited first");
        let mut j = crate::journal::Journal::new(&work);
        let id = crate::approvals::plain(meta.get("id").unwrap());
        assert!(j.records_for(&id, Some("due")).iter().any(|r| r.get("op").and_then(|o| o.as_str()) == Some("supersede")), "this device's earlier edit is superseded, never silently dropped");
        std::env::remove_var("KNOWLU_DEVICE");
    }

    /// A1: a rebase that fails for a reason that is NOT a conflict — here a tracked file dirty
    /// outside `STAGED_PATHS`, which git refuses to rebase over — must read as an error, not as a
    /// pull. Before the fix `resolve_conflicts` found no unmerged paths, returned `Ok(vec![])`,
    /// and `pulled` went true: the push that followed was rejected non-fast-forward and the user
    /// was shown push stderr for a rebase that had never run.
    #[test]
    fn a_rebase_that_fails_for_another_reason_is_an_error_and_never_a_pull() {
        let (bare, work) = repo_pair("rebase-dirty");
        // A tracked file OUTSIDE the seven staged folders, agreed on by both sides first.
        std::fs::write(work.join("README.md"), "clean\n").unwrap();
        git(&work, &["add", "--", "README.md"]).unwrap();
        git(&work, &["commit", "-q", "-m", "readme"]).unwrap();
        git(&work, &["push", "-q"]).unwrap();
        // The other device moves the remote on, so this sync is behind and has to rebase.
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        std::fs::write(other.join("info").join("from-other.md"), "---\ntype: info\nid: info_0000000002\nstatus: open\ntitle: x\n---\n").unwrap();
        git(&other, &["add", "info"]).unwrap(); git(&other, &["commit", "-q", "-m", "other"]).unwrap(); git(&other, &["push", "-q"]).unwrap();
        let remote_tip = git(&bare, &["rev-parse", "main"]).unwrap();
        // …and here README.md is mid-edit.
        std::fs::write(work.join("README.md"), "dirty, mid-edit\n").unwrap();
        edit(&work, "importance", "1");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed, "the console's own edit is still committed locally: {out:?}");
        assert!(!out.pulled, "a failed rebase is not a pull: {out:?}");
        assert!(!out.pushed, "nothing is pushed after a failed rebase: {out:?}");
        let err = out.status.last_error.clone().unwrap_or_default();
        assert!(err.starts_with("rebase: "), "the rebase's own first line reaches the user: {err:?}");
        assert_eq!(std::fs::read_to_string(work.join("README.md")).unwrap(), "dirty, mid-edit\n", "the dirty file is left exactly as it was");
        assert_eq!(remote_tip, git(&bare, &["rev-parse", "main"]).unwrap(), "the remote never moved");
        assert!(!rebase_in_progress(&work), "no half-started rebase is left behind");
    }

    /// A2: this repo's `.gitignore` hides `state/.sync.lock` and `state/events-ui/`, but a scratch
    /// vault, a friend's vault and every repo in these tests have none — so the exclusion has to
    /// be in the pathspec. The lock is live on disk for the whole of the commit that stages it.
    #[test]
    fn a_vault_with_no_gitignore_never_commits_the_lock_or_the_ui_event_ledger() {
        let (_bare, work) = repo_pair("noignore");
        assert!(!work.join(".gitignore").exists(), "the fixture vault brings no .gitignore of its own");
        // A real interaction event, written through the module that owns that ledger — no test
        // here names a ledger file directly (`ledger::tests::only_this_module_opens_ledger_files`).
        let ev = crate::uievents::UiEvent { session: "sess_history_test", view: "today", action: "view_opened", object_id: None, object_kind: None, ms: None };
        crate::uievents::record(&work, &ev, None).unwrap();
        assert!(work.join("state").join("events-ui").is_dir(), "the ui-event ledger is on disk");
        edit(&work, "importance", "4");
        let ctx = crate::write::WriteContext::new("quinn", "dashboard");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &ctx, &mut j, 1);
        assert!(out.committed && out.pushed, "{out:?}");
        let names = git(&work, &["show", "--name-only", "--format=", "HEAD"]).unwrap();
        assert!(names.contains("tasks/ph-106-exam-1-prep.md"), "the edit itself is committed: {names}");
        assert!(!names.contains("state/.sync.lock"), "the sync lock is never committed: {names}");
        assert!(!names.contains("state/events-ui"), "ui events are opt-in only: {names}");
        let count_before = git(&work, &["rev-list", "--count", "HEAD"]).unwrap();
        let mut j2 = crate::journal::Journal::new(&work);
        let out2 = sync(&work, &ctx, &mut j2, 0);
        assert!(!out2.committed, "a second sync with no edits makes no commit: {out2:?}");
        assert_eq!(count_before, git(&work, &["rev-list", "--count", "HEAD"]).unwrap(), "…so history does not grow one empty commit per sync");
    }

    /// A3 / Plan 2 R-P2-1: `push HEAD:main` publishes whatever is checked out, and a rebase onto
    /// origin/main is just as unsafe on a side branch. The refusal now lands right after the
    /// `has_remote` check and before the fetch, so neither a rebase nor a push ever reaches a
    /// side branch — the tip must not quietly become the shared history.
    #[test]
    fn a_sync_is_refused_before_the_fetch_when_the_vault_is_not_on_main() {
        let (bare, work) = repo_pair("branch");
        git(&work, &["checkout", "-q", "-b", "side"]).unwrap();
        let remote_tip = git(&bare, &["rev-parse", "main"]).unwrap();
        edit(&work, "importance", "2");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed && !out.pulled && !out.pushed, "{out:?}");
        assert_eq!(out.status.last_error.as_deref(), Some("not on main (side) — sync refused"));
        assert_eq!(remote_tip, git(&bare, &["rev-parse", "main"]).unwrap(), "the side branch never became main");
    }

    /// A4 / R-T6: mid-rebase with no `ORIG_HEAD` there is no safe boundary to reconcile against.
    /// The old fallback silently used origin/main's own tip, which dates every un-synced local
    /// record as older than the boundary and drops it with no supersede record.
    #[test]
    fn a_rebase_without_orig_head_is_an_error_not_a_fallback_to_the_remote_tip() {
        let (_bare, work) = repo_pair("no-orig-head");
        let planted = work.join(git(&work, &["rev-parse", "--git-path", "rebase-merge"]).unwrap());
        std::fs::create_dir_all(&planted).unwrap();
        assert!(rebase_in_progress(&work), "a planted rebase-merge dir reads as a rebase in progress");
        let _ = std::fs::remove_file(work.join(".git").join("ORIG_HEAD"));
        let err = conflict_base(&work).unwrap_err();
        assert!(err.starts_with("rebase in progress without ORIG_HEAD"), "{err}");
    }

    /// Plan 2 Task 4 (F12): a vault parked on a side branch is not rebased onto origin/main and
    /// not pushed — the local commit still happens (edits are never lost), the sync says why.
    #[test]
    fn a_side_branch_is_committed_locally_and_neither_rebased_nor_pushed() {
        let (bare, work) = repo_pair("side2");
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        push_a_commit_from(&other, "tasks/remote-edit.md", "remote moved on");
        let remote_before = remote_tip(&bare);
        git(&work, &["checkout", "-q", "-b", "side"]).unwrap();
        edit(&work, "importance", "2");
        let before = tip_of(&work);
        // Final fix wave B1: R-P2-1 says the refusal lands BEFORE the fetch, and nothing here was
        // watching that. The other clone pushed after this vault was cloned, so this vault's
        // origin/main is stale by exactly one commit — a fetch would move it, and reading the
        // same value back afterwards is what says no fetch ran.
        let origin_before = git(&work, &["rev-parse", "origin/main"]).unwrap();
        assert_ne!(origin_before, remote_before, "the guard is only meaningful while origin/main is stale");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed && !out.pulled && !out.pushed, "{out:?}");
        assert_eq!(out.status.last_error.as_deref(), Some("not on main (side) — sync refused"));
        assert_ne!(tip_of(&work), before, "the local commit happened");
        assert_eq!(git(&work, &["rev-parse", "HEAD~1"]).unwrap(), before, "…and nothing was rebased under it");
        assert_eq!(remote_tip(&bare), remote_before, "the remote never moved");
        assert_eq!(git(&work, &["rev-parse", "origin/main"]).unwrap(), origin_before, "…and the refusal came before the fetch — origin/main is still stale");
    }

    /// A rebase that stops with the working tree in `rebase-merge` but NO unmerged paths (an
    /// untracked-file collision, an empty commit needing --skip) is not a conflict to resolve:
    /// it is aborted and reported. `resolve_conflicts` runs only when `git status` lists a `U`.
    #[test]
    fn a_stopped_rebase_without_unmerged_paths_is_aborted_and_reported() {
        let (bare, work) = repo_pair("stuck");
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        push_a_commit_from(&other, "tasks/remote-edit.md", "remote moved on");
        let dir = git(&work, &["rev-parse", "--git-path", "rebase-merge"]).unwrap();
        std::fs::create_dir_all(work.join(&dir)).unwrap();
        assert!(rebase_in_progress(&work) && !unmerged_paths_exist(&work));
        assert!(!conflicts_pending(&work), "conflicts_pending now means unmerged paths, nothing else");
        // Drive it through `sync`: the vault is behind (the push above), so `sync` tries
        // `git rebase origin/main` — which git itself refuses outright, seeing the planted
        // `rebase-merge` directory as an already-in-progress rebase. That refusal has no unmerged
        // paths, so it must land in the abort-and-report arm, not `resolve_conflicts`.
        edit(&work, "importance", "2");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(!out.pulled && !out.pushed, "{out:?}");
        let err = out.status.last_error.clone().unwrap_or_default();
        assert!(err.contains("rebase"), "{err}");
        // `git rebase --abort` on a directory this test planted by hand (no `head-name`/`onto`
        // files — nothing coherent for git to unwind) is attempted but git itself declines to
        // clean it up: observed here, the directory is still `dir_exists() == true` afterwards.
        // That is fine — the module's job is to attempt the abort and report, not to guarantee
        // git can always complete one on a hand-planted, incomplete state. The binding assertions
        // are the ones above: `!pulled && !pushed` and an error naming `rebase`.
        assert!(work.join(&dir).exists(), "git left the planted, incoherent rebase dir in place — this abort was attempted, not completed");
        std::fs::remove_dir_all(work.join(&dir)).ok();
    }

    /// A git invocation that hangs (a credential prompt that GIT_TERMINAL_PROMPT could not stop,
    /// a remote that never answers) is killed at the cap and reported, never waited on forever.
    #[cfg(windows)]
    #[test]
    fn a_hanging_git_call_is_killed_at_the_cap() {
        let (_bare, work) = repo_pair("hang");
        let shim = work.join("slow-git.cmd");
        std::fs::write(&shim, "@echo off\r\nping -n 6 127.0.0.1 >nul\r\n").unwrap();
        let t0 = std::time::Instant::now();
        let out = git_with(shim.to_str().unwrap(), &work, &["status"], std::time::Duration::from_secs(1));
        assert!(t0.elapsed() < std::time::Duration::from_secs(4), "returned at the cap, not at ping's end");
        assert!(out.as_ref().unwrap_err().contains("timed out after 1 s"), "{out:?}");
    }
}
