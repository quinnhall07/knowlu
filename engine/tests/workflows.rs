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

#[test]
fn ci_enforces_zero_warnings_and_the_eol_contract() {
    let c = workflow("ci.yml");
    assert!(c.contains("RUSTFLAGS") && c.contains("-D warnings"), "ci.yml must set RUSTFLAGS=-D warnings");
    assert!(c.contains("scripts/ci/eol-check.ps1"), "ci.yml must run the eol check");
    assert!(c.contains("cargo test --workspace"), "ci.yml must test the whole workspace");
}

/// A-6 (C2 final review): the eval gate exists, never runs on a plain push (a fork PR's `push` to
/// its own branch must not spend the staging service-role key), and names both secrets it needs —
/// so a secret renamed here but not in the repository's settings fails this test rather than only
/// on the first PR that touches a prompt.
#[test]
fn the_eval_gate_job_exists_is_pull_request_only_and_names_both_its_secrets() {
    let c = workflow("ci.yml");
    let start = c.find("\n  eval-gate:").expect("ci.yml must define an eval-gate job");
    let job = &c[start..];
    assert!(job.contains("if: github.event_name == 'pull_request'"), "eval-gate must run only on pull_request: {job}");
    assert!(job.contains("secrets.ANTHROPIC_API_KEY"), "eval-gate must reference secrets.ANTHROPIC_API_KEY");
    assert!(
        job.contains("secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY"),
        "eval-gate must reference secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY"
    );
}
