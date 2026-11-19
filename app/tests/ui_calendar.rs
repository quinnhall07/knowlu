//! v0.4.0 T11 (T12 folded in): the Calendar's pins (note §4 "Calendar", ruling 3, A6, M3, N7; plan
//! §5 T11, §6, §7's three re-homed coming-up rows). Two files: the pure data module
//! `js/pages/calendar_data.js` (`KnowluCalData`) and the grid `js/pages/calendar.js`, with
//! `css/calendar.css`. Every pin reads only those three files (P3). The DOM check is
//! `scripts/ui-fixture/checks/t11.py`.
#[path = "support/ui_common.rs"]
mod ui_common;

use std::collections::BTreeSet;
use ui_common::*;

fn data_js() -> String { strip_comments(&read("js/pages/calendar_data.js")) }
fn grid_js() -> String { strip_comments(&read("js/pages/calendar.js")) }
fn cal_css() -> String { strip_css_comments(&read("css/calendar.css")) }

/// Both of T11's scripts, comments stripped.
fn both() -> String { format!("{}\n{}", data_js(), grid_js()) }

fn not_placeholder(rel: &str) {
    assert!(!is_placeholder(&read(rel)), "{rel} is still T5's placeholder: T11 replaces it");
}

// ---- the data module (KnowluCalData) -------------------------------------------------------------

/// N7, note §4: `your_week.week.commitments[]` expands per day (each `meets` entry's `days`, inside
/// the note's `from`/`until`), and a commitment's class comes from the one course lookup,
/// `KnowluCourses.match` (slug, else `compact(course)` in an entry's `codes`); with no match it draws
/// neutral. The page has no compact rule of its own.
#[test]
fn commitments_expand_per_day_and_match_by_codes() {
    not_placeholder("js/pages/calendar_data.js");
    let js = data_js();
    assert!(js.contains("window.KnowluCalData = "), "calendar_data.js publishes KnowluCalData");
    let expand = fn_body(&js, "function expand(");
    for f in ["meets", "days", "start", "end", "\"from\"", "\"until\""] {
        assert!(expand.contains(f), "expand reads each commitment's {f}");
    }
    assert!(js.contains("[\"mon\", \"tue\", \"wed\", \"thu\", \"fri\", \"sat\", \"sun\"]"), "the engine's day keys, Monday first");
    assert!(expand.contains("slotOf("), "each occurrence takes its class slot");
    let slot = fn_body(&js, "function slotOf(");
    assert!(slot.contains("KnowluCourses.match(") && slot.contains("KnowluCourses.slot("), "the slot comes from KnowluCourses.match, the N7 lookup");
    assert!(slot.contains("\"\""), "no match draws neutral (no slot)");
    let shape = strip_strings_and_comments(&both());
    assert!(!shape.contains("toUpperCase"), "no compact rule of the page's own: KnowluCourses.compact is the one");
    // The day keys come from the date itself: a level and a kind are carried, never guessed.
    for f in ["level", "kind", "title", "course"] {
        assert!(expand.contains(&format!(".{f}")), "an occurrence carries the commitment's {f}");
    }
}

/// M3: only `coming_up` entries with `accepted` true draw; the rest are not on the Calendar at all.
/// The grid reads `coming_up` only through the data module.
#[test]
fn only_accepted_coming_up_events_draw() {
    not_placeholder("js/pages/calendar_data.js");
    let js = data_js();
    let ev = fn_body(&js, "function accepted(");
    assert!(ev.contains("coming_up") && ev.contains(".accepted === true"), "accepted() keeps coming_up entries whose accepted is true");
    let grid = grid_js();
    assert!(!grid.contains(".coming_up"), "calendar.js reads coming_up only through KnowluCalData.accepted");
    assert!(grid.contains("KnowluCalData.accepted("), "the grid draws what accepted() keeps");
}

/// A6: today's unconfirmed events are `S.the_day.blocks` of kind `busy` that no commitment covers (a
/// confirmed commitment draws its own block in the_day too, labelled with its title, so it is never
/// drawn twice). Other days show none: `the_day` is today only.
#[test]
fn today_busy_blocks_not_covered_by_a_commitment() {
    not_placeholder("js/pages/calendar_data.js");
    let js = data_js();
    let un = fn_body(&js, "function unconfirmed(");
    assert!(un.contains("the_day") && un.contains("blocks") && un.contains("\"busy\""), "unconfirmed() reads the_day's busy blocks");
    assert!(un.contains("covered("), "a busy block a commitment covers is left out");
    let cov = fn_body(&js, "function covered(");
    assert!(cov.contains(".label") && cov.contains(".title"), "covered: the engine labels a commitment's block with its title");
    let grid = grid_js();
    let at = grid.find("KnowluCalData.unconfirmed(").expect("the grid draws unconfirmed() blocks");
    let line = grid[..at].rsplit('\n').next().unwrap_or("").to_string() + &grid[at..at + grid[at..].find('\n').unwrap_or(0)];
    assert!(line.contains("TODAY") || line.contains("isToday"), "unconfirmed blocks draw on today's column only: {line}");
}

/// Note §4: a day's load is `S.ahead.buckets[].hours` within the fourteen buckets; beyond them, the
/// summed `hours` of the open rows due that day; before today, none; zero shows no figure.
#[test]
fn day_load_from_ahead_buckets_then_due_hours() {
    not_placeholder("js/pages/calendar_data.js");
    let js = data_js();
    let load = fn_body(&js, "function dayLoad(");
    assert!(load.contains("ahead") && load.contains("buckets") && load.contains(".date") && load.contains(".hours"), "dayLoad reads the bucket for the date");
    assert!(load.contains("list") && load.contains("rows"), "beyond the buckets it sums the open rows due that day");
    assert!(load.contains("return null"), "no figure: before today, or nothing due");
    let grid = grid_js();
    assert!(grid.contains("KnowluCalData.dayLoad(") && grid.contains("KnowluFmt.minutes("), "the day's name carries the load as minutes, through fmt.js");
}

// ---- the grid -------------------------------------------------------------------------------------

/// Ruling 3: free navigation. T today, J and K the next and previous range, G go to date, Shift with
/// Left or Right one day, 1 to 7 the number of days, Ctrl with = or - or 0 the hour height. Below
/// 1000 px of center the page opens at 3 days (D6). The page registers itself as `calendar`.
#[test]
fn calendar_navigation_keys_zoom_and_days() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    assert!(js.contains("KnowluRouter.add(\"calendar\""), "the page registers its route");
    let keys = fn_body(&js, "function onKey(");
    for k in ["\"t\"", "\"j\"", "\"k\"", "\"g\"", "\"ArrowLeft\"", "\"ArrowRight\"", "shiftKey", "ctrlKey", "\"=\"", "\"-\"", "\"0\""] {
        assert!(keys.contains(k), "onKey handles {k}");
    }
    assert!(keys.contains("\"1\"") || keys.contains(">= \"1\""), "the digits pick the days shown");
    assert!(keys.contains("\"7\"") || keys.contains("<= \"7\""), "up to seven days");
    assert!(keys.contains("isOpen()"), "a key waits while a sheet is open");
    assert!(js.contains("var ZOOMS = ["), "the hour heights are named");
    let start = fn_body(&js, "function firstDays(");
    assert!(start.contains("1000") && start.contains("3") && start.contains("7"), "opens at 3 days below 1000 px of center, else the week");
    assert!(js.contains("\"wheel\""), "sideways pan: a sideways wheel moves by whole days");
}

/// The cut list (note §1): the Calendar is read-only in v0.4.0. It sends no command but the two reads;
/// nothing drags, resizes or creates; no C, Delete or Backspace key; the one write is the shared check
/// control (kit.js).
#[test]
fn calendar_is_read_only() {
    not_placeholder("js/pages/calendar.js");
    not_placeholder("js/pages/calendar_data.js");
    let js = both();
    let sent: BTreeSet<String> = commands_sent(&js).into_iter().collect();
    let allowed: BTreeSet<String> = ["your_week", "dropped_events"].iter().map(|s| s.to_string()).collect();
    assert!(sent.is_subset(&allowed), "the Calendar reads your_week and dropped_events only, sent {sent:?}");
    assert!(sent.contains("your_week"), "the meetings come from your_week");
    for banned in ["pointermove", "mousemove", "dragstart", "draggable", "setPointerCapture", "contenteditable"] {
        assert!(!js.contains(banned), "no drag, resize or editing: {banned}");
    }
    let keys = fn_body(&grid_js(), "function onKey(").to_string();
    for key in ["\"c\"", "\"C\"", "\"Delete\"", "\"Backspace\"", "\"z\""] {
        assert!(!keys.contains(key), "onKey handles no {key} (create, delete, undo are cut)");
    }
    for cmd in ["set_fields", "create_task", "commitments_confirm", "remove_lane_date", "decide", "answer_card"] {
        assert!(!js.contains(cmd), "the Calendar never writes: {cmd}");
    }
    let css = cal_css();
    for banned in ["ns-resize", "cursor: grab", "is-ghost", "is-mov"] {
        assert!(!css.contains(banned), "calendar.css draws no drag or resize affordance: {banned}");
    }
}

/// Note §4: the Due lane is `S.list.rows` by `due`; its check is the one shared control
/// (`KnowluKit.checkHtml`, whose done state fills with the class colour, A8), so check-off writes
/// `set_fields {progress: 100, status: "done"}` with Undo through kit.js and nowhere else. A due
/// name opens Details (the `task` panel); today's all-day lines join the lane.
#[test]
fn due_lane_check_off() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    let lane = fn_body(&js, "function laneHtml(");
    assert!(lane.contains("KnowluKit.checkHtml("), "each due row carries the shared check control");
    assert!(lane.contains("h(") && lane.contains(".title"), "the row's name goes through h()");
    assert!(js.contains("KnowluCalData.dueOn(") || js.contains("dueOn("), "the lane groups the open rows by their due day");
    let data = data_js();
    let due = fn_body(&data, "function dueOn(");
    assert!(due.contains("list") && due.contains("rows") && due.contains(".due"), "dueOn reads S.list.rows by due");
    assert!(js.contains("KnowluPanel.open(\"task\""), "a due name opens Details");
    assert!(js.contains("all_day"), "today's all-day lines are in the lane");
    assert!(!js.contains("progress") && !data.contains("progress:"), "no progress write of the page's own");
}

/// D4: the Calendar keeps its two stored choices, the hour height and the Due fold, in
/// `knowlu.cal.zoom` and `knowlu.cal.due`, and no others; storage that refuses lasts the visit.
#[test]
fn zoom_and_due_fold_in_storage() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    let keys: BTreeSet<String> = string_literals(&both()).into_iter().filter(|s| s.starts_with("knowlu.")).collect();
    let want: BTreeSet<String> = ["knowlu.cal.zoom", "knowlu.cal.due"].iter().map(|s| s.to_string()).collect();
    assert_eq!(keys, want, "the Calendar's two keys and no others");
    assert!(js.contains("localStorage.getItem(") && js.contains("localStorage.setItem("), "both are read at mount and written on change");
    assert!(js.matches("try {").count() >= 2, "a refused storage keeps the choice for the visit");
    let fold = fn_body(&js, "function setFold(");
    assert!(fold.contains("DUE_KEY"), "the fold is stored");
    let zoom = fn_body(&js, "function setZoom(");
    assert!(zoom.contains("ZOOM_KEY"), "the zoom is stored");
}

// ---- re-homed from static_assets.rs (plan §7) -----------------------------------------------------

/// Re-homed: "Not shown (N)" lists `dropped_events` (title, date, reason, each escaped), read-only; an
/// empty list draws nothing, a refusal is one quiet line; it is read when the page mounts, never on
/// every poll.
#[test]
fn coming_up_offers_a_not_shown_list_from_dropped_events() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    let ns = fn_body(&js, "function loadNotShown(");
    assert!(ns.contains(".call(\"dropped_events\""), "the page reads dropped_events");
    assert!(js.contains("Not shown (") , "a quiet control titled Not shown (N)");
    let draw = fn_body(&js, "function notShownHtml(");
    for f in ["d.title", "d.date", "d.reason"] {
        assert!(draw.contains(&format!("h({f}")), "{f} is escaped text");
    }
    assert!(ns.contains("!list.length") || ns.contains("list.length === 0"), "an empty list renders nothing");
    assert!(js.contains("Not shown is unavailable"), "a refusal is one quiet line");
    assert!(!draw.contains("<button") && !draw.contains("data-ck") && !draw.contains("ev("), "read-only: no write action");
    let mount = fn_body(&js, "mount: function (");
    assert!(mount.contains("loadNotShown("), "read when the page mounts");
    let update = fn_body(&js, "update: function (");
    assert!(!update.contains("loadNotShown(") && !update.contains("dropped_events"), "not on every poll");
}

/// Re-homed (M3): only accepted events draw; each says why it is there, the carry's provenance as the
/// engine gives it (escaped), else "Accepted".
#[test]
fn coming_up_shows_accepted_events() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    let b = fn_body(&js, "function eventHtml(");
    assert!(b.contains("h(e.provenance") && b.contains("\"Accepted\""), "the provenance, escaped, or Accepted");
    assert!(b.contains("h(e.title") , "the title is escaped");
    assert!(!js.contains(" title=\"") && !js.contains("title=\\\""), "no title= tooltip (ruling): data-tip");
    assert!(b.contains("data-tip"), "why it is there rides the shared tooltip");
}

/// Re-homed: the provenance is tried first; the plain "Accepted" is the fallback.
#[test]
fn coming_up_shows_the_carrys_provenance() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    let b = fn_body(&js, "function eventHtml(");
    let p = b.find("e.provenance").expect("reads the provenance");
    let a = b.find("\"Accepted\"").expect("the fallback");
    assert!(p < a, "provenance is tried first; plain Accepted is the fallback");
}

// ---- review fixes (wave A, M3 and M5) --------------------------------------------------------------

/// M3: every page's CSS loads into the one document, so every selector in `calendar.css` sits under
/// `.cal` (the page's root): `.day`, `.lane`, `.hr`, `.off`, `.now`, `.menu-wrap` and `.obtn.ico`
/// can restyle no other page. Each rule's selectors, inside `@media`/`@container` too.
#[test]
fn every_calendar_selector_sits_under_cal() {
    not_placeholder("css/calendar.css");
    let css = cal_css();
    let mut seen = 0;
    let mut start = 0;
    for (i, ch) in css.char_indices() {
        match ch {
            '{' => {
                let sel = css[start..i].trim();
                start = i + 1;
                if sel.is_empty() || sel.starts_with('@') { continue; }
                for s in sel.split(',').map(str::trim) {
                    seen += 1;
                    assert!(s == ".cal" || s.starts_with(".cal "), "calendar.css: {s:?} is not scoped under .cal");
                }
            }
            '}' | ';' => start = i + 1,
            _ => {}
        }
    }
    assert!(seen > 100, "the scan found the calendar's selectors ({seen})");
    for bare in [".day", ".lane", ".hr", ".off", ".now", ".menu-wrap", ".obtn.ico"] {
        assert!(css.contains(&format!(".cal {bare}")), "{bare} is styled, under .cal");
    }
}

/// M5: the grid's keys never fire while focus is in a text field, the right panel (`#panel`), the
/// sheet, the re-consent layer, or anywhere outside `#center` but the page itself.
#[test]
fn calendar_keys_stay_out_of_the_panel_sheet_layer_and_fields() {
    not_placeholder("js/pages/calendar.js");
    let js = grid_js();
    let on_key = fn_body(&js, "function onKey(");
    assert!(on_key.contains("elsewhere(e.target)"), "onKey asks where focus is first");
    let guard = fn_body(&js, "function elsewhere(");
    for s in ["input", "textarea", "select", "#panel", "#sheet", "#layer", "isContentEditable", "\"#center\"", "document.body"] {
        assert!(guard.contains(s), "the key guard covers {s}");
    }
}
