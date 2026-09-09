# Knowlu

Knowlu is a desktop app for students that answers one question every morning: **what should I work
on today, and in what order?** It reads the deadlines and coursework you connect (an LMS calendar
feed, zyBooks, VHL, a busy-time calendar), keeps a plain-markdown vault of tasks on your own
machine, ranks them deterministically — no model in the ranking path — and shows the day in one
window. Windows, Rust + Tauri.

## Layout

```
Cargo.toml        the workspace: members engine + app, one Cargo.lock, one target/, one release profile
engine/           knowlu-engine — the deterministic engine and its CLI (rank, ingest, coursework, judge,
                  surface, write, info, issues, runs); no GUI dependency, ever
engine/tests/     integration tests and the frozen fixtures (bytes, never re-encoded)
app/              knowlu — the Tauri desktop app; links the engine as a library and runs
                  knowlu-engine.exe as a sibling process for scheduled slots
site/             knowlu.com: the download page and privacy page, plain HTML
scripts/          release.ps1 (the one release command), sign.ps1, find-signtool.ps1,
                  scratch-vault.ps1, the headless screenshot/check scripts, starvation.ps1
docs/             specs/, plans/, notes/, reports/ (historical, unchanged), surface/ (the read
                  model's living reference), procedures/ (go-live and phase-2 checklists)
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

## Where things are

- `CLAUDE.md` — the working rules for this repository.
- `HANDOFF.md` — where the work stands and what is next.
- `PROVENANCE.md` — where this code came from, what moved and what was renamed.
- `VISION.md` — the end state.
- `docs/specs/` — the designs; `docs/plans/` — how they were built; `docs/reports/` — what the
  reviews found; `docs/notes/` — product, pricing, the redesign program, the legal landscape.
- `docs/surface/anatomy.md` — the read model behind the window; `app/README.md` — the app itself.
