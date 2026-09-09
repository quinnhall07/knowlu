# Spike: the sidecar protocol, measured — loopback server vs one process per call

**Date:** 2026-09-07. **Asked because** Knowlu plan 3a Tasks 3–7 need one decision made before
they can be written: does the app talk to `llama.cpp` through a loopback server started once per
slot, or one process per call? See `docs/superpowers/reports/2026-09-07-llama-cpp-on-gnu-spike.md`
for why there is a process boundary at all (linking `llama.cpp` into the crate does not build on
this GNU toolchain). **Nothing was committed to either crate**; the probe ran in a scratch
directory under `%TEMP%` that has been deleted.

## Outcome: B

**One process per call.** By the pre-committed decision rule (step 8), applied to the numbers
actually measured below — not to the expected answer. See "Arithmetic" for the exact test.

## Runtime obtained

- **Release tag:** `b10840` (the repo's bare "latest" GitHub release, `v0.4.0`, carries no
  binaries — it is a pointer file, `nightly-tag.txt`, naming the current nightly build tag. The
  actual binary releases are tagged `b<number>`; `b10840` was the newest at fetch time.)
- **Asset name:** `llama-b10840-bin-win-cpu-x64.zip` (the brief's literal
  `llama-bin-win-cpu-x64.zip` on `/releases/latest/download/` 404s for the reason above — the
  asset naming embeds the build number, and "latest" has no assets at all)
- **Byte size:** 18417566 (matches the GitHub API's declared asset size and the downloaded file)
- **SHA-256:** `7063dfc6b874e7eee0ddf601bdf8e70e6f4a3d708926641ffad046ec51e8e30b`

**Zip internal layout — flat, no nesting.** Everything lands directly under the extraction root
(`rt/llama-server.exe`, `rt/llama-cli.exe`, all the other `llama-*.exe` tools, and the `ggml-*.dll` /
`llama-*.dll` runtime libraries side by side). There is no `bin/` subdirectory and no per-tool
folder. A later task that extracts this zip and walks it for the executable should look for
`llama-server.exe` / `llama-cli.exe` at the top level of the extracted tree, not nested.

## Model obtained

- **URL:** `https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf`
- **Size:** 807694464 bytes (~770 MiB)
- **SHA-256:** `6f85a640a97cf2bf5b8e764087b1e83da0fdb51d7c9fab7d0fece9385611df83`

## Flags

All six of Task 3's expected flags are spelled exactly as expected on `llama-server.exe --help`
(release `b10840`):

**Flags:** `-m`/`--model` (model path to load) — `--host` (ip address to listen) — `--port`
(port to listen, default 8080) — `-c`/`--ctx-size` (size of the prompt context) —
`-ngl`/`--n-gpu-layers` (also aliased `--gpu-layers`) — `/health` and `/completion` confirmed live
by actually calling them in step 5 (the `--help` text does not document REST paths, only CLI
flags, so this is empirical, not textual, confirmation). No divergence from Task 3's expectations
on any of the six.

**One divergence found, on `llama-cli.exe`, not on the six above:** the brief's step 6 script
passes `-no-cnv` to suppress interactive/conversation mode. On this build, `-no-cnv` is not a
recognized argument at all (`error: invalid argument: -no-cnv`) — the flag has apparently been
removed upstream. `llama-cli.exe --help` shows no `-cnv`/`--conversation` toggle of any kind
in its place; the closest equivalent is `-st`/`--single-turn` ("run conversation for a single
turn only, then exit when done"), which is what the measurement below actually used. **This
matters directly for the Outcome B replacement code below**, which is pasted verbatim from the
brief and still says `-no-cnv` — see the caveat placed immediately above that code block.

## Batch

Thirty synthesized items built per the brief's script (no `needs_enrichment: true` tasks exist in
the fixture vault, confirmed empty by grep before running). `batch.txt` was 4418 bytes.

## Measurement: server

Windows Firewall was **not observed to prompt** at any point (see risk answer 1). The server was
started via a Python `subprocess.Popen` (not via a bash `&` background, for reliability — see the
task's ambiguity note 3) so that the load timer could start at the exact moment of spawn, and the
completions ran through the same process.

```
SERVER load=1.11s items=30 total=43.39s median=1.38s p90=1.90s
```

(An earlier run using `Start-Process` with a gap between spawning the server and starting the
health-poll loop — spent partly observing for a firewall dialog — produced a `load=0.01s` line
that is **not used**: the model had already finished loading in the gap between the two tool
calls, before polling began. That number is not a real load time and is recorded here only so a
reader of the working history is not misled by it. The number above, from a script that spawns
the process and starts polling in the same statement, is the one used in the arithmetic.)

## Measurement: one process per call

Ran with `-st` in place of the brief's `-no-cnv` (see Flags, above); everything else exactly as
the brief's step 6 script. One process per item, full reload every time.

```
PERCALL items=30 total=95.72s median=3.16s p90=3.95s
```

## Risk answers (step 7)

1. **Did Windows Firewall prompt?** No. Checked twice — once shortly after the server was
   started, once again with the process confirmed still alive — by listing all processes with a
   non-empty `MainWindowTitle` (a non-destructive, non-clicking observation, per the desktop-safety
   constraint). No firewall or "Windows Security Alert" window appeared either time. Matches the
   expectation that Windows does not prompt for a `127.0.0.1`-only listener.
2. **What happens to the port on a crash?** The server was killed mid-request
   (`taskkill /T /F /PID <pid>`, fired ~200ms into an in-flight `/completion` call, which then
   failed client-side with `ConnectionResetError(10054, ...)` as expected). Immediately afterward,
   a fresh Python `socket.socket().bind(("127.0.0.1", 18099))` **bound on the first try** — no
   `TIME_WAIT`/`WSAEADDRINUSE` observed. Confirms Task 3 is right to ask the OS for an ephemeral
   port rather than assume a fixed one is free, but a crash does not itself leave the port stuck.
3. **How long is the load?** `load=1.11s` (see above), for a model of 807694464 bytes (~770 MiB)
   on CPU with `-ngl 0`.

## Arithmetic

- `SERVER total + SERVER load` = 43.39 + 1.11 = **44.50s**
- Outcome A's threshold (`PERCALL total >= 3×` that sum) = 3 × 44.50 = **133.50s**
- `PERCALL total` = **95.72s** — this is **less than** 133.50s, so Outcome A's first condition
  (≥3×) is **not met**.
- `PERCALL median` = **3.16s** — not greater than 5s, so Outcome A's second condition is **not
  met** either.
- Outcome A therefore does not apply, by the rule as written.
- Outcome B's third disjunct — "came within 3× of per-call" — reads on the same numbers:
  95.72 / 44.50 ≈ **2.15×**, which is within (less than) 3×. That disjunct is **met**, independent
  of the (also-false) firewall and bind-failure disjuncts.
- Outcome B is the result. The server was still faster in absolute terms on every measure (total,
  median, and p90 all lower for SERVER than PERCALL) — it just did not clear the 3× amortisation
  bar the pre-committed rule set as the threshold for taking on the server's added lifecycle
  complexity, at this batch size (30) and this model size (1B Q4).

## Outcome B replacement

**Caveat before the pasted code:** the brief's `PerCall::complete` below spawns `llama-cli.exe`
with a literal `"-no-cnv".into()` argument. On the runtime actually measured here (release
`b10840`), that flag does not exist and the process exits immediately with
`error: invalid argument: -no-cnv` rather than doing any inference — this cost the per-call
measurement one round of diagnosis before the correct flag, `-st` (`--single-turn`), was found
in `llama-cli.exe --help` and substituted for the timing run above. **Pasted verbatim from the
brief, unedited, per its instruction** — do not assume the snippet below runs as-is.

**Controller ruling R-3a-2 (2026-09-07), which settles it:** Task 3 spells that argument
`--single-turn`, the long form, in place of `-no-cnv`. Not runtime detection, and not pinning an
older release: `inference::SUPPORTED_RUNTIMES` decides which releases may be executed at all, so
the flag only has to be accepted by the releases in that table, and the table starts at `b10840`,
which rejects `-no-cnv` and accepts `--single-turn`. The long form is used because it survives
short-flag reshuffles and reads as what it means. Any later release added to the table is checked
against this flag before it goes in.

```rust
/// Outcome B: one process per call. No port, no health check, no lifetime — `llama-cli` is
/// spawned per judgment, writes its completion to stdout, and exits. The model re-loads every
/// time, which is why this is the losing branch whenever the server is available.
pub struct PerCall {
    runtime: PathBuf,
    model: PathBuf,
    timeout: std::time::Duration,
}

impl PerCall {
    pub fn new(runtime: &Path, model: &Path, timeout: std::time::Duration) -> PerCall {
        PerCall { runtime: runtime.to_path_buf(), model: model.to_path_buf(), timeout }
    }

    pub fn complete(&self, prompt: &str, grammar: &str) -> Result<String, ModelError> {
        // The grammar goes through a file, never an argument: a GBNF rule set carries quotes,
        // backslashes and newlines, and PowerShell 5.1 has already cost this repo one debugging
        // session over a quoted native argument (CLAUDE.md).
        let dir = std::env::temp_dir().join(format!("knowlu-gbnf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| ModelError::Failed(e.to_string()))?;
        let gbnf = dir.join("judge.gbnf");
        std::fs::write(&gbnf, grammar).map_err(|e| ModelError::Failed(e.to_string()))?;
        let args: Vec<String> = vec![
            "-m".into(), self.model.to_string_lossy().into_owned(),
            "-ngl".into(), "0".into(),
            "-c".into(), "4096".into(),
            "--grammar-file".into(), gbnf.to_string_lossy().into_owned(),
            "-n".into(), "256".into(),
            "--temp".into(), "0".into(),
            "-no-cnv".into(),
            "-p".into(), prompt.to_string(),
        ];
        let mut child = std::process::Command::new(&self.runtime)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| ModelError::Failed(format!("llama-cli: {} ({e})", self.runtime.display())))?;
        let deadline = std::time::Instant::now() + self.timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() >= deadline => {
                    kill_tree(&mut child);
                    return Err(ModelError::Failed("llama-cli: timed out".into()));
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(e) => return Err(ModelError::Failed(format!("llama-cli: {e}"))),
            }
        }
        let mut out = String::new();
        if let Some(mut s) = child.stdout.take() {
            use std::io::Read;
            let _ = s.read_to_string(&mut out);
        }
        let _ = std::fs::remove_file(&gbnf);
        Ok(out)
    }
}

impl crate::judge::Model for PerCall {
    fn judge(
        &self,
        item: &crate::judge::Item,
        h: &crate::judge::Heuristics,
        seed: &crate::judge::Verdict,
    ) -> Result<crate::judge::Verdict, ModelError> {
        let text = self.complete(&crate::judge::prompt_for(item, h, seed), crate::judge::GRAMMAR)?;
        crate::judge::parse_reply(&text)
    }
}
```

Under outcome B, Task 3 keeps `RUNTIME_EXE = "llama-cli.exe"`, `resolve`, `kill_tree` and the
resolution tests, drops `Server`/`free_port`/`wait_healthy` and the loopback tests, and
`enrich::run_lines` constructs `PerCall::new(&rt, &gg, CALL_TIMEOUT)` where it would have called
`Server::start`. Per the brief's own routing table for this outcome, `LOAD_TIMEOUT` goes and
`CALL_TIMEOUT` stays as `PerCall`'s; Task 4 step 5's smoke test constructs
`PerCall::new(&rt, &gg, CALL_TIMEOUT)`; Task 7's `enrich::run_lines` match arm becomes
`Ok((rt, gguf)) => enrich_with(vault, opts, Ok(&crate::runtime::PerCall::new(&rt, &gguf, crate::runtime::CALL_TIMEOUT)))`;
Task 8 needs no change provided its tests reference `RUNTIME_EXE` rather than a literal.

## Notes for later tasks

- **`/releases/latest/download/<name>` does not work for this repo** because the GitHub "latest"
  release is a nightly pointer with no binary assets. Whatever downloads the runtime at install
  time (or whatever seeds `inference::SUPPORTED_RUNTIMES`) needs to resolve a real `b<number>`
  release — e.g. `GET /repos/ggml-org/llama.cpp/releases/tags/b<N>` after reading the nightly tag
  from `.../releases/latest/download/nightly-tag.txt`, or by walking `/releases?per_page=…` — not
  assume a fixed download URL.
- The zip is flat (see "Zip internal layout" above) — no nested `bin/` directory.
- `llama-cli.exe`'s `-no-cnv` flag does not exist on release `b10840`; `-st`/`--single-turn` is
  the replacement found by trial. This is exactly the kind of runtime-release drift `Flags:`
  exists to catch — a later task pinning a specific release should re-check this flag against
  whatever release it actually ships.
- The server was faster on every measure (load+total, median, p90) but did not clear the
  pre-committed 3× bar — see Arithmetic. **A larger batch would not change that**, and an earlier
  draft of this note said the opposite. Per item the server costs 43.39/30 = **1.45s** and one
  process per call costs 95.72/30 = **3.19s**, so as the batch grows the ratio rises from 2.15×
  only towards its asymptote of 3.19/1.45 ≈ **2.21×**. It never reaches 3×: the fixed 1.11s load
  is already too small for further amortisation to buy anything.
- **The lever is model size, not batch size.** Per-call's ~1.75s penalty per item is a model
  reload. With a ~2 GB model loading in ~3s, per-call's item cost would be ~4.4s against the
  server's unchanged ~1.45s — 3.05×, which would flip the outcome to A. This product ships the
  1–2 B Q4 model §5.3 specifies, and that is what was measured, so B is the answer for what
  actually ships. If the shipped model ever grows past roughly 1.5 GB, re-run this spike.
