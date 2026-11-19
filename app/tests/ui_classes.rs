//! T13, the Classes list and a class's page (v0.4.0 note §4 "Classes list", "Class page"; plan §5
//! T13, §6, §7): pins on `js/pages/classes.js`, `js/pages/class.js` and `css/classes.css`.
//! Red first: against T5's placeholders every test here fails. The re-homed tests keep their old
//! names and behaviour (plan §7, "the figures, now on Classes and the class page") and read the new
//! files; where the note changes what is asserted, the test says so.

#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

fn classes_js() -> String { strip_comments(&read("js/pages/classes.js")) }
fn class_js() -> String { strip_comments(&read("js/pages/class.js")) }
fn classes_css() -> String { strip_css_comments(&read("css/classes.css")) }

/// Pin 6 (ruling 6; successor of the retired rings test): a grade is text from the one formatter,
/// `KnowluFmt.grade(g, hidden)` ("95.2% A", "No scores", "Hidden"), never a ring, an arc or a
/// gradient; neither page reads a percent itself, so a hidden grade can draw no figure anywhere.
/// While `grades_status` has not answered, no figure is drawn at all.
#[test]
fn grades_are_text_never_a_ring() {
    let list = classes_js();
    let page = class_js();
    assert!(list.contains("KnowluRouter.add(\"classes\""), "the Classes list registers its route");
    assert!(page.contains("KnowluRouter.add(\"class\""), "a class's page registers its route");
    for (f, js) in [("classes.js", &list), ("class.js", &page)] {
        assert!(js.contains("KnowluFmt.grade("), "{f}: a grade is said by the one formatter");
        for ring in ["<circle", "stroke-dasharray", "conic-gradient", "ringSvg", "class=\"ring", "--grade-", "\"ring\""] {
            assert!(!js.contains(ring), "{f}: a grade ring ({ring})");
        }
        assert!(!js.contains(".pct"), "{f}: the page reads no percent itself; only KnowluFmt.grade says one");
        assert!(!js.contains("toFixed(") && !js.contains("+ \"%\"") && !js.contains("+ '%'"), "{f}: no percent text of its own");
        let hidden = fn_body(js, "function hiddenFlag(");
        assert!(hidden.contains("KnowluData.last(\"grades_status\")"), "{f}: hidden is grades_status's word: {hidden}");
        assert!(hidden.contains("return null;") && hidden.contains("!!g.hidden"), "{f}: unknown until grades_status answers, then its hidden flag: {hidden}");
        let text = fn_body(js, "function gradeText(");
        assert!(text.contains("if (hidden === null) { return \"\"; }"), "{f}: no figure before grades_status answers: {text}");
        assert!(text.contains("KnowluFmt.grade(gradeOf(st, slug), hidden)"), "{f}: the course's S.grades entry through the formatter: {text}");
    }
    let css = classes_css();
    for ring in ["conic-gradient", "stroke-dasharray", ".ring", "--grade-"] {
        assert!(!css.contains(ring), "classes.css draws a ring ({ring})");
    }
}

/// M7: Classes' "Next due" is the open row of that course with the soonest `due`, never the first by
/// rank; a class's page lists its open rows soonest first. Both read the horizon view's `S.list.rows`
/// and match a row to a class the engine's way (`KnowluCourses.match`, N7).
#[test]
fn next_due_is_the_soonest() {
    let list = classes_js();
    assert!(list.contains("st.list.rows"), "Classes reads S.list.rows (the all view)");
    let next = fn_body(&list, "function nextDue(");
    assert!(next.contains("KnowluCourses.match(r.course)"), "a row's class by the engine's match: {next}");
    assert!(next.contains("r.status === \"done\""), "done rows are not due: {next}");
    assert!(next.contains("if (!best || r.due < best.due) { best = r; }"), "the soonest due wins, whatever the rank: {next}");
    assert!(!next.contains("[0]") && !next.contains("find("), "never the first by rank: {next}");
    let page = class_js();
    assert!(page.contains("st.list.rows"), "a class's page reads S.list.rows");
    let rows = fn_body(&page, "function dueRows(");
    assert!(rows.contains("KnowluCourses.match(r.course)") && rows.contains("r.status !== \"done\""), "that class's open rows: {rows}");
    assert!(rows.contains(".sort(") && rows.contains("a.r.due < b.r.due"), "soonest first: {rows}");
    assert!(rows.contains("a.i - b.i"), "a tie keeps the engine's order: {rows}");
    // D6: below 900px of center Classes drops Next due.
    let css = classes_css();
    assert!(css.contains("@container cl (max-width: 900px)") && css.contains(".cl-next"), "Next due drops below 900px of center");
}

/// Note §4 "Class page": Hide and Show go through `set_settings {patch: {grades_hidden}}` (the
/// setting `commands.rs` keeps), then `grades_status` is read again, and are never on an undo list.
/// Hidden, the Grade body is the one control Show and nothing else.
#[test]
fn class_page_hide_show_through_set_settings() {
    let page = class_js();
    let set = fn_body(&page, "function setHidden(");
    assert!(set.contains("KnowluData.call(\"set_settings\", { patch: { grades_hidden: on } })"), "through set_settings: {set}");
    assert!(set.contains("KnowluData.call(\"grades_status\", {})"), "then grades_status is read again: {set}");
    assert!(set.contains("KnowluKit.refusal(env)"), "a refusal is said: {set}");
    assert!(!set.contains("Undo") && !set.contains("uncheck"), "Hide and Show are never on the undo list: {set}");
    assert!(page.contains("closest(\"[data-grades-hide]\")") && page.contains("closest(\"[data-grades-show]\")"), "Hide and Show are bound");
    assert!(page.contains("setHidden(true, ") && page.contains("setHidden(false, "), "Hide sets it, Show clears it");
    let body = fn_body(&page, "function gradeBodyHtml(");
    assert!(body.contains("if (hidden === null) { return \"\"; }"), "nothing before grades_status answers: {body}");
    assert!(body.contains("if (hidden) { return SHOW_HTML; }"), "hidden: Show alone: {body}");
    let show = page.lines().map(str::trim).find(|l| l.starts_with("var SHOW_HTML = ")).expect("var SHOW_HTML");
    assert!(show.contains("data-grades-show") && show.contains(">Show<") && !show.contains('%'), "SHOW_HTML is the one control: {show}");
    assert!(page.contains("data-grades-hide") && page.contains(">Hide<"), "Hide sits beside the figure");
    // Classes reads the same flag; it has no Hide of its own.
    assert!(!classes_js().contains("set_settings"), "the list never changes the setting");
}

/// Note §4 "Class page": Meets is the class's matched commitments plus office hours, from
/// `your_week` (read on mount): `week.commitments` and the `week.office_hours` proposals whose course
/// matches this class the engine's way (`KnowluCourses.match`, N7). Each says the engine's own
/// `when` and `where`, through `h()`; the page writes no meeting label of its own.
#[test]
fn meets_from_matched_commitments() {
    let page = class_js();
    assert!(page.contains("KnowluData.call(\"your_week\", {})"), "your_week is read");
    let of = fn_body(&page, "function meetsOf(");
    assert!(of.contains("week.commitments") && of.contains("week.office_hours"), "commitments and office hours: {of}");
    assert!(of.contains("KnowluCourses.match(cm.course)"), "matched the engine's way: {of}");
    assert!(of.contains("cm.until"), "a commitment that has ended is not a meeting: {of}");
    let row = fn_body(&page, "function meetHtml(");
    assert!(row.contains("h(m.when)") && row.contains("h(m.where)"), "the engine's when and where, through h(): {row}");
    assert!(page.contains("\"office-hours\": \"Office hours\""), "office hours are named");
    for own in ["days.join", "meets_label", ".start + ", "DAY_NAME"] {
        assert!(!page.contains(own), "class.js writes its own meeting label ({own})");
    }
}

/// Re-homed (plan §7; the old grades drawer, now the class page's breakdown in the right panel): the
/// breakdown is read-only, and its foot says where the number came from: Blackboard, the Overall
/// Grade or the points so far (both when the course carries both), and when it was read. Changed by
/// the note: the age is the one date rule's "Read 7:30 PM" (`KnowluFmt.read`, DESIGN.md) in place of
/// the old "updated 2 h ago", which counted from the machine's clock. Every status the engine sends
/// has words.
#[test]
fn the_breakdown_drawer_is_read_only_and_the_footer_says_where_the_number_came_from() {
    let page = class_js();
    assert!(page.contains("KnowluPanel.add(\"grades\""), "the breakdown is a panel kind");
    let drawer = fn_body(&page, "function breakdownHtml(");
    for banned in ["set_fields", "data-field", "contenteditable", "<input", "<textarea", "create_task", "delete_note"] {
        assert!(!drawer.contains(banned), "the breakdown contains {banned}: it is read-only");
    }
    for (f, js) in [("class.js", &page), ("classes.js", &classes_js())] {
        for w in ["set_fields", "create_task", "delete_note", "set_body"] {
            assert!(!js.contains(w), "{f} writes a note ({w})");
        }
    }
    let footer = fn_body(&page, "function breakdownFootHtml(");
    assert!(footer.contains("From Blackboard") && footer.contains("Overall Grade") && footer.contains("points so far"), "{footer}");
    assert!(footer.contains("g.basis === \"overall\"") && footer.contains("g.earned") && footer.contains("g.possible"),
        "points so far sits beside the Overall Grade when the course carries both: {footer}");
    assert!(footer.contains("data-grades-refresh") && footer.contains("KnowluFmt.read("), "Refresh and when it was read: {footer}");
    assert!(footer.contains("data-start") && footer.contains("KnowluKit.owed()") && footer.contains("KnowluKit.owedHtml("), "Refresh is a start control and takes the owed lock");
    assert!(page.contains("KnowluData.call(\"grades_refresh\", {})"), "Refresh calls grades_refresh");
    for s in ["graded", "needs-grading", "in-progress", "not-submitted", "exempt"] {
        assert!(page.contains(&format!("\"{s}\":")), "status {s} has words");
    }
    let row = fn_body(&page, "function entryHtml(");
    assert!(row.contains("h(e.title)") && row.contains("not counted"), "an entry's title through h(), and not counted said: {row}");
}

/// Re-homed (plan §7): every number on Classes and a class's page is the engine's. Nothing sums,
/// rounds, divides or scales a grade.
#[test]
fn every_number_on_the_strip_is_the_engines() {
    for (f, js) in [("classes.js", classes_js()), ("class.js", class_js())] {
        assert!(js.contains("KnowluFmt.grade("), "{f} draws the grade, through the one formatter");
        for banned in ["reduce(", "Math.round(", "/ c.possible", "/ g.possible", "* 100", "/ 100", "parseFloat(", "Math.floor("] {
            assert!(!js.contains(banned), "{f} computes ({banned}): every number comes from the state");
        }
    }
}

/// Split from `a_note_drawer_is_never_hijacked_by_a_grades_repaint` (plan §7, R2-1; T13's half): a
/// repaint redraws the breakdown only while the panel shows it. The breakdown's open marker is set
/// when it renders and cleared when the panel closes it (opening a task's Details closes it first),
/// so a repaint never takes a note's panel.
#[test]
fn the_grades_breakdown_never_takes_a_note_panel() {
    let page = class_js();
    let refresh = fn_body(&page, "function refreshBreakdown(");
    assert!(refresh.contains("if (!GRADES_OPEN || !KnowluPanel.isOpen()) { return; }"), "only an open breakdown is redrawn: {refresh}");
    assert!(refresh.contains("KnowluPanel.open(\"grades\", GRADES_OPEN)"), "{refresh}");
    let panel = fn_body(&page, "KnowluPanel.add(\"grades\"");
    assert!(panel.contains("GRADES_OPEN = slug;"), "rendering marks it open: {panel}");
    assert!(panel.contains("close: function () { GRADES_OPEN = null; }"), "the panel closing clears the marker: {panel}");
    assert_eq!(page.matches("GRADES_OPEN = ").count(), 3, "set only by its declaration, the render and the close");
}

/// The list's shape (note §4 "Classes list", the dispatch's ruling): every entry of `S.courses`, the
/// active ones (status absent or `active`, M1, `KnowluCourses.active`) open under Current and the rest
/// under a closed Not current; a row opens the class's page by its slug. A class row carries no
/// `data-id`: a slug is never an object id, so it never reaches telemetry (I9).
#[test]
fn the_list_reads_every_course_and_opens_its_page() {
    let list = classes_js();
    let draw = fn_body(&list, "function draw(");
    assert!(draw.contains("KnowluCourses.all()"), "every entry: {draw}");
    assert!(draw.contains("KnowluCourses.active(c)") && draw.contains("!KnowluCourses.active(c)"), "split by the engine's active rule: {draw}");
    assert!(list.contains("\"Current\"") && list.contains("\"Not current\""), "the two groups");
    let row = fn_body(&list, "function rowHtml(");
    assert!(row.contains("h(\"#class/\" + c.slug)"), "the row opens #class/<slug>: {row}");
    assert!(row.contains("h(c.title)") && row.contains("c.name ? h(c.name) : h(c.title)"), "code and name through h(): {row}");
    assert!(row.contains("KnowluCourses.slot(c)"), "its slot from the entry: {row}");
    assert!(!row.contains("data-id"), "a class row carries no data-id: {row}");
    for no in ["Instructor", "GPA", "Final"] {
        assert!(!list.contains(no), "no {no}: v0.4.0 has no source for it (note §1)");
    }
}

/// A8 and plan §3: a Due row's check is the one kit control, which fills with the row's class colour;
/// classes.css never repaints it gray. A name opens Details; the list holds a reorder while the
/// pointer is over it.
#[test]
fn class_due_rows_use_the_one_check_and_open_details() {
    let page = class_js();
    let row = fn_body(&page, "function dueRowHtml(");
    assert!(row.contains("KnowluKit.checkHtml(r)"), "the one check control: {row}");
    assert!(row.contains("data-det") && row.contains("data-open=\"") && row.contains("data-id=\"") && row.contains("data-kind=\"task\""), "a row opens Details and is observed as a task: {row}");
    assert!(row.contains("h(r.title)") && row.contains("KnowluFmt.due(r.due)"), "the name through h(), the due by the one rule: {row}");
    assert!(page.contains("KnowluPanel.open(\"task\", "), "a name opens the Details panel");
    assert!(page.contains("data-rows"), "the list holds a reorder under the pointer");
    assert!(!page.contains("progress: 100"), "the page writes no check of its own");
    let css = classes_css();
    for (i, _) in css.match_indices(".ck") {
        let rule = &css[i..i + css[i..].find('}').unwrap_or(css.len() - i)];
        for gray in ["--ink-3", "--ink-2", "--cc:", "--cc-edge:", "gray", "grey"] {
            assert!(!rule.contains(gray), "classes.css repaints the check ({gray}): {rule}");
        }
    }
    // D6: a class's page is one column at 820 x 600 (about 872px of center), three from 1000px.
    assert!(css.contains("@container cp (min-width: 1000px)"), "the columns open from 1000px of page");
}

/// Review I1: "No scores" is said only of an S.grades entry, or of a source that is available and
/// connected. A class with no entry while grades cannot be read draws nothing in the list, and on its
/// page a blank Grade line over one line pointing to Settings > Sources ("Connect Blackboard to see
/// your grades" only when the source is available), with no figure and no Hide.
#[test]
fn no_scores_only_for_an_existing_entry_or_a_connected_source() {
    for (f, js) in [("classes.js", classes_js()), ("class.js", class_js())] {
        let unread = fn_body(&js, "function unread(");
        assert!(unread.contains("!gradeOf(st, slug)") && unread.contains("s.available") && unread.contains("s.connected"), "{f}: unread is no entry and no readable source: {unread}");
        let text = fn_body(&js, "function gradeText(");
        let at = text.find("if (unread(st, slug)) { return \"\"; }").unwrap_or_else(|| panic!("{f}: an unread class's grade is blank: {text}"));
        assert!(at < text.find("KnowluFmt.grade(").unwrap(), "{f}: blank before the formatter can say No scores");
    }
    let js = class_js();
    let body = fn_body(&js, "function gradeBodyHtml(");
    let at = body.find("if (unread(st, SLUG)) { return sourcesHtml(); }").expect("the page's unread body is the Sources line");
    assert!(at < body.find("if (hidden) { return SHOW_HTML; }").unwrap() && at < body.find("HIDE_HTML").unwrap(), "before Show, Hide and the figure");
    let src = fn_body(&js, "function sourcesHtml(");
    assert!(src.contains("s && s.available ? \"Connect Blackboard to see your grades\""), "the Connect line only when available: {src}");
    assert!(src.contains("href=\"#settings/sources\"") && src.contains("Settings > Sources"), "it points to Settings > Sources: {src}");
    assert!(!src.contains("data-grades") && !src.contains("No scores"), "no figure, no Hide: {src}");
}

/// Review M1: a poll redraws only what it changed, so the keyboard's focus stays put. The list
/// redraws on a poll only when the hidden flag or grades_status changed; the page's Grade only when
/// those or the class's entry changed; the breakdown only when its entry did.
#[test]
fn a_poll_redraws_only_what_changed() {
    let list = classes_js();
    assert!(list.contains("KnowluData.onPoll(function () { if (ROOT && ST && pollSig() !== SIG) { draw(); } });"), "the list's poll hook compares");
    let sig = fn_body(&list, "function pollSig(");
    assert!(sig.contains("hiddenFlag()") && sig.contains("KnowluData.last(\"grades_status\")"), "{sig}");
    assert!(fn_body(&list, "function draw(").contains("SIG = pollSig();"), "draw records what it drew from");
    let page = class_js();
    let grade = fn_body(&page, "function drawGrade(");
    assert!(grade.contains("gradeOf(ST, SLUG)") && grade.contains("KnowluData.last(\"grades_status\")") && grade.contains("if (sig === GRADE_SIG) { return; }"), "{grade}");
    let rb = fn_body(&page, "function refreshBreakdown(");
    assert!(rb.contains("if (breakdownSig(GRADES_OPEN) === BREAKDOWN_SIG) { return; }"), "an unchanged entry never reopens the breakdown: {rb}");
    assert!(fn_body(&page, "function breakdownSig(").contains("gradeOf("), "the breakdown's entry");
}
