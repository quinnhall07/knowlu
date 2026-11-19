//! T10, Tasks (v0.4.0 note §4 "Tasks", plan §5 T10): pins on `js/pages/tasks.js` and `css/tasks.css`.
//! Red first: against T5's placeholders every test here fails. The re-homed test keeps its old name
//! and behaviour (plan §7) and reads the new file.

#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

fn tasks_js() -> String { strip_comments(&read("js/pages/tasks.js")) }
fn tasks_css() -> String { strip_css_comments(&read("css/tasks.css")) }

/// The one line of `js` that starts (after its indent) with `head`.
fn line_starting<'a>(js: &'a str, head: &str) -> &'a str {
    js.lines().map(str::trim).find(|l| l.starts_with(head)).unwrap_or_else(|| panic!("tasks.js has no line starting {head:?}"))
}

/// Pin 7 (ruling 7): the default view has no Priority column, and its order is the engine's. The
/// columns are check, Task, Class, Due, Estimate; Sort offers priority (the default, the rows exactly
/// as `S.list.rows` gives them), due date, name, class and estimate, and nothing else: no row carries a
/// type (note §1 cuts the Type column to v0.5.0), so no Type column or sort is offered.
#[test]
fn tasks_default_view_has_no_priority_column_and_state_order() {
    let js = tasks_js();
    assert!(js.contains("KnowluRouter.add(\"tasks\""), "Tasks registers its route");
    let head = line_starting(&js, "var HEAD_HTML = ");
    for col in [">Task<", ">Class<", ">Due<", ">Estimate<"] {
        assert!(head.contains(col), "the column head names {col}: {head}");
    }
    for no in ["Priority", "Type", "Importance", "Rank"] {
        assert!(!head.contains(no), "no {no} column: {head}");
    }
    let sort = line_starting(&js, "var SORT_HTML = ");
    let values: Vec<&str> = sort.split("<option value=\"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
    assert_eq!(values, ["rank", "due", "name", "class", "estimate"], "the five sorts, priority first");
    assert!(!sort.contains("type") && !sort.contains("Type"), "no Type sort: no row carries a type");
    assert!(js.contains("var SORT = \"rank\";"), "priority is the default sort");
    let sorted = fn_body(&js, "function sorted(");
    assert!(sorted.contains("if (SORT === \"rank\") { return rows.slice(); }"), "priority keeps the engine's order unchanged: {sorted}");
    assert!(js.contains("st.list.rows"), "the rows are S.list.rows (the all view)");
    for no in ["importance", "rank_override", "urgency"] {
        assert!(!sorted.contains(no) && !fn_body(&js, "function cmp(").contains(no), "the page never ranks by {no} itself");
    }
}

/// Note §4: Open is grouped Late / Today / This week / Later / No date by the row's `due`, read
/// through the one date rule (`KnowluFmt.parse`, "now" from `KnowluFmt.now()`, which is `S.now`).
#[test]
fn tasks_groups_by_due() {
    let js = tasks_js();
    assert!(js.contains("var GROUPS = [\"Late\", \"Today\", \"This week\", \"Later\", \"No date\"];"), "the five groups, in order");
    let group = fn_body(&js, "function groupOf(");
    assert!(group.contains("KnowluFmt.parse(r.due)") && group.contains("KnowluFmt.now()"), "grouped by due against S.now: {group}");
    assert!(!js.contains("due_short") && !js.contains("new Date()") && !js.contains("Date.now()"), "no second clock or date text of the page's own");
    let open = fn_body(&js, "function openHtml(");
    assert!(open.contains("GROUPS.forEach(") && open.contains("groupOf(r) === g"), "each group lists its rows, in the sorted order");
    let due = fn_body(&js, "function dueHtml(");
    assert!(due.contains("KnowluFmt.due(r.due)"), "a due is said by the one date rule");
    assert!(due.contains("tk-late") && due.contains(">Late<"), "late is said in words, not colour alone");
    assert!(tasks_css().contains(".tk-late") && tasks_css().contains("var(--error)"), "and in the error colour");
}

/// I1 (re-homed F7 line, Tasks half): `S.list.unreadable` is a line above the list, with each path,
/// never hidden.
#[test]
fn tasks_renders_list_unreadable() {
    let js = tasks_js();
    assert!(js.contains("st.list.unreadable"), "Tasks reads S.list.unreadable");
    let line = fn_body(&js, "function unreadHtml(");
    assert!(line.contains("\"1 task note can't be read\"") && line.contains("\" task notes can't be read\""), "the count, singular at one: {line}");
    assert!(line.contains("paths.map(pathHtml).join(\"\")"), "every path is listed");
    let path = fn_body(&js, "function pathHtml(");
    assert!(path.contains("h(p)"), "a path goes through h()");
    let draw = fn_body(&js, "function draw(");
    assert!(draw.contains("UNREAD.hidden = !bad.length;"), "the line shows whenever a note is unreadable, whatever the tab or filter");
}

/// Re-homed (plan §7): Done lists `S.closed_this_week`, and Undo is offered only on a row whose `undo`
/// the engine set. Undo is the student's own `set_fields {status: "active"}`, its button held while
/// the write is out and given back on a refusal.
#[test]
fn closed_this_week_offers_undo_only_where_the_engine_says_so() {
    let js = tasks_js();
    assert!(js.contains("st.closed_this_week"), "Done reads S.closed_this_week");
    let row = fn_body(&js, "function closedHtml(");
    assert!(row.contains("data-undo-close=\""), "the Undo control");
    assert!(row.contains("k.undo && k.id ?"), "Undo only where the engine says undo (and the row has an id): {row}");
    assert!(row.contains("h(k.title)"), "the title through h()");
    assert!(!row.contains("ev("), "a closed row sends no event of its own");
    let undo = fn_body(&js, "function undoClose(");
    assert!(undo.contains("KnowluData.call(\"set_fields\", { id: id, fields: { status: \"active\" } })"), "Undo is set_fields status active: {undo}");
    assert!(undo.contains("btn.disabled = true;") && undo.contains("btn.disabled = false;"), "held while pending, given back on a refusal");
    assert!(undo.contains("KnowluKit.refusal(env)"), "a refusal is said");
    assert!(js.contains("closest(\"[data-undo-close]\")"), "the button is bound");
}

/// Note §4: New task is `create_task {fields: {title, course, due, effort_hours}}`; data.js adds the
/// view and adopts the answer, so the page passes neither.
#[test]
fn new_task_calls_create_task() {
    let js = tasks_js();
    let submit = fn_body(&js, "function submitNew(");
    assert!(submit.contains("KnowluData.call(\"create_task\", { fields: fields })"), "create_task with the fields: {submit}");
    for f in ["title: title", "course: ", "due: ", "effort_hours: "] {
        assert!(submit.contains(f), "the form sends {f}");
    }
    assert!(!submit.contains("view:"), "the view is data.js's to add");
    assert!(submit.contains("KnowluKit.refusal(env)"), "a refusal is said");
    let page = line_starting(&js, "var FORM_HTML = ");
    for name in ["name=\"title\"", "name=\"course\"", "name=\"due\"", "name=\"effort_hours\""] {
        assert!(page.contains(name), "the form has {name}");
    }
    assert!(js.contains("data-tk-new"), "the New task control");
}

/// Re-homed, Tasks half (plan §7): the page renders the engine's empty strings, never its own copy of
/// them: `S.list.empty_text` for Open, `S.texts.closed` for Done.
#[test]
fn the_page_renders_the_engines_empty_strings_not_its_own() {
    let js = tasks_js();
    for own in ["Nothing active", "Nothing is at risk", "Queue clear", "Nothing closed this week.", "No open issues", "Nothing to know right now."] {
        assert!(!js.contains(own), "tasks.js hardcodes {own:?}; it renders the engine's empty_text");
    }
    assert!(js.contains("st.list.empty_text"), "Open's empty line is S.list.empty_text");
    assert!(js.contains("st.texts.closed"), "Done's empty line is S.texts.closed");
}

/// A8 and plan §3: a row's check is the one kit control (`KnowluKit.checkHtml`), which fills with the
/// row's class colour; Tasks never repaints it ink or gray. A name opens Details (`data-det`, the
/// `task` panel), and the list holds a reorder while the pointer is over it (`data-rows`).
#[test]
fn tasks_rows_use_the_one_check_and_open_details() {
    let js = tasks_js();
    let row = fn_body(&js, "function rowHtml(");
    assert!(row.contains("KnowluKit.checkHtml(r)"), "the one check control");
    assert!(row.contains("data-det=\"") && row.contains("data-id=\"") && row.contains("data-kind=\"task\""), "a row opens Details and is observed as a task");
    assert!(row.contains("h(r.title)"), "the name through h()");
    assert!(!js.contains("set_fields\", { id: r.id") && !js.contains("progress: 100"), "Tasks writes no check of its own");
    assert!(js.contains("KnowluPanel.open(\"task\", "), "a name opens the Details panel");
    assert!(js.contains("data-rows"), "the list holds a reorder under the pointer");
    let css = tasks_css();
    for (i, _) in css.match_indices(".ck") {
        let rule = &css[i..i + css[i..].find('}').unwrap_or(css.len() - i)];
        for gray in ["--ink-3", "--ink-2", "--cc:", "--cc-edge:", "gray", "grey"] {
            assert!(!rule.contains(gray), "tasks.css repaints the check ({gray}): {rule}");
        }
    }
}

/// Note §4 and §8 (Waiting's "Needs a date" link): the class, due and search filters are page-local;
/// the due filter can be named by the route (`#tasks/none`), read as `ctx.arg`, never the address's
/// query.
#[test]
fn tasks_filters_are_page_local_and_the_route_names_the_due_filter() {
    let js = tasks_js();
    let due = line_starting(&js, "var DUE_HTML = ");
    let values: Vec<&str> = due.split("<option value=\"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
    assert_eq!(values, ["", "late", "today", "week", "none"], "any, Late, Today, This week, No date");
    assert!(js.contains("ctx.arg"), "the route's argument names a due filter");
    // Wave-A review M2: a due link filters that visit only; Tasks opened without one shows every due date.
    let mount = fn_body(&js, "mount: function (");
    assert!(mount.contains("DUE = ctx && DUE_ARGS.indexOf(ctx.arg) >= 0 ? ctx.arg : \"\";"), "mount resets the due filter: {mount}");
    let keep = fn_body(&js, "function keep(");
    for f in ["CLS", "DUE", "Q"] {
        assert!(keep.contains(f), "the {f} filter applies");
    }
    for no in ["localStorage", "location.search", "URLSearchParams", "location.hash ="] {
        assert!(!js.contains(no), "the filters stay on the page: {no}");
    }
}
