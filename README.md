# Knowlu

Knowlu is a desktop app for students that answers one question every morning: **what should I work
on today, and in what order?** It reads the deadlines and coursework you connect (an LMS calendar
feed, zyBooks, VHL, grades, forwarded mail, a busy-time calendar), keeps a plain-markdown vault of
tasks on your own machine, ranks them deterministically — no model in the ranking path — and shows
the day in one window. It needs an account; the judgments (what course, how long, does it matter)
run in our cloud, and portal passwords never leave your device. Windows, Rust + Tauri.

## Layout

```
Cargo.toml        the workspace: members engine + app, one Cargo.lock, one target/, one release profile
engine/           knowlu-engine — the deterministic engine and its CLI (rank, ingest, coursework,
                  coursework-discover, grades, commitments, judge, sync, surface, mail-applied, write,
                  info, issues, runs); no GUI dependency, ever
engine/tests/     integration tests and the frozen fixtures (bytes, never re-encoded)
app/              knowlu — the Tauri desktop app; links the engine as a library and runs
                  knowlu-engine.exe as a sibling process for scheduled slots
cloud/            supabase/ — the backend: migrations, edge functions, their README (cloud/eval/ too)
site/             knowlu.com: the home page, the terms and privacy pages, and the confirmation pages a
                  sign-in link or a checkout returns to, plain HTML
scripts/          release.ps1 (the one release command), sign.ps1, find-signtool.ps1,
                  scratch-vault.ps1, starvation.ps1, ci/ (the workflows' helper scripts),
                  site-check/, ui-fixture/, the screenshot and icon scripts
.github/          workflows: ci.yml, release.yml, site.yml, security.yml
docs/             specs/, plans/, notes/, reports/ (what the reviews found), reference/ (commands, app,
                  toolchain, price change), surface/ (the read model's living reference), design/,
                  procedures/ (go-live and phase-2 checklists)
```

## Build and test

```
cargo build --workspace
cargo test --workspace          # 0 warnings is part of green
cargo build --release --workspace
```

Toolchain: `stable-x86_64-pc-windows-gnu` (1.98) with mingw-w64 binutils on `PATH`; TLS is
`rustls`/`ring`, never OpenSSL. `cargo test --release` does not link (the release profile is
`panic = "abort"`) — test in the dev profile. Releases go through `scripts\release.ps1` **only**; a
hand-run `cargo tauri build` skips its gates and can ship a zero-byte engine. Develop and demo
against a scratch vault (`scripts\scratch-vault.ps1 -Source <vault>`), never a live one.

## Releases

Releases are built, signed and published by CI only. Push a tag `v<version>` that matches the
workspace `version` in `Cargo.toml` and `app/tauri.conf.json`, and `.github/workflows/release.yml`
runs `scripts\release.ps1` on a Windows runner: both binaries from that one commit, the engine
staged as the app's sidecar, every binary Authenticode-signed by Azure Trusted Signing over OIDC
(no signing secret exists anywhere; the switch below turns that step off), the size gate, then
`site\` — with `releases/latest.json` — deployed to Cloudflare Pages at `knowlu.com`, and a GitHub
Release carrying the installer, its `.sig` and `latest.json`. Installed apps poll `https://knowlu.com/releases/latest.json` and verify
the `.sig` against the public key in `app/tauri.conf.json`; the private half lives only as the
GitHub secret `TAURI_SIGNING_PRIVATE_KEY`.

One switch exists, and it is a person's decision, never a default: with the repository variable
`RELEASE_AUTHENTICODE` set to `off`, the Azure steps are skipped and the release ships without an
Authenticode signature. Windows SmartScreen then warns when the installer first runs (More info,
then Run anyway), the release notes open by saying so, and the workflow's verify step asserts that
every binary is unsigned rather than skipping the check. Unset means signing is required, so a
missing Azure line stops the release instead of shipping unsigned by accident. The updater
signature is never optional. v0.4.0 shipped with the switch off.

The workflow is two jobs joined by an artifact: `build` makes and signs everything and uploads what
leaves the machine, `publish` downloads it, deploys `site\` and creates the Release, and compiles
nothing. So a failure at the deploy or at the Release reruns on its own —
`gh run rerun <id> --failed`, minutes rather than a whole rebuild. The Tauri CLI the build uses is
downloaded prebuilt for the version the workflow pins and verified against a SHA-256 pinned beside
it, never compiled; bumping it means changing the version and the hash together.

A price change needs no release: `.github/workflows/site.yml` (run by hand, from `main` only) publishes
`site\` alone, restoring the latest Release's installer and `latest.json` first; the steps are in
`docs/reference/price-change.md`.

Locally, run only `.\scripts\release.ps1 -DryRun`. It builds and bundles a throwaway installer,
prints `UNSIGNED:` for every file because a laptop holds no signing profile and no updater key,
and publishes nothing. That is the correct local state, not a degraded one.

## Where things are

- `CLAUDE.md` — the working rules for this repository.
- `HANDOFF.md` — where the work stands and what is next.
- `PROVENANCE.md` — where this code came from, what moved and what was renamed.
- `VISION.md` — the end state.
- `docs/specs/` — the designs; `docs/plans/` — how they were built; `docs/reports/` — what the
  reviews found; `docs/notes/` — product, pricing, the redesign program, the legal landscape.
- `docs/reference/` — every engine command's contract, the app's commands, data and CI, the toolchain's
  traps, the price-change runbook.
- `docs/surface/anatomy.md` — the read model behind the window; `app/README.md` — the app itself;
  `cloud/supabase/README.md` — the backend.
