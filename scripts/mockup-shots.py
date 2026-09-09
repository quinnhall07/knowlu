"""Render an artifact-style mockup at many viewports and report overflow.

Design work on the S2 surface must be *looked at*, not reasoned about — three
separate layout bugs in the 2026-08-31 session were invisible in the source and
obvious in a screenshot (an 800px void between task titles and their hours, a
breakpoint that detached every rail heading from its content, and a CSS class
collision that deleted run-status labels from the grid).

Usage
-----
    <playwright-python> scripts/mockup-shots.py docs/mockups/<file>.html out/

Playwright is NOT a project dependency and must never become one — the engine
stays stdlib + PyYAML + tzdata (see CLAUDE.md). Use a throwaway venv:

    python -m venv .wv && .wv\\Scripts\\python -m pip install playwright
    .wv\\Scripts\\python -m playwright install chromium
    .wv\\Scripts\\python scripts/mockup-shots.py docs/mockups/x.html shots/

The mockups are authored as artifact bodies (no <!doctype>/<html>/<head>/<body>),
because that is how claude.ai/code publishes them. This script wraps the file in
the same skeleton the artifact host applies, so what you screenshot matches what
is published. Do not "fix" a mockup by adding a doctype.
"""

import pathlib
import sys

from playwright.sync_api import sync_playwright

# The artifact host's skeleton: charset + viewport meta and a small reset.
SKELETON_HEAD = """<!doctype html><html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>:root{color-scheme:light}body{margin:0;font:14px system-ui,-apple-system,sans-serif;
background:#fafaf9}img{max-width:100%}[hidden]{display:none!important}</style>
</head><body>"""

# Desktop, laptop, both breakpoint edges, small laptop, tablet, phone.
VIEWPORTS = [
    (1920, 1080),
    (1512, 945),
    (1280, 800),
    (1180, 820),
    (1024, 768),
    (900, 900),
    (820, 900),
    (390, 844),
]


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2

    src = pathlib.Path(sys.argv[1]).resolve()
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)

    wrapped = src.with_name(src.stem + ".wrapped.html")
    wrapped.write_text(
        SKELETON_HEAD + src.read_text(encoding="utf-8") + "</body></html>",
        encoding="utf-8",
    )

    failures = 0
    with sync_playwright() as p:
        browser = p.chromium.launch()
        for width, height in VIEWPORTS:
            ctx = browser.new_context(
                viewport={"width": width, "height": height},
                color_scheme="dark",
                device_scale_factor=1,
            )
            page = ctx.new_page()
            page.goto(wrapped.as_uri())
            page.wait_for_timeout(900)  # let webfonts settle before measuring

            metrics = page.evaluate(
                """() => ({
                    scrollWidth: document.documentElement.scrollWidth,
                    innerWidth: window.innerWidth,
                    pageHeight: document.body.scrollHeight,
                    overflowing: [...document.querySelectorAll('*')]
                        .filter(el => el.getBoundingClientRect().right > window.innerWidth + 1)
                        .slice(0, 6)
                        .map(el => el.tagName.toLowerCase() + '.' +
                             (el.className || '').toString().split(' ')[0])
                })"""
            )

            overflows = metrics["scrollWidth"] > metrics["innerWidth"] + 1
            if overflows:
                failures += 1
            print(
                f"{width}x{height}: scrollW={metrics['scrollWidth']} "
                f"innerW={metrics['innerWidth']} pageH={metrics['pageHeight']}"
                f"{'  <-- HORIZONTAL SCROLL' if overflows else ''}"
            )
            if metrics["overflowing"]:
                print("     overflowing:", "; ".join(metrics["overflowing"]))

            page.screenshot(path=str(out / f"{width}.png"), full_page=True)
            ctx.close()
        browser.close()

    wrapped.unlink(missing_ok=True)
    print(f"\nwrote {len(VIEWPORTS)} screenshots to {out}")
    if failures:
        print(f"{failures} viewport(s) scroll horizontally — the page body never should")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
