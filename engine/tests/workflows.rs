//! Static pins on the two GitHub workflows: an action referenced by a floating tag can change under
//! us, `pull_request_target` hands secrets to fork code, and a secret renamed in the workflow but not
//! in the repository settings fails only on release day.
use std::path::Path;

fn workflow(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".github").join("workflows").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn uses_lines(text: &str) -> Vec<String> {
    text.lines().map(str::trim).filter(|l| l.starts_with("- uses:") || l.starts_with("uses:")).map(String::from).collect()
}

/// R2-7: the block for one top-level job, `<name>:` through the line directly before the NEXT
/// top-level job key (`^  [a-z-]+:$`, two-space indent) or EOF — never simply "to end of file",
/// which was correct only by accident while the named job happened to be the workflow's last one.
/// A job appended after it would otherwise let that later job's own content satisfy assertions
/// meant for this one.
fn job_block<'a>(text: &'a str, name: &str) -> &'a str {
    let key_re = regex::Regex::new(r"(?m)^  [a-z-]+:\s*$").unwrap();
    let starts: Vec<usize> = key_re.find_iter(text).map(|m| m.start()).collect();
    let needle = format!("  {name}:");
    let job_start = *starts
        .iter()
        .find(|&&s| text[s..].starts_with(&needle))
        .unwrap_or_else(|| panic!("{name}: no such top-level job key"));
    let job_end = starts.into_iter().find(|&s| s > job_start).unwrap_or(text.len());
    &text[job_start..job_end]
}

#[test]
fn every_action_is_pinned_to_a_full_commit_sha() {
    for name in ["ci.yml", "release.yml"] {
        for line in uses_lines(&workflow(name)) {
            let re = regex::Regex::new(r"uses:\s*\S+@[0-9a-f]{40}(\s+#.*)?$").unwrap();
            assert!(re.is_match(&line), "{name}: not SHA-pinned: {line}");
        }
    }
}

#[test]
fn no_workflow_uses_pull_request_target() {
    for name in ["ci.yml", "release.yml"] {
        assert!(!workflow(name).contains("pull_request_target"), "{name} uses pull_request_target");
    }
}

#[test]
fn release_runs_only_on_version_tags_and_names_every_secret_the_script_needs() {
    let r = workflow("release.yml");
    assert!(r.contains("tags:") && r.contains("'v*'"), "release.yml must trigger on tags v*");
    for s in [
        "TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
        "AZURE_CLIENT_ID", "AZURE_TENANT_ID", "AZURE_SUBSCRIPTION_ID",
        "CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID",
    ] {
        assert!(r.contains(&format!("secrets.{s}")), "release.yml does not reference secrets.{s}");
    }
    assert!(r.contains("id-token: write"), "release.yml needs id-token: write for Azure OIDC");
}

/// The unsigned-release switch (C0, 2026-09-17: Quinn cannot complete Trusted Signing's identity
/// verification for now). Authenticode is skipped ONLY when the repository VARIABLE
/// `RELEASE_AUTHENTICODE` is the literal `off` — never because a secret or a variable happens to be
/// missing, which is how an unsigned installer would otherwise ship by accident — and the release
/// then says so in its notes and asserts it in its check. The updater signature is never optional.
#[test]
fn release_ships_unsigned_only_on_the_explicit_variable() {
    let r = workflow("release.yml");
    let gate = "if: vars.RELEASE_AUTHENTICODE != 'off'";
    assert_eq!(r.matches(gate).count(), 3, "azure/login, the signing client and the profile step must each carry `{gate}`");
    let branch = "$env:RELEASE_AUTHENTICODE -eq \"off\"";
    assert!(r.matches(branch).count() >= 3, "the notes, the release.ps1 call and the verify step must each branch on `{branch}`");
    assert!(r.contains("NotSigned"), "the verify step must assert NotSigned on the unsigned path, not skip the check");
    assert!(r.contains("**Unsigned build.**"), "the release notes must say the build is unsigned");
    for line in r.lines().map(str::trim).filter(|l| l.starts_with("if:")) {
        assert!(!line.contains("secrets."), "never key a step on a secret's presence: {line}");
    }
}

/// C0 (2026-09-17): the first release run spent 21 of its 40 minutes compiling tauri-cli from
/// source (run 35179886527). The CLI is a tool, not the product: it is downloaded prebuilt for the
/// exact pinned version and checked against a SHA-256 spelled in the workflow - the contract
/// every downloaded tool follows - so a bump is the version AND the hash, and a wrong
/// archive stops the run before anything is built.
#[test]
fn release_downloads_tauri_cli_prebuilt_and_pinned_by_sha256_never_compiles_it() {
    let r = workflow("release.yml");
    assert!(!r.contains("cargo install tauri-cli"), "release.yml must not compile tauri-cli from source");
    assert!(r.contains(r"scripts\ci\tauri-cli.ps1"), "the tauri-cli step must run scripts\\ci\\tauri-cli.ps1");
    let version = regex::Regex::new(r#"TAURI_CLI_VERSION:\s*"(\d+\.\d+\.\d+)""#).unwrap()
        .captures(&r).map(|c| c[1].to_string()).expect("release.yml must pin TAURI_CLI_VERSION");
    let sha = regex::Regex::new(r#"TAURI_CLI_SHA256:\s*"([0-9a-f]{64})""#).unwrap()
        .captures(&r).map(|c| c[1].to_string()).expect("release.yml must pin TAURI_CLI_SHA256 as 64 lowercase hex characters");
    assert_eq!(sha.len(), 64);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let script = std::fs::read_to_string(root.join("scripts").join("ci").join("tauri-cli.ps1")).expect("scripts/ci/tauri-cli.ps1");
    assert!(script.contains("Get-FileHash") && script.contains("SHA256"), "the script must verify the archive with Get-FileHash SHA256");
    assert!(script.contains("mismatch"), "the script must throw on a hash mismatch");
    assert!(script.contains("cargo-tauri-x86_64-pc-windows-msvc.zip"), "the script must download Tauri's own prebuilt Windows archive");
    // release.ps1 names the tauri-cli version it was written against; the two pins never drift apart.
    let release_ps1 = std::fs::read_to_string(root.join("scripts").join("release.ps1")).expect("scripts/release.ps1");
    assert!(release_ps1.contains(&format!("tauri-cli {version}")), "scripts/release.ps1 must name tauri-cli {version}, the version release.yml pins");
}

/// C0 (2026-09-17): build and publish are two jobs joined by an artifact, so a publish-only failure
/// - the first release run's, a wrong Cloudflare token after a 40-minute build - reruns in minutes
/// with `gh run rerun --failed`; the build's cache is saved even when the build fails; and each job
/// holds only the permission it uses (OIDC in build, the Release in publish). The publish job never
/// compiles, signs or reads a signing secret.
#[test]
fn release_builds_and_publishes_in_two_jobs_joined_by_an_artifact() {
    let r = workflow("release.yml");
    let build = job_block(&r, "build");
    let publish = job_block(&r, "publish");
    assert!(publish.contains("needs: build"), "publish must depend on build");
    assert!(build.contains("actions/upload-artifact@") && !publish.contains("actions/upload-artifact@"), "build uploads the artifact");
    assert!(publish.contains("actions/download-artifact@") && !build.contains("actions/download-artifact@"), "publish downloads the artifact");
    assert!(build.contains("cache-on-failure: true"), "the release cache must be saved on failure, as ci.yml's is");
    assert!(build.contains("id-token: write") && !publish.contains("id-token: write"), "only build needs the OIDC token");
    assert!(publish.contains("contents: write") && !build.contains("contents: write"), "only publish needs to write the Release");
    let before_jobs = r.split("\njobs:").next().unwrap();
    assert!(!before_jobs.lines().any(|l| l.trim() == "permissions:"), "no workflow-level permissions block: each job declares its own");
    for s in ["release.ps1", "cargo", "azure/login", "verify Authenticode", "TAURI_SIGNING_PRIVATE_KEY"] {
        assert!(build.contains(s), "build must carry {s}");
        assert!(!publish.contains(s), "publish must not mention {s} - it never compiles or signs");
    }
    for s in ["cloudflare/wrangler-action@", "gh release create"] {
        assert!(publish.contains(s), "publish must carry {s}");
        assert!(!build.contains(s), "build must not publish: {s}");
    }
    assert_eq!(r.matches("environment: release").count(), 2, "both jobs declare the release environment: build for the OIDC subject, publish for the reviewer gate");
    assert!(publish.contains("needs.build.outputs.installer"), "publish names the installer through the build job's outputs");
}

#[test]
fn ci_enforces_zero_warnings_and_the_eol_contract() {
    let c = workflow("ci.yml");
    assert!(c.contains("RUSTFLAGS") && c.contains("-D warnings"), "ci.yml must set RUSTFLAGS=-D warnings");
    assert!(c.contains("scripts/ci/eol-check.ps1"), "ci.yml must run the eol check");
    assert!(c.contains("cargo test --workspace"), "ci.yml must test the whole workspace");
}

/// 2026-09-22 (Quinn: the Actions minutes ran out; this laptop becomes a runner). Every `ci.yml` job
/// takes its runner from ONE switch — the repository variable `CI_SELF_HOSTED` — and falls back to
/// the hosted image it used before, so going back is deleting a variable, never a code change. The
/// release workflow never runs self-hosted: its signing and publish secrets stay on GitHub's machines.
#[test]
fn ci_jobs_run_self_hosted_only_behind_the_switch_and_release_never_does() {
    let c = workflow("ci.yml");
    let switch = "vars.CI_SELF_HOSTED == 'on' && fromJSON('[\"self-hosted\",\"windows\",\"x64\",\"knowlu-ci\"]')";
    for (job, fallback) in [("test", "windows-latest"), ("cloud", "ubuntu-latest"), ("eval-gate", "ubuntu-latest")] {
        let block = job_block(&c, job);
        let expected = format!("runs-on: ${{{{ {switch} || '{fallback}' }}}}");
        assert!(block.contains(&expected), "{job}: expected `{expected}` in:\n{block}");
        assert_eq!(block.matches("runs-on:").count(), 1, "{job}: exactly one runs-on");
    }
    assert!(!workflow("release.yml").contains("self-hosted"), "release.yml must never run self-hosted");
    assert!(!workflow("release.yml").contains("CI_SELF_HOSTED"), "release.yml must not read the CI switch");
}

/// A-6 (C2 final review): the eval gate exists, never runs on a plain push (a fork PR's `push` to
/// its own branch must not spend the staging service-role key), and names both secrets it needs —
/// so a secret renamed here but not in the repository's settings fails this test rather than only
/// on the first PR that touches a prompt.
#[test]
fn the_eval_gate_job_exists_is_pull_request_only_and_names_both_its_secrets() {
    let c = workflow("ci.yml");
    let job = job_block(&c, "eval-gate");
    assert!(
        job.contains("if: ${{ !cancelled() && github.event_name == 'pull_request' }}"),
        "eval-gate must run only on pull_request: {job}"
    );
    assert!(job.contains("secrets.OPENROUTER_API_KEY"), "eval-gate must reference secrets.OPENROUTER_API_KEY");
    assert!(
        job.contains("secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY"),
        "eval-gate must reference secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY"
    );
    // Provider swap Task 4: the eval runner now calls OpenRouter, so the Anthropic secret has no
    // reason to be in this job at all — its lingering presence would mean a key nothing reads.
    assert!(
        !job.contains("secrets.ANTHROPIC_API_KEY"),
        "eval-gate must not reference secrets.ANTHROPIC_API_KEY any more: {job}"
    );
}

/// R2-7's own regression test: with `job_block` unbounded (the pre-fix "to end of file" slice), a
/// job appended after `eval-gate:` whose OWN body happens to contain everything the assertions look
/// for would let a real `eval-gate` job that is missing all three pass anyway. This proves the block
/// stops at the next top-level job key, so `eval-gate`'s own (empty) body is what gets asserted on.
#[test]
fn job_block_stops_at_the_next_top_level_job_never_reads_into_it() {
    let synthetic = "\
name: ci
jobs:
  eval-gate:
    runs-on: ubuntu-latest
  zzz-later-job:
    if: github.event_name == 'pull_request'
    steps:
      - run: echo \"uses secrets.OPENROUTER_API_KEY and secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY\"
";
    let job = job_block(synthetic, "eval-gate");
    assert!(!job.contains("zzz-later-job"), "the block must not reach the next job at all: {job:?}");
    assert!(!job.contains("if: github.event_name == 'pull_request'"), "the later job's content leaked in: {job:?}");
    assert!(!job.contains("secrets.OPENROUTER_API_KEY"), "the later job's content leaked in: {job:?}");

    // And with no job after it, the block still reaches EOF exactly as before.
    let last = "\
name: ci
jobs:
  earlier-job:
    runs-on: ubuntu-latest
  eval-gate:
    if: github.event_name == 'pull_request'
";
    let job = job_block(last, "eval-gate");
    assert!(job.contains("if: github.event_name == 'pull_request'"), "eval-gate as the last job must still be read to EOF: {job:?}");
}

/// The docs-only deadlock: `paths-ignore` on the triggers meant a document-only PR never reported the
/// three REQUIRED checks (branch protection names `test`, `cloud`, `eval-gate`), so it could never
/// merge. The triggers now carry no path filter; a first `changes` job decides, with plain git, whether
/// anything outside the document paths changed, and the heavy steps of the three required jobs skip
/// on `needs.changes.outputs.code` while the jobs themselves always run and succeed. A job-level `if`
/// that names `changes` (or any `if` on `test`/`cloud`) would skip the check again and reopen the
/// deadlock; `eval-gate`'s one job-level `if` is its pull-request-only trigger and nothing else.
///
/// The gate must also never fail open (a skipped job reads as a pass to a required check). Two holes
/// are closed here: `git diff --name-only` pairs a rename into one destination path, so a source file
/// moved into `docs/` would read as a document-only change (`--no-renames` lists the deleted source
/// too); and a failed `changes` job would skip every job that `needs` it. So each of the three jobs
/// carries exactly one job-level `if`, `!cancelled()` (plus `eval-gate`'s pull-request trigger), which
/// runs it even when `changes` failed, and every step condition reads `code != 'false'` or
/// `code == 'false'`, never `== 'true'`, so a missing output runs everything.
#[test]
fn docs_only_changes_still_report_the_three_required_checks() {
    let c = workflow("ci.yml");
    assert!(!c.contains("paths-ignore"), "ci.yml must not use paths-ignore: a docs-only PR would never report the required checks");
    let changes = job_block(&c, "changes");
    assert!(changes.contains("git diff --no-renames --name-only"), "`changes` must decide with plain git and without rename detection: {changes}");
    assert!(changes.contains("outputs:") && changes.contains("code:"), "`changes` must publish a `code` output: {changes}");
    assert!(changes.contains("0000000000000000000000000000000000000000"), "`changes` must treat the zero before-sha as code changed");
    for job in ["test", "cloud", "eval-gate"] {
        let block = job_block(&c, job);
        assert!(block.contains("changes"), "{job}: must depend on the `changes` job");
        let job_ifs: Vec<&str> = block.lines().filter(|l| l.starts_with("    if:")).collect();
        match job {
            "eval-gate" => assert_eq!(
                job_ifs,
                ["    if: ${{ !cancelled() && github.event_name == 'pull_request' }}"],
                "eval-gate's only job-level if is its trigger, behind !cancelled()"
            ),
            _ => assert_eq!(job_ifs, ["    if: ${{ !cancelled() }}"], "{job}: the only job-level if is !cancelled(), so a failed `changes` cannot skip it"),
        }
        assert!(!block.contains("outputs.code == 'true'"), "{job}: a step keyed on `== 'true'` fails open when `changes` fails: {block}");
        assert!(!block.contains("code != 'true'"), "{job}: `!= 'true'` runs the no-op when the output is missing: {block}");
        assert!(block.contains("needs.changes.outputs.code"), "{job}: heavy steps must key on needs.changes.outputs.code");
    }
}

/// D18, as Quinn ruled it on 2026-10-04 (session 4): email forwarding may merge to `main` before
/// privacy bump #1, but no release may ship it until the page discloses it. The guard is
/// mechanical: `scripts/ci/privacy-guard.ps1` is the build job's first step after checkout, runs
/// unconditionally, and fails the release while any mail marker is in the tree and
/// `account::PRIVACY_VERSION` is on or before `2026-09-24`, the version the lane was signed against.
#[test]
fn release_refuses_mail_code_until_privacy_bump_1() {
    let r = workflow("release.yml");
    let build = job_block(&r, "build");
    let call = r".\scripts\ci\privacy-guard.ps1";
    assert_eq!(r.matches(call).count(), 1, "release.yml must run {call} exactly once");
    let at = build.find(call).expect("the guard must run in the build job, before anything is built");
    for later in [r".\scripts\ci\toolchain.ps1", "name: tauri-cli", r".\scripts\release.ps1", "actions/upload-artifact@"] {
        let l = build.find(later).unwrap_or_else(|| panic!("build job lost {later}"));
        assert!(at < l, "the guard must run before {later}");
    }
    // The step itself: from its `- name:` to the next step. No `if:` may skip it.
    let start = build[..at].rfind("      - ").expect("the guard is a step");
    let end = build[at..].find("\n      - ").map_or(build.len(), |e| at + e);
    let step = &build[start..end];
    assert!(step.contains("name: privacy guard (D18)"), "the step is named for D18: {step}");
    assert!(!step.contains("if:") && !step.contains("continue-on-error"), "the guard is never skipped or softened: {step}");
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Runs the guard against `tree`; `(passed, output)`.
fn privacy_guard(tree: &Path) -> (bool, String) {
    let script = root().join("scripts").join("ci").join("privacy-guard.ps1");
    assert!(script.is_file(), "{} is missing", script.display());
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg("-Root")
        .arg(tree)
        .output()
        .expect("powershell");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// A synthetic tree under the system temp directory: `account.rs` (when `version` is `Some`) and
/// the named marker paths. Removed on drop.
struct Tree(std::path::PathBuf);
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn tree(tag: &str, version: Option<&str>, markers: &[&str]) -> Tree {
    let dir = std::env::temp_dir().join(format!("knowlu-privacy-guard-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app").join("src")).unwrap();
    if let Some(v) = version {
        let account = format!("pub const TOS_VERSION: &str = \"2026-09-16\";\npub const PRIVACY_VERSION: &str = \"{v}\";\n");
        std::fs::write(dir.join("app").join("src").join("account.rs"), account).unwrap();
    }
    for m in markers {
        let p = dir.join(m);
        if m.ends_with(".rs") {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "// marker\n").unwrap();
        } else {
            std::fs::create_dir_all(&p).unwrap();
        }
    }
    Tree(dir)
}

const MAIL_MARKERS: [&str; 3] = ["cloud/supabase/functions/mail-inbound", "app/src/mail_steps.rs", "engine/src/mailreconcile"];

/// The guard's condition, run for real against synthetic trees: each mail marker alone blocks a
/// release at or before `2026-09-24`; a later version or a tree without mail code passes; an
/// unreadable version blocks (fail closed); and on this very tree the verdict is the one the rule
/// gives, so a moved marker or `account.rs` cannot quietly turn the guard into a pass.
#[test]
fn the_privacy_guard_blocks_mail_code_at_or_before_the_pre_bump_version() {
    for (i, m) in MAIL_MARKERS.iter().enumerate() {
        for v in ["2026-09-24", "2026-09-16"] {
            let t = tree(&format!("m{i}-{v}"), Some(v), &[m]);
            let (ok, out) = privacy_guard(&t.0);
            assert!(!ok, "{m} with PRIVACY_VERSION {v} must block the release:\n{out}");
            assert!(out.contains("D18") && out.contains(v), "the refusal names D18 and the version:\n{out}");
        }
    }
    let t = tree("bumped", Some("2026-10-15"), &MAIL_MARKERS);
    let (ok, out) = privacy_guard(&t.0);
    assert!(ok, "a version past 2026-09-24 lets mail code ship:\n{out}");
    let t = tree("no-mail", Some("2026-09-24"), &[]);
    let (ok, out) = privacy_guard(&t.0);
    assert!(ok, "a tree without mail code is not D18's business:\n{out}");
    let t = tree("no-account", None, &[MAIL_MARKERS[0]]);
    let (ok, out) = privacy_guard(&t.0);
    assert!(!ok, "no readable PRIVACY_VERSION must block, never pass:\n{out}");
    let t = tree("bad-version", Some("soon"), &[MAIL_MARKERS[0]]);
    let (ok, out) = privacy_guard(&t.0);
    assert!(!ok, "a PRIVACY_VERSION that is not YYYY-MM-DD must block:\n{out}");

    // This tree.
    let account = std::fs::read_to_string(root().join("app").join("src").join("account.rs")).unwrap();
    let version = account.split("pub const PRIVACY_VERSION: &str = \"").nth(1).and_then(|s| s.split('"').next()).unwrap();
    let mail = MAIL_MARKERS.iter().any(|m| root().join(m).exists());
    let (ok, out) = privacy_guard(&root());
    assert_eq!(ok, !(mail && version <= "2026-09-24"), "this tree (mail {mail}, PRIVACY_VERSION {version}):\n{out}");
}

/// The default `-Root "."` is PowerShell's location, not the process's working directory: run by
/// hand after `Set-Location` into the repo from a session started elsewhere, the guard still reads
/// `account.rs` (the .NET file API resolves a relative path against the process directory).
#[test]
fn the_privacy_guard_reads_a_relative_root_from_the_powershell_location() {
    let t = tree("relative", Some("2026-10-15"), &MAIL_MARKERS);
    let script = root().join("scripts").join("ci").join("privacy-guard.ps1");
    let command = format!("Set-Location -LiteralPath '{}'; & '{}'; exit $LASTEXITCODE", t.0.display(), script.display());
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &command])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("powershell");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success() && text.contains("is past 2026-09-24"), "a relative root after Set-Location must read the tree's account.rs:\n{text}");
}
