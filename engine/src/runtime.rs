//! The local inference runtime as a **separate process** (Knowlu spec §5.3 as amended 2026-09-07,
//! decision 12), run **one process per judgment** — never a server, never started once per slot.
//!
//! **Why one process per call, not a loopback server.** A spike
//! (`docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`) measured both shapes against
//! release `b10840` with the 1 B Q4 model this product ships: `SERVER load=1.11s total=43.39s
//! median=1.38s` against `PERCALL total=95.72s median=3.16s`, over a thirty-item batch. The server
//! was faster on every measure, but the plan's pre-committed rule only gives it the design at 3×
//! or a per-call median over 5s, and it managed 2.15×. Per item the server costs 1.45s and
//! per-call 3.19s, so the ratio only ever approaches 2.21× as the batch grows — a bigger batch does
//! not rescue it, because the 1.11s load is already too small to amortise. The lever is **model
//! size, not batch size**: at ~2 GB the per-call cost would be ~4.4s against the server's
//! unchanged 1.45s, which is 3.05× and flips the decision. This product ships the 1–2 B Q4 model
//! spec §5.3 specifies, so: no port, no health check, no lifecycle, and about fifty seconds more
//! per twice-daily background pass.
//!
//! **Why a process and not a library.** `llama-cpp-2` cannot be linked on this product's
//! `stable-x86_64-pc-windows-gnu` toolchain without forking a dependency — llama.cpp's vendored
//! `cpp-httplib` calls `CreateFile2`, which this mingw-w64 does not declare, and it is built
//! whatever `LLAMA_BUILD_SERVER` says (`docs/superpowers/reports/2026-09-07-llama-cpp-on-gnu-spike.md`).
//! The deciding argument is ownership rather than that failure: linking makes us own llama.cpp's
//! build, a separate process lets us consume its releases. Grammar-constrained decoding — which the
//! product plan requires on every model call without exception — is available across the boundary.
//!
//! **Why it is not bundled (decision D6b).** The amended §5.3 says "a second Tauri sidecar
//! (`bundle.externalBin`)". The process boundary is kept and the bundling is not, for three reasons
//! and none of them about installer size: the runtime is **optional** (the free tier runs with no
//! model at all, so bundling makes every install pay for something most will never enable), it
//! **versions independently** (llama.cpp moves far faster than this app; coupling its version to our
//! installer means an app release to pick up a runtime fix), and it is **consistent with the
//! models**, which §5.3 already downloads after install for the same reasons. The only size figures
//! this module cites are the product plan's own `+20–50 MB`
//! (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`) and the spike's measured
//! asset, 18,417,566 bytes.
//!
//! So the app downloads or installs the runtime into its app-data root, checks its SHA-256 against a
//! compiled-in table of supported upstream releases before it is ever run (R-P3a-2,
//! `app/src/inference.rs`), and passes the path in. [`resolve`]'s third branch is nevertheless a
//! sibling `llama-cli.exe` — the same shape `scheduler::engine_exe()` uses — so a future bundled
//! build needs no change here.
//!
//! **Nothing in this module is reached by `rank`** (decision 11) and nothing in it is reached by a
//! test: `cargo test` never starts llama-cli, never downloads a model, and never leaves the
//! loopback interface — there is no interface here at all, only a child process and its stdout and
//! stderr pipes.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::judge::{self, ModelError};

/// The binary this crate looks for. Named once; `app/src/inference.rs` uses the same spelling and a
/// test there asserts the two agree. `llama-cli.exe`, not `llama-server.exe`: outcome B runs one
/// process per judgment, so there is no server to name.
pub const RUNTIME_EXE: &str = "llama-cli.exe";

/// How long one completion — spawn, load, generate, exit — may take. A judge step runs inside a
/// slot whose own child cap is twenty minutes (`scheduler::CHILD_TIMEOUT`), so a per-call bound
/// well under that is what keeps a wedged process from eating the whole slot.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// Where the runtime and the model are — or which of the two is missing.
///
/// Order, for each half: **the argument** (the app passes what it installed), then the environment
/// variable (a developer's own copy, and what the `#[ignore]`d smoke test uses), then — for the
/// runtime only — **a sibling beside the running exe**, which is exactly how
/// `scheduler::engine_exe()` finds the engine.
///
/// A path that exists but is not a file is treated as absent: an app that passed a deleted runtime
/// must get "not installed" now, not a spawn failure later.
pub fn resolve(
    runtime_arg: Option<&Path>,
    model_arg: Option<&Path>,
) -> Result<(PathBuf, PathBuf), judge::Missing> {
    let file = |p: PathBuf| p.is_file().then_some(p);
    let env = |name: &str| {
        std::env::var(name).ok().filter(|s| !s.is_empty()).map(PathBuf::from).and_then(file)
    };
    let runtime = runtime_arg
        .map(Path::to_path_buf)
        .and_then(file)
        // `KNOWLU_RUNTIME`, not `KNOWLU_LLAMA_SERVER` (fix round 1, m8): this module names a
        // process spawned per call, never a server, and the variable is user-facing enough to
        // land in a settings row and a README.
        .or_else(|| env("KNOWLU_RUNTIME"))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|d| d.join(RUNTIME_EXE)))
                .and_then(file)
        })
        .ok_or(judge::Missing::Runtime)?;
    let model = model_arg
        .map(Path::to_path_buf)
        .and_then(file)
        .or_else(|| env("KNOWLU_MODEL"))
        .ok_or(judge::Missing::Model)?;
    Ok((runtime, model))
}

/// Kill the child **and everything it spawned**, then reap it.
///
/// `Child::kill` is one `TerminateProcess` against one pid; a child that forked hands its children
/// inherited duplicates of its handles, so killing the parent alone can leave the tree alive. Windows
/// has no process group to signal, so this is `taskkill /T /F`, with the plain kill after it as the
/// fallback for a machine where `taskkill` is missing or refuses. Deliberately the same shape as
/// `app/src/scheduler.rs`'s, which was written for exactly this failure.
pub fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use crate::childproc::NoConsole;
        let _ = Command::new("taskkill")
            .no_console()
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Read a pipe into bytes, keeping the first 1 MiB and discarding the rest, and say so on a
/// genuine I/O error rather than dropping it (fix round 1, M1/m5). Bytes, not `String`:
/// `Read::read_to_string` leaves its buffer **unchanged** on invalid UTF-8, which turned one bad
/// byte anywhere in the stream into a silent `Ok("")` — the same symptom a crashed `llama-cli`
/// produces, from an unrelated cause.
///
/// Draining continues to EOF **past** the cap (fix round 2, N3): stopping at the cap once meant
/// stopping reading, and a child whose output exceeds it then blocks in `WriteFile` waiting for a
/// reader that has already walked away — the exact deadlock R-3a-6 removed, reintroduced at a
/// 1 MiB threshold instead of a 4 KB one. Unreachable today (`-n 256` cannot produce a megabyte of
/// output), but a reader that stops before EOF is the class of bug this module exists to avoid, at
/// any threshold, not just the one currently reachable.
fn read_capped(mut stream: impl std::io::Read) -> std::io::Result<Vec<u8>> {
    use std::io::Read as _;
    let mut buf = Vec::new();
    (&mut stream).take(1 << 20).read_to_end(&mut buf)?;
    std::io::copy(&mut stream, &mut std::io::sink())?;
    Ok(buf)
}

/// The last ~2000 characters of the child's stderr, as a suffix ready to append to an error
/// string — empty when stderr was empty, so a message never gains a dangling separator.
///
/// By characters, not bytes: `from_utf8_lossy` has already turned the raw bytes into a `String`,
/// and slicing that by byte index can land inside a multi-byte character.
fn stderr_tail(err: &str) -> String {
    let trimmed = err.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let start = chars.len().saturating_sub(2000);
    let tail: String = chars[start..].iter().collect();
    format!(" — stderr: {tail}")
}

/// Wait for the child, reading its stdout **and** stderr on their own threads the whole time.
///
/// Two reader threads, not one. Windows gives a piped stdout a ~4 KB buffer, `llama-cli` echoes
/// the prompt as well as the completion, and this crate's prompt bound allows nearly 6 KB — so a
/// parent that waits first and reads afterwards deadlocks the child in `WriteFile` and then blames
/// the model for timing out (R-3a-6). stderr gets the identical treatment for the identical
/// reason: an undrained second pipe reintroduces exactly that deadlock, just on the other stream
/// (fix round 1, M1). Both streams are read as bytes and capped ([`read_capped`]) then decoded
/// with `from_utf8_lossy` — the prompt echo carries the note's body, which is arbitrary UTF-8, and
/// a byte cap can slice a multi-byte character; one sliced character must not cost the whole
/// completion.
///
/// The judge always exits 0 by design (spec §5.2), so **this function's error text is the only
/// diagnostic that will ever exist** for a friend's laptop: a runtime unzipped without its sibling
/// `ggml-*.dll`, a corrupt or unsupported `.gguf`, an out-of-memory load, an invalid flag. Every
/// one of those writes a clear line to stderr and exits non-zero — discarding it, as the pre-fix
/// version did, turned all of them into "the model gave a bad reply", indistinguishable from an
/// actual bad reply, twice a day, forever.
///
/// The join's own bound is conditional, not absolute: a reader thread ends when the pipe's last
/// write handle closes, and `kill_tree`'s `taskkill /T /F` takes the whole tree, so for
/// `llama-cli` — which spawns nothing — EOF arrives promptly after the kill. A future runtime that
/// forks and re-parents a grandchild outside the tree could hold a write end open past this
/// function's own deadline; nothing here defends against that today.
fn wait_and_read(child: &mut std::process::Child, deadline: Instant) -> Result<String, ModelError> {
    let (stdout, stderr) = match (child.stdout.take(), child.stderr.take()) {
        (Some(o), Some(e)) => (o, e),
        _ => {
            kill_tree(child);
            return Err(ModelError::Failed("llama-cli: no stdout/stderr pipe".to_string()));
        }
    };
    let out_reader = std::thread::spawn(move || read_capped(stdout));
    let err_reader = std::thread::spawn(move || read_capped(stderr));

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                kill_tree(child);
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => {
                kill_tree(child);
                let _ = out_reader.join();
                let _ = err_reader.join();
                return Err(ModelError::Failed(format!("llama-cli: {e}")));
            }
        }
    };

    // Joined before the status (or its absence) is judged: what the child managed to print, on
    // either stream, is what tells a later debugger how far it got — even, especially, when the
    // exit itself is the failure.
    //
    // Both are joined on **every** path below (fix round 2, N2) — a stdout-side panic or I/O
    // error used to return before `err_reader` was ever joined, which detached rather than leaked
    // (the child is already reaped by this point either way) but also threw away the one thing
    // this round exists to preserve: whatever the child had managed to explain about itself on
    // stderr. `err_text` is decoded best-effort — if the stderr reader itself panicked or failed,
    // this falls back to an empty tail rather than compounding the report with a second error.
    let out_result = out_reader.join();
    let err_result = err_reader.join();
    let err_text = match err_result {
        Ok(Ok(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
        _ => String::new(),
    };

    let out = match out_result {
        Ok(Ok(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
        Ok(Err(e)) => {
            return Err(ModelError::Failed(format!(
                "llama-cli: could not read stdout ({e}){}",
                stderr_tail(&err_text)
            )));
        }
        Err(_) => {
            return Err(ModelError::Failed(format!(
                "llama-cli: stdout reader panicked{}",
                stderr_tail(&err_text)
            )));
        }
    };

    let Some(status) = status else {
        // The tail is appended here too (fix round 2, N4): a stall is where llama.cpp's own load
        // progress — a half-loaded `.gguf`, a machine paging — is most likely to be sitting on
        // stderr, already decoded, and the pre-fix version discarded it right where it mattered
        // most.
        return Err(ModelError::Failed(format!("llama-cli: timed out{}", stderr_tail(&err_text))));
    };
    if !status.success() {
        return Err(ModelError::Failed(format!(
            "llama-cli: exited with {status}{}",
            stderr_tail(&err_text)
        )));
    }
    if out.is_empty() {
        return Err(ModelError::Failed(format!(
            "llama-cli: exited 0 without answering{}",
            stderr_tail(&err_text)
        )));
    }
    Ok(out)
}

/// One `llama-cli` process, spawned per judgment. No port, no health check, no lifetime: the
/// process loads the model, generates one completion, writes it to stdout, and exits.
pub struct PerCall {
    runtime: PathBuf,
    model: PathBuf,
    timeout: Duration,
}

impl PerCall {
    pub fn new(runtime: &Path, model: &Path, timeout: Duration) -> PerCall {
        PerCall { runtime: runtime.to_path_buf(), model: model.to_path_buf(), timeout }
    }

    /// One grammar-constrained completion. Returns the model's raw stdout; parsing it is `judge`'s.
    ///
    /// `--single-turn`, not `-no-cnv` (R-3a-2): that flag does not exist on release `b10840` —
    /// `llama-cli` exits immediately with `error: invalid argument: -no-cnv` and does no inference
    /// at all. The long form is used because it survives short-flag reshuffles and reads as what
    /// it means. `inference::SUPPORTED_RUNTIMES` (Task 8) decides which releases may run at all, so
    /// this flag only has to hold for the releases in that table; check it again before adding one.
    pub fn complete(&self, prompt: &str, grammar: &str) -> Result<String, ModelError> {
        // The grammar goes through a file, never an argument: a GBNF rule set carries quotes,
        // backslashes and newlines, and PowerShell 5.1 has already cost this repo one debugging
        // session over a quoted native argument (CLAUDE.md).
        //
        // The leaf is unique per call (fix round 1, M2), not per process: `complete` takes `&self`
        // and the grammar is an argument to it, not a constant of the type — today's only caller
        // passes `judge::GRAMMAR` for every call, but that is a property of the caller, not of
        // this function, and `PerCall` is `Sync`. A shared filename would let one call's delete
        // race another call's child opening it, which — composed with a discarded exit status —
        // used to be a silently wrong answer rather than a loud one.
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!("knowlu-gbnf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| {
            ModelError::Failed(format!("llama-cli: cannot create the grammar directory {} ({e})", dir.display()))
        })?;
        let gbnf = dir.join(format!("judge-{}.gbnf", N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::write(&gbnf, grammar).map_err(|e| {
            ModelError::Failed(format!("llama-cli: cannot write the grammar file {} ({e})", gbnf.display()))
        })?;
        let args: Vec<String> = vec![
            "-m".into(),
            self.model.to_string_lossy().into_owned(),
            "-ngl".into(),
            "0".into(),
            "-c".into(),
            "4096".into(),
            "--grammar-file".into(),
            gbnf.to_string_lossy().into_owned(),
            "-n".into(),
            "256".into(),
            "--temp".into(),
            "0".into(),
            "--single-turn".into(),
            "-p".into(),
            prompt.to_string(),
        ];
        // The containing directory is left in place on every path below (success, spawn failure,
        // timeout, exit failure): removing it would race a concurrent call in this same process
        // that has just written its own uniquely-named grammar file into it.
        use crate::childproc::NoConsole;
        let mut child = match Command::new(&self.runtime)
            .no_console()
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                // A failed spawn returns before a child exists to clean up after; the grammar
                // file is still this call's to remove (fix round 1, m1).
                let _ = std::fs::remove_file(&gbnf);
                return Err(ModelError::Failed(format!("llama-cli: {} ({e})", self.runtime.display())));
            }
        };
        let deadline = Instant::now() + self.timeout;
        let result = wait_and_read(&mut child, deadline);
        let _ = std::fs::remove_file(&gbnf);
        result
    }
}

/// Tier 3, joined up: build the prompt, send it under the grammar, parse what comes back.
///
/// Three lines, and deliberately so — every decision about the schema, the wording and the bounds
/// lives in `judge`, where it is testable without a process, and every decision about the transport
/// lives here, where it is testable without a model.
impl judge::Model for PerCall {
    fn judge(
        &self,
        item: &judge::Item,
        h: &judge::Heuristics,
        seed: &judge::Verdict,
    ) -> Result<judge::Verdict, ModelError> {
        let text = self.complete(&judge::prompt_for(item, h, seed), judge::GRAMMAR)?;
        judge::parse_reply(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-runtime-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The resolution order is the one `scheduler::engine_exe` already uses for the engine, with an
    /// explicit argument in front: the app knows where it put the runtime and says so, and a
    /// developer can drop one beside the exe instead.
    #[test]
    fn resolve_prefers_the_argument_then_the_env_and_names_what_is_missing() {
        let d = tmp("resolve");
        let rt = d.join("llama-cli.exe");
        let gg = d.join("model.gguf");
        std::fs::write(&rt, b"x").unwrap();
        std::fs::write(&gg, b"x").unwrap();

        assert_eq!(resolve(Some(&rt), Some(&gg)).unwrap(), (rt.clone(), gg.clone()));
        // A path that is not a file is not a runtime: an app that passed a deleted one must get
        // "not installed", not a spawn failure twenty seconds later.
        assert_eq!(resolve(Some(&d.join("gone.exe")), Some(&gg)), Err(judge::Missing::Runtime));
        assert_eq!(resolve(Some(&rt), Some(&d.join("gone.gguf"))), Err(judge::Missing::Model));
        assert_eq!(resolve(Some(&rt), None), Err(judge::Missing::Model));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Kill on demand, and it kills the TREE: a child that spawned its own children leaves the
    /// pipes open, which is the failure `scheduler::kill_tree` was written for.
    #[test]
    fn kill_tree_ends_a_process_and_its_children() {
        let mut child = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 60"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("powershell is on every Windows 11 machine");
        assert!(child.try_wait().unwrap().is_none(), "it should still be running");
        kill_tree(&mut child);
        assert!(child.try_wait().unwrap().is_some(), "kill_tree must reap it");
    }

    /// A runtime that is not there is an error naming the path, not a panic, and not a hang.
    #[test]
    fn starting_a_runtime_that_is_not_there_says_which_path() {
        let d = tmp("nostart");
        let rt = d.join("does-not-exist.exe");
        let gg = d.join("model.gguf");
        std::fs::write(&gg, b"x").unwrap();
        let call = PerCall::new(&rt, &gg, Duration::from_secs(1));
        let err = call.complete("p", "g").expect_err("no binary, no completion");
        assert!(format!("{err}").contains("does-not-exist.exe"), "{err}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Without the reader thread this hangs until the deadline and fails: Windows gives a piped
    /// stdout a ~4 KB buffer, and a child writing well past that blocks in `WriteFile` until
    /// someone drains the pipe (R-3a-6).
    #[test]
    fn a_child_that_outwrites_the_pipe_buffer_is_read_whole() {
        let mut child = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "1..4000 | ForEach-Object { Write-Output ('x' * 40) }",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("powershell is on every Windows 11 machine");
        // The deadline here is deliberately the production one (fix round 2, N1 — reversing my
        // own round-1 fix 8, which shortened it to 5s on an unmeasured assumption of "~150ms when
        // passing"). On this test the deadline is not a bound on correctness — it is the FAILURE
        // PATH for the defect the test guards, because the un-fixed implementation fails by
        // hanging. A bound tight enough to be tidy turns machine load into a false positive that
        // looks identical to the regression: measured, this test took 1.03-1.23s idle and failed
        // 3 of 3 runs under 16 concurrent spinner threads and 3 of 3 under 32, panicking with
        // `Failed("llama-cli: timed out")` — the exact message a genuine undrained-pipe
        // regression produces. A slow failure is cheap; a failure that lies is not.
        let deadline = Instant::now() + CALL_TIMEOUT;
        let out = wait_and_read(&mut child, deadline).expect("the full output, read whole");
        // 4000 lines of 40 'x' plus a line terminator each: comfortably over the ~4 KB pipe
        // buffer that would deadlock an unthreaded reader. Asserted exactly, not just "big enough"
        // — `> 100_000` alone would pass a reader that silently lost 40% of the output, which is
        // what a future capped or short-read implementation looks like now that one exists.
        assert_eq!(out.matches('x').count(), 160_000, "every x written must come back");
        assert!(out.len() >= 4000 * 41, "and the line structure with it: {}", out.len());
    }

    /// A call that never comes back must not wedge for twenty minutes.
    #[test]
    fn a_call_that_never_answers_is_bounded() {
        let mut child = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 60"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("powershell is on every Windows 11 machine");
        let started = Instant::now();
        let deadline = started + Duration::from_millis(200);
        let err = wait_and_read(&mut child, deadline).expect_err("a hung process must time out");
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
        assert!(format!("{err}").contains("timed out"), "{err}");
        assert!(child.try_wait().unwrap().is_some(), "the child must be reaped");
    }

    /// The one test that needs a real runtime and a real model — the whole tier-3 path, end to
    /// end: a process starts, a model loads, the grammar constrains the output, and `parse_reply`
    /// reads it back. `#[ignore]`d with the reason in the attribute, the way traps 4 and 5 already
    /// are (spec §8), so `cargo test` never downloads anything and never starts llama-cli.
    #[test]
    #[ignore = "needs a real llama-cli.exe and a .gguf: set KNOWLU_RUNTIME and KNOWLU_MODEL, then cargo test -- --ignored real_runtime"]
    fn real_runtime_loads_a_model_and_answers_under_the_grammar() {
        let (rt, gg) = resolve(None, None).expect("set KNOWLU_RUNTIME and KNOWLU_MODEL");
        let s = PerCall::new(&rt, &gg, CALL_TIMEOUT);
        let out = s
            .complete("Title: CS 100 Homework 3\nDue: 2026-10-01\nBody:\nSubmit online.\n\nJSON:", crate::judge::GRAMMAR)
            .expect("a completion");
        let v = crate::judge::parse_reply(&out).expect("the grammar guarantees a parseable object");
        assert!(v.importance.is_some() && v.effort_hours.is_some(), "{out}");
        assert!((0.0..=1.0).contains(&v.confidence), "{out}");
    }
}
