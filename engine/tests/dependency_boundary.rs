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

/// The one hash the account vault needs, and the crates that are not coming back.
///
/// **What left, and why.** Until Quinn's amendment of 2026-09-17 this engine sealed every journal
/// record and every note with AES-256-GCM under a key that lived only in Credential Manager, and
/// named `ring` and `base64` for it. Ruling 2 reversed that: the account holds the student's tasks
/// and notes readable by the service, so there is no envelope, no IV, no key and no recovery code.
/// **`base64` leaves with them.**
///
/// **What stays, and why it is `ring` and not `sha2`.** A content hash is still needed — it is what
/// makes a retried push idempotent and what lets the server check a row against its own body — and
/// `ring::digest::SHA256` costs **nothing**: `ring` is already compiled into this binary through
/// `ureq` to `rustls`, so keeping the direct edge adds no crate, no version and nothing to audit.
/// `sha2` would: `knowlu-engine`'s own dependency graph has none (the workspace's `sha2 0.10.9` is
/// the APP's own direct edge, `app/Cargo.toml:69`, for `inference.rs`'s runtime digest check — and
/// this scan reads only `engine/Cargo.toml` and the workspace root), so adding it pulls `digest 0.10`,
/// `cpufeatures 0.2`, `block-buffer`, `crypto-common`, `generic-array` and `typenum` in beside the
/// engine's existing `sha1 0.11`, which resolves `digest 0.11` and `cpufeatures 0.3` — two majors of
/// two crates, into the binary CI holds under 6 MiB. `sha1` itself stays and is a different thing
/// again: it is `ids::derived_id`, a note-identity contract with every vault that exists.
#[test]
fn the_content_hash_is_rings_and_the_envelope_is_not_coming_back() {
    assert!(MANIFEST.contains("ring = \"0.17\""), "engine/Cargo.toml must keep ring for the sync content hash");
    for (name, manifest) in MANIFESTS {
        for gone in ["base64 = ", "sha2", "aes-gcm", "chacha20", "rust-crypto", "sodiumoxide", "hkdf"] {
            assert!(
                !manifest.contains(gone),
                "`{gone}` must not appear in {name}: the account's copy is plain text (cloud design, \
                 amendment 2026-09-17, ruling 2). There is nothing to encrypt, and the one hash this \
                 crate needs is `ring`'s, which it already links for TLS."
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

/// The module itself, scanned. A manifest check alone would pass on the day the code still held a
/// hand-rolled key schedule; this is the sentence that would have to be argued with.
#[test]
fn the_sync_module_holds_no_key_and_no_envelope() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("sync.rs"),
    )
    .expect("engine/src/sync.rs");
    for gone in ["SyncKey", "recovery", "Recovery", "seal(", "fn open(", "IndexKey", "HKDF", "hkdf", "aead", "CROCKFORD"] {
        assert!(!src.contains(gone), "engine/src/sync.rs still names {gone}");
    }
}
