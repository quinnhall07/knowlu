//! v0.4.0 T23 (note §8, plan §7 "Kept"): the six tests `static_assets.rs` kept unchanged when the swap
//! retired the old page and that file with it. None reads the page: they pin the bundle and updater
//! config, the release scripts, `build.rs`'s sidecar placeholder, the privacy version against the
//! published page, and Delete my data's path arithmetic. The test's working directory is `app/`, which
//! is why the paths are bare.
use std::fs;
use std::path::Path;

/// Knowlu plan 4a Task 9: the bundle facts the installer needs, pinned so a stray edit to
/// `tauri.conf.json` cannot quietly change what a friend installs. `serde_json` is already an
/// `app/` dependency, and the test's working directory is `app/`, which is why the paths are bare.
#[test]
fn the_bundle_config_is_the_one_the_installer_and_the_updater_need() {
    let conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    assert_eq!(conf["identifier"], "com.knowlu.desktop", "decision 8 - baked into every install");
    assert_eq!(conf["bundle"]["active"], true);
    assert_eq!(conf["bundle"]["targets"][0], "nsis");
    assert_eq!(conf["bundle"]["targets"].as_array().unwrap().len(), 1, "nsis is the only target");
    assert_eq!(conf["bundle"]["externalBin"][0], "binaries/knowlu-engine", "the engine travels as a sidecar (decision 7)");
    assert_eq!(conf["bundle"]["windows"]["webviewInstallMode"]["type"], "embedBootstrapper");
    // Fix round 1, m3: `currentUser` is a LOAD-BEARING premise, not a default worth leaving
    // implicit. `updates::install_staged` does not re-verify the staged bundle's signature across
    // the disk hop, and the argument for that is "anyone who can rewrite the bundle in this user's
    // %LOCALAPPDATA% can equally rewrite knowlu.exe" - which is true only while the exe installs
    // into the user's own profile. `perMachine` or `both` would put the exe somewhere only an
    // administrator can write while the bundle stayed user-writable, and the argument would be
    // void. Spelled out here and in tauri.conf.json so flipping it has to pass this line first.
    // (tauri-utils 2.9.3: `bundle.windows.nsis` is `NsisConfig`, whose `install_mode` field is
    // `#[serde(rename_all = "camelCase")]` as `installMode`, taking `NSISInstallerMode`, itself
    // camelCase - so `"currentUser"`, and `deny_unknown_fields` would reject any other spelling.)
    assert_eq!(conf["bundle"]["windows"]["nsis"]["installMode"], "currentUser",
               "R9's disk-hop reasoning depends on this - see updates::install_staged");
    assert_eq!(conf["version"], env!("CARGO_PKG_VERSION"), "one version, in one place (Knowlu spec 6): tauri.conf.json matches the workspace");
    // R-P4a-16: the bundler signs each binary BEFORE it packs, through one wrapper script.
    // tauri-bundler 2.9.4 calls try_sign() on the main binary, on every externalBin, on the
    // resources and on the installer, so all three of the things inside the NSIS package are
    // signed at the moment they are produced - signing the installer afterwards would leave the
    // two binaries within it unsigned.
    let sign = &conf["bundle"]["windows"]["signCommand"];
    assert_eq!(sign["cmd"], "powershell");
    let args: Vec<&str> = sign["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert!(args.contains(&"../scripts/sign.ps1") && args.contains(&"%1"), "{args:?}");
    assert!(Path::new("../scripts/sign.ps1").is_file(), "the wrapper the bundler will run");
    assert!(Path::new("../scripts/release.ps1").is_file(), "the one release command");

    // `createUpdaterArtifacts` and `plugins.updater` are ONE decision, not two, and the test is
    // the coupling rather than a fixed value. tauri-cli 2.11.4's `get_bundle_settings`
    // (src/interface/rust.rs) reads `plugins > updater` whenever createUpdaterArtifacts is
    // anything but `false` and fails the whole build - "failed to get updater configuration:
    // plugins > updater doesn't exist" - when it is absent. So a config with the flag on and the
    // plugin missing cannot build at all. Plan 4a Task 8's key-gated step turned both ON together;
    // this assertion held before that (both off) and holds now (both on).
    let cua = &conf["bundle"]["createUpdaterArtifacts"];
    assert!(!cua.is_null(), "the key is spelled out, so Task 8 flips a value rather than adding one");
    let wants_updater = *cua != false;
    let has_plugin = !conf["plugins"]["updater"].is_null();
    assert_eq!(wants_updater, has_plugin, "createUpdaterArtifacts and plugins.updater land together (Task 8)");
}

/// Plan 4a Task 8, key-gated step: the endpoint and the public key live in `app/tauri.conf.json`
/// and nowhere else. Shape, not value — a test that pinned the host would have to be edited to move
/// the release site, and a test that pinned the key would carry key material into the suite.
///
/// `pubkey` is why this test exists at all: `tauri-plugin-updater` 2.11.0 deserializes its config
/// with NO serde default on that field (`config.rs`), and the plugin's `setup` runs at startup - so
/// an empty or missing pubkey is not a degraded updater, it is a Knowlu that does not launch.
#[test]
fn the_updater_endpoint_lives_in_the_app_config() {
    let conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    let ep = conf["plugins"]["updater"]["endpoints"][0].as_str().unwrap_or("");
    assert!(ep.ends_with("/releases/latest.json"), "the endpoint: {ep}");
    // https, not http: the plugin REFUSES an insecure endpoint in a release build
    // (`validate_endpoints`), and a config that only works in dev is worse than one that fails now.
    assert!(ep.starts_with("https://"), "the endpoint must be https in a release build: {ep}");
    assert!(!conf["plugins"]["updater"]["pubkey"].as_str().unwrap_or("").is_empty(),
            "pubkey is required by the plugin - an empty one panics at startup");
}

/// Knowlu plan 4a Task 9: both release scripts are syntactically valid PowerShell 5.1.
/// `[scriptblock]::Create` PARSES without executing, so this never signs, builds or uploads
/// anything - it only catches a script committed with a syntax error, which would otherwise
/// surface for the first time in the middle of a release.
#[test]
fn the_release_scripts_parse_under_powershell_5_1() {
    for s in ["../scripts/sign.ps1", "../scripts/release.ps1", "../scripts/find-signtool.ps1"] {
        assert!(Path::new(s).is_file(), "{s}");
    }
    let script = "$ErrorActionPreference = 'Stop'; \
        foreach ($f in @('..\\scripts\\sign.ps1', '..\\scripts\\release.ps1', '..\\scripts\\find-signtool.ps1')) { \
          [void][scriptblock]::Create([System.IO.File]::ReadAllText((Resolve-Path $f))) \
        }";
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .expect("powershell.exe");
    assert!(
        out.status.success(),
        "a release script does not parse:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// R-P4a-26: `build.rs` must keep dropping the zero-byte sidecar placeholder, or a plain
/// `cargo build` / `cargo test` stops working on any checkout that has never run
/// `scripts\release.ps1`. `app/binaries/` is git-ignored, and `tauri-build`'s `copy_binaries`
/// treats a missing `externalBin` as a hard error - "resource path ... doesn't exist" - raised
/// before a single test compiles. The guard is invisible when it works, which is exactly why it
/// needs a test: nothing else would notice it being deleted until someone cloned the repo.
#[test]
fn build_rs_still_stages_the_zero_byte_sidecar_placeholder() {
    let build_rs = fs::read_to_string("build.rs").unwrap();
    assert!(build_rs.contains("TARGET"), "the placeholder is named for cargo's TARGET triple");
    assert!(build_rs.contains("binaries/knowlu-engine-"), "the placeholder path");
    assert!(
        build_rs.contains("cargo:rerun-if-changed=binaries/"),
        "staging a real engine later must re-run the build script, not reuse the placeholder run"
    );
    // That path is the THIRD spelling of the sidecar stem, after release.ps1's $sidecar and
    // bundle.externalBin. release.ps1 cross-checks its own against the config; this pins the last
    // one, so plan 2 Task 11's rename (quinn-ops -> knowlu-engine) cannot leave one behind.
    let conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    let declared = conf["bundle"]["externalBin"][0].as_str().unwrap();
    let stem = declared.rsplit('/').next().unwrap();
    assert!(
        build_rs.contains(&format!("binaries/{stem}-")),
        "build.rs must name {stem}, the stem bundle.externalBin declares"
    );
}

/// The version constant and the page's own date are one fact in two files (`account.rs`'s rule).
#[test]
fn the_privacy_version_constant_is_the_published_pages_date() {
    let rust = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    let version = rust
        .split("pub const PRIVACY_VERSION: &str = \"").nth(1)
        .and_then(|s| s.split('"').next())
        .expect("account.rs must declare `pub const PRIVACY_VERSION: &str = \"…\";`");
    let page = std::fs::read_to_string("../site/privacy.html").expect("site/privacy.html");
    assert!(page.contains(&format!("Effective {version}.")), "the page's Effective date is {version}");
    assert!(page.contains(&format!("This page is version <strong>{version}</strong>")), "…and so is its version line");
    // Spec §10: the policy must not describe a password the product no longer has.
    assert!(!page.contains("password hash"), "the password-hash clause is gone");
    assert!(!page.contains("your password, which Supabase holds"), "…and the account-clause's own password mention is gone");
    assert!(page.contains("There is no password on a Knowlu account at all"), "…and the page says so");
    assert!(page.contains("Signing in with Google tells us three things"), "Google sign-in is disclosed");
    // F3: the "Your account" collection entry must not be narrower than the page's own Google
    // paragraph — both must name the same three things Google hands over at sign-in.
    let account_entry = page.split("<dt>Your account</dt>").nth(1).and_then(|s| s.split("<dt>").next())
        .expect("the Your account entry");
    assert!(account_entry.contains("the Google account id that identifies it"), "{account_entry}");
    assert!(account_entry.contains("your name") && account_entry.contains("a link to your profile picture"),
        "the Your account entry must agree with the Google paragraph: {account_entry}");
}

/// I11: *Delete my data* takes **this profile's** snapshots out of the shared backups root, never the
/// root. Two profiles on one machine share `%USERPROFILE%\Knowlu\Backups`, and the other one's only
/// other copy of their work is in there.
#[test]
fn deleting_my_data_leaves_another_profiles_snapshots_alone() {
    // The path arithmetic, driven directly: the command itself needs a `ConsoleState`, an `AppHandle`
    // and a live account, and none of the three is what this is about. The real thing —
    // `account::delete_local_data` over a scratch vault, its backups subtree, its registry row and
    // its credentials — is `app/tests/account.rs`'s
    // `deleting_my_data_removes_this_profiles_things_and_nothing_else`.
    let root = std::env::temp_dir().join(format!("knowlu-backups-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mine = root.join("profile_1111111111").join("vault");
    let theirs = root.join("profile_2222222222").join("vault");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(mine.join("a.md"), "x").unwrap();
    std::fs::write(theirs.join("b.md"), "y").unwrap();
    // What `delete_my_data` does: the root JOINED with this profile's id, and nothing above it.
    std::fs::remove_dir_all(root.join("profile_1111111111")).unwrap();
    assert!(!root.join("profile_1111111111").exists());
    assert!(theirs.join("b.md").is_file(), "another profile's snapshots were destroyed");
    let _ = std::fs::remove_dir_all(&root);
}
