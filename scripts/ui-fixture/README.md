# The new page's fixture harness

Loads the new page (`app/static/next/`) in headless Edge against staged read-model envelopes, checks
it and takes shots (v0.4.0 note §8 M12, plan T5c). It replaces the old page's `?fixture=` path: no
review aid ships, and nothing here is in `app/static`.

## Run it

```
python scripts/ui-fixture/make_base_courses.py   # builds the engine (-j 2), writes envelopes/
python scripts/ui-fixture/run.py base            # every set whose name starts with "base"
python scripts/ui-fixture/serve.py --set base-courses   # serve one set and open the printed URL yourself
```

`run.py` needs Python with Playwright (`pip install playwright`); it drives the system's Edge
(`channel="msedge"`), headless, so no browser download is needed. Shots go to `target/ui-shots/`
(never committed): `<set>-<w>x<h>-<theme>-<text>.png` for 820 x 600 and 1280 x 800, light and dark,
default and larger text; and `sidebar-{peek,pinned}-<w>x<h>-<theme>.png` for the peek and the pinned
sidebar. `--scale 1.25` (or 1.5) sets the device scale. Exit 1 on any failure.

## What it checks

For every combination: no console or page error; `ui-ready` emitted once; `launch_state` asked once
and `state` asked with view `today`; no `google_status` or `mark_seen` at launch; no horizontal
scroll, and only `#center` scrolls; the theme and the 88% / 100% zoom follow the stored preference;
the sidebar lists the courses whose status is absent or `active`; the clock reads the envelope's
`now`; the window controls show; the Status dot is amber for warnings or unreadable notes and red
when a source needs the student. Then `#tasks` asks `state` with view `all`, `#class/<slug>` marks
that class, `#constructor` falls back to Home, and a route with no page leaves `#center` empty.
On `base-courses` it then checks the sidebar: the peek opens after about 350 ms and not before, a
mouse click inside does not hold it, it folds about 400 ms after the pointer leaves, and Escape folds
it; pinned, it is the wide sidebar.
Page tasks add their own DOM checks to `run.py` as their pages land.

## The pieces

- `serve.py` binds `127.0.0.1` on a port the OS picks and serves `app/static/next/` with one
  `<script>` added at the top of `<head>`: a stub `window.__TAURI__` (in `serve.py` as `STUB`).
  `core.invoke` answers from the named set and records every call in `window.__fixture.calls`;
  `event.emit` records in `window.__fixture.events`; the window does nothing. The stub freezes `Date`
  at `2026-08-28T09:00` local, the surface oracle's pinned now, so every date word is the same on
  any day. A command that takes `view` answers with the set's envelope for that view; anything not
  staged answers `{ok: false}`.
- `make_base_courses.py` builds every envelope from the engine, never by hand: a vault in a temporary
  folder with three files of `engine/tests/fixtures/vault-full/config/` (read only: `events.yaml`,
  `planning.yaml`, `runners.yaml`; never `ingest.yaml` or `week_template.yaml`), an invented week of
  class times, invented courses (`ABC 101`, `DEF 210`, `GHI 150` dropped, `JKL 120`) and invented
  tasks; then
  `knowlu-engine surface --view today|all --today 2026-08-28 --now 2026-08-28T09:00 --build-sha pinned`,
  wrapped as the app's `state` wraps it (the seven topline keys the app adds). Running it twice
  gives byte-identical files; it stops if an answer names the temporary vault, carries a course slug
  the fixture vault's tasks name, or holds a class block that is not one of the invented times.
- `envelopes/`: one set is `<set>-today.json` and `<set>-all.json`, plus an optional
  `<set>-answers.json` of other commands' answers.

| Set | Made by |
|---|---|
| `base-courses` | the base vault |
| `base-overload` | eight more tasks due today and tomorrow, far past the daily budget |
| `base-unreadable` | one task note whose frontmatter does not parse |
| `base-digest-only` | Waiting holds only an events digest |
| `base-first-run` | the app's `first_run` block, state null (the vault has never been ranked) |
| `base-reconsent-owed` | the base envelopes, and `privacy_status` answering `needs_accept: true` |
| `base-signed-out` | the base envelopes, and `grades_status` signed out (the Status dot turns red after the paint) |
| `base-engine-down` | `state` answers `ok: false` (the Status dot turns red: the engine did not answer) |

After T23 (the swap) the page lives in `app/static`; `serve.py`'s `ROOT` is the one line that moves.
