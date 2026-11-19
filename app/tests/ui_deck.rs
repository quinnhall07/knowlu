//! T15, the Waiting deck (v0.4.0 note §4 "Waiting: cards", §5 "Decisions deck", plan §5 T15): pins on
//! `js/carried/deck.js` and `css/deck.css`. The deck group's re-homed tests keep their names (plan §7) and read the
//! new file; the deck is now one card at a time inside Home's Waiting (handed to `KnowluSlots.deck` by T8), with
//! snooze and the flag in the card's overflow and the note and the Answer form in Details (the `card` panel). Red
//! first: against T5's placeholder every test here fails.

#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

fn deck_js() -> String { strip_comments(&read("js/carried/deck.js")) }
fn deck_css() -> String { strip_css_comments(&read("css/deck.css")) }

/// Every `ev(` line of `js`, trimmed.
fn ev_lines(js: &str) -> Vec<&str> { js.lines().filter(|l| is_ev_call(l)).map(str::trim).collect() }

/// Re-homed: the deck decides (approve, reject, snooze), and its card's flag is eight categories as toggles with free
/// text beside them, never chips; each act sends its one event.
#[test]
fn the_deck_acts_and_the_flag_has_eight_chips_and_free_text() {
    let js = deck_js();
    for f in ["renderDeck", "decideCard", "openFlag", "sendFlag", "cardHtml"] {
        assert!(js.contains(&format!("function {f}(")), "{f}");
    }
    assert!(js.contains("KnowluSlots.addDeck(renderDeck);"), "the deck registers its renderer for Waiting (P3)");
    for cmd in ["\"decide\"", "\"open_issue\""] {
        assert!(js.contains(cmd), "{cmd}");
    }
    for cat in ["wrong-effort", "wrong-course", "duplicate", "should-not-exist", "wrong-tier", "wrong-date", "wrong-verdict", "other"] {
        assert!(js.contains(&format!("\"{cat}\"")), "{cat}");
    }
    assert!(js.contains("<textarea"), "free text, always, beside the toggles");
    assert!(!js.contains("class=\"chip") && !js.contains("chips"), "toggles, not chips (ruling: no chips)");
    assert!(!js.contains("disabled>Approve") && !js.contains("disabled>Reject"), "the deck's buttons are live");
    let flag = fn_body(&js, "function sendFlag(");
    assert!(flag.contains("KnowluData.call(\"open_issue\", { target: id, categories: cats, text: ta.value })"), "open_issue: {flag}");
    assert!(flag.contains("ev(\"issue_opened\", id, \"approval\");"), "issue_opened names the approval");
    let decide = fn_body(&js, "function decideCard(");
    assert!(decide.contains("ev(verdict === \"snoozed\" ? \"decision_deferred\" : \"decision_made\", id, \"approval\");"), "one event per decision: {decide}");
    let css = deck_css();
    assert!(css.contains(".wt-card.gone"), "a card in flight is marked gone");
}

/// Re-homed: a fast double click never sends two decides: decideCard refuses a busy card and the card's own click
/// handler ignores one; a refusal with no state gives the card back. An open flag on a card survives a repaint of the
/// same card (the same node, so its toggles and its text), and is dropped when another card takes the slot.
#[test]
fn the_deck_guards_double_decide_and_flagpops_survive_a_repaint() {
    let js = deck_js();
    let decide = fn_body(&js, "function decideCard(");
    assert!(decide.contains("if (card && card.dataset.busy) { return; }"), "decideCard refuses a busy card: {decide}");
    // Wave-A review M1: a decision in flight from the deck blocks one from its Details, and the other way round.
    let first = decide.find("if (BUSY[id]) { return; }").expect("decideCard checks BUSY[id]");
    assert!(first < decide.find("KnowluData.call(\"decide\"").unwrap(), "before any call");
    assert!(fn_body(&js, "function answerAsk(").contains("BUSY[id]"), "the answer form honours the same guard");
    assert!(decide.contains("card.dataset.busy = \"1\";") && decide.contains("card.classList.add(\"gone\");"), "the card is marked busy and gone before the call");
    assert!(decide.contains("delete card.dataset.busy;") && decide.contains("card.classList.remove(\"gone\");"), "a refusal gives the card back");
    let bind = fn_body(&js, "function bindHost(");
    assert!(bind.contains("if (card && card.dataset.busy) { return; }"), "the click handler ignores a busy card: {bind}");
    assert!(js.contains("function preserveAcross("), "preserveAcross");
    let render = fn_body(&js, "function drawHost(");
    assert!(render.contains("preserveAcross(el, id, function () {"), "the redraw keeps the flag of the same card: {render}");
    let keep = fn_body(&js, "function preserveAcross(");
    assert!(keep.contains(".wt-flag") && keep.contains("getAttribute(\"data-id\") === id"), "only onto the same card: {keep}");
    let css = deck_css();
    assert!(css.contains(".wt-card.gone { pointer-events: none;"), "a card in flight is not hit-testable");
    assert!(css.contains("prefers-reduced-motion") || read("css/shell.css").contains(".no-motion"), "motion follows the setting");
}

/// Re-homed: one click commits a decision; nothing relabels to "Confirm?" and the deck asks no confirm() (a decision
/// is undone in the vault, not guarded on the page). The note and the snooze date are read when the click lands.
#[test]
fn the_page_can_write_and_never_relabels_to_confirm() {
    let js = deck_js();
    assert!(!js.contains("Confirm?") && !js.contains("Are you sure") && !js.contains("window.confirm("), "one click commits");
    let decide = fn_body(&js, "function decideCard(");
    assert!(decide.contains("KnowluData.call(\"decide\", { id: id, verdict: verdict, note: note, snoozeUntil: snoozeUntil || null })"), "decide's arguments: {decide}");
    assert!(decide.contains("var note = own(NOTES, id) ? NOTES[id] : \"\";"), "the note typed in Details rides with the decision");
    assert!(decide.contains("KnowluKit.refusal(env)"), "a refusal is said");
}

/// Re-homed (note §4: one card at a time, every pending card reachable in order). The deck draws `d.cards` whole, in
/// the engine's order, with no urgency filter; Back and Next step through it and the position says "N of M"; each card
/// is observed as an approval and sends approved, rejected or snoozed.
#[test]
fn the_decisions_view_lists_every_pending_approval_and_acts_like_the_deck() {
    let js = deck_js();
    assert!(js.contains("function decideCard(id, verdict, snoozeUntil, host)"), "decideCard is host-aware");
    let render = fn_body(&js, "function drawHost(");
    assert!(render.contains("var cards = cardsOf(st);"), "the engine's cards: {render}");
    assert!(!render.contains("urgency !==") && !render.contains("urgency ==="), "no urgency filter");
    assert!(!js.contains(".slice(0, 1)"), "every card is reachable, not only the first");
    let nav = fn_body(&js, "function navHtml(");
    assert!(nav.contains("data-deck-prev") && nav.contains("data-deck-next") && nav.contains("\" of \""), "Back, Next and the position: {nav}");
    let card = fn_body(&js, "function cardHtml(");
    assert!(card.contains("data-kind=\"approval\""), "a card is observed as an approval");
    assert!(card.contains("data-verdict=\"approved\"") && card.contains("data-verdict=\"rejected\""), "approved and rejected");
    assert!(js.contains("data-verdict=\"snoozed\""), "snoozed, from the overflow");
    assert!(js.contains("st.ahead.buckets[1].date"), "the snooze date is the engine's tomorrow, never computed here");
}

/// Re-homed: an event card reads Accept and Decline and sends the same verdicts; every other card keeps Approve and
/// Reject; a commitment-ask card offers Answer.
#[test]
fn event_cards_read_accept_and_decline_and_send_the_same_verdicts() {
    let js = deck_js();
    let l = fn_body(&js, "function verdictLabels(");
    assert!(l.contains("event-accept") && l.contains("event-check") && l.contains("Accept") && l.contains("Decline"), "one switch names both kinds");
    assert!(l.contains("Approve") && l.contains("Reject"), "other kinds keep Approve and Reject");
    for f in ["function cardHtml(", "function cardPanelHtml("] {
        let b = fn_body(&js, f);
        assert!(b.contains("verdictLabels(c.kind)"), "{f} reads the switch");
        assert!(b.contains("data-verdict=\"rejected\""), "{f} still sends rejected");
        assert!(!b.contains(">Approve<") && !b.contains(">Reject<"), "{f} has no fixed label");
    }
    assert!(js.contains("data-answer-in>Answer&hellip;"), "commitment-ask keeps Answer");
}

/// Re-homed: a card from Gmail says so (its source_uid), and never names a sender: only a mail card carries one, and
/// the page reads `from`, `mail_date` and `evidence` in `mailCardLineHtml` and nowhere else. (The old
/// page's `sourceLabel` and `mailCardLine` carry the `Html` suffix the shell's markup scan trusts.)
#[test]
fn a_gmail_card_shows_its_gmail_attribution_and_never_a_sender() {
    let js = deck_js();
    assert!(js.contains("function sourceLabelHtml(c)"), "one helper names a card's source");
    assert!(js.contains("from Gmail"), "the label's words");
    assert!(fn_body(&js, "function cardHtml(").contains("sourceLabelHtml(c)"), "the card carries the label");
    assert!(fn_body(&js, "function cardPanelHtml(").contains("sourceLabelHtml(c)"), "so does Details");
    assert!(!js.contains("c.sender"), "never the sender");
    let label = fn_body(&js, "function sourceLabelHtml(c)");
    assert!(!label.contains("c.from") && !label.contains("mail_date") && !label.contains("evidence"), "the Gmail label never names a sender");
    let line = fn_body(&js, "function mailCardLineHtml(c)");
    assert!(line.contains("if (!c.from && !c.mail_date && !c.evidence) { return \"\"; }"), "no keys, no line: {line}");
    for key in ["c.from", "c.mail_date", "c.evidence"] {
        assert!(line.contains(key), "mailCardLineHtml reads {key}");
        assert_eq!(js.matches(key).count(), line.matches(key).count(), "{key} is read outside mailCardLineHtml");
    }
}

/// Re-homed: a mail card names its sender and the mail's day and quotes its evidence, on the card and in Details.
#[test]
fn a_mail_card_shows_its_sender_date_and_quote() {
    let js = deck_js();
    let f = fn_body(&js, "function mailCardLineHtml(");
    assert!(f.contains("c.from") && f.contains("c.mail_date") && f.contains("c.evidence"), "the three read-model fields");
    assert!(f.contains("class=\"mail-from\"") && f.contains("class=\"mail-quote\""), "its two lines");
    assert!(f.contains("<q>"), "the quote reads as a quote");
    let css = deck_css();
    assert!(css.contains(".mail-from") && css.contains(".mail-quote"), "styled");
    assert!(fn_body(&js, "function cardHtml(").contains("mailCardLineHtml(c)"), "the card");
    assert!(fn_body(&js, "function cardPanelHtml(").contains("mailCardLineHtml(c)"), "and Details");
}

/// Re-homed, the ask-form half (the confirm screen is T18's): a commitment-ask card is answered in Details with day
/// toggles, start and end times, "Add another time", Save times (answer_card), No set times (rejected) and Snooze.
#[test]
fn the_confirm_screen_and_the_ask_form_are_there() {
    let js = deck_js();
    for f in ["askFormHtml", "askTimeHtml", "answerAsk"] {
        assert!(js.contains(&format!("function {f}(")), "missing function {f}");
    }
    let ask = fn_body(&js, "function askFormHtml(");
    for piece in ["data-ask-save", "Save times", "No set times", "data-verdict=\"rejected\"", "data-verdict=\"snoozed\"", "data-ask-more", "Add another time"] {
        assert!(ask.contains(piece), "the ask form lacks {piece}");
    }
    let row = fn_body(&js, "function askTimeHtml(");
    assert!(row.contains("data-ask-day") && row.contains("type=\"time\""), "day toggles and two time pickers: {row}");
    assert!(!row.contains("type=\"text\""), "nothing typed");
    assert!(js.contains("var DAYS = [\"mon\", \"tue\", \"wed\", \"thu\", \"fri\", \"sat\", \"sun\"];"), "the engine's day words");
    assert!(fn_body(&js, "function answerAsk(").contains("KnowluData.call(\"answer_card\", { id: id, meets: meets })"), "answer_card");
    assert!(js.contains("KnowluPanel.open(\"card\", "), "Answer opens Details");
    assert!(js.contains("KnowluPanel.add(\"card\", {"), "the card panel registers");
}

/// N1: the deck draws `S.decisions.cards` only. Every pending amendment is already a card, so neither
/// `must_do.amendments` nor `hidden_amendments` is read.
#[test]
fn cards_are_decisions_cards_only() {
    let js = deck_js();
    let cards = fn_body(&js, "function cardsOf(");
    assert!(cards.contains("st.decisions.cards"), "the cards: {cards}");
    for bad in ["amendments", "must_do", "hidden_"] {
        assert!(!js.contains(bad), "the deck reads {bad}");
    }
}

/// Plan T15: the mail card's three fields reach the markup only through h(), and the helper only builds a string.
#[test]
fn mail_cards_go_through_h() {
    let js = deck_js();
    let f = fn_body(&js, "function mailCardLineHtml(");
    for part in ["h(c.from)", "h(c.mail_date)", "h(c.evidence)"] {
        assert!(f.contains(part), "{part}");
    }
    assert!(!f.contains("innerHTML") && !f.contains("insertAdjacentHTML") && !f.contains("textContent"), "the helper builds a string, never writes the DOM");
}

/// R2-1 positive half of `mail_derived_text_renders_inert`: the card line and the deck's card use h() for every
/// card-derived word: its title and its reason, as well as the mail fields.
#[test]
fn mail_card_line_and_the_deck_use_h() {
    let js = deck_js();
    let card = fn_body(&js, "function cardHtml(");
    assert!(card.contains("h(c.title)") && card.contains("h(c.why || c.kind)"), "title and why through h(): {card}");
    let panel = fn_body(&js, "function cardPanelHtml(");
    assert!(panel.contains("h(c.why || c.kind)"), "Details' reason through h()");
    assert!(fn_body(&js, "function drawHost(").contains("cardHtml("), "the deck draws the card through cardHtml");
}

/// R2-1 positive half of `no_mail_field_reaches_the_dom_without_h` (the shell's scan has no minimum): the deck's
/// reads are found and every one is safe, and a card's title or reason is read only inside h() or as a test.
#[test]
fn mail_fields_checked_in_the_deck() {
    let js = deck_js();
    let mut checked = 0;
    for (n, line, upto) in field_reads(&js, &MAIL_FIELDS) {
        checked += 1;
        assert!(read_is_safe(line, &upto), "line {n}: a mail field outside h(): {}", line.trim());
    }
    assert!(checked >= 3, "the scan found the card line's reads ({checked})");
    for (n, line, upto) in field_reads(&js, &["title", "why"]) {
        let var = upto.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).next().unwrap_or("");
        if !var.starts_with("c.") { continue; }
        assert!(read_is_safe(line, &upto), "line {n}: {var} not inside h(): {}", line.trim());
    }
}

/// R2-1: the card line is markup only and no usage event the deck sends carries a mail field, a card's words, the
/// note or the flag's text.
#[test]
fn the_card_line_sends_no_mail_text() {
    let js = deck_js();
    assert!(!fn_body(&js, "function mailCardLineHtml(").contains("ev("), "the card line sends nothing");
    let lines = ev_lines(&js);
    assert!(lines.len() >= 3, "the deck's events: {lines:?}");
    for line in lines {
        for bad in [".from", ".mail_date", ".evidence", ".sender", ".why", ".title", ".text", ".value", "note", "ta."] {
            assert!(!line.contains(bad), "a usage event carries {bad}: {line}");
        }
    }
}

/// Note §5: the decision note lives in Details, kept per card while the page is open; a card that leaves the state
/// closes its Details; Details decides through the same decideCard.
#[test]
fn the_card_panel_holds_the_note_and_decides_through_the_deck() {
    let js = deck_js();
    assert!(js.contains("var NOTES = Object.create(null);"), "the notes, per card id");
    let panel = fn_body(&js, "function cardPanelHtml(");
    assert!(panel.contains("data-card-note") && panel.contains("<textarea"), "the note field: {panel}");
    assert!(js.contains("NOTES[id] = ta.value;"), "typed into the note map");
    let on = fn_body(&js, "KnowluData.on(function (");
    assert!(on.contains("KnowluPanel.close()"), "a decided card's Details closes: {on}");
    assert!(on.contains("drawHost("), "and the deck redraws");
    let render = fn_body(&js, "render: function (");
    assert!(render.contains("findCard(id)"), "Details reads the card from the state, by id");
}
