//! v0.4.0 T7: Home's main pane (note §4 Home; plan T7, and the rows plan §7 re-homes to this file).
//! Every pin reads T7's own files, `js/pages/home.js` and `css/home.css`, under `ui_common::ROOT`,
//! plus the shell files already merged (kit.js, data.js) where a pin follows a call into them. The
//! DOM checks are `scripts/ui-fixture/checks/t07.py`.
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

/// home.js with its comments stripped: what a pin reads is what runs.
fn home() -> String {
    let raw = read("js/pages/home.js");
    assert!(!is_placeholder(&raw), "js/pages/home.js is still T5's placeholder");
    strip_comments(&raw)
}

/// home.css with its comments stripped.
fn home_css() -> String {
    let raw = read("css/home.css");
    assert!(!is_placeholder(&raw), "css/home.css is still T5's placeholder");
    strip_css_comments(&raw)
}

/// Note §4 Home (A4, N1): Home registers itself, maps Do today from `must_do.groups[].rows` in order
/// and Recommended from `recommended.rows[]` (`take_hours` of `of_hours`), each with its engine's
/// empty text; the lead is the first unchecked Do today row, else the first Recommended row, with
/// Lock in going to `#lockin/<id>`; the domain row lists the active `courses` and filters Do today
/// and Recommended only, never the aside (today's schedule and free time); a pending amendment shows
/// only as its row's own marker, and `must_do.amendments` is never drawn.
#[test]
fn home_maps_do_today_recommended_and_the_domain_row() {
    let js = home();
    assert!(js.contains("KnowluRouter.add(\"home\", {"), "Home registers itself with the router");
    for read in [".must_do", ".groups", ".rows", ".recommended", ".take_hours", ".of_hours", "empty_text"] {
        assert!(js.contains(read), "home.js reads {read}");
    }
    for fmt in ["KnowluFmt.minutes(", "KnowluFmt.due(", "KnowluFmt.lead(", "KnowluKit.markHtml(", "KnowluCourses.match("] {
        assert!(js.contains(fmt), "figures and marks come from the shared formatter and table: {fmt}");
    }
    let lead = fn_body(&js, "function leadOf(");
    assert!(lead.contains("open[0]") && lead.contains("rec[0]"), "the lead is the first unchecked Do today row, else the first Recommended row");
    assert!(js.contains("\"#lockin/\""), "Lock in goes to #lockin with the lead's id");
    // The domain row: active classes by the engine's rule, a press filters, a second clears.
    let doms = fn_body(&js, "function domsHtml(");
    assert!(doms.contains("KnowluCourses.active("), "the domain row lists the active classes (M1)");
    assert!(js.contains("aria-pressed") && js.contains("data-dom"), "each class is a pressed-state button");
    let toggle = fn_body(&js, "function setFilter(");
    assert!(toggle.contains("FILTER === slug ? null : slug"), "a second press clears the filter");
    let shown = fn_body(&js, "function shown(");
    assert!(shown.contains("FILTER"), "shown() is the filter");
    for (n, line) in js.lines().enumerate() {
        if line.contains("KnowluHomeAside.") {
            assert!(!line.contains("FILTER") && !line.contains("shown("), "home.js:{}: the aside (schedule, free time, Waiting) is never filtered: {}", n + 1, line.trim());
        }
    }
    assert!(js.contains("KnowluHomeAside.update(st)"), "the aside gets the whole state");
    // N1: the row's own marker, never the amendments list.
    assert!(js.contains(".amend_badge"), "a pending amendment is the row's own marker");
    assert!(!js.contains(".amendments"), "must_do.amendments is not drawn (N1)");
}

/// A4: Start by, slack and "Won't fit" live in Details only, never on Home.
#[test]
fn home_shows_no_start_by_slack_or_wont_fit() {
    let js = home();
    let css = home_css();
    for banned in ["start_by", "slack_days", "fit_text", "fit_why", "overdue_start", "Start by", "start by", "Won't fit", "won't fit", "Won\\u2019t fit", "slack"] {
        assert!(!js.contains(banned), "home.js names {banned:?} (A4: Details only)");
        assert!(!css.contains(banned), "home.css names {banned:?}");
    }
}

/// I1 and the F7 line re-homed (`console_js_is_one_render_function_per_region_and_uses_invoke`):
/// Home renders the state's top-level `unreadable` in Do today, with each path through `h()`, and
/// never hides it.
#[test]
fn home_renders_state_unreadable() {
    let js = home();
    let un = fn_body(&js, "function unreadableHtml(");
    assert!(un.contains("st.unreadable"), "the state's top-level unreadable list");
    assert!(un.contains("\"1 task note can't be read\"") && un.contains("\" task notes can't be read\""), "the line names how many");
    assert!(un.contains("map(pathHtml)"), "every path is listed");
    assert!(fn_body(&js, "function pathHtml(").contains("h(p)"), "a path goes through h()");
    assert!(!un.contains("hidden"), "the line is never hidden");
    assert!(fn_body(&js, "function todayHtml(").contains("unreadableHtml("), "the line is part of Do today");
}

/// A8: Home draws one check control, the kit's: no check markup of its own, and no write of its own.
#[test]
fn home_checks_use_kit_check() {
    let js = home();
    assert!(js.matches("KnowluKit.checkHtml(").count() >= 2, "the lead and every row draw KnowluKit.checkHtml");
    for banned in ["class=\"ck", "class=\\\"ck", "'ck'", "\"set_fields\"", "progress:", "status: \"done\""] {
        assert!(!js.contains(banned), "home.js has a check of its own: {banned}");
    }
    assert!(commands_sent(&read("js/pages/home.js")).is_empty(), "Home's main pane sends no command: the kit writes, the router reads");
    // The lead field's check is drawn in the field's words (DESIGN.md "Check circle"); a done check
    // still never takes a gray.
    let css = home_css();
    for (i, _) in css.match_indices(".ck") {
        let rule = &css[i..i + css[i..].find('}').unwrap_or(css.len() - i)];
        for gray in ["--ink-3", "--ink-2", "--line", "--field-line", "gray", "grey", "--shade-"] {
            assert!(!rule.contains(gray), "a check rule in home.css names {gray}: {rule}");
        }
    }
}

/// I2: when the verdict carries an overflow sentence, Do today's totals add "Over by" the meter's
/// spill, and it opens a panel with the sentence, the meter's give text and its spill items.
#[test]
fn over_capacity_reads_overflow_give_and_spill() {
    let js = home();
    let sum = fn_body(&js, "function sumHtml(");
    assert!(sum.contains("verdict.overflow_sentence") && sum.contains("meter.spill_hours") && sum.contains("\"Over by \""), "the totals add Over by N");
    assert!(js.contains("KnowluPanel.add(\"overload\", {") && js.contains("KnowluPanel.open(\"overload\""), "the figure opens the right panel");
    let panel = fn_body(&js, "function overloadHtml(");
    for read in ["overflow_sentence", ".give_text", ".spill_items", "h(it.title)"] {
        assert!(panel.contains(read), "the panel reads {read}");
    }
}

/// I6, N10: "Clear" and "Clear through" show only when Do today has no unchecked rows; then the
/// first `ahead.buckets[]` with items: none in fourteen days is "Clear for two weeks", bucket 0 is no
/// line, else "Clear through" the day before it (the bucket before it).
#[test]
fn clear_through_only_with_do_today_done() {
    let js = home();
    let clear = fn_body(&js, "function clearLine(");
    let open = clear.find("if (open.length) { return \"\"; }").expect("no line while Do today has unchecked rows");
    let buckets = clear.find("ahead.buckets").expect("the line reads ahead.buckets");
    assert!(open < buckets, "the open-rows test comes first");
    assert!(clear.contains(".items > 0"), "the first bucket with items");
    assert!(clear.contains("i === 0 ? \"\""), "bucket 0 is no line");
    assert!(clear.contains("\"Clear through \"") && clear.contains("b[i - 1].date"), "Clear through the day before it");
    assert!(clear.contains("\"Clear for two weeks\""), "none in fourteen days");
}

/// A3: the domain row maps the engine's standing to three words; null draws no word; Behind says it in
/// words in the error colour.
#[test]
fn the_domain_row_maps_standing_to_three_words() {
    let js = home();
    assert!(js.contains("var STANDING = { behind: \"Behind\", tight: \"On track\", steady: \"On track\", ahead: \"Ahead\" };"), "A3's mapping, exactly");
    let dom = fn_body(&js, "function domHtml(");
    assert!(dom.contains("Object.prototype.hasOwnProperty.call(STANDING, k.standing)"), "a word only for a known standing; null draws none");
    assert!(dom.contains("h(word)") && dom.contains("h(k.title)") && dom.contains("h(KnowluCourses.slot(k))"), "the code, the word and the slot go through h()");
    let css = home_css();
    let at = css.find(".hm-dom.is-behind span").expect("Behind has its rule");
    assert!(css[at..at + css[at..].find('}').unwrap()].contains("var(--error)"), "Behind is the error colour, in words");
}

/// I4: while the envelope carries `first_run`, the state is null: Home shows "Setting up your day"
/// with the steps and the step in progress, and the rest of Home waits.
#[test]
fn first_run_view_lists_steps() {
    let js = home();
    assert!(js.contains("KnowluData.firstRun()"), "Home reads the first-run block");
    assert!(js.contains("\"Setting up your day\""), "the view's heading (note §4)");
    let fr = fn_body(&js, "function drawFirstRun(");
    assert!(fr.contains("fr.steps") && fr.contains("fr.current"), "the steps and the step in progress");
    assert!(fr.contains("$(\"hm-day\").hidden = true") && fr.contains("$(\"hm-first\").hidden = false"), "the view replaces the day");
    let day = fn_body(&js, "function drawDay(");
    assert!(day.contains("$(\"hm-first\").hidden = true") && day.contains("$(\"hm-day\").hidden = false"), "and the day replaces it");
}

/// Pin 8, A8: a check writes `progress: 100` with `status: "done"` and offers Undo, which puts the old
/// progress back with `status: "active"`; Home draws the kit's control, so it is the kit's write.
#[test]
fn check_writes_progress_and_done_with_undo() {
    let js = home();
    assert!(fn_body(&js, "function rowHtml(").contains("KnowluKit.checkHtml(row)"), "every row's check is the kit's");
    assert!(fn_body(&js, "function leadHtml(").contains("KnowluKit.checkHtml(row)"), "the lead's check is the kit's");
    let kit = strip_comments(&read("js/kit.js"));
    let check = fn_body(&kit, "check: function (");
    assert!(check.contains("\"set_fields\"") && check.contains("{ progress: 100, status: \"done\" }"), "the kit's check");
    assert!(check.contains("\"Undo\"") && check.contains("KIT.uncheck("), "with Undo");
    let uncheck = fn_body(&kit, "uncheck: function (");
    assert!(uncheck.contains("progress: back, status: \"active\""), "Undo restores the old progress with status active");
}

/// Ruling: Home's words are names, figures and short labels: no exclamation mark, no "AI", no
/// browser tooltip (`title=`), no gradient, ring or chip.
#[test]
fn home_copy_is_plain() {
    let raw = read("js/pages/home.js");
    for s in string_literals(&raw) {
        assert!(!s.contains('!'), "home.js says {s:?}: no exclamation marks");
        assert!(!s.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == "AI"), "home.js says {s:?}: never the word AI");
        assert!(!s.contains(" title=") && !s.starts_with("title="), "home.js builds a title= tooltip: {s:?}");
    }
    let css = home_css();
    for banned in ["gradient", "conic", "chip", "pill", "999px"] {
        assert!(!css.contains(banned), "home.css: {banned}");
    }
}

// ---- re-homed from static_assets.rs (plan §7) ----------------------------------------------------

/// Home half: Home renders the engine's empty strings, never its own (the Tasks half is `ui_tasks`).
#[test]
fn the_page_renders_the_engines_empty_strings_not_its_own() {
    let js = home();
    for own in ["Nothing active", "Nothing is at risk", "No spare capacity", "Queue clear", "No open issues", "No run recorded", "No events coming up.", "Nothing to know right now.", "Nothing closed this week."] {
        assert!(!js.contains(own), "home.js hardcodes \"{own}\": it must render state.*.empty_text");
    }
    assert!(js.contains("md.empty_text") && js.contains("r.empty_text"), "Do today's and Recommended's empty text are the engine's");
    assert!(home_css().contains(".hm-empty {"), "the empty line has its look");
}

/// D7 and §6, R-C1c-8, re-homed: the minute between Finish and the first `rank` is a view of its own
/// that replaces the day, with the steps in plain words from the live slot, a failed account check
/// shown as failed with a short note, and a line once a first slot ended without a day. The page
/// asks every three seconds until the day is there (data.js, the shell's), then goes back to its
/// minute.
#[test]
fn the_first_run_view_says_what_is_happening_and_polls_until_the_day_arrives() {
    let js = home();
    for say in [
        "sync: \"Syncing with your account\"",
        "entitlement: \"Checking your account\"",
        "coursework: \"Fetching your coursework\"",
        "ingest: \"Reading your school calendar\"",
        "judge: \"Working out what each task needs\"",
        "rank: \"Putting your day in order\"",
    ] {
        assert!(js.contains(say), "the step sentence {say}");
    }
    let step = fn_body(&js, "function stepHtml(");
    assert!(step.contains("String(s[0]).indexOf(\"entitlement (refresh failed\") === 0"), "the failed account check is recognised by name");
    assert!(js.contains("\"couldn't check \u{2014} will retry\""), "and says so in a short note");
    let fr = fn_body(&js, "function drawFirstRun(");
    assert!(fr.contains("fr.running ? firstRunSays(fr.current) : null"), "the step in progress is the live slot's own");
    assert!(fr.contains("var failed = (fr.steps || []).some(function (s) { return s[1] !== 0; });"), "the didn't-finish line counts every step");
    assert!(fr.contains("$(\"hm-first-end\").hidden = !(!fr.running && failed);"), "once the slot has ended");
    assert!(js.contains("The first run didn't finish. Knowlu will try again on its own."), "the line for a first slot that ended without a day");
    let data = strip_comments(&read("js/data.js"));
    assert!(data.contains("var FIRST_RUN_MS = 3000;") && data.contains("var POLL_MS = 60000;"), "three seconds while the first run lasts, else the minute");
}
