PASS

# Review: fix/rustsec-cookie-url (cookie 0.18.3, url 2.5.9)

- Branch: `fix/rustsec-cookie-url`, one commit `1144f832` on `origin/main` `9f593aa2` (merge base equals `origin/main`).
- Size: Small. `Cargo.lock` only; no contract-list file in `git diff --stat`.
- Reviewer: `reviewer`, 2026-11-24.

No findings.

## 1. Scope of the diff

`git diff origin/main...HEAD --stat`: `Cargo.lock | 12 ++++++------`, one file. The three hunks change only
the `version` and `checksum` lines of:

| Crate | From | To | `source` |
|---|---|---|---|
| `cookie` | 0.18.2 | 0.18.3 | crates.io registry, unchanged |
| `form_urlencoded` | 1.2.2 | 1.2.3 | crates.io registry, unchanged |
| `url` | 2.5.8 | 2.5.9 | crates.io registry, unchanged |

No package entry is added or removed, no `source` line changes, no git or path source appears, and no
crate's `dependencies` list changes (the context lines of all three hunks show the dependency lists
intact, and nothing outside the hunks moved). No manifest changed. Each crate still has exactly one
entry in the lockfile, so no dependent names a versioned form such as `"url 2.5.8"`.

The new checksums could not be checked against the registry index from this review; cargo verifies
them on fetch, so a mismatch would fail CI's build rather than ship.

## 2. Advisory fixes and cargo-deny

- RUSTSEC-2026-0594 (`cookie`, fixed in 0.18.3): the lockfile now has 0.18.3. Met.
- RUSTSEC-2026-0592 (`url`, fixed in 2.5.9): the lockfile now has 2.5.9. Met.

cargo-deny was not run by this review: this role runs no cargo command, so neither
`cargo deny check advisories` nor `cargo deny check bans licenses sources` (the two matrix legs of the
`cargo-deny` job in `.github/workflows/security.yml`, run through `EmbarkStudios/cargo-deny-action`)
has a result line here. The commit message reports `advisories ok` for the first. The second cannot
change on this diff: `deny.toml`'s bans (`openssl`, `openssl-sys`), `unknown-registry = "deny"`,
`unknown-git = "deny"` and `wildcards = "deny"` are all decided by package names, sources and
manifests, none of which the diff touches, and patch bumps of these three crates keep their
MIT OR Apache-2.0 licensing. The PR's own CI run of both legs is the proof to read before merge.

## 3. `form_urlencoded` moves with `url`

From the lockfile (in place of `cargo tree -i form_urlencoded`): exactly one package lists
`form_urlencoded` as a dependency, and it is `url`. There is one `form_urlencoded` entry (1.2.3);
nothing pins 1.2.2.

## 4. Where the two crates sit (context, not a finding)

From the lockfile's reverse dependencies (in place of `cargo tree -i`):

- `cookie` is used by `cookie_store`, `tauri`, `tauri-runtime` and `wry`. `cookie_store` comes in
  through `ureq`, which both `knowlu` and `knowlu-engine` depend on directly; the engine enables
  `ureq`'s `cookies` feature for the VHL login (`engine/Cargo.toml`), the app does not. So `cookie`
  is on the engine's HTTP path (VHL's session cookies) and in the app's webview layer through `tauri`.
  Its lockfile entry depends only on `percent-encoding`, `time` and `version_check`: no `aes-gcm` or
  `hmac`, so the private-cookie code the advisory concerns is not compiled into this graph. The
  advisory was not reachable here; the bump clears the check rather than a live exposure.
- `url` is used by `cookie_store`, `reqwest`, `schemars`, `tao`, `tauri`, `tauri-codegen`,
  `tauri-plugin-updater`, `tauri-runtime`, `tauri-runtime-wry`, `tauri-utils`, `tower-http`, `ureq`,
  `urlpattern` and `wry`. That puts it on every sensitive path named in the dispatch: the updater
  (`tauri-plugin-updater` and its `reqwest`), sign-in and every account call in `app/src/account.rs`
  (through `ureq`), and the engine's HTTP calls (through `ureq`). The advisory's trigger is
  `Url::origin()` on nested `blob:` URLs; whether any of these callers reach it with untrusted input
  was not traced, and it no longer matters once 2.5.9 ships.

## 5. CLAUDE.md rules

- TLS stays rustls and ring: the diff adds no package, so it adds no OpenSSL crate. The lockfile has
  no `openssl`, `openssl-sys` or `native-tls` entry. The one `openssl`-named entry, `openssl-probe`
  (a certificate-path probe that links nothing, through `rustls-native-certs` under
  `rustls-platform-verifier`), is on `origin/main` already and untouched.
- `tauri` stays out of the engine: `knowlu-engine`'s lockfile dependency list (`clap`, `getrandom`,
  `html-escape`, `jiff`, `regex`, `ring`, `serde`, `serde_json`, `serde_yaml_ng`, `sha1`, `ureq`,
  `windows`, `zip`) has no `tauri`, and the diff does not touch it.

## Other gates

- Line endings: the diff carries no carriage returns; `Cargo.lock` stays LF.
- Ignored tests: the commit's `3 ignored` matches the tree, which has three `#[ignore]` attributes
  (traps 4 and 5 in `engine/src/events.rs`, `app/tests/scheduler.rs::run_slot_end_to_end`). The diff
  cannot change them.
- Single-user assumptions, frozen references, engine and app invariants, cloud, privacy: not touched.

PASS
