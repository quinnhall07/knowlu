//! Guards on the dependency budget.
//!
//! The engine's Python rule — stdlib + PyYAML + tzdata — exists so the runner never drags a heavy
//! tree into its environment. The Rust equivalent is a short, justified list (rewrite spec section
//! 5), and these tests pin the two mistakes this project has already explicitly decided against.
//!
//! Cheap, and they fail at the moment someone adds the dependency rather than at the moment
//! somebody notices the binary doubled.

/// The engine crate's own manifest.
const MANIFEST: &str = include_str!("../Cargo.toml");
/// The workspace root's manifest: the only place Cargo reads `[profile.*]` from, and the other
/// place a dependency could reach this crate (`[workspace.dependencies]`).
const WORKSPACE: &str = include_str!("../../Cargo.toml");
/// Both, named, so a failure says which one grew the dependency.
const MANIFESTS: [(&str, &str); 2] = [("engine/Cargo.toml", MANIFEST), ("Cargo.toml", WORKSPACE)];

/// `tauri` belongs to the SHELL, not the engine.
///
/// `CLAUDE.md` already enforces this boundary between `engine/` and the window layer, and it exists
/// so the engine stays linkable from a scheduler, a test harness, and eventually a mobile target —
/// none of which want a GUI toolkit. Rewrite spec section 5.
#[test]
fn engine_does_not_depend_on_a_gui_toolkit() {
    for (name, manifest) in MANIFESTS {
        for forbidden in ["tauri", "webview", "wry"] {
            assert!(
                !manifest.contains(forbidden),
                "`{forbidden}` must not be a dependency of the engine crate ({name}). \
                 GUI dependencies belong to the shell. See the rewrite spec section 5."
            );
        }
    }
}

/// TLS is `rustls`/`ring`, never OpenSSL.
///
/// This is what lets the GNU host toolchain build without a Visual Studio install, and it keeps the
/// cross-compilation story simple for the eventual mobile targets. `ureq` selects rustls by default;
/// this test catches someone enabling `native-tls` to work around a certificate problem.
#[test]
fn tls_is_rustls_never_openssl() {
    for (name, manifest) in MANIFESTS {
        for forbidden in ["openssl", "native-tls"] {
            assert!(
                !manifest.contains(forbidden),
                "`{forbidden}` must not appear in {name}. TLS is rustls/ring - see the rewrite spec \
                 section 5 and CLAUDE.md. Enabling it also reintroduces a C toolchain requirement."
            );
        }
    }
}

/// The sync envelope's two crates, and why neither is a new thing to audit.
///
/// `ring` is what `ureq`'s rustls already links in this binary for TLS — using it for AES-256-GCM
/// and SHA-256 keeps one implementation of each primitive rather than two. `base64` is already
/// resolved in this workspace's lockfile through the same graph. Naming them in the manifest adds a
/// direct edge and no new crate; this test is here so the next person to reach for `aes-gcm`,
/// `openssl` or a second base64 has to argue with a sentence rather than with a diff.
#[test]
fn the_crypto_dependencies_are_the_two_already_in_the_graph() {
    assert!(MANIFEST.contains("ring = \"0.17\""), "engine/Cargo.toml must name ring for the sync envelope");
    assert!(MANIFEST.contains("base64 = \"0.22\""), "engine/Cargo.toml must name base64 for the sync envelope");
    for (name, manifest) in MANIFESTS {
        for forbidden in ["aes-gcm", "chacha20", "rust-crypto", "sodiumoxide"] {
            assert!(
                !manifest.contains(forbidden),
                "`{forbidden}` must not appear in {name}: the sync envelope uses `ring`, which this \
                 binary already links for TLS, and two implementations of one primitive is one too many."
            );
        }
    }
}

/// The release profile is a shipping requirement, not an optimisation.
///
/// The product plan section 5.2 targets a sub-50 MB app with 1-5 MB budgeted for the compiled
/// engine. These five settings routinely cut a naive release build 30-50%, and they are fixed from
/// wave 0 so the number never comes as a surprise at cutover. They live in the WORKSPACE ROOT
/// manifest — Cargo ignores `[profile.*]` in a member crate — so that is where this test looks.
#[test]
fn release_profile_is_size_tuned() {
    for setting in [
        "opt-level = \"z\"",
        "lto = true",
        "codegen-units = 1",
        "panic = \"abort\"",
        "strip = true",
    ] {
        assert!(
            WORKSPACE.contains(setting),
            "the workspace root's release profile is missing `{setting}` - compiled engine size is a product budget \
             line (product plan section 5.2), not a preference."
        );
    }
}

/// Judgment is a service, and the engine reaches it with `ureq` and nothing else.
///
/// Cloud design §8: "`dependency_boundary.rs` now also forbids `tauri` and any cloud SDK in the
/// engine; the engine talks HTTP with `ureq` and nothing else." A provider SDK in this crate would
/// bring a runtime, a TLS stack of its own and — the actual cost — a second place where a request
/// body is assembled, out of reach of the one test that proves what a request may carry.
#[test]
fn no_cloud_sdk_enters_the_engine() {
    for (name, manifest) in MANIFESTS {
        for forbidden in [
            "anthropic", "openai", "supabase", "postgrest", "aws-sdk", "azure_", "google-cloud",
            "reqwest", "hyper", "isahc", "curl",
        ] {
            assert!(
                !manifest.contains(forbidden),
                "`{forbidden}` must not be a dependency of the engine crate ({name}). The engine \
                 talks to the judgment service over `ureq` and nothing else (cloud design §8), so \
                 every request body is assembled in `cloudmodel.rs` where one test can see it."
            );
        }
    }
}
