//! T14, Details (v0.4.0 note §5 "Task drawer", plan §5 T14): pins on `js/carried/details.js` and
//! `css/details.css`. The drawer's re-homed tests keep their names (plan §7 group "drawer") and read the
//! new file. Red first: against T5's placeholder every test here fails.

#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

fn details_js() -> String { strip_comments(&read("js/carried/details.js")) }

/// The names in a one-line `var NAME = [...]` (or `pub const NAME: [&str; N] = [...]`) list.
fn list_after(text: &str, head: &str) -> Vec<String> {
    let at = text.find(head).unwrap_or_else(|| panic!("no {head}"));
    let open = at + text[at..].find("= [").unwrap_or_else(|| panic!("{head} is not a list")) + 3;
    let close = open + text[open..].find(']').unwrap();
    text[open..close].split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect()
}

/// Every `ev(` line of `js`, trimmed.
fn ev_lines(js: &str) -> Vec<&str> { js.lines().filter(|l| is_ev_call(l)).map(str::trim).collect() }

/// Re-homed, Details half (R2-1): a repaint never replaces an open task's Details. The panel
/// re-reads its own note on a new state, by the id it was opened with, and never opens another; the
/// panel registers as the `task` kind.
#[test]
fn a_note_drawer_is_never_hijacked_by_a_grades_repaint() {
    let js = details_js();
    assert!(js.contains("KnowluPanel.add(\"task\", {"), "Details registers the task panel");
    let on = fn_body(&js, "KnowluData.on(function (");
    assert!(on.contains("load(CUR.id)"), "a new state re-reads the open note by its own id: {on}");
    assert!(!on.contains("KnowluPanel.open(") && !on.contains("CUR.id = "), "a repaint never switches the open note");
    let load = fn_body(&js, "function load(");
    assert!(load.contains("KnowluData.call(\"note\", { id: id })"), "the note command");
    assert!(load.contains("CUR.id !== id"), "an answer for a note no longer open is dropped");
}

/// Re-homed: the body is saved with `set_body {id, expected, body}`, `expected` being the body the panel
/// last read; only a note in tasks/ or courses/ offers the editor ("Add notes" when empty).
#[test]
fn the_drawer_saves_a_body_with_expected() {
    let js = details_js();
    assert!(js.contains("var BODY_FOLDERS = { tasks: 1, courses: 1 };"), "the two body folders (commands.rs BODY_EDITABLE_FOLDERS)");
    let commands = read_app("src/commands.rs");
    assert!(commands.contains("pub const BODY_EDITABLE_FOLDERS: [&str; 2] = [\"tasks\", \"courses\"];"), "the host's list is still those two");
    let notes = fn_body(&js, "function notesHtml(");
    assert!(notes.contains("BODY_FOLDERS") && notes.contains("data-body-edit") && notes.contains("\"Add notes\""), "the editor's way in: {notes}");
    let save = fn_body(&js, "function saveBody(");
    assert!(save.contains("KnowluData.call(\"set_body\", { id: id, expected: ed.expected, body: ed.ta.value })"), "set_body with expected: {save}");
    let open = fn_body(&js, "function openBodyEditor(");
    assert!(open.contains("data-open-editor") && open.contains("ta.value = expected;"), "the editor opens on the body it read");
    for key in ["\"Enter\"", "ctrlKey", "\"Escape\""] {
        assert!(open.contains(key), "Ctrl+Enter saves, Escape cancels ({key})");
    }
}

/// Re-homed: no usage event carries body text, or any typed value; a body's events name the note's id
/// and the word "body" only.
#[test]
fn no_ui_event_carries_body_text() {
    let js = details_js();
    let lines = ev_lines(&js);
    assert!(lines.len() >= 6, "Details sends its edit events ({lines:?})");
    for line in &lines {
        for bad in [".value", ".body", "ta.", ".text", ".title", "before", "expected", "fields"] {
            assert!(!line.contains(bad), "a usage event carries {bad}: {line}");
        }
    }
    for kind in ["\"edit_started\"", "\"edit_committed\"", "\"edit_cancelled\""] {
        assert!(lines.iter().any(|l| l.contains(kind) && l.contains("\"body\"")), "the body sends {kind} as \"body\"");
    }
}

/// Re-homed: a poll (any newly painted state) never rebuilds an open editor, a field's or the body's.
#[test]
fn a_poll_leaves_an_open_editor_alone() {
    let js = details_js();
    let on = fn_body(&js, "KnowluData.on(function (");
    assert!(on.contains("if (!CUR || CUR.edit || CUR.bodyEd) { return; }"), "an open editor holds the panel as it is: {on}");
    let draw = fn_body(&js, "function draw(");
    assert!(draw.contains("if (CUR.edit || CUR.bodyEd) { return; }"), "draw() never replaces an open editor");
}

/// Re-homed: Saved · Undo for ten seconds. Undo's `expected` is the body of the re-read note (never the
/// textarea) and its body is the text before the edit.
#[test]
fn body_undo_sends_the_reread_body_as_expected() {
    let js = details_js();
    let undo = fn_body(&js, "function offerBodyUndo(");
    assert!(undo.contains("KnowluData.call(\"note\", { id: id })"), "the note is re-read first");
    assert!(undo.contains("KnowluData.call(\"set_body\", { id: id, expected: env.note.body || \"\", body: before })"), "Undo's expected is the re-read body: {undo}");
    assert!(undo.contains("10000"), "ten seconds");
    assert!(undo.contains("savedtoast") && undo.contains("\"Undo\""), "the Saved toast and its Undo");
    assert!(undo.contains("\"copy_text\""), "a refused Undo offers the previous text to copy");
    let save = fn_body(&js, "function saveBody(");
    assert!(save.contains("offerBodyUndo(id, before)") && save.contains("var before = ed.expected;"), "Undo offers the body as it was before the save");
}

/// Re-homed: the Saved toast belongs to the panel it was raised from. Opening another note or closing
/// the panel drops it, and a toast only rises for the note still open.
#[test]
fn the_saved_toast_dies_with_its_drawer() {
    let js = details_js();
    let drop = fn_body(&js, "function dropSaved(");
    assert!(drop.contains(".savedtoast"), "dropSaved removes every Saved toast");
    let render = fn_body(&js, "render: function (");
    assert!(render.contains("dropSaved();"), "opening a note drops the last one's toast");
    let close = fn_body(&js, "close: function (");
    assert!(close.contains("dropSaved();") && close.contains("CUR = null;"), "closing the panel drops it");
    let undo = fn_body(&js, "function offerBodyUndo(");
    assert!(undo.contains("if (!CUR || CUR.id !== id) { return; }"), "no toast for a note no longer open");
}

/// Re-homed: a conflict keeps the draft: the editor stays, its text is never refilled, the sentence
/// says so, and Copy and Reload (which asks before it drops a changed draft) are offered.
#[test]
fn a_body_conflict_keeps_the_draft() {
    let js = details_js();
    let save = fn_body(&js, "function saveBody(");
    assert!(save.contains("if (env && env.conflict) {") && save.contains("conflictState("), "a conflict (never read from the words) keeps the editor: {save}");
    let conflict = fn_body(&js, "function conflictState(");
    assert!(!conflict.contains("ta.value =") && !conflict.contains(".value = "), "the draft is never refilled");
    assert!(js.contains("This note changed since you opened it. Your text is still here. Copy it, then reload to see the new version."), "the shipped sentence");
    assert!(conflict.contains("data-body-copy") && conflict.contains("data-body-reload"), "Copy and Reload");
    assert!(conflict.contains("KnowluData.call(\"copy_text\", { text: ed.ta.value })"), "Copy copies the draft");
    assert!(conflict.contains("window.confirm("), "Reload asks before it drops a changed draft");
}

/// Re-homed parts of `the_page_says_it_once_and_says_it_honestly` (plan §7): a one-sided range names the
/// bound that exists (never "from 0 to null"), and a progress of 100 closes the task in the same write.
#[test]
fn details_bounds_are_one_sided_and_progress_100_is_done() {
    let js = details_js();
    assert!(js.contains("var NUMERIC_RANGE = { importance: [1, 5, 1], progress: [0, 100, 5], effort_hours: [0, null, 0.5], slice_hours: [0, null, 0.5], rank_override: [null, null, null] };"), "the ranges");
    let bound = fn_body(&js, "function boundWords(");
    assert!(bound.contains("\" from \"") && bound.contains("\" at least \"") && bound.contains("\" at most \""), "each bound in its own words: {bound}");
    assert!(js.contains("function progressFields(pct) { var f = { progress: pct }; if (pct === 100) { f.status = \"done\"; } return f; }"), "progressFields");
    let finish = fn_body(&js, "function finishEdit(");
    assert!(finish.contains("progressFields(num)"), "a typed progress commits through progressFields");
    assert!(finish.contains("if (v === \"\") {"), "an emptied number is a cancel, never a null write");
}

/// A4: Start by, slack and "Won't fit" (with its reason) appear in Details, from the engine's row, and
/// nowhere else (Home and Tasks never show them). The row's `why` and a pending change's sentence too.
#[test]
fn details_shows_start_by_slack_and_wont_fit() {
    let js = details_js();
    let top = fn_body(&js, "function topHtml(");
    for (read, label) in [("row.start_by", "\"Start by\""), ("row.slack_days", "\"Slack\""), ("row.fit_text", "\"Won't fit\"")] {
        assert!(top.contains(read) && top.contains(label), "Details shows {label} from {read}: {top}");
    }
    assert!(top.contains("h(row.fit_why)"), "and the reason it won't fit");
    assert!(top.contains("h(row.why)"), "the row's why");
    assert!(top.contains("row.amend_badge"), "a pending change's sentence");
    for (f, text) in scripts() {
        if f == "js/carried/details.js" { continue; }
        let code = strip_comments(&text);
        for no in [".start_by", ".slack_days", ".fit_text", ".fit_why"] {
            assert!(!code.contains(no), "{f} shows {no}; it lives in Details only (A4)");
        }
    }
}

/// Note §5: Details edits the fourteen fields the host accepts (`EDITABLE`, app/src/commands.rs), and no
/// other; `kind` and `level` only on a commitment note; id, source_uid, also_uids and judgment are shown
/// read-only; click to edit, Enter or blur commits, Escape cancels (and keeps the panel open), each with
/// its event; a field write is `set_fields`; delete is `delete_note` behind confirm(); the flag is
/// `open_issue` with eight categories and free text.
#[test]
fn details_edits_the_fourteen_fields() {
    let js = details_js();
    let ours = list_after(&js, "var EDITABLE");
    let host = list_after(&read_app("src/commands.rs"), "pub const EDITABLE");
    assert_eq!(host.len(), 14, "the host accepts fourteen fields");
    assert_eq!(ours, host, "Details lists exactly the host's EDITABLE, in its order");
    assert_eq!(list_after(&js, "var COMMITMENT_ONLY"), ["kind", "level"], "kind and level are a commitment's");
    assert!(js.contains("n.folder === \"commitments\""), "and are shown only on a commitment note");
    assert_eq!(list_after(&js, "var READ_ONLY"), ["id", "source_uid", "also_uids", "judgment"], "the four read-only fields");
    let ro = fn_body(&js, "function roRowHtml(");
    assert!(!ro.contains("data-field"), "a read-only field takes no edit");
    let start = fn_body(&js, "function startEdit(");
    assert!(start.contains("ev(\"edit_started\", CUR.id, field);"), "edit_started");
    for key in ["\"Enter\"", "\"Escape\"", "\"blur\"", "stopPropagation()"] {
        assert!(start.contains(key), "click to edit: {key}");
    }
    let finish = fn_body(&js, "function finishEdit(");
    assert!(finish.contains("ev(\"edit_cancelled\", id, field);"), "edit_cancelled");
    let commit = fn_body(&js, "function commitEdit(");
    assert!(commit.contains("KnowluData.call(\"set_fields\", { id: id, fields: fields })"), "a field write is set_fields: {commit}");
    assert!(commit.contains("if (env && env.ok) {") && commit.contains("ev(\"edit_committed\", id, field);"), "edit_committed only on success");
    let del = fn_body(&js, "function archive(");
    let ask = del.find("window.confirm(").expect("delete asks first");
    assert!(ask < del.find("KnowluData.call(\"delete_note\", { id: id })").expect("delete_note"), "confirm() comes before delete_note");
    for cat in ["wrong-effort", "wrong-course", "duplicate", "should-not-exist", "wrong-tier", "wrong-date", "wrong-verdict", "other"] {
        assert!(js.contains(&format!("\"{cat}\"")), "the flag offers {cat}");
    }
    let flag = fn_body(&js, "function sendFlag(");
    assert!(flag.contains("KnowluData.call(\"open_issue\", { target: id, categories: cats, text: ta.value })"), "the flag is open_issue: {flag}");
    assert!(flag.contains("ev(\"issue_opened\", id, kind);"), "issue_opened");
    assert!(js.contains("<textarea"), "free text beside the categories");
    assert!(!js.contains("class=\"chip") && !js.contains("chips"), "toggles, never chips");
}
