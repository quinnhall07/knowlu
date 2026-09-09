"""Screenshot the console page at eight viewports from a frozen State JSON. Layout is looked at,
not reasoned about (S2 §11.12): three bugs in the 2026-08-31 session were invisible in source.

Usage (throwaway venv; Playwright must never enter requirements.txt):
    python -m venv .wv && .wv\\Scripts\\python -m pip install playwright && .wv\\Scripts\\python -m playwright install chromium
    .wv\\Scripts\\python scripts/console-shots.py engine/tests/fixtures/surface-today-full.json shots/

Serves a FRESH TEMP DIRECTORY over loopback — never the repo root (ruling R40, final whole-branch
review: config/ingest.yaml holds a live Blackboard token and a secret Google Calendar capability
URL, in the working tree and in history; this script must never put that tree behind a socket,
even loopback-only, even for the run's lifetime). The temp directory holds a copy of app/static/
(as app/static/) and the fixture JSON (as fixtures/<name>.json); it is removed at exit whether the
run succeeds or not.

Opens app/static/index.html?fixture=/fixtures/<name>.json, shoots, reports overflow. Exit 1 if any
viewport scrolls horizontally. Read-only; the temp server dies with the process.

The console is desktop-only (spec decision 4; the Tauri window's minWidth is 820). All eight
viewports below are shot and reported, but the exit-1 overflow gate only counts viewports
>= 820px wide; the 390x844 phone shot is informational only — the app is not meant to fit there,
and it is not a gate failure if it doesn't.

Every viewport is shot at each of VIEWS — the default hash (Today) plus the three main-column
views plan 2 added, whose rows are .row and so inherit the task row's grid unless they say
otherwise (final fix wave A1: they did not, and it showed). The default hash keeps its
"<width>.png" name and its one-line-per-viewport report; a named view writes
"<width>-<view>.png" and reports with the view name after the size. The state comes from one
frozen fixture, so a view is only worth shooting if the fixture has rows for it — plant them in
a copy and pass that copy.
"""
import http.server
import shutil
import socketserver
import sys
import tempfile
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

VIEWPORTS = [(1920, 1080), (1512, 945), (1280, 800), (1180, 820), (1024, 768), (900, 900), (820, 900), (390, 844)]
VIEWS = ["", "decisions", "good-to-know", "issues"]   # "" is the default hash (Today)
GATE_MIN_WIDTH = 820  # desktop-only (spec decision 4); narrower is informational, not a gate failure
REPO = Path(__file__).resolve().parents[1]


def main(argv: list[str]) -> int:
    fixture = Path(argv[1]).resolve()
    out = Path(argv[2]); out.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="qo-console-shots-"))
    try:
        shutil.copytree(REPO / "app" / "static", tmp / "app" / "static")
        fixtures_dir = tmp / "fixtures"; fixtures_dir.mkdir()
        shutil.copy2(fixture, fixtures_dir / fixture.name)
        handler = lambda *a, **k: http.server.SimpleHTTPRequestHandler(*a, directory=str(tmp), **k)
        with socketserver.TCPServer(("127.0.0.1", 0), handler) as srv:
            port = srv.server_address[1]
            threading.Thread(target=srv.serve_forever, daemon=True).start()
            url = f"http://127.0.0.1:{port}/app/static/index.html?fixture=/fixtures/{fixture.name}"
            bad = 0
            with sync_playwright() as p:
                browser = p.chromium.launch()
                for w, hgt in VIEWPORTS:
                    page = browser.new_context(viewport={"width": w, "height": hgt}, color_scheme="dark", device_scale_factor=1).new_page()
                    page.goto(url); page.wait_for_timeout(900)
                    for view in VIEWS:
                        # One page per viewport, routed by hash — the console is a single document
                        # and route() redraws the main column on hashchange, so a reload per view
                        # would only cost time.
                        if view:
                            page.evaluate("h => { location.hash = h; }", "#" + view)
                            page.wait_for_timeout(300)
                        sw, iw = page.evaluate("document.documentElement.scrollWidth"), page.evaluate("window.innerWidth")
                        name = f"{w}.png" if not view else f"{w}-{view}.png"
                        page.screenshot(path=str(out / name), full_page=True)
                        overflow = sw > iw
                        flag = "OVERFLOW" if overflow else "ok"
                        gated = w >= GATE_MIN_WIDTH
                        suffix = "" if gated else " (informational, below desktop minWidth)"
                        label = f"{w}x{hgt}" if not view else f"{w}x{hgt} {view}"
                        print(f"{label}: scrollWidth={sw} innerWidth={iw} {flag}{suffix}")
                        if overflow and gated:
                            bad += 1
                    # Plan 4a Tasks 6-7: the two vault-less windows and the settings overlay. None
                    # is a hash route and all three need a backend that is not there, so they are
                    # put on screen through the one seam the page exports (window.KNOWLU_SHOTS) and
                    # shot like any other view. The settings panel is an overlay over the rendered
                    # Today, and with no backend its rows keep their static markup — the layout is
                    # what this shot is for; scripts/settings-check.py is the one that drives the
                    # rows with answers and shoots them filled in.
                    for panel, js in [("wizard", "KNOWLU_SHOTS.startWizard({tz:'America/Chicago',documents:'C:/Users/x/Documents/Knowlu',campuses:[{key:'none',label:'None'}]})"),
                                      ("picker", "KNOWLU_SHOTS.renderPicker({profiles:[{id:'p1',name:'Ada',vault:'C:/v'}]})"),
                                      ("settings", "KNOWLU_SHOTS.openSettings()")]:
                        page.evaluate(js)
                        page.wait_for_timeout(200)
                        page.screenshot(path=str(out / f"{w}-{panel}.png"), full_page=True)
                        page.reload(); page.wait_for_timeout(600)
                browser.close()
        return 1 if bad else 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
