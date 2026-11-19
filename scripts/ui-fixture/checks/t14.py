"""T14 Details: the DOM check (v0.4.0 plan §5 T14, M12). The driver is t10.py's.

    python scripts/ui-fixture/checks/t14.py           # check Details against the staged t14 envelopes
    python scripts/ui-fixture/checks/t14.py --make    # rebuild envelopes/t14-*.json from the engine first

The state envelopes come from the engine (t10.make_sets): T5c's base vault plus an invented task too big for its
deadline ("Won't fit"), a pending amendment on it (its row's amend badge) and one progress write through
`knowlu-engine write` (its history), its journal stamp moved onto the pinned week.

The `note` answer is staged in <set>-answers.json, since the engine has no command line for note_detail. It is
put together the way app/src/commands.rs note_inner and engine/src/surface.rs note_detail build it, and from
nothing else: `row` is the engine's own Row for that id from the all envelope; `frontmatter` and `body` are read
back from the note file the engine last wrote; `history` is the engine's own records for that id from the today
today view's delta when `surface` is asked with --seen-at before the record (newest first). t14-conflict also stages set_body's refusal as commands.rs
set_body_inner returns it for the engine's Conflict variant: {ok: false, error: <WriteError::Conflict's words>,
state, conflict: true}.

Checks, on #tasks, with Details opened from the row's name: the head and the rows (Class, Needs, Due, Start by,
Slack, Won't fit); every size x theme x text size without horizontal scroll or console error. At 1280x800 light and
820x600 dark larger: Escape cancels an edit (edit_cancelled) and leaves the panel open; out-of-range numbers are
refused on the page with the bound that exists; progress 100 writes status done in the same set_fields; More shows
the twelve task fields (never kind or level); a poll leaves an open body editor and its draft alone; Ctrl+Enter
saves with expected; Saved and Undo, whose expected is the re-read body; no ui_event carries the typed text; the
flag's eight toggles and free text; Escape closes the panel and its Saved toast. t14-conflict: the draft is kept
and the conflict line shows. Shots go to target/ui-shots/t14-*.png. Exit 1 on any failure.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import t10  # noqa: E402

mb = t10.mb
TID = "task_a1b2c3d409"
STEM = "jkl-120-portfolio"
CONFLICT = "tasks/%s.md changed since it was read; nothing was written" % STEM
TYPED = "Outline the three essays first"


def portfolio(vault):
    mb.write(vault / "tasks" / (STEM + ".md"), mb.task_note(TID, "JKL 120 Portfolio", "jkl-120", "2026-08-29T23:59", 40.0, 4, "active", 0))
    mb.write(vault / "approvals" / "amend-jkl-120-portfolio-due.md", "\n".join([
        "---", "type: approval", "kind: amend", "title: " + mb.yaml_text("JKL 120 Portfolio moved"), "status: pending",
        "target: tasks/%s.md" % STEM, "proposed_at: 2026-08-27", "first_proposed_at: 2026-08-27", "expires: 2026-09-10",
        "snooze_until: null", "created_by: fixture", "changes:", "  due:", "    from: 2026-08-29T23:59", "    to: 2026-09-01T23:59",
        "id: appr_f1x7ure141", "---", "", "**Why proposed:** the course page lists a later date.", "",
    ]))
    t10.engine_write(vault, TID, [("progress", "10")])
    t10.pin_journal(vault, ["2026-08-27T16:00:00.000Z"])


def scalar(text):
    """One frontmatter value as the engine's YAML reader gives it to JSON (the forms task_note writes)."""
    if text == "null":
        return None
    if text.startswith('"'):
        return json.loads(text)
    if re.fullmatch(r"-?\d+", text):
        return int(text)
    if re.fullmatch(r"-?\d+\.\d+", text):
        return float(text)
    return text


def read_note(vault):
    text = (vault / "tasks" / (STEM + ".md")).read_text(encoding="utf-8")
    _, front, body = text.split("---\n", 2)
    fm = {}
    for line in front.splitlines():
        key, _, value = line.partition(": ")
        fm[key] = scalar(value.strip())
    return fm, body


def delta_records(vault):
    """The engine's own DeltaRecords: `surface --seen-at` before the journal's first record (read only)."""
    out = subprocess.run([str(mb.ENGINE), "surface", "--vault", str(vault), "--view", "today", "--seen-at", "2026-08-26T00:00:00.000Z"] + mb.PINS, capture_output=True)
    if out.returncode != 0:
        sys.exit("surface --seen-at failed: %s" % out.stderr.decode("utf-8", "replace"))
    return (json.loads(out.stdout.decode("utf-8")).get("delta") or {}).get("records") or []


def note_answer(vault, envs):
    fm, body = read_note(vault)
    row = next(r for r in envs["all"]["state"]["list"]["rows"] if r["id"] == TID)
    history = [r for r in reversed(delta_records(vault)) if r.get("id") == TID]
    if not history:
        sys.exit("the engine gave no record for %s's history" % TID)
    note = {"id": TID, "path": "tasks/%s.md" % STEM, "slug": STEM, "folder": "tasks", "frontmatter": fm, "body": body,
            "judgment": fm.get("judgment"), "history": history, "row": row}
    return {"ok": True, "error": None, "note": note}


def details_answers(vault, envs):
    return {"note": note_answer(vault, envs)}


def conflict_answers(vault, envs):
    return {"note": note_answer(vault, envs), "set_body": {"ok": False, "error": CONFLICT, "state": envs["all"]["state"], "conflict": True}}


SETS = [("t14-details", portfolio, details_answers), ("t14-conflict", portfolio, conflict_answers)]
TASK_FIELDS = ["title", "course", "due", "effort_hours", "importance", "importance_reason", "status", "progress", "slice_hours", "domain", "rank_override", "effort_confidence"]


def open_details(page):
    page.wait_for_selector('.tk-row[data-id="%s"] .tk-nm' % TID, timeout=5000)
    page.click('.tk-row[data-id="%s"] .tk-nm' % TID)
    page.wait_for_selector("#panel[data-open] .dt", timeout=5000)
    page.wait_for_timeout(120)


def panel_text(page):
    return page.evaluate("() => document.getElementById('panel').innerText")


def toast_text(page):
    return page.evaluate("() => document.getElementById('toasts').innerText")


def edit(page, field, value, key="Enter"):
    page.click('#panel [data-field="%s"]' % field)
    page.wait_for_selector("#panel .dt-in", timeout=2000)
    if value is not None:
        page.fill("#panel .dt-in", value)
    page.press("#panel .dt-in", key)
    page.wait_for_timeout(150)


def check_details(page, full, need):
    open_details(page)
    st = t10.envelope_of("t14-details", "all")
    row = next(r for r in st["list"]["rows"] if r["id"] == TID)
    need(page.evaluate("() => document.getElementById('panel-h').textContent") == "JKL 120 Portfolio", "the panel's title")
    need(page.locator("#panel-ck .ck").count() == 1, "the task's own check beside the title")
    text = panel_text(page)
    for label in ["Class", "Needs", "Due", "Start by", "Slack", "Won't fit", "Progress", "Estimate", "Notes", "History"]:
        need(label in text, "Details shows %s" % label)
    need(row.get("fit_text") and row["fit_text"] in text, "the engine's Won't fit line: %r" % row.get("fit_text"))
    need(row.get("fit_why") and row["fit_why"] in text, "and its reason")
    need(row.get("amend_badge") and row["amend_badge"]["sentence"] in text, "the pending change's sentence")
    need(page.locator('#panel [data-field="kind"], #panel [data-field="level"]').count() == 0, "no kind or level on a task")
    if not full:
        return
    # Escape cancels an edit and leaves the panel open.
    edit(page, "due", None, "Escape")
    need(page.locator("#panel[data-open]").count() == 1, "Escape in an edit leaves the panel open")
    need(page.locator("#panel .dt-in").count() == 0, "Escape cancels the edit")
    ev = t10.events(page, "edit_cancelled")
    need(ev and ev[-1]["objectId"] == TID and ev[-1]["objectKind"] == "due", "edit_cancelled for due: %s" % ev[-1:])
    need(t10.events(page, "edit_started"), "edit_started was sent")
    # More: the twelve task fields, and the id read-only.
    page.click("#panel [data-dt-more]")
    page.wait_for_timeout(80)
    fields = page.evaluate("() => Array.from(document.querySelectorAll('#panel [data-field]')).map(b => b.getAttribute('data-field'))")
    need(sorted(set(fields)) == sorted(TASK_FIELDS), "More shows the twelve task fields: %s" % sorted(set(fields)))
    need(page.locator('#panel [data-ro="id"]').count() == 1, "the id is shown read-only")
    # Out of range: refused on the page, with the bound that exists.
    before = len(t10.calls(page, "set_fields"))
    edit(page, "importance", "9")
    need("from 1 to 5" in toast_text(page), "importance 9 is refused with its range: %r" % toast_text(page))
    edit(page, "effort_hours", "-1")
    need("at least 0" in toast_text(page) and "null" not in toast_text(page), "a negative estimate names the one bound: %r" % toast_text(page))
    need(len(t10.calls(page, "set_fields")) == before, "nothing is written for a refused number")
    # Progress 100 closes the task in the same write.
    edit(page, "progress", "100")
    sent = t10.calls(page, "set_fields")
    need(sent and sent[-1]["args"] == {"id": TID, "fields": {"progress": 100, "status": "done"}, "view": "all"}, "progress 100: %s" % sent[-1:])
    ok = t10.events(page, "edit_committed")
    need(ok and ok[-1]["objectKind"] == "progress", "edit_committed for progress")
    page.wait_for_timeout(200)
    need(page.locator("#panel .dt-rest").is_visible(), "a redraw keeps More open")
    # The body: a poll never rebuilds an open editor; Ctrl+Enter saves with expected; Saved and Undo.
    note = json.loads((t10.OUT / "t14-details-answers.json").read_text(encoding="utf-8"))["note"]["note"]
    page.click("#panel [data-body-edit]")
    page.wait_for_selector("#panel textarea[data-body-ta]", timeout=2000)
    need(page.input_value("#panel textarea[data-body-ta]") == note["body"], "the editor opens on the note's body")
    page.fill("#panel textarea[data-body-ta]", TYPED)
    # a write's answer repaints every subscriber, as a poll with a new revision does (the staged envelope never changes)
    page.evaluate("() => KnowluData.call('sync', {})")
    page.wait_for_timeout(300)
    need(page.locator("#panel textarea[data-body-ta]").count() == 1 and page.input_value("#panel textarea[data-body-ta]") == TYPED, "a poll left the editor and its draft alone")
    page.press("#panel textarea[data-body-ta]", "Control+Enter")
    page.wait_for_timeout(300)
    saved = t10.calls(page, "set_body")
    need(saved and saved[-1]["args"] == {"id": TID, "expected": note["body"], "body": TYPED, "view": "all"}, "set_body with expected: %s" % saved[-1:])
    page.wait_for_selector("#toasts .savedtoast [data-body-undo]", timeout=3000)
    page.click("#toasts .savedtoast [data-body-undo]")
    page.wait_for_timeout(300)
    undo = t10.calls(page, "set_body")
    need(len(undo) == 2 and undo[-1]["args"] == {"id": TID, "expected": note["body"], "body": note["body"], "view": "all"}, "Undo's expected is the re-read body: %s" % undo[-1:])
    for a in t10.events(page):
        need(TYPED not in json.dumps(a), "a ui_event carries the typed text: %s" % a)
    need(any(a["objectKind"] == "body" and a["action"] == "edit_committed" for a in t10.events(page)), "edit_committed for the body")
    # The flag: eight toggles and free text; Flag waits for one.
    page.click("#panel [data-flag]")
    page.wait_for_selector("#panel .dt-flag [data-cat]", timeout=2000)
    need(page.locator("#panel .dt-flag [data-cat]").count() == 8, "eight categories")
    need(page.locator("#panel [data-flag-send]").is_disabled(), "Flag waits for a category")
    page.click('#panel .dt-flag [data-cat="wrong-date"]')
    page.fill("#panel .dt-flag textarea", "The date is a week off")
    page.click("#panel [data-flag-send]")
    page.wait_for_timeout(200)
    issue = t10.calls(page, "open_issue")
    need(issue and issue[-1]["args"] == {"target": TID, "categories": ["wrong-date"], "text": "The date is a week off", "view": "all"}, "open_issue: %s" % issue[-1:])
    need(any(a["action"] == "issue_opened" and a["objectId"] == TID for a in t10.events(page)), "issue_opened")
    # Saved and Undo again, then Escape closes the panel and the toast with it.
    page.click("#panel [data-body-edit]")
    page.fill("#panel textarea[data-body-ta]", TYPED)
    page.click("#panel [data-body-save]")
    page.wait_for_selector("#toasts .savedtoast", timeout=3000)
    page.keyboard.press("Escape")
    page.wait_for_timeout(150)
    need(page.locator("#panel[data-open]").count() == 0, "Escape closes the panel")
    need(page.locator("#toasts .savedtoast").count() == 0, "the Saved toast dies with its panel")
    open_details(page)


def check_conflict(page, full, need):
    open_details(page)
    page.click("#panel [data-body-edit]")
    page.wait_for_selector("#panel textarea[data-body-ta]", timeout=2000)
    page.fill("#panel textarea[data-body-ta]", TYPED)
    page.click("#panel [data-body-save]")
    page.wait_for_selector("#panel [data-body-conflict]", timeout=3000)
    need(page.input_value("#panel textarea[data-body-ta]") == TYPED, "the draft is kept")
    need("Your text is still here" in panel_text(page), "the conflict line")
    need(page.locator("#panel [data-body-copy]").count() == 1 and page.locator("#panel [data-body-reload]").count() == 1, "Copy and Reload")
    page.click("#panel [data-body-copy]")
    page.wait_for_timeout(100)
    copied = t10.calls(page, "copy_text")
    need(copied and copied[-1]["args"] == {"text": TYPED}, "Copy sends the draft: %s" % copied[-1:])


CASES = [
    {"name": "details", "set": "t14-details", "route": "tasks", "check": check_details},
    {"name": "conflict", "set": "t14-conflict", "route": "tasks", "check": check_conflict, "combos": t10.FULL},
]


def main():
    ap = argparse.ArgumentParser(description="T14's DOM check (Details) in headless Edge.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t14-*.json from the engine first")
    args = ap.parse_args()
    if args.make:
        t10.make_sets(SETS)
    t10.run("t14", CASES)


if __name__ == "__main__":
    main()
