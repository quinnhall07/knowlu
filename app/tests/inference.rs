use knowlu::inference::{
    check_runtime_supported, extract_runtime_zip, install_from_file, install_model_from_file,
    install_runtime_from_zip, judgments_dir, model_file, models_dir, remove_model, runtime_dir,
    runtime_exe, runtime_release_for, sha256_of, status, validate_asset, Asset, Half, Manifest,
    SUPPORTED_RUNTIMES,
};
use std::io::Write;
use std::path::{Path, PathBuf};

fn root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-inference-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The empty digest of the empty string, and one known vector — so a wrong hash function is caught
/// here rather than by an install that mysteriously always refuses.
#[test]
fn sha256_of_matches_the_known_vectors() {
    let d = root("sha");
    let f = d.join("x");
    std::fs::write(&f, b"").unwrap();
    assert_eq!(sha256_of(&f).unwrap(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    std::fs::write(&f, b"abc").unwrap();
    assert_eq!(sha256_of(&f).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert!(sha256_of(&d.join("nope")).is_err(), "a missing file is an error, never a hash of nothing");
    let _ = std::fs::remove_dir_all(&d);
}

/// The layout: `runtime\` and `models\` under the install-wide root, `judgments\` under the
/// PROFILE's folder. Two vaults share a model and do not share a judgment log (spec §5.4).
#[test]
fn the_layout_puts_artefacts_install_wide_and_the_log_per_profile() {
    let r = root("layout");
    assert_eq!(runtime_dir(&r), r.join("runtime"));
    assert_eq!(models_dir(&r), r.join("models"));
    let profile = r.join("profiles").join("profile_abc");
    assert_eq!(judgments_dir(&profile), profile.join("judgments"));
    assert!(!judgments_dir(&profile).starts_with(models_dir(&r)));
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn nothing_is_installed_on_a_fresh_root() {
    let r = root("fresh");
    let s = status(&r);
    assert!(s.runtime.is_none() && s.model.is_none());
    assert_eq!(s.model_bytes, 0);
    assert!(runtime_exe(&r).is_none() && model_file(&r).is_none());
    let _ = std::fs::remove_dir_all(&r);
}

/// A model is verified against the digest it was offered under, and a mismatch leaves NOTHING
/// behind — not the bad bytes, not a partial, not a half-installed directory.
#[test]
fn a_model_is_verified_before_it_is_installed_and_a_bad_hash_leaves_nothing() {
    let r = root("model");
    let src = r.join("downloaded.gguf");
    std::fs::write(&src, b"abc").unwrap();
    let good = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    let err = install_model_from_file(&r, &src, Some("0000000000000000000000000000000000000000000000000000000000000000")).unwrap_err();
    assert!(err.contains("does not match"), "{err}");
    assert!(model_file(&r).is_none(), "a refused model must not be installed");
    assert!(!models_dir(&r).join("downloaded.gguf").exists());
    assert_eq!(std::fs::read_dir(models_dir(&r)).map(|d| d.flatten().count()).unwrap_or(0), 0, "no partial left behind");

    let out = install_model_from_file(&r, &src, Some(good)).unwrap();
    assert_eq!(out, models_dir(&r).join("downloaded.gguf"));
    assert_eq!(model_file(&r), Some(out.clone()));
    assert_eq!(status(&r).model_bytes, 3);
    assert!(src.is_file(), "the source the user pointed at is never moved or deleted");

    remove_model(&r).unwrap();
    assert!(model_file(&r).is_none(), "removing the model frees the gigabytes and nothing else");
    let _ = std::fs::remove_dir_all(&r);
}

/// A file that is not a `.gguf` is refused by name: pointing at the wrong file in a picker is the
/// commonest mistake, and a two-gigabyte copy is an expensive way to find out.
#[test]
fn only_a_gguf_is_accepted_as_a_model() {
    let r = root("ext");
    let src = r.join("notes.txt");
    std::fs::write(&src, b"abc").unwrap();
    let err = install_model_from_file(&r, &src, None).unwrap_err();
    assert!(err.contains(".gguf"), "{err}");
    let _ = std::fs::remove_dir_all(&r);
}

/// S6: the binary's name comes from the engine's constant, never a literal — so Task 1's Outcome B,
/// which renames it to `llama-cli.exe`, costs this file nothing.
fn server_exe() -> &'static str { knowlu_engine::runtime::RUNTIME_EXE }

fn make_zip(path: &Path, entries: &[(String, &[u8])]) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let o: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    for (name, body) in entries {
        z.start_file(name.as_str(), o).unwrap();
        z.write_all(body).unwrap();
    }
    z.finish().unwrap();
}

/// The runtime arrives as a zip. Every entry is extracted, the server binary is found at any depth,
/// and a zip that does not contain one is refused with everything cleaned up.
///
/// Drives `extract_runtime_zip` — the half without the digest check — because a zip this test builds
/// can never be in `SUPPORTED_RUNTIMES`, and pinning a fake digest into the shipped table to make a
/// test pass would defeat the table's whole purpose. The check itself is asserted below.
#[test]
fn a_runtime_zip_is_extracted_and_the_server_is_found_at_any_depth() {
    let r = root("zip");
    let src = r.join("rt.zip");
    make_zip(&src, &[(format!("build/bin/{}", server_exe()), b"MZfake" as &[u8]), ("build/bin/ggml.dll".to_string(), b"dll")]);
    let out = extract_runtime_zip(&r, &src).unwrap();
    assert!(out.ends_with(server_exe()), "{out:?}");
    assert_eq!(runtime_exe(&r), Some(out));
    assert!(runtime_dir(&r).join("build").join("bin").join("ggml.dll").is_file(), "its DLLs come with it");

    let r2 = root("zipbad");
    let bad = r2.join("rt.zip");
    make_zip(&bad, &[("readme.txt".to_string(), b"nothing useful" as &[u8])]);
    let err = extract_runtime_zip(&r2, &bad).unwrap_err();
    assert!(err.contains(server_exe()), "{err}");
    assert!(runtime_exe(&r2).is_none());
    assert!(!runtime_dir(&r2).exists(), "a refused runtime leaves no half-extracted directory");
    let _ = std::fs::remove_dir_all(&r);
    let _ = std::fs::remove_dir_all(&r2);
}

/// **R-P3a-2, and this is the test that proves the rule is on the production path.** The zip above
/// extracts cleanly through `extract_runtime_zip`; through `install_runtime_from_zip` — the only
/// function `commands.rs` calls — the same zip is refused, because its digest is not in the table,
/// and the refusal prints the digest so a user can report it.
#[test]
fn a_runtime_whose_digest_is_not_in_the_table_is_refused_by_the_production_path() {
    let r = root("unsupported");
    let src = r.join("rt.zip");
    make_zip(&src, &[(server_exe().to_string(), b"MZfake" as &[u8])]);
    let computed = sha256_of(&src).unwrap();
    let err = install_runtime_from_zip(&r, &src).unwrap_err();
    assert!(err.contains(&computed), "the refusal must print the digest to report: {err}");
    assert!(err.to_lowercase().contains("not a runtime release knowlu knows"), "{err}");
    assert!(err.contains("report this one"), "fix round 1, m8: the actionable half of the message must survive too: {err}");
    assert!(runtime_exe(&r).is_none(), "nothing was installed");
    assert!(!runtime_dir(&r).exists(), "and nothing was extracted on the way to finding out");
    assert!(runtime_release_for(&computed).is_none());
    let _ = std::fs::remove_dir_all(&r);
}

/// m2 (final fix wave): `check_runtime_supported` was public, uncalled and untested — part of the
/// documented security surface (its own doc: "a convenience for a caller that wants to ask whether
/// a file is supported without installing it"), kept public rather than deleted, so it needs a test
/// proving what its doc promises: the identical verdict and the identical message
/// `install_runtime_from_zip` reaches for the same file, via the shared `unsupported_runtime_msg`.
#[test]
fn check_runtime_supported_answers_exactly_like_the_production_path() {
    let r = root("check-supported");
    let src = r.join("rt.zip");
    make_zip(&src, &[(server_exe().to_string(), b"MZfake" as &[u8])]);
    let computed = sha256_of(&src).unwrap();
    let checked_err = check_runtime_supported(&src).unwrap_err();
    assert!(checked_err.contains(&computed), "{checked_err}");
    let install_err = install_runtime_from_zip(&r, &src).unwrap_err();
    assert_eq!(checked_err, install_err, "the two paths must say exactly the same thing for the same digest");
    assert!(runtime_exe(&r).is_none(), "checking must never install");
    let _ = std::fs::remove_dir_all(&r);
}

/// Whatever the table holds, it holds it in the shape the lookup expects: a 64-character lowercase
/// hex digest, a non-empty tag and a non-empty asset name. Vacuously true on an empty table, which
/// is Outcome C's state — Task 12 step 8 is what refuses to close on an empty one.
#[test]
fn every_pinned_runtime_digest_is_well_formed() {
    for r in SUPPORTED_RUNTIMES {
        assert_eq!(r.sha256.len(), 64, "{}: {}", r.asset, r.sha256);
        assert!(r.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{}", r.sha256);
        assert!(!r.tag.is_empty() && !r.asset.is_empty(), "{r:?}");
        assert_eq!(runtime_release_for(&r.sha256.to_ascii_uppercase()).map(|f| f.asset), Some(r.asset),
            "the lookup is case-insensitive: a digest pasted from a checksum file is usually upper case");
    }
}

/// Zip slip: an archive entry naming `..\` or an absolute path must never write outside the runtime
/// directory. This is the one place this app extracts an archive it did not create.
#[test]
fn an_entry_that_escapes_the_runtime_directory_is_refused() {
    let r = root("slip");
    let src = r.join("evil.zip");
    make_zip(&src, &[("../escaped.exe".to_string(), b"MZ" as &[u8]), (server_exe().to_string(), b"MZ")]);
    let err = extract_runtime_zip(&r, &src).unwrap_err();
    assert!(err.to_lowercase().contains("outside"), "{err}");
    assert!(!r.join("escaped.exe").exists(), "nothing was written outside the runtime directory");
    let _ = std::fs::remove_dir_all(&r);
}

/// The manifest is data, and a malformed one is an error rather than a panic — it comes off the
/// network and this app never trusts what it is handed.
#[test]
fn a_manifest_parses_or_says_why_not() {
    let m: Manifest = serde_json::from_str(
        r#"{"runtime":{"name":"llama-b1.zip","url":"https://example.invalid/a.zip","sha256":"aa","bytes":1},
            "model":{"name":"m.gguf","url":"https://example.invalid/m.gguf","sha256":"bb","bytes":2}}"#,
    ).unwrap();
    assert_eq!(m.runtime.name, "llama-b1.zip");
    assert_eq!(m.model.sha256, "bb");
    assert!(serde_json::from_str::<Manifest>(r#"{"runtime":{}}"#).is_err());
}

/// The engine and the app must look for the same file name, or an installed runtime is invisible.
///
/// Asserted through the constant on both sides rather than against a literal (S6): under Task 1's
/// Outcome B that constant is `llama-cli.exe`, and this test must keep meaning the same thing.
#[test]
fn the_app_and_the_engine_agree_on_the_runtime_binarys_name() {
    let r = root("name");
    std::fs::create_dir_all(runtime_dir(&r)).unwrap();
    std::fs::write(runtime_dir(&r).join(server_exe()), b"MZ").unwrap();
    assert!(runtime_exe(&r).is_some(), "the app must find what the engine will look for");
    assert!(server_exe().ends_with(".exe"), "it is an executable and the resolver matches on the name");
    let _ = std::fs::remove_dir_all(&r);
}

/// `Half` is what both commands parse their `kind` argument into, so a typo is refused by name in
/// one place rather than by an `else` arm in each of them (M7).
#[test]
fn half_parses_exactly_two_words() {
    assert_eq!(Half::parse("runtime"), Some(Half::Runtime));
    assert_eq!(Half::parse("model"), Some(Half::Model));
    assert_eq!(Half::parse("Runtime"), None, "exact, so a typo is a refusal and not a surprise");
    assert_eq!(Half::parse(""), None);
}

/// Fix round 1, M1 (Major): `install_from_file` and `install_from_manifest` are the composition —
/// the brief's own words are "the composition lives here, not in `commands.rs`" — and until this
/// test existed, neither production entry point was driven by anything: swapping
/// `Half::Runtime`'s arm for the unchecked `extract_runtime_zip` left every other test green. This
/// drives the real entry point and proves the check runs before anything is extracted.
#[test]
fn install_from_file_puts_the_runtime_through_the_digest_check() {
    let r = root("fromfile");
    let src = r.join("rt.zip");
    make_zip(&src, &[(server_exe().to_string(), b"MZ" as &[u8])]);
    let err = install_from_file(&r, Half::Runtime, &src).unwrap_err();
    assert!(err.contains("not a runtime release Knowlu knows"), "{err}");
    assert!(!runtime_dir(&r).exists(), "the check ran before anything was extracted");
    let _ = std::fs::remove_dir_all(&r);
}

/// Fix round 1, M3 (Major): `extract_runtime_zip` is the digest-free half and is `pub` only so
/// tests can drive it — it must never be reachable from a command, where "the check already ran,
/// call the extractor" is a plausible and silent way to route around both the signature gate and
/// the zip-bomb protection that check-then-extract ordering provides. `app/tests/static_assets.rs`
/// is this crate's existing precedent for a test that scans source text; test binaries run with the
/// crate root (`app\`) as the working directory, so these paths need no `../`.
///
/// m3 (final fix wave): named three files rather than a rule, so a fourth command file — `state.rs`
/// already exists and did not need this guard when it was written — would silently escape it. Scan
/// every `.rs` in `src/` except `inference.rs` itself, which is where the digest-free half is
/// defined and doc-references its own name.
///
/// Residual 3 (final fix wave re-review): a relative `"src"` is exactly the fragility m7 removed
/// from `tests/starvation.rs` three items earlier — it resolves against whatever `cargo test`'s
/// working directory happens to be, so on the wrong one it can silently scan some OTHER `src/`
/// (this workspace's root `src/`, which exists and has plenty of `.rs` files but none of app's
/// commands) and the guard would pass having checked nothing that matters. `CARGO_MANIFEST_DIR`
/// is fixed at compile time to this crate's root, so the directory is unambiguous regardless of
/// cwd — and a guard that cannot fail is worse than no guard, so a missing directory panics with
/// a named path rather than `read_dir` silently iterating zero entries.
#[test]
fn no_command_reaches_the_unchecked_extraction_half() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(src_dir.is_dir(), "{} is not a directory — the guard cannot see this crate's source", src_dir.display());
    let mut checked = 0;
    for e in std::fs::read_dir(&src_dir).unwrap_or_else(|e| panic!("{}: {e}", src_dir.display())).flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "rs") != Some(true) || p.file_name().map(|n| n == "inference.rs") == Some(true) {
            continue;
        }
        let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert!(!src.contains("extract_runtime_zip"), "{} calls the digest-free extraction half", p.display());
        checked += 1;
    }
    assert!(checked >= 3, "the scan found suspiciously few source files: {checked}");
}

/// Fix round 1, m6 (Minor): the well-formedness test above checks shape only (64 hex chars, non-empty
/// tag/asset) — a single mistyped hex digit would make the shipped app refuse the real `b10840`
/// release forever and every other test would still pass. Pin the content itself, guarded on the
/// table being non-empty so Outcome C's empty table (Task 12 step 8) stays a legitimate state this
/// test does not fail on.
#[test]
fn the_seeded_runtime_matches_the_spike_report() {
    if SUPPORTED_RUNTIMES.is_empty() {
        return;
    }
    assert!(
        SUPPORTED_RUNTIMES.iter().any(|r| r.tag == "b10840"
            && r.asset == "llama-b10840-bin-win-cpu-x64.zip"
            && r.sha256 == "7063dfc6b874e7eee0ddf601bdf8e70e6f4a3d708926641ffad046ec51e8e30b"),
        "{SUPPORTED_RUNTIMES:?}"
    );
}

/// Fix round 1, m7 (Minor): `remove_model` must also clean up an abandoned `.gguf.part<pid>` left
/// by a copy that died half way — otherwise "free the gigabytes" does not free all of them.
#[test]
fn remove_model_also_clears_an_abandoned_partial() {
    let r = root("partial");
    std::fs::create_dir_all(models_dir(&r)).unwrap();
    std::fs::write(models_dir(&r).join("real.gguf"), b"abc").unwrap();
    std::fs::write(models_dir(&r).join("stuck.gguf.part4242"), b"half").unwrap();
    remove_model(&r).unwrap();
    assert_eq!(std::fs::read_dir(models_dir(&r)).map(|d| d.flatten().count()).unwrap_or(0), 0, "the partial must go too");
    let _ = std::fs::remove_dir_all(&r);
}

/// Fix round 1, C1 (Critical): a manifest field must never be trusted as a filesystem path.
/// `asset.name` is a `String` off the network with no invariant, and `Path::join` discards the base
/// entirely for an absolute path and keeps `..` for the OS to resolve — so a manifest naming a
/// Startup-folder path or an absolute path must be refused, not silently joined onto `downloads\`.
/// Driven directly against `validate_asset`, and against a `Manifest`/`Asset` built in the test, so
/// the property is proven with no network involved.
#[test]
fn a_manifest_asset_that_is_not_a_plain_file_name_is_refused() {
    let base = Asset {
        name: String::new(),
        url: "https://example.invalid/a.zip".to_string(),
        sha256: "aa".to_string(),
        bytes: 1,
    };

    // Fix round 2: the eleven hostile forms the scoped re-review checked `validate_asset` against.
    // Three of them (trailing dot, trailing space, embedded NUL) got through the round-1 check —
    // none of them escapes `downloads\`, so C1's actual threat stayed closed, but `plain_asset_name`
    // now refuses all eleven by construction (an allowlist) rather than by a growing refusal list.
    let hostile: &[(&str, &str)] = &[
        ("C:\\Users\\quinn\\x.zip", "absolute path"),
        ("\\\\server\\share\\x.zip", "UNC path"),
        ("..", "bare .."),
        ("..\\evil.zip", "embedded ..\\"),
        ("../x", "../x"),
        ("C:x", "drive-relative"),
        ("", "empty"),
        ("foo\\", "directory-only"),
        ("foo.", "trailing dot"),
        ("foo ", "trailing space"),
        ("evil\0.zip", "embedded NUL"),
    ];
    assert_eq!(hostile.len(), 11, "all eleven forms the scoped re-review checked, or this test has drifted from it");
    for (name, why) in hostile {
        let asset = Asset { name: name.to_string(), ..base.clone() };
        assert!(validate_asset(&asset).is_err(), "{why} ({name:?}) must be refused");
    }

    // Not one of the eleven, but belongs in the same predicate (fix round 2): a Windows reserved
    // device name does not create a file at all — it opens the device, and every byte written to it
    // vanishes silently, which would present as an inexplicably corrupt download. Checked with and
    // without an extension, and case-insensitively.
    for reserved in ["NUL", "NUL.zip", "com1", "COM1.gguf", "lpt9"] {
        let asset = Asset { name: reserved.to_string(), ..base.clone() };
        assert!(validate_asset(&asset).is_err(), "{reserved} is a reserved device name and must be refused");
    }

    let plain = Asset { name: "llama-b10840-bin-win-cpu-x64.zip".to_string(), ..base };
    assert_eq!(
        validate_asset(&plain).unwrap(),
        std::ffi::OsStr::new("llama-b10840-bin-win-cpu-x64.zip"),
        "a real asset name must be unaffected"
    );
}

/// Fix round 1, m3 (Minor): a manifest served over HTTPS must not be able to name a plaintext or
/// local asset for the model half, whose only verification is the digest that travels beside this
/// URL.
#[test]
fn a_manifest_asset_url_that_is_not_https_is_refused() {
    let asset = Asset {
        name: "m.gguf".to_string(),
        url: "http://example.invalid/m.gguf".to_string(),
        sha256: "bb".to_string(),
        bytes: 1,
    };
    let err = validate_asset(&asset).unwrap_err();
    assert!(err.contains("https"), "{err}");
}
