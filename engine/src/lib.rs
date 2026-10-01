//! knowlu-engine — Knowlu's deterministic engine.
//!
//! Rust port of `engine/`. See `docs/superpowers/specs/2026-09-01-rust-rewrite-design.md`.
//!
//! # The rules that govern every module here
//!
//! - **Deterministic.** Same input, same output. No inference. Judgment happens at the
//!   boundary and enters as data.
//! - **Never rewrite a vault file wholesale.** Edits are single-line frontmatter surgery, and
//!   no note is ever parsed and re-dumped. The crate's one YAML emitter (`yamlemit`, a PyYAML
//!   transcription) exists for the two things that must be *produced*: the `judgment:` literal
//!   the surgery splices in, and the frontmatter of a brand-new `info/` or `issues/` note.
//! - **Journal first, then edit.** Every note write records itself before it touches a file.
//! - **Behaviour parity is the acceptance test** while the port is in progress: this engine is
//!   correct when it emits the same bytes as the Python engine, from the same fixture vault, at
//!   the same pinned date. No feature work lands until cutover.
//!
//! # Module map (see the plan's wave table)
//!
//! Wave 1: models, ledger, planning, provenance, weekcal, wincred, eventledger
//! Wave 2: journal, ids, ingest, write
//! Wave 3: scheduling, ranking, approvals, render, passes, reconcile
//! Wave 4: calfeed
//! Wave 5: events, eventfilter, eventfeed, eventroster, eventemit
//! Wave 6: coursework, zybooks, vhl
//! Wave 7: runs, yamlemit (lifted out of provenance), info, issues, cli
//!
//! Modules are declared here as each wave lands. The four import cycles the Python graph
//! contains are preserved — they are legal between modules of one crate.

// Wave 1 — leaves. No cross-module dependencies except planning::day_key, which weekcal shares
// because Python's two DAY_KEYS lists are the same list.
pub mod pystr;
pub mod yaml;
pub mod yamlemit;
pub mod models;
pub mod planning;
pub mod weekcal;
pub mod ledger;
pub mod provenance;
pub mod eventledger;
// Wave 2 — the write path.
pub mod journal;
pub mod ingest;
pub mod ids;
pub mod write;
// Wave 4 — calendar busy time.
pub mod calfeed;
// Wave 5 - event discovery.
pub mod events;
pub mod eventfilter;
pub mod eventfeed;
pub mod eventroster;
pub mod eventemit;

// Wave 6 - coursework: zyBooks and VHL.
pub mod coursework;
pub mod zybooks;
pub mod vhl;
// Stream J T8 — completion detection, tier 1: a vendor's own 100% becomes a `status: done` amend.
pub mod completion;
// Wave 7 — runs landed early because coursework::main needs it; info and issues are the two
// note constructors that needed yamlemit's block style (Task 15).
pub mod runs;
pub mod info;
pub mod issues;

// Knowlu foundation plan, Task 3 — pure slot arithmetic over runs::RunnerConfig and run
// records for the app's scheduler thread. Never reads the clock, never writes.
pub mod schedule;

// Wave 3 — unblocked early: both depend only on models/planning/weekcal.
pub mod scheduling;
pub mod ranking;
pub mod approvals;
pub mod render;
pub mod passes;
pub mod reconcile;
pub mod cli;

// The read model for the desktop console (plan 2026-09-02): read-only, computed from the vault
// `cli` already knows how to load, never written to it.
pub mod surface;

// The student's profile (M2 spec §7.6, hand-off H1): `profile/preferences.md` and
// `profile/interests.md` read into plain data and written through `write`, never parsed and re-dumped.
pub mod profile;

// The commitment model (spec 2026-09-23): the notes in `commitments/`, read into plain data for
// `weekcal` and the piece-2 overlap API. Pure but for `load`'s reads; never a model or the network.
pub mod commitments;

// The console's write path (Knowlu plan 1): interaction events only, through the same
// `JsonlLedger` seam as the journal. Never read by the engine.
pub mod uievents;

// Knowlu plan 3a — the judgment seam (spec §5.2). Pure: three tiers, a model behind a trait, a
// rule table behind a trait. `rank` never reaches it; `enrich` is its only production caller.
pub mod judge;

// Knowlu plan 3a — the judge's process half: llama.cpp one process per call, killed if it hangs.
// Consumed as a release artefact, never linked (spec §5.3 as amended 2026-09-07).
pub mod runtime;

// Knowlu plan 3a — what was judged, by id and by field value, into the profile's app data and never
// into the vault (spec §5.4, §5.6). 3b's rule promotion reads it.
pub mod judgelog;

// Knowlu C2 — the judgment service as seen from the device (cloud design §3.2, §5.2): one HTTPS
// call per item behind `judge::Model`, `judge::EventModel` and `judge::EmailModel`, with the
// account's session token out of Credential Manager. `rank` never reaches it (decision 11).
pub mod cloudmodel;

// Knowlu plan 3a — the enrichment pass (spec §5.5 step 1): select, judge, write as
// `agent:knowlu.enrich`, log. The cloud routine's step 3, run locally, so that step starves.
pub mod enrich;

// Knowlu C3′ — journal sync (cloud design §5.5, as amended 2026-09-17). Every record this device
// writes, and the text of every note that changed, goes up to the account's own copy as it is —
// readable by our service, encrypted at rest, and said so on the privacy page (there is no
// device-held key); every record another desktop of the same account wrote comes down and is
// applied through `write`. Transport, never judgment: `rank` does not reach it, and `sync` always
// exits 0.
pub mod sync;
// Knowlu C3′ — the entitlement gate (cloud design, amendment 2026-09-17, ruling 3). The app caches
// `GET /entitlement` with a 72-hour grace; past it, the four cloud slot steps refuse to run and say
// so in one line at exit 0. Reads the app's cache, never the network, and never writes it.
pub mod entitle;

// The console's backup mirror + zip snapshots (Knowlu plan 1, Task 4). The console is the
// only caller; never read by the engine.
pub mod backup;

// `childproc` outlives `history`: `runs::git_sha` and `runtime.rs` both spawn children, and a GUI
// application's child process must never flash a console window.
pub mod childproc;

#[cfg(windows)]
pub mod wincred;

/// The date the golden fixture renders at. Tests pin to this; nothing else may.
pub const PINNED_FIXTURE_DATE: &str = "2026-08-28";

/// The checkout's short git SHA at build time, embedded by `build.rs` via `KNOWLU_BUILD_SHA`.
/// `build.rs` always sets the var — falling back to the literal `"unknown"` when `git` is
/// unavailable or the build runs outside a checkout, and never failing the build over it — so in
/// practice this is always `Some`; `option_env!` is the honest type for "a build script sets this,
/// but nothing stops a build without one". Lets two binaries — the console and the engine — say
/// whether they were built from the same commit (spec §15 Q1: the console shows `engine newer than
/// console` when not).
pub const BUILD_SHA: Option<&str> = option_env!("KNOWLU_BUILD_SHA");
