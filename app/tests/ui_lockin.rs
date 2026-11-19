//! T9, Lock in (v0.4.0 note §4 "Lock in", plan §5 T9, §6): pins on `js/pages/lockin.js` and
//! `css/lockin.css`. Red first: against T5's placeholders every test here fails.

#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

fn lockin_js() -> String { strip_comments(&read("js/pages/lockin.js")) }
fn lockin_css() -> String { strip_css_comments(&read("css/lockin.css")) }

/// Note §4: the queue is Home's order (Do today's rows, then Recommended's) from the chosen id
/// (`#lockin/<id>`, the router's argument), open rows only; the page never ranks, and stores
/// nothing: no storage, no setting, no command of its own (the one check is the kit's).
#[test]
fn lockin_queue_is_homes_order_and_stores_nothing() {
    let js = lockin_js();
    assert!(js.contains("KnowluRouter.add(\"lockin\""), "Lock in registers its route");
    let order = fn_body(&js, "function homeOrder(");
    let md = order.find("st.must_do").expect("Do today's rows");
    let rec = order.find("st.recommended").expect("then Recommended's");
    assert!(md < rec, "Do today first, then Recommended: {order}");
    assert!(order.contains("x.row"), "a Recommended entry's row: {order}");
    let from = fn_body(&js, "function queueFrom(");
    assert!(from.contains("homeOrder(st)") && from.contains("r.status !== \"done\""), "open rows in Home's order: {from}");
    assert!(from.contains(".slice(at)"), "from the chosen id: {from}");
    assert!(from.contains("return at < 0 ? open : open.slice(at);"), "an unknown id starts at the lead: {from}");
    let mount = fn_body(&js, "mount: function (");
    assert!(mount.contains("queueFrom(ctx.state, ctx.arg || \"\")"), "the chosen id is the route's argument: {mount}");
    assert!(!js.contains(".sort("), "the page never ranks");
    for no in ["localStorage", "sessionStorage", "indexedDB", "set_settings", "set_fields", "create_task"] {
        assert!(!js.contains(no), "Lock in stores something ({no})");
    }
    assert!(commands_sent(&js).is_empty(), "Lock in sends no command of its own: {:?}", commands_sent(&js));
}

/// Note §4: Done is the check (`KnowluKit.check`: `set_fields {progress: 100, status: "done"}`), and
/// the beat's Undo is its Undo (`KnowluKit.uncheck`). The ring on the field is the page's own button,
/// not the kit's `data-ck`, so one press writes once. Then the beat: what is left, the finished one,
/// the next one, Start next; with nothing left, the closing page and Home. End leaves for Home.
#[test]
fn lockin_done_is_check() {
    let js = lockin_js();
    let done = fn_body(&js, "function finish(");
    assert!(done.contains("KnowluKit.check(cur)"), "Done is the kit's check: {done}");
    assert!(done.contains("if (!env || !env.ok)"), "a refused check stays on the assignment: {done}");
    let undo = fn_body(&js, "function undo(");
    assert!(undo.contains("KnowluKit.uncheck("), "the beat's Undo is the kit's uncheck: {undo}");
    assert!(!js.contains("data-ck") && !js.contains("progress:") && !js.contains("\"progress\""), "no second check control, no write of its own");
    assert!(js.contains("closest(\"[data-lk-done]\")"), "Done and the ring are bound");
    for (hook, words) in [("data-lk-next", ">Start next<"), ("data-lk-break", ">Break now<"), ("data-lk-skip", ">Skip break<"), ("data-lk-hide", "Hide time")] {
        assert!(js.contains(hook) && js.contains(words), "the {words} control ({hook})");
    }
    assert!(js.contains("href=\"#home\"") && js.contains(">End<") && js.contains(">Home<"), "End and the closing page's Home go to #home");
    // the clocks, the break and the beat are the page's own and store nothing (note §4)
    assert!(js.contains("var FOCUS_MS = 25 * 60000;") && js.contains("var BREAK_MS = 5 * 60000;"), "a 25-minute stretch and a 5-minute break");
    // review M2: after each step the new primary control takes the focus
    for (f, want) in [
        ("function finish(", "focusOn(MODE === \"beat\" ? \"[data-lk-next]\" : \".lk-home\")"),
        ("function undo(", "focusOn(\".lk-acts [data-lk-done]\")"),
        ("function next(", "focusOn(\".lk-acts [data-lk-done]\")"),
        ("function takeBreak(", "focusOn(\"[data-lk-skip]\")"),
        ("function skipBreak(", "focusOn(\"[data-lk-hide]\")"),
    ] {
        assert!(fn_body(&js, f).contains(want), "{f} moves the focus ({want})");
    }
    let unmount = fn_body(&js, "unmount: function (");
    assert!(unmount.contains("clearInterval(TIMER)"), "the clock stops with the page: {unmount}");
}

/// DESIGN.md "Lock in", A8, D6: the page hides the sidebar on its own route and scales by one unit
/// held between .7 and 1.25; a finished assignment's mark fills with its class colour, never gray;
/// the soft field and the filled controls come from the slot table (`--soft`, `--do-bg`, `--nx-bg`).
#[test]
fn lockin_scales_hides_the_sidebar_and_fills_done_with_the_class_colour() {
    let css = lockin_css();
    assert!(css.contains("html[data-route=\"lockin\"] .side"), "the sidebar steps aside on Lock in");
    assert!(css.contains("clamp(.7px,") && css.contains("1.25px)"), "one unit, .7 to 1.25");
    for v in ["var(--soft)", "var(--do-bg", "var(--nx-bg"] {
        assert!(css.contains(v), "the slot table's {v}");
    }
    assert!(!css.contains("color-mix(in oklab, var(--cc)"), "no soft mix written by hand");
    let mut rules = 0usize;
    for (i, _) in css.match_indices(".lk-on") {
        let rule = &css[i..i + css[i..].find('}').unwrap_or(css.len() - i)];
        rules += 1;
        for gray in ["--ink-3", "--ink-2", "--line", "gray", "grey", "--shade-"] {
            assert!(!rule.contains(gray), "a finished mark is gray ({gray}): {rule}");
        }
    }
    assert!(rules >= 1, "the finished mark has its rule");
    // review M3: every page's CSS shares one document, so a rule on span.ck is Lock in's own
    assert!(css.contains(".lk-stage span.ck"), "the passive mark's rule is scoped");
    for (i, _) in css.match_indices("span.ck") {
        assert!(css[..i].ends_with(".lk-stage "), "an unscoped span.ck rule would reach other pages' marks");
    }
    for (i, _) in css.match_indices(".ck") {
        let rule = &css[i..i + css[i..].find('}').unwrap_or(css.len() - i)];
        for gray in ["--ink-3", "gray", "grey"] {
            assert!(!rule.contains(gray), "lockin.css repaints a check gray ({gray}): {rule}");
        }
    }
}
