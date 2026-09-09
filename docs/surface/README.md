# S2 Surface — picking this up on another machine

Everything needed to continue building the dashboard. Nothing about S2 lives only in a chat
transcript; if something is missing here, that is a bug in this file.

## Read in this order

1. **`docs/superpowers/notes/2026-08-28-redesign-program.md` §11** — every S2 decision, why it was
   made, and what was rejected. §11.3 (what "customizable" means) constrains the whole stack, so
   read it before touching any surface file. §10 is S3 (data quality), which runs in parallel.
2. **`docs/mockups/2026-08-31-s2-console-CHOSEN.html`** — open it in a browser. This is the agreed
   design, at real data, with working interactions (progress, the decisions deck, info close).
3. `docs/mockups/2026-08-31-s2-runway.html` and `-ledger.html` — the two directions not chosen.
   Kept because Runway's threshold strip is worth stealing and Ledger is the only auditable one.
4. **`VISION.md`** and **`CLAUDE.md`** — standing rules the surface must not break, especially
   the engine's stdlib-only dependency rule and "nothing lives only in a UI".

The mockups are authored as **artifact bodies** — no `<!doctype>`, `<html>`, `<head>` or `<body>`,
because that is how claude.ai/code publishes them. Opening one directly in a browser works
(quirks mode) but is not exactly what ships. Use the screenshot script below for a faithful render.

## Design skills used

**Installed 2026-08-31 (second session): 22 skills in `~/.claude/skills/`, ~11 MB.** None are
vendored into this repo — reinstall them per machine with the commands below. **Claude Code must
be restarted before the `Skill` tool sees them;** until then, read `~/.claude/skills/<name>/SKILL.md`
directly.

- **superpowers** (`obra/superpowers`) — 14 skills, and the one this repo's whole workflow assumes:
  `brainstorming`, `writing-plans`, `executing-plans`, `subagent-driven-development`,
  `test-driven-development`, `systematic-debugging`, `requesting-code-review`,
  `verification-before-completion`, `using-git-worktrees`, and more. Every plan in
  `docs/superpowers/plans/` opens by naming `superpowers:subagent-driven-development` or
  `superpowers:executing-plans`, and `docs/HANDOFF.md` tells the next session to brainstorm with
  `superpowers:brainstorming` — **none of that worked before this install.**
  ```bash
  git clone --depth 1 https://github.com/obra/superpowers.git /tmp/sp
  cp -r /tmp/sp/skills/* ~/.claude/skills/
  ```
- **frontend-design** (Anthropic's official plugin) — one skill, aesthetic direction and typography.
- **ui-ux-pro-max and its siblings** — the whole `nextlevelbuilder` set was installed, not just
  `ui-ux-pro-max`: `design`, `design-system`, `ui-styling`, `brand`, `banner-design`, `slides`.
  `ui-styling` (5.8 MB) and `ui-ux-pro-max` (3.7 MB) carry pattern databases and search scripts.

Original two-skill instructions, still current:

```bash
# Anthropic's official frontend-design plugin
git clone --depth 1 --filter=blob:none --sparse https://github.com/anthropics/claude-code.git /tmp/cc
cd /tmp/cc && git sparse-checkout set plugins/frontend-design
cp -r plugins/frontend-design/skills/frontend-design ~/.claude/skills/

# ui-ux-pro-max (third-party design database + search tool)
git clone --depth 1 https://github.com/nextlevelbuilder/ui-ux-pro-max-skill.git /tmp/uiux
cp -r /tmp/uiux/.claude/skills/ui-ux-pro-max ~/.claude/skills/
```

Newly installed skills are not visible to the `Skill` tool until Claude Code restarts; until then,
read `~/.claude/skills/<name>/SKILL.md` directly.

Note from use: ui-ux-pro-max's `--design-system` generator **misrouted this brief**, returning a
landing-page pattern ("Hero → product video → feature breakdown → CTA") and a teal/orange palette
for what is a dense ops console. Its font-pairing and pre-delivery checklist were useful; its
palette was not used. Treat its output as a suggestion, never as a spec.

## Verifying a layout — look at it

Design bugs in this project have consistently been invisible in source and obvious on screen.
Before claiming a layout works:

```bash
python -m venv .wv
.wv\Scripts\python -m pip install playwright
.wv\Scripts\python -m playwright install chromium
.wv\Scripts\python scripts/mockup-shots.py docs/mockups/2026-08-31-s2-console-CHOSEN.html shots/
```

It renders eight viewports (1920 → 390), reports any horizontal overflow with the offending
elements, and writes a full-page screenshot per width. **Then actually read the screenshots.**

`.wv/` is a throwaway venv — Playwright must never enter `requirements.txt`. The engine stays
stdlib + PyYAML + tzdata.

## Native window — verified, and how to re-verify

The dashboard ships as a real desktop window (pywebview + Edge WebView2), not a browser tab.
Confirmed working on Quinn's desktop 2026-08-31: Python 3.14.2, `pythonnet 3.1.0` (cp314 wheel —
this was the risk and it is cleared), rendered under `Chrome/151.0.0.0`, closed cleanly.

```bash
python -m venv .wv2 && .wv2\Scripts\python -m pip install pywebview
```

```python
import pathlib, time, webview

win = webview.create_window("Knowlu", url=pathlib.Path("<mockup>.html").resolve().as_uri(),
                            width=1500, height=940, background_color="#0A0B0D")

def probe(window):
    time.sleep(2.5)
    print("title :", window.evaluate_js("document.title"))
    print("blocks:", window.evaluate_js("document.querySelectorAll('.spill i').length"))
    print("agent :", window.evaluate_js("navigator.userAgent"))
    time.sleep(3)
    window.destroy()

webview.start(probe, win)
```

`webview` exposes no `__version__`; use `importlib.metadata.version("pywebview")`.

## Shape of what gets built

One process. `engine/serve.py` runs a stdlib `http.server` on `127.0.0.1:8765` on a daemon thread
with the engine in-process; `app/window.py` opens the pywebview window against it on the main
thread. Closing the window stops both. While it runs, `localhost:8765` also works in a browser.

**The dependency boundary is load-bearing.** `engine/` imports nothing new and stays stdlib +
PyYAML + tzdata, so the cloud runner never drags a GUI toolkit into its environment.
`app/window.py` is the only file importing pywebview, and `requirements-desktop.txt` is separate
from `requirements.txt`.

All writes go through `engine.write` exactly as the runners do — journal record first, then
single-line frontmatter surgery — so desktop and laptop reconcile under S1's per-field
later-timestamp-wins rule. Never hand-edit a note from the surface.

## What is actually on the app

`docs/surface/inventory.md` (written 2026-08-31) — every element on the console, sorted by
provenance: **ported** from the Obsidian dashboard (all 35 things `today.md` and the two Bases
views carry today, including the 137-row ranked list and the half-broken Snooze button),
**added** by the chosen mockup, and **asked for in notes but never mocked up**. Its §3.2 lists
the **fourteen** wants that have **no home in S2** and still need a ruling — read that before
assuming a feature is covered. It is the scope check: if something is not in one of those tables,
it is not being built. §5 flags the asymmetry that matters most: event relevance is about to
learn from Quinn's decisions, and ranking still is not.

## What each region is and why

`docs/surface/anatomy.md` (written 2026-08-31) — the artifact §11.3 requires. One entry per region:
the question it answers, what computes it, which ruling put it there, and **what was deliberately
left out**. Read a region's "left out" line before reshaping it — most record something already
tried and rejected. Also carries the eight-page map, the cross-cutting behaviours (⚑ flags, the
"why" disclosure, click-to-edit, empty states, `took:`, interaction events), all 30 design tokens,
and the do-not-re-propose list.

`inventory.md` covers provenance; `anatomy.md` covers intent.
