//! v0.4.0 T8: Home's aside, Waiting (its header, the deck's slot, What changed, Needs a date) and today's
//! schedule and free time (note §4 Home; plan T8, and the rows plan §7 re-homes to this file). Every pin
//! reads T8's own files, `js/pages/home_aside.js` and `css/home_aside.css`, under `ui_common::ROOT`. The
//! DOM checks are `scripts/ui-fixture/checks/t08.py`.
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

/// home_aside.js with its comments stripped: what a pin reads is what runs.
fn aside() -> String {
    let raw = read("js/pages/home_aside.js");
    assert!(!is_placeholder(&raw), "js/pages/home_aside.js is still T5's placeholder");
    strip_comments(&raw)
}

/// home_aside.css with its comments stripped.
fn aside_css() -> String {
    let raw = read("css/home_aside.css");
    assert!(!is_placeholder(&raw), "css/home_aside.css is still T5's placeholder");
    strip_css_comments(&raw)
}

/// I3, N1: Waiting's header reads the engine's counts (`pending`, `deferred`, `awaiting_calendar`,
/// `events_in_digest`); it says the engine's `empty_text` only when the engine sets it, and with only
/// a digest waiting it says how many events are in today's digest, never that nothing is waiting.
/// `hidden_amendments` and `must_do.amendments` are not drawn.
#[test]
fn waiting_header_reads_decisions_counts_and_empty_text() {
    let js = aside();
    assert!(js.contains("window.KnowluHomeAside = {"), "Home's aside is one module Home mounts");
    let counts = fn_body(&js, "function countsText(");
    for read in ["d.pending", "d.deferred", "d.awaiting_calendar", "d.events_in_digest"] {
        assert!(counts.contains(read), "the header reads {read}");
    }
    let clear = fn_body(&js, "function clearText(");
    assert!(clear.contains("if (d.empty_text) { return d.empty_text; }"), "the engine's own empty text, only when it sets one");
    assert!(clear.contains("\" event in today's digest\"") && clear.contains("\" events in today's digest\""), "a digest alone says so");
    for banned in ["Nothing waiting", "Queue clear", "hidden_amendments", ".amendments"] {
        assert!(!js.contains(banned), "home_aside.js names {banned:?}");
    }
}

/// P3: Waiting hands its card slot to the deck (T15's), and draws no card of its own.
#[test]
fn waiting_calls_kit_slots_deck() {
    let js = aside();
    let wait = fn_body(&js, "function drawWaiting(");
    assert!(wait.contains("KnowluSlots.deck($(\"ha-deck\"))"), "the card slot is the deck's");
    assert!(!js.contains(".cards"), "the cards are the deck's to read");
    let sent = commands_sent(&read("js/pages/home_aside.js"));
    for cmd in ["decide", "answer_card"] {
        assert!(!sent.iter().any(|c| c == cmd), "the aside sends {cmd}: a card's buttons are the deck's");
    }
}

/// What changed: the engine's `delta` (its summary and each record's text), the moved line
/// (`moved.text`), good to know (each row an `info` row with Close calling `close_info`) and, when
/// email is set up, a "From your email" line from `mail_applied` that opens the mail sheet's view
/// (T17's, where Undo lives). Each piece is drawn only when the engine sends it.
#[test]
fn what_changed_reads_delta_moved_good_to_know_and_mail_applied() {
    let js = aside();
    let changed = fn_body(&js, "function changedHtml(");
    for read in ["st.delta", ".summary", ".records", "st.moved", "st.good_to_know", "mailHtml("] {
        assert!(changed.contains(read), "What changed reads {read}");
    }
    assert!(fn_body(&js, "function recordHtml(").contains("h(r.text)"), "a record's text goes through h()");
    let gtk = fn_body(&js, "function gtkHtml(");
    assert!(gtk.contains("data-kind=\"info\"") && gtk.contains("data-close-info=\""), "a good-to-know row is an info row with Close");
    assert!(gtk.contains("h(x.title)") && gtk.contains("h(x.id)"), "escaped");
    let close = fn_body(&js, "function closeInfo(");
    assert!(close.contains("KnowluData.call(\"close_info\", { id: id })"), "Close calls close_info with the row's id");
    assert!(close.contains("KnowluKit.refusal("), "a refusal says the engine's sentence");
    // From your email: asked once a mount, only when a mailbox is set up; its rows are T17's.
    let mail = fn_body(&js, "function loadMailApplied(");
    assert!(mail.contains("KnowluData.last(\"mail_status\")") && mail.contains(".mailboxes"), "only with a mailbox");
    assert!(mail.contains("KnowluData.call(\"mail_applied\", {})"), "mail_applied, the app's own read");
    assert!(fn_body(&js, "function mailCount(").contains("archived.count"), "the count adds every kind of item");
    assert!(js.contains("\"From your email\""), "its label");
    assert!(js.contains("window.KnowluMail") && js.contains(".openApplied"), "it opens the mail sheet's view when T17 offers it");
}

/// Needs a date: `nav_counts.undated` (count and hours) links Tasks' undated list; nothing undated
/// draws nothing.
#[test]
fn needs_a_date_links_tasks() {
    let js = aside();
    let un = fn_body(&js, "function undatedHtml(");
    assert!(un.contains("st.nav_counts") && un.contains(".undated"), "it reads nav_counts.undated");
    assert!(un.contains("if (!u || !u.count) { return \"\"; }"), "nothing undated, no line");
    // Review I1: Tasks applies a route argument only from its DUE_ARGS (late, today, week, none).
    assert!(un.contains("href=\"#tasks/none\""), "it links Tasks' No date filter");
    assert!(!js.contains("#tasks/undated"), "no argument Tasks does not take");
    assert!(un.contains("KnowluFmt.minutes("), "its hours through the shared formatter");
}

/// Today's schedule: the day's class and busy blocks (`start`, `end`, `label`) and its all-day dates,
/// a class drawn in its slot when its label matches a class; Remove from my day on an accepted
/// date hides it at once and calls `remove_lane_date` when the ten-second Undo toast closes.
#[test]
fn schedule_and_remove_lane_date_with_the_ten_second_toast() {
    let js = aside();
    let sched = fn_body(&js, "function schedHtml(");
    assert!(sched.contains("st.the_day") && sched.contains(".blocks"), "the day's blocks");
    assert!(sched.contains("b.kind === \"class\" || b.kind === \"busy\""), "class and busy blocks only");
    assert!(sched.contains("laneHtml("), "with the all-day dates");
    let row = fn_body(&js, "function blockHtml(");
    assert!(row.contains("KnowluCourses.match(b.label)") && row.contains("KnowluFmt.span("), "a class's slot by its label, its times by the formatter");
    assert!(row.contains("h(b.start)") && row.contains("h(b.end)"), "the times ride the row");
    // Review I2: data-start marks a start control (plan §3, KnowluKit.owed()); a row's times are data-from / data-to.
    assert!(row.contains("data-from=\"") && row.contains("data-to=\""), "a schedule row's times");
    assert!(fn_body(&js, "function freeRowHtml(").contains("data-from=\""), "a free window's times");
    assert!(!js.contains("data-start"), "no schedule or free row is marked as a start control");
    assert!(js.contains("var LANE_MS = 10000;"), "ten seconds");
    let rm = fn_body(&js, "function removeFromMyDay(");
    assert!(rm.contains("setTimeout(") && rm.contains("LANE_MS"), "behind the ten-second timer");
    assert!(rm.contains("\"Removed\"") && rm.contains("\"Undo\""), "Removed, Undo");
    assert!(!js.contains("confirm("), "no window.confirm");
    let css = aside_css();
    assert!(css.contains(".ha-sc {") && css.contains(".ha-free-row {"), "schedule and free rows have their look");
}

/// Free time: the day's `open_hours`, and the free blocks of the same `the_day.blocks` list the
/// schedule draws, so the two always agree; each window lists what the engine puts in it (`takes[]`).
#[test]
fn free_time_from_open_hours() {
    let js = aside();
    let free = fn_body(&js, "function freeHtml(");
    assert!(free.contains("st.the_day") && free.contains(".open_hours") && free.contains(".blocks"), "open hours and the same blocks");
    assert!(free.contains("b.kind === \"free\""), "the free blocks");
    assert!(free.contains("KnowluFmt.minutes(") && free.contains("\" free\""), "N h free");
    let row = fn_body(&js, "function freeRowHtml(");
    assert!(row.contains("b.takes") && row.contains("KnowluFmt.span("), "each window and what goes in it");
    let take = fn_body(&js, "function takeHtml(");
    assert!(take.contains("h(t.title)") && take.contains("t.of_hours"), "a take's title is escaped; a part of a task says of how much");
}

/// Ruling: the aside's words are names, figures and short labels: no exclamation mark, no "AI", no
/// browser tooltip, no gradient, ring or chip.
#[test]
fn home_aside_copy_is_plain() {
    let raw = read("js/pages/home_aside.js");
    for s in string_literals(&raw) {
        assert!(!s.contains('!'), "home_aside.js says {s:?}: no exclamation marks");
        assert!(!s.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == "AI"), "home_aside.js says {s:?}: never the word AI");
        assert!(!s.contains(" title=") && !s.starts_with("title="), "home_aside.js builds a title= tooltip: {s:?}");
    }
    let css = aside_css();
    for banned in ["gradient", "conic", "chip", "pill", "999px"] {
        assert!(!css.contains(banned), "home_aside.css: {banned}");
    }
}

// ---- re-homed from static_assets.rs (plan §7) ----------------------------------------------------

#[test]
fn the_lane_offers_remove_on_accepted_dates_only() {
    let js = aside();
    let b = fn_body(&js, "function laneHtml(");
    assert!(b.contains("all_day_uids"), "reads the uids");
    assert!(b.contains("Remove from my day") && b.contains("data-lane-remove=\"") && b.contains("data-title=\""), "the button carries uid and title");
    assert!(b.contains("if (!uid) {"), "an entry without a uid renders as today, with no Remove");
    assert!(b.contains("h(uid)") && b.contains("h(t)"), "escaped");
    assert!(b.contains("PENDING_LANE[uid]"), "a pending uid is skipped");
}

#[test]
fn remove_from_my_day_hides_at_once_and_calls_when_the_undo_toast_closes() {
    let js = aside();
    let b = fn_body(&js, "function removeFromMyDay(");
    assert!(b.contains("PENDING_LANE[uid] = ") && b.contains("drawDay("), "hidden at once");
    assert!(b.contains("\"Removed\"") && b.contains("\"Undo\"") && b.contains("LANE_MS"), "Removed - Undo for 10 seconds");
    assert!(b.contains("t.className = \"toast\"") && b.contains("data-lane-toast"), "the shell's toast look, its own marker (the kit's toasts close at six seconds)");
    assert!(b.contains("KnowluData.call(\"remove_lane_date\", { uid: uid })"), "the one call, with the uid (data.js adds the view)");
    let call = b.find("KnowluData.call(\"remove_lane_date\"").unwrap();
    let to = b.find("setTimeout(").unwrap();
    assert!(to < call, "the call sits behind the timer");
    assert!(js.contains("[data-lane-remove]") && js.contains("removeFromMyDay("), "the delegated handler");
    assert!(!b.contains("ev("), "no telemetry row of its own (D12)");
}

#[test]
fn a_refused_removal_repaints_the_lane() {
    let js = aside();
    let b = fn_body(&js, "function removeFromMyDay(");
    let d = b.find("delete PENDING_LANE[uid];").unwrap();
    let r = b[d..].find("drawDay(").expect("the .then branch re-paints");
    let a = b[d..].find("KnowluKit.refusal(").unwrap();
    assert!(r < a, "the lane is repainted before the refusal is said");
}

#[test]
fn undo_within_the_toast_never_calls_remove_lane_date() {
    let js = aside();
    let b = fn_body(&js, "function removeFromMyDay(");
    let u = b.find("data-lane-undo").expect("an Undo button");
    assert!(b.contains("clearTimeout("), "undo cancels the timer");
    assert!(b[u..].contains("delete PENDING_LANE["), "undo clears the pending uid");
    assert_eq!(js.matches("KnowluData.call(\"remove_lane_date\"").count(), 1, "one call site, behind the timer");
}

/// Good-to-know half of `the_good_to_know_and_issues_views_are_real_and_every_observed_row_names_its_kind`
/// (the Issues half is Settings > Status's): every row that stamps `data-id` stamps `data-kind` in the same
/// string, and one click on Close is one `close_info`.
#[test]
fn the_good_to_know_and_issues_views_are_real_and_every_observed_row_names_its_kind() {
    let js = aside();
    assert!(js.contains("data-kind=\"info\""), "R-T15b: the good-to-know kind");
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
    let click = fn_body(&js, "function onClick(");
    let at = click.find("[data-close-info]").expect("Close has its branch");
    assert!(click[at..].contains("e.stopPropagation();"), "one click, one close_info");
}

/// Moved-line half of `the_schedule_view_the_window_editor_and_the_moved_line_are_there` (the window
/// editor is Settings > Day's): the today view prints `moved.text`, and nothing when the engine sends none.
#[test]
fn the_schedule_view_the_window_editor_and_the_moved_line_are_there() {
    let js = aside();
    let m = fn_body(&js, "function movedHtml(");
    assert!(m.contains("var m = st.moved;"), "the engine's moved line");
    assert!(m.contains("if (!m || !m.text) { return \"\"; }") && m.contains("h(m.text)"), "printed through h(), nothing when absent");
    assert!(!js.contains("data-remove="), "D8: no remove control on a commitment");
}
