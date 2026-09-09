//! Where the local inference runtime and the model live, and how a verified one gets there
//! (Knowlu spec §5.3, decision 12).
//!
//! **Three rules, and every one of them is a function below.**
//!
//! 1. **Never automatic** (§5.3: "the model download is a settings action and an onboarding offer,
//!    never automatic"). Nothing in this module is called by a tick, a housekeeping thread or a
//!    slot. Every entry point is reached from a command the user clicked.
//! 2. **There is no path *through Knowlu* that installs a runtime it has not verified** (ruling
//!    R-P3a-2; fix round 1, m9 — the unqualified claim was one word too strong: a file dropped into
//!    `runtime\` by hand, `KNOWLU_RUNTIME`, or a sibling exe beside the install (all `src/runtime.rs`,
//!    outside this module) are unverified too, and deliberately so — in every one of those cases the
//!    person who placed the bytes is the person running the app, so no privilege boundary is
//!    crossed. What this module guarantees is narrower and exactly true: nothing *this app itself*
//!    fetches or installs reaches `runtime\` unchecked). `RUNTIME_EXE`
//!    (`knowlu_engine::runtime::RUNTIME_EXE`, `llama-cli.exe`) is an executable this app will run with
//!    the user's own privileges, and the updater's minisign discipline does not reach it — upstream's
//!    build is not signed by us. So the digests live **in the app**: [`SUPPORTED_RUNTIMES`] is a
//!    compiled-in table of release tag, asset name and SHA-256, and **both** install paths check
//!    against it. A manifest is a convenience for finding the bytes, never the root of trust — which
//!    matters because the path that works today is *Install from a file…*, where there is no
//!    manifest at all and where a design that took the digest from the caller would be taking it
//!    from nobody.
//!
//!    An unrecognised digest is **refused**, and the refusal prints the computed digest so a user
//!    can report it and a supported release can be added deliberately.
//!
//!    **The model is not in the table, and the asymmetry is deliberate.** A `.gguf` is *data*,
//!    parsed by a runtime the table has already vouched for and never executed, and §5.3 allows any
//!    1–4 B Q4 model — pinning model digests would make the feature useless to anyone who wants a
//!    different one. A model is checked against the manifest's digest when it came from a manifest,
//!    and accepted on the user's own authority when they pointed at it.
//! 3. **Temp-then-rename**, like `updates::stage_bytes`: a download that dies half way leaves a
//!    `.part<pid>` beside the target and never a partial at the name the judge resolves.
//!
//! **Where things live.** `runtime\` and `models\` are under the **install-wide** app-data root,
//! beside `updates\` — a llama.cpp build and a `.gguf` are immutable artefacts identical for every
//! profile and a friend with two vaults must not download two gigabytes twice. The judgment LOG is
//! per profile (`judgments_dir`), because two vaults are two different sets of judgments.
//!
//! **A deviation from spec §5.3, recorded** (ruling R-P3a-1). The amended §5.3 says the runtime
//! ships as a second Tauri sidecar (`bundle.externalBin`). The process boundary is kept and the
//! bundling is not, for three reasons and none of them about installer size: the runtime is
//! **optional** (most installs will never enable local judgment), it **versions independently** of
//! this app, and it is **consistent with the models**, which §5.3 already downloads after install.
//! The repo's own figure for its size is the product plan's +20–50 MB
//! (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`).
//! `app/tauri.conf.json` is untouched, and `knowlu_engine::runtime::resolve`'s sibling branch leaves the
//! door open to bundling later without a code change.
//!
//! **Values seeded from Task 1's spike, Outcome B**
//! (`docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`): the runtime obtained was
//! release `b10840`, asset `llama-b10840-bin-win-cpu-x64.zip`, 18,417,566 bytes, SHA-256
//! `7063dfc6b874e7eee0ddf601bdf8e70e6f4a3d708926641ffad046ec51e8e30b`. That report ran to
//! completion — this is not Outcome C — so [`SUPPORTED_RUNTIMES`] carries exactly that one entry,
//! never an invented one.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The install-wide runtime directory. The zip is extracted here whole, structure and all.
pub fn runtime_dir(root: &Path) -> PathBuf {
    root.join("runtime")
}

/// The install-wide model directory. One `.gguf` at a time in practice; `model_file` picks the
/// newest if a user has left two.
pub fn models_dir(root: &Path) -> PathBuf {
    root.join("models")
}

/// **Per profile**, not per install (spec §5.4): `profiles\<id>\judgments\`. Takes the profile's own
/// `data_dir`, which `ConsoleState` already carries, so nothing here has to re-derive an id.
pub fn judgments_dir(profile_data_dir: &Path) -> PathBuf {
    profile_data_dir.join("judgments")
}

/// Find `RUNTIME_EXE` (`llama-cli.exe`) anywhere under `runtime\`, to a bounded depth.
///
/// Bounded because upstream has shipped the Windows build both flat and under `build\bin\`, and a
/// recursive walk of a directory the user can put anything in is not something to leave unbounded.
/// The name is `knowlu_engine::runtime::RUNTIME_EXE`, so the app and the engine cannot look for
/// different files — a test asserts it.
pub fn runtime_exe(root: &Path) -> Option<PathBuf> {
    find_exe(&runtime_dir(root), 4)
}

/// The recursive search, lifted out of `runtime_exe` (fix round 1, M2) so `extract_runtime_zip` can
/// run it directly against a staging tree — one that is not yet, and may never be, at `<root>\runtime`
/// — instead of renaming that tree into a fake parent purely to reuse the `<root>/runtime` join
/// `runtime_exe`'s callers want. That rename dance cost two directory renames of a tree whose
/// freshly written exe an on-access scanner may still be holding open, immediately after tens of
/// megabytes of writes, and its failure branches could strand the extracted tree under a name
/// nothing ever cleaned up or found.
fn find_exe(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path.file_name().map(|n| n.eq_ignore_ascii_case(knowlu_engine::runtime::RUNTIME_EXE)) == Some(true) {
                return Some(path);
            }
        } else if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();
    dirs.into_iter().find_map(|d| find_exe(&d, depth - 1))
}

/// The newest `.gguf` directly in `models\`. Newest rather than first so that installing a second
/// model switches to it without the user having to delete the old one by hand.
pub fn model_file(root: &Path) -> Option<PathBuf> {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(models_dir(root))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("gguf")) == Some(true))
        .filter_map(|p| {
            let t = p.metadata().ok().and_then(|m| m.modified().ok())?;
            Some((t, p))
        })
        .collect();
    found.sort();
    found.pop().map(|(_, p)| p)
}

/// What the settings row renders. `Serialize` so `commands.rs` marshals it and computes nothing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    pub runtime: Option<PathBuf>,
    pub model: Option<PathBuf>,
    /// The model's size, so the row can say "2.1 GB" beside the name. `0` when there is none.
    pub model_bytes: u64,
}

pub fn status(root: &Path) -> Status {
    let model = model_file(root);
    let model_bytes = model.as_ref().and_then(|p| p.metadata().ok()).map(|m| m.len()).unwrap_or(0);
    Status { runtime: runtime_exe(root), model, model_bytes }
}

/// The hashing loop, shared by `sha256_of` and `install_runtime_from_zip` (fix round 1, m1). The
/// latter hashes an already-open handle and rewinds it rather than opening `src` a second time, so
/// the digest checked and the bytes extracted are provably **the same file, not a swapped one** —
/// one open handle defeats the classic delete-and-rename race between two opens of the same path.
/// **What this does not close** (fix round 2, precision on m1's claim): Windows' default share mode
/// (`FILE_SHARE_READ|WRITE|DELETE`, taken with no lock) still lets a second writable handle mutate
/// these exact bytes in place between the hash read and the later seek-and-extract read. That
/// residual window is accepted rather than closed, for the same reason m9 narrows the module doc's
/// other claim: exploiting it needs local write access to `src` as the same user already running
/// this app, which is outside this module's threat boundary, not inside it.
///
/// Streamed, in 1 MiB chunks: a `.gguf` is gigabytes and reading it whole to hash it would allocate
/// the file into memory on a laptop that is about to load it anyway.
fn hash_stream<R: Read>(mut r: R) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn sha256_of(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    hash_stream(file).map_err(|e| format!("{}: {e}", path.display()))
}

/// `Ok` when `expect` is `None` — used only for a **model** a user pointed at themselves, never for
/// a runtime — or when the digest matches, case-insensitively.
fn verify(path: &Path, expect: Option<&str>) -> Result<(), String> {
    let Some(want) = expect else { return Ok(()) };
    let got = sha256_of(path)?;
    if got.eq_ignore_ascii_case(want.trim()) {
        Ok(())
    } else {
        Err(format!("{} does not match the digest it was offered under (expected {want}, got {got})", path.display()))
    }
}

/// One upstream llama.cpp release this build is prepared to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct SupportedRuntime {
    /// The upstream release tag, e.g. `b1234`. Shown in the settings row so a user knows what they
    /// have.
    pub tag: &'static str,
    /// The asset's own file name in that release.
    pub asset: &'static str,
    /// Lower-case hex SHA-256 of the **archive as published**, not of the extracted exe.
    pub sha256: &'static str,
}

/// **The root of trust for executing a runtime** (ruling R-P3a-2). Compiled in, so it cannot be
/// edited by whatever also edited the file being installed, and so *Install from a file…* — the only
/// path that works before the site exists — is as verified as *Download*.
///
/// **Seeded by Task 1's spike, never invented.** The `Runtime:` line from
/// `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`: its release tag, its asset name,
/// and the SHA-256 the spike computed over the archive it actually downloaded and then measured
/// (Outcome B — the spike ran to completion, so this table is not empty). If a later spike's
/// outcome were C — nothing could be obtained — this table would stay empty, every runtime install
/// would be refused, and that would be the honest state: a digest that was never seen cannot be
/// vouched for. Task 12 step 8 refuses to close the plan on an empty table without the spike report
/// saying Outcome C.
///
/// Adding a release later is a code change and a release of this app, which is the point: it is a
/// deliberate act with a diff, not a file someone dropped in a folder.
pub const SUPPORTED_RUNTIMES: &[SupportedRuntime] = &[
    SupportedRuntime {
        tag: "b10840",
        asset: "llama-b10840-bin-win-cpu-x64.zip",
        sha256: "7063dfc6b874e7eee0ddf601bdf8e70e6f4a3d708926641ffad046ec51e8e30b",
    },
];

/// The table entry for a digest, case-insensitively — a checksum pasted from an upstream `.sha256`
/// file is usually upper case.
pub fn runtime_release_for(sha256: &str) -> Option<&'static SupportedRuntime> {
    let want = sha256.trim();
    SUPPORTED_RUNTIMES.iter().find(|r| r.sha256.eq_ignore_ascii_case(want))
}

/// The refusal text, in one place: `check_runtime_supported` and `install_runtime_from_zip` must
/// say exactly the same thing for the same digest, and a test (m8) pins both the computed digest
/// and the "report this one" sentence in it.
fn unsupported_runtime_msg(src: &Path, got: &str) -> String {
    format!(
        "{} is not a runtime release Knowlu knows (sha256 {got}). \
         Knowlu only runs llama.cpp builds whose digest it ships; report this one and it can be \
         added in a release.",
        src.display()
    )
}

/// Hash `src` and look it up. A convenience for a caller that wants to *ask* whether a file is
/// supported without installing it; `install_runtime_from_zip` does not call this — it hashes and
/// extracts from one open handle instead (fix round 1, m1) — but reaches the identical verdict and
/// the identical message via [`unsupported_runtime_msg`].
pub fn check_runtime_supported(src: &Path) -> Result<&'static SupportedRuntime, String> {
    let got = sha256_of(src)?;
    runtime_release_for(&got).ok_or_else(|| unsupported_runtime_msg(src, &got))
}

/// Copy a `.gguf` into `models\`, verified first.
///
/// **The source is never moved or deleted** — it is a file the user pointed at, quite possibly the
/// only copy, and quite possibly in their Downloads folder where they will look for it again.
/// Copied to `.part<pid>` and renamed, so a copy that dies half way never leaves a truncated model
/// at a name `model_file` would then hand to `RUNTIME_EXE`.
///
/// **The destination's mtime is set to now** (fix round 1, m5): `std::fs::copy` on Windows is
/// `CopyFileEx`, which carries the *source's* last-write time to the destination, and `rename`
/// preserves it. Left alone, installing a model downloaded weeks ago beside one downloaded
/// yesterday would leave `model_file`'s "newest wins" pointing at the old one with no visible
/// reason — "newest" is meant to mean "most recently installed", not "most recently touched
/// wherever it came from".
pub fn install_model_from_file(root: &Path, src: &Path, expect: Option<&str>) -> Result<PathBuf, String> {
    if src.extension().map(|x| x.eq_ignore_ascii_case("gguf")) != Some(true) {
        return Err(format!("{} is not a .gguf model file", src.display()));
    }
    let dir = models_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    verify(src, expect)?;
    let name = src.file_name().ok_or_else(|| format!("{}: no file name", src.display()))?;
    let dest = dir.join(name);
    let tmp = dir.join(format!("{}.part{}", name.to_string_lossy(), std::process::id()));
    let copied = std::fs::copy(src, &tmp).map_err(|e| format!("{}: {e}", tmp.display()));
    if let Err(e) = copied {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", dest.display())
    })?;
    let _ = std::fs::File::open(&dest).and_then(|f| f.set_modified(std::time::SystemTime::now()));
    Ok(dest)
}

/// Free the gigabytes. Every `.gguf` in `models\` goes, plus any abandoned `*.gguf.part<pid>` left
/// by a copy that died half way (fix round 1, m7) — the exact failure `install_model_from_file`
/// designs for, and the same gigabytes this button exists to free. The runtime stays, because it is
/// small and re-downloading it is the annoying half.
pub fn remove_model(root: &Path) -> Result<(), String> {
    let dir = models_dir(root);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(()) };
    for path in entries.flatten().map(|e| e.path()) {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let is_model = path.extension().map(|x| x.eq_ignore_ascii_case("gguf")) == Some(true);
        let is_partial = name.contains(".gguf.part");
        if is_model || is_partial {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// **Check the digest, then extract.** The only runtime install `commands.rs` may call, and the
/// reason the check cannot be forgotten at a call site: it is inside the function, before anything
/// is written (ruling R-P3a-2).
///
/// Opens `src` exactly once (fix round 1, m1): the digest is hashed from that handle, which is then
/// rewound and handed straight to the zip reader, so extraction is provably reading **the same
/// file, not a swapped one** — one open handle closes the delete-and-rename race a second `open()`
/// of the same path would be exposed to. It does not close every race: Windows' default share mode
/// still allows a second writable handle to mutate these bytes in place between the hash read and
/// the extraction read (fix round 2, precision on this claim — see [`hash_stream`]'s doc), and that
/// residual window is accepted rather than closed, because exploiting it already requires local
/// write access to `src` as the same user running this app. Calling `check_runtime_supported(src)`
/// followed by `extract_runtime_zip(root, src)` would still be strictly worse — a second, needless
/// `open()` — which is why `install_from_file`/`install_from_manifest` never call
/// `extract_runtime_zip` directly, and a test (M3) asserts no command does either.
pub fn install_runtime_from_zip(root: &Path, src: &Path) -> Result<PathBuf, String> {
    let mut file = std::fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let got = hash_stream(&mut file).map_err(|e| format!("{}: {e}", src.display()))?;
    if runtime_release_for(&got).is_none() {
        return Err(unsupported_runtime_msg(src, &got));
    }
    file.seek(SeekFrom::Start(0)).map_err(|e| format!("{}: {e}", src.display()))?;
    extract_runtime_from_reader(root, src, file)
}

/// Extract a llama.cpp Windows zip into `runtime\`. **No digest check** — that is
/// [`install_runtime_from_zip`]'s, and this half is separated out so the extraction rules
/// (structure, zip slip, the `.new` swap) are testable against an archive a test built, which by
/// construction can never be in [`SUPPORTED_RUNTIMES`]. Pinning a fake digest into the shipped table
/// to make a test pass would defeat the table.
///
/// **`commands.rs` must never call this.** It is a test seam, not a second install path: a test
/// (fix round 1, M3) scans every command source file and fails the build if `extract_runtime_zip`
/// appears in one, because "the check already ran, call the extractor" is exactly the mistake that
/// would route around the digest gate silently.
///
/// **Into a fresh `.new` directory, then swapped**, so a half-extracted archive is never what
/// `runtime_exe` finds — and if the archive turns out not to contain `RUNTIME_EXE` at all, nothing
/// is left behind at the real name.
///
/// **Zip slip is refused by name.** This is the one place the app extracts an archive it did not
/// create: an entry naming `..` or an absolute path would otherwise write anywhere the user can
/// write, and one of the things this directory sits beside is `updates\`.
///
/// **Zip bombs are bounded by ordering, not by a cap.** There is no limit on decompressed output
/// here — that is safe only because [`install_runtime_from_zip`] runs the digest check first on
/// every production path: a bomb cannot be in [`SUPPORTED_RUNTIMES`], so it is refused before a byte
/// is inflated. This is the other reason M3's guard matters: reaching this function directly loses
/// the ordering, not just the signature.
pub fn extract_runtime_zip(root: &Path, src: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    extract_runtime_from_reader(root, src, file)
}

/// The extraction body, shared by [`extract_runtime_zip`] (opens `src` itself) and
/// [`install_runtime_from_zip`] (hands in the handle it already hashed and rewound) so there is
/// exactly one implementation of "extract this reader into `runtime\`" regardless of which of the
/// two callers reached it.
fn extract_runtime_from_reader<R: Read + Seek>(root: &Path, src: &Path, reader: R) -> Result<PathBuf, String> {
    let dir = runtime_dir(root);
    let staging = root.join("runtime.new");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("{}: {e}", staging.display()))?;

    let result = (|| -> Result<(), String> {
        let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("{}: not a zip ({e})", src.display()))?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| format!("{}: {e}", src.display()))?;
            // `enclosed_name` is the zip crate's own zip-slip guard: `None` for an absolute path,
            // a `..` component or anything else that would escape. Refused, never skipped — an
            // archive carrying one is not an archive to install part of.
            let Some(rel) = entry.enclosed_name() else {
                return Err(format!("{}: an entry points outside the runtime directory", src.display()));
            };
            let out = staging.join(&rel);
            if entry.is_dir() {
                std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            let mut sink = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
            std::io::copy(&mut entry, &mut sink).map_err(|e| format!("{}: {e}", out.display()))?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }

    // The archive has to contain the thing the judge will look for, or it is the wrong archive.
    // Searched directly against the staging tree (fix round 1, M2) rather than by renaming it into
    // a fake `<root>\runtime` just to reuse `runtime_exe`'s join — that dance cost two directory
    // renames of a tree whose freshly written exe an on-access scanner may still hold open right
    // after tens of megabytes of writes, and its `?` branches could strand the tree under a name
    // nothing ever cleaned up.
    if find_exe(&staging, 4).is_none() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!("{} contains no {}", src.display(), knowlu_engine::runtime::RUNTIME_EXE));
    }

    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&staging, &dir).map_err(|e| {
        let _ = std::fs::remove_dir_all(&staging);
        format!("{}: {e}", dir.display())
    })?;
    runtime_exe(root).ok_or_else(|| format!("{} contains no {}", src.display(), knowlu_engine::runtime::RUNTIME_EXE))
}

/// One downloadable artefact, as the manifest describes it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}

/// What `https://knowlu.com/inference/manifest.json` serves: the runtime build and the extraction
/// model this release was tested against, each with the digest it is verified by.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub runtime: Asset,
    pub model: Asset,
}

/// Beside `latest.json` on the same static site (spec §6: Cloudflare Pages, models on R2). **The
/// site does not exist yet** — Quinn's Cloudflare account is open item §11 — so this endpoint is
/// unreachable today, which is the expected state and not an incident, exactly as the updater's is.
/// Until it answers, *Install from a file…* is the working path, and it is a better one for a
/// friend on metered wifi anyway.
pub const MANIFEST_URL: &str = "https://knowlu.com/inference/manifest.json";

/// The manifest round-trip's budget, and the download's. Different numbers for the same reason
/// `updates::CHECK_TIMEOUT` and `DOWNLOAD_TIMEOUT` are: one is a small JSON GET, the other is two
/// gigabytes on a student's hotel wifi.
const MANIFEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60 * 60);

fn agent(timeout: std::time::Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(true)
        .build()
        .into()
}

pub fn fetch_manifest(url: &str) -> Result<Manifest, String> {
    let mut response = agent(MANIFEST_TIMEOUT).get(url).call().map_err(|e| e.to_string())?;
    let text = response
        .body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{url}: not an inference manifest ({e})"))
}

/// Which half is being installed. Parsed in one place so a typo is refused by name once, rather
/// than by an `else` arm in each of the two commands (M7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Runtime,
    Model,
}

impl Half {
    pub fn parse(kind: &str) -> Option<Half> {
        match kind {
            "runtime" => Some(Half::Runtime),
            "model" => Some(Half::Model),
            _ => None,
        }
    }
}

/// *Install from a file…*, for either half. **The runtime path checks the digest table**; the model
/// path takes the user's own word for a file they chose (see the module doc's rule 2).
pub fn install_from_file(root: &Path, half: Half, src: &Path) -> Result<PathBuf, String> {
    match half {
        Half::Runtime => install_runtime_from_zip(root, src),
        Half::Model => install_model_from_file(root, src, None),
    }
}

/// Refuse an `Asset` before any of its fields are used for a network request or a filesystem path
/// (fix round 1, C1 — Critical, and m3 — Minor). Both checks exist because `Asset` is `String`s
/// deserialised straight off the network with no invariant of their own:
///
/// - **`name` must be a plain file name** — see [`plain_asset_name`]. `Path::join` **discards the
///   base entirely** for an absolute path and keeps `..` components for the OS to resolve — so a
///   manifest naming `..\..\Startup\x.zip` or `C:\Users\<user>\…\x.zip`, joined onto `downloads\`
///   unchecked, would have sent the download to that path, where the parents get created and
///   whatever already existed there gets renamed over and then deleted: arbitrary overwrite and
///   delete, as the user. The digest check cannot catch this — the bytes and the digest they are
///   measured against come from the same manifest — and this module's own premise is that a
///   manifest is "never the root of trust" (module doc, rule 2); treating one of its fields as a
///   trusted path is exactly trusting it. A manifest that meant the real asset is unaffected by this
///   check; one that meant anything else is a visible error instead of a write wherever it likes.
/// - **`url` must be `https://`.** A manifest served over HTTPS could otherwise name a plaintext or
///   local asset, which for the model half — whose only verification *is* the digest that travelled
///   beside this URL — would root that verification in nothing. The runtime half is unaffected
///   either way, since the compiled-in table decides it regardless.
///
/// Returns the validated bare file name on success, so a caller does not re-derive it.
pub fn validate_asset(asset: &Asset) -> Result<&std::ffi::OsStr, String> {
    if !asset.url.starts_with("https://") {
        return Err(format!("{}: manifest names an asset URL that is not https://", asset.url));
    }
    plain_asset_name(&asset.name)
        .ok_or_else(|| format!("manifest names an asset that is not a plain file name: {}", asset.name))
}

/// Is `name` safe to become **the whole** of a single file directly under `downloads\`?
///
/// **Fix round 2 tightens this from a five-case refusal list into an allowlist** — the same "remove
/// the class, not the instances" reasoning as the drain-past-the-cap fix in Task 3. The scoped
/// re-review found the previous `Path::file_name()`-equality check alone let three hostile forms
/// through (none of which escapes `downloads\`, so C1's actual threat — writing outside it — stayed
/// closed; this closes a smaller, adjacent one): a **trailing dot** or **trailing space**, both of
/// which the Win32 file-creation APIs silently strip before the file exists at all, so the name
/// never round-trips to what the manifest said; and a **NUL byte embedded in the name**, which
/// `Path`'s own component parsing does not treat specially (unlike the `zip` crate's
/// `enclosed_name`, which explicitly rejects it — see `extract_runtime_from_reader`'s doc).
///
/// Refused, all as one predicate rather than one refusal per case:
/// - empty;
/// - contains `/`, `\`, `:`, or a NUL byte — any of which is either a separator, a drive marker, or
///   something a filesystem call would choke on;
/// - ends with `.` or ` ` (see above);
/// - is a **Windows reserved device name** — `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`,
///   matched case-insensitively against the part of the name before its first `.`, so an *extension*
///   does not launder one (`NUL.zip` is exactly as reserved as `NUL`). Naming a download `NUL` does
///   not create a file at all — it opens the null device, and every byte written to it vanishes
///   silently, which would present as an inexplicably corrupt download with nothing to explain it;
/// - not returned unchanged by `Path::file_name()` — the belt-and-braces case that still catches an
///   absolute path, a UNC path, `..`, or anything with a directory component, exactly as before.
fn plain_asset_name(name: &str) -> Option<&std::ffi::OsStr> {
    if name.is_empty() || name.contains(['/', '\\', ':', '\0']) {
        return None;
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return None;
    }
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED.iter().any(|r| stem.eq_ignore_ascii_case(r)) {
        return None;
    }
    Path::new(name).file_name().filter(|n| n.as_encoded_bytes() == name.as_bytes())
}

/// *Download*: read the manifest, fetch the named asset, install it.
///
/// **The composition lives here, not in `commands.rs`** (M7): `commands.rs` computes nothing, and
/// three network-and-disk steps chained together with two caps and a cleanup is computation. The
/// caps are per half — a runtime archive is tens of megabytes and a model is gigabytes — and a wrong
/// URL serving an endless stream therefore fills no disk on either.
///
/// **Every field of the fetched `Asset` is validated by [`validate_asset`] before it is used**
/// (fix round 1, C1 — Critical, and m3 — Minor); see that function's doc for why `name` and `url`
/// each needed it.
///
/// **The download completes before the compiled-in table is consulted, for the runtime half, and
/// that is not a bug** (fix round 1, m4, ruled no change): the table can only be checked against
/// bytes that have already been hashed, and hashing needs the bytes. With `asset.name` validated
/// above, the worst case of a manifest naming an unsupported release is a wasted download into
/// `downloads\` of attacker-chosen bytes that are then refused and deleted by
/// `install_runtime_from_zip` — never executed, and never written anywhere but a scratch path this
/// module already owns.
///
/// The staged archive is removed on success: the runtime is now unpacked where it belongs, and the
/// zip is a second copy of it on a disk that is about to hold a model too.
pub fn install_from_manifest(root: &Path, half: Half, manifest_url: &str) -> Result<PathBuf, String> {
    let manifest = fetch_manifest(manifest_url)?;
    let (asset, cap) = match half {
        Half::Runtime => (&manifest.runtime, 512u64 << 20),
        Half::Model => (&manifest.model, 8u64 << 30),
    };
    let name = validate_asset(asset)?;
    let dest = root.join("downloads").join(name);
    download_to(&asset.url, &dest, Some(&asset.sha256), cap)?;
    // The runtime is re-checked against the compiled-in table by `install_runtime_from_zip`: the
    // manifest's digest proved the transport, the table decides whether we will run it (R-P3a-2).
    let out = match half {
        Half::Runtime => install_runtime_from_zip(root, &dest),
        Half::Model => install_model_from_file(root, &dest, Some(&asset.sha256)),
    };
    let _ = std::fs::remove_file(&dest);
    out
}

/// Download to `<dest>.part<pid>`, verify, then rename — the updater's `stage_bytes` shape, with the
/// verification in the middle so a bad transfer never reaches the name the installer will read.
///
/// `expect` is the **manifest's** digest and proves only the transport; for a runtime, whether the
/// bytes may be *executed* is decided afterwards by [`check_runtime_supported`] against the
/// compiled-in table. `None` is accepted for the same reason `verify` accepts it — a model the user
/// chose — and is never passed for a runtime.
///
/// `cap` bounds the response, so a wrong URL serving an endless stream fills no disk.
pub fn download_to(url: &str, dest: &Path, expect: Option<&str>, cap: u64) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let tmp = {
        let mut s = dest.as_os_str().to_os_string();
        s.push(format!(".part{}", std::process::id()));
        PathBuf::from(s)
    };
    let out = (|| -> Result<(), String> {
        let mut response = agent(DOWNLOAD_TIMEOUT).get(url).call().map_err(|e| e.to_string())?;
        let mut reader = response.body_mut().with_config().limit(cap).reader();
        let mut file = std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::io::copy(&mut reader, &mut file).map_err(|e| format!("{}: {e}", tmp.display()))?;
        drop(file);
        verify(&tmp, expect)
    })();
    if let Err(e) = out {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", dest.display())
    })
}
