"""T15 the Waiting deck: the DOM check (v0.4.0 plan §5 T15, M12). The driver is t10.py's.

    python scripts/ui-fixture/checks/t15.py           # check the deck against the staged t15 envelopes
    python scripts/ui-fixture/checks/t15.py --make    # rebuild envelopes/t15-*.json from the engine first

The state envelopes come from the engine (t10.make_sets): T5c's base vault plus five invented pending approvals, one of
each shape the deck draws differently: an amendment to a task, a mail card (created_by: mail, with its sender, the
mail's day and a quote carrying markup), a Gmail card (a source_uid of gmail:, and a top-level `from` the engine must
not pass on), an event card (Accept and Decline) and a commitment-ask card (answered in Details).

`decide`'s answer in t15-deck is the engine's own state after the first card is rejected through `knowlu-engine write`
on a copy of the vault, so a decision repaints the deck as the host's adopted state would. t15-refused stages a refusal
with no state, as commands.rs decide returns when the write itself fails.

Home's Waiting section is T8's and is not built on this branch, so the check gives the deck a host of its own inside
#center and hands it to KnowluSlots.deck, as home_aside.js will. Checks: one card at a time, "1 of 5", every card
reachable in order with Next and Back; Approve, Reject and Snooze (the date from state.ahead) send decide with the
verdict the engine knows and one event each; a double click sends one decide; the flag's eight toggles and free text
survive a repaint; the mail card's sender, day and quote, escaped; the Gmail card says so and names no sender; the event
card reads Accept and Decline; the commitment-ask card's Answer opens Details with the form, and Save times sends
answer_card; Details carries the decision note into decide. t15-refused: the card comes back and says why. Every
size x theme x text size without horizontal scroll or console error. Shots go to target/ui-shots/t15-*.png.
"""
import argparse
import json
import pathlib
import shutil
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import t10  # noqa: E402

mb = t10.mb
IDS = ["appr_a1b2c3d501", "appr_a1b2c3d502", "appr_a1b2c3d503", "appr_a1b2c3d504", "appr_a1b2c3d505"]
QUOTE = "Lab write-up now due Monday <b>at noon</b>"
SENDER = "Course staff <staff@school.example>"
REFUSAL = "vault lock poisoned"


def card(slug, lines, why):
    return "\n".join(["---", "type: approval"] + lines + ["---", "", why, ""])


def five_cards(vault):
    common = ["status: pending", "expires: 2026-09-10", "snooze_until: null"]
    mb.write(vault / "approvals" / "amend-abc-101-problem-set-2-due.md", card("", [
        "kind: amend", "title: " + mb.yaml_text("ABC 101 Problem set 2 moved"), "target: tasks/abc-101-problem-set-2.md",
        "proposed_at: 2026-08-24", "first_proposed_at: 2026-08-24"] + common + ["created_by: fixture",
        "changes:", "  due:", "    from: 2026-08-27T23:59", "    to: 2026-08-31T23:59", "id: " + IDS[0]],
        "**Why proposed:** the course page lists a later date."))
    mb.write(vault / "approvals" / "amend-def-210-lab-write-up-due.md", card("", [
        "kind: amend", "title: " + mb.yaml_text("Email changed due for DEF 210 Lab write-up"), "target: tasks/def-210-lab-write-up.md",
        "proposed_at: 2026-08-25", "first_proposed_at: 2026-08-25"] + common + ["created_by: mail",
        "source_uid: " + mb.yaml_text("mail:fixture:0"), "from: " + mb.yaml_text(SENDER), "mail_date: " + mb.yaml_text("2026-08-26"),
        "evidence: " + mb.yaml_text(QUOTE), "changes:", "  due:", "    from: 2026-08-29T17:00", "    to: 2026-08-31T12:00", "id: " + IDS[1]],
        "**Why proposed:** the course staff moved it."))
    mb.write(vault / "approvals" / "task-study-group.md", card("", [
        "kind: task", "title: " + mb.yaml_text("Study group"), "proposed_at: 2026-08-26", "first_proposed_at: 2026-08-26"] + common + [
        "created_by: gmail", "source_uid: " + mb.yaml_text("gmail:fixture1"), "from: " + mb.yaml_text(SENDER), "id: " + IDS[2]],
        "A study group meets Tuesday evening."))
    mb.write(vault / "approvals" / "event-career-fair.md", card("", [
        "kind: event-accept", "title: " + mb.yaml_text("Worth a look · Career fair"), "verdict: opportunity",
        "source_uid: " + mb.yaml_text("fixture:event-3"), "proposed_at: 2026-08-27", "first_proposed_at: 2026-08-27"] + common + [
        "created_by: events", "id: " + IDS[3]],
        "On campus Thursday afternoon, open to every student."))
    mb.write(vault / "approvals" / "ask-ghi-150.md", card("", [
        "kind: commitment-ask", "title: " + mb.yaml_text("When does GHI 150 meet?"), "source_uid: " + mb.yaml_text("card:ghi-150"),
        "course: ghi-150", "proposed_at: 2026-08-28", "first_proposed_at: 2026-08-28"] + common + ["created_by: agent:commitments", "id: " + IDS[4]],
        "No class times were found for GHI 150."))


def after_reject(vault):
    """The engine's today state once the first card is rejected, as the student, on a copy of the vault."""
    copy = vault.parent / (vault.name + "-after")
    shutil.copytree(vault, copy)
    t10.engine_write(copy, IDS[0], [("status", "rejected")])
    t10.pin_journal(copy, ["2026-08-28T13:30:00.000Z"])
    state = mb.envelope(mb.surface(copy, "today"))["state"]
    if any(c["id"] == IDS[0] for c in state["decisions"]["cards"]):
        sys.exit("the rejected card is still pending")
    return state


def deck_answers(vault, envs):
    if [c["id"] for c in envs["today"]["state"]["decisions"]["cards"]] != IDS:
        sys.exit("the engine did not list the five cards in order: %s" % [c["id"] for c in envs["today"]["state"]["decisions"]["cards"]])
    return {"decide": {"ok": True, "error": None, "state": after_reject(vault), "decision": {"executed": [], "expired": [], "woken": [], "rejected": [], "warnings": []}}}


def refused_answers(vault, envs):
    return {"decide": {"ok": False, "error": REFUSAL, "state": None}}


SETS = [("t15-deck", five_cards, deck_answers), ("t15-refused", five_cards, refused_answers)]

HOST = """() => {
  const c = document.getElementById('center');
  const s = document.createElement('section');
  s.className = 'page';
  const el = document.createElement('div');
  el.id = 't15-host';
  el.style.maxWidth = '520px';
  s.appendChild(el);
  c.appendChild(s);
  KnowluSlots.deck(el);
}"""


def host(page):
    page.evaluate(HOST)
    page.wait_for_selector("#t15-host [data-kind=approval]", timeout=3000)


def shown_id(page):
    return page.evaluate("() => { const c = document.querySelector('#t15-host [data-kind=approval]'); return c ? c.getAttribute('data-id') : null; }")


def host_text(page):
    return page.evaluate("() => document.getElementById('t15-host').innerText")


FIRST = """() => {
  const back = () => document.querySelector('#t15-host [data-deck-prev]');
  while (back() && !back().disabled) { back().click(); }
}"""


def go_to(page, cid):
    page.evaluate(FIRST)
    for _ in range(len(IDS)):
        if shown_id(page) == cid:
            return
        page.click("#t15-host [data-deck-next]")
        page.wait_for_timeout(40)


def decides(page):
    return [c["args"] for c in t10.calls(page, "decide")]


def check_deck(page, full, need):
    host(page)
    need(page.locator("#t15-host [data-kind=approval]").count() == 1, "one card at a time")
    need(shown_id(page) == IDS[0], "the first card is the engine's first")
    need("1 of 5" in host_text(page), "the position says 1 of 5")
    text = host_text(page)
    need("ABC 101 Problem set 2 moved" in text, "the card's title")
    need("**" not in text, "no markdown markers in the reason")
    if not full:
        return
    # every card reachable, in the engine's order, and back again
    seen = [shown_id(page)]
    for _ in range(4):
        page.click("#t15-host [data-deck-next]")
        page.wait_for_timeout(40)
        seen.append(shown_id(page))
    need(seen == IDS, "Next walks the cards in order: %s" % seen)
    need(page.locator("#t15-host [data-deck-next]").is_disabled(), "Next stops at the last card")
    page.click("#t15-host [data-deck-prev]")
    page.wait_for_timeout(40)
    need(shown_id(page) == IDS[3], "Back goes one card back")
    # the event card reads Accept and Decline and sends the same verdicts
    acts = page.evaluate("() => Array.from(document.querySelectorAll('#t15-host [data-verdict]')).map(b => [b.getAttribute('data-verdict'), b.textContent.trim()])")
    need(["approved", "Accept"] in acts and ["rejected", "Decline"] in acts, "the event card reads Accept and Decline: %s" % acts)
    # the commitment-ask card is answered in Details
    page.click("#t15-host [data-deck-next]")
    page.wait_for_timeout(40)
    need(page.locator("#t15-host [data-answer-in]").count() == 1 and page.locator("#t15-host [data-verdict=approved]").count() == 0, "the ask card offers Answer, not Approve")
    page.click("#t15-host [data-answer-in]")
    page.wait_for_selector("#panel[data-open] [data-ask-save]", timeout=3000)
    page.click('#panel [data-ask-day="mon"]')
    page.click('#panel [data-ask-day="wed"]')
    page.fill('#panel [data-part="start"]', "10:00")
    page.fill('#panel [data-part="end"]', "10:50")
    page.click("#panel [data-ask-more]")
    need(page.locator("#panel .ask-time").count() == 2, "Add another time adds a row")
    page.click("#panel [data-ask-save]")
    page.wait_for_timeout(200)
    ans = [c["args"] for c in t10.calls(page, "answer_card")]
    need(len(ans) == 1 and ans[0].get("id") == IDS[4] and ans[0].get("meets") == [{"days": ["mon", "wed"], "start": "10:00", "end": "10:50"}], "Save times sends answer_card with the times: %s" % ans)
    page.keyboard.press("Escape")
    page.wait_for_timeout(120)
    # the Gmail card says so and never names a sender
    go_to(page, IDS[2])
    text = host_text(page)
    need("from Gmail" in text, "the Gmail card says from Gmail")
    need("Course staff" not in text and "staff@" not in text, "the Gmail card names no sender")
    # the mail card: its sender, day and quote, escaped
    go_to(page, IDS[1])
    need(shown_id(page) == IDS[1], "the mail card")
    need(page.locator("#t15-host .mail-from").inner_text().find("staff@school.example") >= 0, "the mail card names its sender")
    need("2026-08-26" in page.locator("#t15-host .mail-from").inner_text() or "Aug 26" in page.locator("#t15-host .mail-from").inner_text(), "and the mail's day")
    need(page.locator("#t15-host .mail-quote q").inner_text() == QUOTE, "the quote, as text")
    need(page.locator("#t15-host .mail-quote b").count() == 0, "the quote's markup is inert")
    # the flag: eight toggles and free text, kept across a repaint of the same card
    page.click("#t15-host [data-card-more]")
    page.click("#t15-host [data-flag]")
    need(page.locator("#t15-host .wt-flag [data-cat]").count() == 8, "eight categories")
    page.click('#t15-host .wt-flag [data-cat="wrong-date"]')
    page.fill("#t15-host .wt-flag textarea", "The date is right on the syllabus")
    page.evaluate("() => KnowluData.call('sync', {})")
    page.wait_for_timeout(200)
    need(page.locator("#t15-host .wt-flag").count() == 1, "the flag survives a repaint")
    need(page.locator('#t15-host .wt-flag [data-cat="wrong-date"][aria-pressed="true"]').count() == 1, "with its toggle")
    need(page.locator("#t15-host .wt-flag textarea").input_value() == "The date is right on the syllabus", "and its text")
    page.click("#t15-host [data-flag-send]")
    page.wait_for_timeout(150)
    iss = [c["args"] for c in t10.calls(page, "open_issue")]
    need(len(iss) == 1 and iss[0].get("target") == IDS[1] and iss[0].get("categories") == ["wrong-date"], "the flag sends open_issue: %s" % iss)
    ev_iss = t10.events(page, "issue_opened")
    need(len(ev_iss) == 1 and ev_iss[0].get("objectKind") == "approval", "issue_opened names the approval")
    # Details holds the note; the panel's Approve carries it
    page.click("#t15-host [data-card]")
    page.wait_for_selector("#panel[data-open] [data-card-note]", timeout=3000)
    need("Course staff" in page.evaluate("() => document.getElementById('panel').innerText"), "Details shows the mail line too")
    page.fill("#panel [data-card-note]", "Checked with the course page")
    page.click("#panel [data-verdict=approved]")
    page.wait_for_timeout(250)
    d = decides(page)
    need(len(d) == 1 and d[0].get("id") == IDS[1] and d[0].get("verdict") == "approved" and d[0].get("note") == "Checked with the course page" and d[0].get("snoozeUntil") is None, "Details' Approve sends the note: %s" % d)
    # after the adopted state: the first card is gone, and Reject and Snooze still act
    page.keyboard.press("Escape")
    page.wait_for_timeout(120)
    page.evaluate(FIRST)
    need("1 of 4" in host_text(page), "the decided card has left: %s" % host_text(page)[:80])
    need(shown_id(page) == IDS[1], "the deck shows the engine's next card")
    page.click("#t15-host [data-verdict=rejected]")
    page.wait_for_timeout(200)
    d = decides(page)
    need(len(d) == 2 and d[1].get("verdict") == "rejected" and d[1].get("note") == "", "Reject sends rejected: %s" % d[1:])
    # a double click sends one decide
    page.evaluate("() => { const b = document.querySelector('#t15-host [data-verdict=approved]'); b.click(); b.click(); }")
    page.wait_for_timeout(200)
    need(len(decides(page)) == 3, "a double click sends one decide (%d)" % len(decides(page)))
    # the deck and its Details at once send one decide (wave-A review M1)
    page.click("#t15-host .wt-nm")
    page.wait_for_selector("#panel[data-open] [data-card-note]", timeout=3000)
    page.evaluate("() => { document.querySelector('#t15-host [data-verdict=approved]').click(); document.querySelector('#panel [data-verdict=rejected]').click(); }")
    page.wait_for_timeout(200)
    d = decides(page)
    need(len(d) == 4 and d[3].get("verdict") == "approved", "the deck and Details together send one decide: %s" % d[3:])
    need(page.locator("#panel[data-open]").count() == 0, "a decided card's Details closes")
    # Snooze, from the card's overflow, defaults to the engine's tomorrow
    tomorrow = t10.envelope_of("t15-deck", "today")["ahead"]["buckets"][1]["date"]
    if page.locator("#t15-host .wt-menu").is_hidden():  # an open overflow stays open across a repaint of its card
        page.click("#t15-host [data-card-more]")
    need(page.locator("#t15-host [data-snooze-date]").input_value() == tomorrow, "the snooze date is the engine's tomorrow")
    page.click("#t15-host [data-verdict=snoozed]")
    page.wait_for_timeout(200)
    d = decides(page)
    need(len(d) == 5 and d[4].get("verdict") == "snoozed" and d[4].get("snoozeUntil") == tomorrow, "Snooze sends snoozed with the date: %s" % d[4:])
    made = t10.events(page, "decision_made")
    deferred = t10.events(page, "decision_deferred")
    need(len(made) == 5 and len(deferred) == 1, "one event per decision (%d made, %d deferred)" % (len(made), len(deferred)))
    need(all(e.get("objectKind") == "approval" and e.get("objectId") in IDS for e in made + deferred), "each names the approval by id")
    page.evaluate("() => { document.querySelectorAll('#t15-host .wt-card.gone').forEach(c => c.classList.remove('gone')); }")


def check_refused(page, full, need):
    host(page)
    page.click("#t15-host [data-verdict=approved]")
    page.wait_for_timeout(250)
    card_ = page.locator("#t15-host [data-kind=approval]")
    need(card_.count() == 1 and shown_id(page) == IDS[0], "the refused card is still there")
    need(not page.evaluate("() => { const c = document.querySelector('#t15-host [data-kind=approval]'); return !!c.dataset.busy || c.classList.contains('gone'); }"), "and no longer busy")
    need(not page.locator("#t15-host [data-verdict=approved]").is_disabled(), "its buttons work again")
    toast = page.evaluate("() => document.getElementById('toasts').innerText")
    need(REFUSAL in toast, "the refusal is said: %r" % toast)
    need(t10.events(page, "decision_made") == [], "a refused decision sends no event")


CASES = [
    {"name": "deck", "set": "t15-deck", "route": "home", "check": check_deck},
    {"name": "refused", "set": "t15-refused", "route": "home", "check": check_refused, "combos": t10.FULL},
]


def main():
    ap = argparse.ArgumentParser(description="T15's DOM check (the Waiting deck) in headless Edge.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t15-*.json from the engine first")
    args = ap.parse_args()
    if args.make:
        t10.make_sets(SETS)
    t10.run("t15", CASES)


if __name__ == "__main__":
    main()
