use std::fs;
use std::path::Path;

fn read(rel: &str) -> String { fs::read_to_string(Path::new("static").join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}")) }

#[test]
fn no_network_reference_in_the_shipped_page() {
    for f in ["index.html", "console.css", "console.js"] {
        let text = read(f);
        assert!(!text.contains("http://") && !text.contains("https://"), "{f} references the network");
    }
}

#[test]
fn the_three_deletions_stayed_deleted_and_the_tokens_are_all_there() {
    let css = read("console.css");
    assert!(!css.contains(".kb"), ".kb legend must be deleted, not hidden");
    assert!(!css.contains("content: \"8.7h\""), "the peak label must be a real element");
    assert!(!css.contains("p.foot") && !css.contains(".foot"));
    for token in ["--canvas", "--s1c", "--s2c", "--s3c", "--hair", "--hair-2", "--t1", "--t2", "--t3", "--t4", "--acc", "--acc-dim", "--crit", "--crit-dim", "--warn", "--warn-dim", "--r1", "--r2", "--r3", "--s1", "--s2", "--s3", "--s4", "--s5", "--s6", "--s7", "--s8", "--measure", "--shell", "--sans", "--mono"] {
        assert!(css.contains(&format!("{token}:")), "token {token} missing");
    }
    assert!(css.contains("color-scheme: dark"));
    assert!(css.contains(".ln.run") && css.contains("46px 62px"), "the .ln/.ln.run grid collision fix must be carried");
}

#[test]
fn every_font_the_page_names_is_vendored() {
    let css = read("console.css");
    for f in ["instrument-sans-400", "instrument-sans-500", "instrument-sans-600", "instrument-sans-700", "jetbrains-mono-400", "jetbrains-mono-500", "jetbrains-mono-600"] {
        assert!(css.contains(&format!("fonts/{f}.woff2")), "@font-face for {f}");
        assert!(Path::new("static/fonts").join(format!("{f}.woff2")).is_file(), "{f}.woff2 vendored");
    }
    assert!(Path::new("static/fonts/SOURCES.md").is_file());
}

#[test]
fn the_page_has_every_region_and_none_of_the_mock() {
    let html = read("index.html");
    for sel in ["class=\"topline\"", "class=\"delta\"", "id=\"fits\"", "id=\"spill\"", "id=\"overflow\"", "id=\"mustdo\"", "id=\"recommended\"", "id=\"main-list\"", "id=\"deck\"", "id=\"dcount\"", "class=\"ribbon\"", "id=\"coming-up\"", "id=\"gtk\"", "id=\"closed\"", "id=\"runs\"", "id=\"drawer\"", "id=\"theday\"", "id=\"allday\""] {
        assert!(html.contains(sel), "missing {sel}");
    }
    // R-P2-3 (rider, Knowlu plan 2 Task 3): every nav view now has a real renderer, so the
    // not-built placeholder is gone from the page, not merely hidden.
    assert!(!html.contains("id=\"main-notbuilt\""), "the not-built placeholder must be removed (R-P2-3)");
    assert!(!html.contains("class=\"kb\"") && !html.contains("class=\"foot\"") && !html.contains("data-when"));
    assert!(!html.contains("Career Fair Game Plan") && !html.contains("24.5 hours"), "mock content must be gone");
    assert!(html.contains("console.js") && html.contains("console.css"));
    for href in ["#today", "#overdue", "#week", "#later", "#all", "#decisions", "#good-to-know", "#issues", "#runs"] { assert!(html.contains(&format!("href=\"{href}\"")), "nav {href}"); }
}

#[test]
fn console_js_is_one_render_function_per_region_and_uses_invoke() {
    let js = read("console.js");
    for f in ["renderNav", "renderTopline", "renderDelta", "renderVerdict", "renderMeter", "renderMustDo", "renderRecommended", "renderList", "renderTheDay", "renderDeck", "renderAhead", "renderComingUp", "renderGoodToKnow", "renderClosed", "renderRuns", "route", "openDrawer", "poll"] {
        assert!(js.contains(&format!("function {f}(")), "missing function {f}");
    }
    assert!(js.contains("__TAURI__.core.invoke"));
    assert!(!js.contains("import ") && !js.contains("require("), "no modules, no bundler");
    // R28: route() must clear current.state too, or a view change's first paint can be
    // mistaken for a reorder (orderOf(stale today state) vs orderOf(new list state) always
    // differ) and the hold path leaves the new view's main column blank.
    assert!(js.contains("current.state = null"), "route must reset current.state (R28)");
    // R29/R30: the old whole-page early return — reordered gated on current.view === "today",
    // holding renderNav/Topline/TheDay/Deck/Ahead/ComingUp/GoodToKnow/Closed/Runs too — must be
    // gone; the hold now skips only the order-bearing regions (renderMustDo/Recommended/List).
    assert!(!js.contains("current.view === \"today\") { current.pendingOrder"), "the old whole-page reorder hold must be gone (R29/R30)");
    // F7/spec §12: an unreadable task note that would have been a must-do must not be silently
    // absent from Today — renderMustDo has to render the top-level state.unreadable too, not
    // only list.unreadable (which renderList already covers on `all`).
    assert!(js.contains("state.unreadable"), "renderMustDo must render state.unreadable (F7)");
}

#[test]
fn the_page_renders_the_engines_empty_strings_not_its_own() {
    let js = read("console.js");
    for own in ["Nothing active", "Nothing is at risk", "No spare capacity", "Queue clear", "No open issues", "No run recorded", "No events coming up.", "Nothing to know right now.", "Nothing closed this week."] {
        assert!(!js.contains(own), "console.js hardcodes \"{own}\" — it must render state.*.empty_text");
    }
    assert!(js.contains("empty_text"));
    // Final fix wave A6: those strings land in <div class="empty"> in the main column too, and
    // only the rail's copy was styled.
    let css = read("console.css");
    assert!(css.contains("main .empty {"), "the main column's empty states need the rail's look");
}

/// Plan 2 Task 7: the gauge is a strip of the last fourteen runs' runway beside the runway
/// figure. Nothing about it is computed on the page — the points, the direction word and the
/// no-history phrase are all payload, which is the whole reason the engine started recording
/// three counts on every run.
#[test]
fn the_gauge_renders_from_the_payload_and_invents_nothing() {
    let js = read("console.js");
    assert!(js.contains("function renderGauge("), "the gauge needs its own renderer");
    assert!(js.contains("no-history"), "the direction word the engine sends when there is no trend yet");
    // The element is emitted by renderTopline, not by index.html: that div's innerHTML is
    // rewritten on every paint, so a static span in the template would vanish on repaint.
    assert!(js.contains("id=\"gauge\""), "renderTopline must emit the gauge element");
    // F18: the phrase belongs to the engine. If the page ever spells it itself, the two drift.
    assert!(js.contains("state.empty.gauge"), "the no-history phrase is read from the payload");
    assert!(!js.contains("no history yet"), "console.js must not hardcode the engine's phrase");
    let css = read("console.css");
    assert!(css.contains(".gauge {"), "the strip needs its rule");
}

/// The final fix wave's page items: the health line is said once, bounds are honest, a refused
/// slot says why, a progress edit means the same thing wherever it is made, and no copy names a
/// slice of the plan.
#[test]
fn the_page_says_it_once_and_says_it_honestly() {
    let js = read("console.js");
    // C2: `engine newer than console` belongs to the sync line — the health line — and is
    // rendered there and nowhere else. renderTopline used to say it too, one line above, so the
    // same warning read as two problems.
    assert_eq!(js.matches("engine newer than console").count(), 1, "the stale-build warning is rendered exactly once");
    assert!(!js.contains("class=\"stale\""), "renderTopline's duplicate span is gone");
    assert!(!read("console.css").contains(".stale"), "…and so is its now-dead rule");
    // C4: effort_hours/slice_hours have a floor and no ceiling — the refusal must not read
    // "from 0 to null".
    assert!(js.contains("\" at least \"") && js.contains("\" at most \""), "a one-sided bound is spelled out");
    assert!(!js.contains("\" from \" + range[0] + \" to \" + range[1]"), "the two-sided-only formatting is gone");
    // C5: no time at all when the slot carries neither stamp; a refusal says refused, and why.
    assert!(js.contains("(when ? \" \" + when : \"\")"), "the time is omitted when there is none");
    assert!(js.contains("\" refused — \""), "a refused slot renders its reason after the state");
    // C6: one bundling function — the drag handler and click-to-edit (row chip and drawer dd) all
    // go through it, so typing 100 closes the task exactly as dragging to the end does.
    assert!(js.contains("function progressFields("), "progressFields");
    assert!(js.matches("progressFields(").count() >= 3, "declared once, used by the drag handler and by editField");
    assert!(js.contains("\"pointercancel\""), "a cancelled pointer ends the drag without committing");
    // C7: no user-visible copy names a slice — the writes slice is this plan, and the others are
    // simply not built.
    for stale in ["decide in the writes slice", "arrives with the third slice", "the next slice"] {
        assert!(!js.contains(stale), "stale copy names a slice: {stale}");
    }
}

#[test]
fn the_brand_is_knowlu_everywhere_a_user_reads_it() {
    let html = read("index.html");
    assert!(html.contains("<title>Knowlu</title>") && html.contains("<b>Knowlu</b>"), "index.html");
    assert!(!html.contains("quinn-ops"), "the old name must not be visible in the page");
    let conf = fs::read_to_string("tauri.conf.json").unwrap();
    assert!(conf.contains("\"productName\": \"Knowlu\"") && conf.contains("\"title\": \"Knowlu\""));
}

#[test]
fn the_deck_acts_and_the_flag_has_eight_chips_and_free_text() {
    let js = read("console.js");
    for f in ["bindDeck", "decideCard", "openFlag", "submitFlag", "closeInfoItem"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    for cmd in ["\"decide\"", "\"open_issue\"", "\"close_info\""] { assert!(js.contains(cmd), "{cmd}"); }
    for cat in ["wrong-effort", "wrong-course", "duplicate", "should-not-exist", "wrong-tier", "wrong-date", "wrong-verdict", "other"] { assert!(js.contains(cat), "{cat}"); }
    assert!(js.contains("<textarea"), "free text, always, beside the chips");
    assert!(!js.contains("disabled>Approve") && !js.contains("disabled>Reject"), "the deck's buttons are live now");
    assert!(js.contains("decision_made") && js.contains("decision_deferred") && js.contains("issue_opened"));
    let css = read("console.css");
    assert!(css.contains("prefers-reduced-motion") && css.contains(".deck .card.gone"));
}

#[test]
fn the_deck_guards_double_decide_and_flagpops_survive_a_repaint() {
    let js = read("console.js");
    assert!(js.contains("function preserveAcross("), "preserveAcross");
    // Review fix 1: a busy card is guarded both ways — decideCard refuses a second call, and
    // bindDeck ignores a click on a card already marked busy.
    assert!(js.contains("dataset.busy"), "the busy flag guards a fast double-click / IPC race");
    assert!(js.contains("card.dataset.busy) { return; }"), "bindDeck must ignore clicks on a busy card");
    // Review fix 2a: the same detach/rewrite/reattach pattern Task 13 used for .newrow now
    // covers .flagpop too, in every region that rewrites innerHTML wholesale.
    assert!(js.contains("preserveAcross(host, \".newrow, .flagpop\""), "renderMustDo keeps both");
    assert!(js.matches("preserveAcross(host, \".flagpop\"").count() == 5, "renderRecommended, renderList, renderDecisionsView, renderGoodToKnowView and renderIssuesView each keep .flagpop");
    let css = read("console.css");
    // Final fix wave A4: decideCard() marks whatever [data-id] element it was handed, so both
    // .gone rules cover the Decisions view's row as well as the deck's card.
    assert!(css.contains(".card.gone, .row.dec.gone { pointer-events: none; }"), "a .gone card or row must not stay hit-testable");
    assert!(css.contains(".deck .card.gone, .row.dec.gone { transform:"), "and both slide out the same way");
}

#[test]
fn the_page_can_write_and_never_relabels_to_confirm() {
    let js = read("console.js");
    for f in ["editField", "commitEdit", "cancelEdit", "bindProgressDrag", "renderNewTaskRow", "submitNewTask", "confirmDelete", "applyEnvelope", "showRefusal"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    for cmd in ["\"set_fields\"", "\"create_task\"", "\"delete_note\""] { assert!(js.contains(cmd), "{cmd}"); }
    assert!(js.contains("paint(env.state, true)"), "a write's returned state is painted with force (F19)");
    assert!(js.contains("Escape") && js.contains("Enter"), "blur/Enter commits, Escape cancels");
    assert!(js.contains("progress\": 100") || js.contains("progress: 100") || js.contains("pct === 100"), "reaching 100 sets status done in the same write");
    assert!(!js.contains("Confirm?") && !js.contains("Are you sure"), "one click commits; delete uses the confirm() dialog, not a relabel");
    assert!(js.contains("readonly") || js.contains("class=\"ro\""), "id/source_uid/also_uids/judgment are shown read-only");
    let html = read("index.html");
    assert!(html.contains("id=\"newtask\""), "the new-task button lives in MUST DO's header");
}

#[test]
fn seen_is_stamped_at_the_end_of_the_look_and_events_are_emitted() {
    let js = read("console.js");
    for f in ["renderSyncLine", "renderRunsView", "endOfLook", "ev", "watchSeen"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    assert_eq!(js.matches("\"mark_seen\"").count(), 1, "mark_seen is invoked from exactly one place");
    assert!(js.contains("\"blur\", endOfLook") && js.contains("visibilitychange"), "stamped on blur/hide (R37)");
    assert!(!js.contains("mark_seen\", {}).then") || !js.contains("route(location.hash.slice(1)).then(function () { if (current.state) { invoke(\"mark_seen\""), "not at first paint");
    for a in ["view_opened", "object_seen", "sync_run", "delta_expanded", "why_expanded"] { assert!(js.contains(a), "{a}"); }
    assert!(js.contains("IntersectionObserver") && js.contains("2000"));
    assert!(js.contains("\"sync\"") && js.contains("\"backup_now\""));
    // C7: Runs has a real renderer, and no copy names a slice any more (see
    // `the_page_says_it_once_and_says_it_honestly`).
    assert!(js.contains("runs: renderRunsView"), "Runs is built; it is not the not-built column");
    let html = read("index.html");
    assert!(html.contains("id=\"main-runs\"") && html.contains("id=\"syncline\""));
}

#[test]
fn sync_line_copy_is_verbatim_and_object_kind_is_never_guessed() {
    let js = read("console.js");
    // R-F1 (re-ruling of R-T15a): `topline.sync.ahead` is `rev-list --count HEAD...origin/main` —
    // a count of COMMITS. The plan's "N edits pending push" assumed a number the app never had, so
    // the copy now says commits and pluralises at 1.
    assert!(js.contains("commit\" + (s.ahead === 1 ? \"\" : \"s\") + \" pending push"), "R-F1: N commit / N commits pending push");
    assert!(!js.contains("edits pending push"), "R-F1: the edit-count copy is gone");
    // R-T15a still stands for the conflict line: flat copy, verbatim, count in the title only.
    assert!(js.contains("conflict — auto-sync stopped"), "R-T15a: the plan's exact flat copy");
    assert!(js.contains("title=\"conflict in "), "the per-note detail moves to a title attribute");
    // R-T15d: auto_sync and last_slot are read, not merely listed as consumed.
    assert!(js.contains("auto-sync off"));
    assert!(js.contains("last slot"));
    // Plan 2 Task 5 (F4): a retried slot names the attempt, so the same slot failing twice reads as
    // one slot being retried rather than two unrelated failures.
    assert!(js.contains("attempt "), "the last-slot line names the attempt on a retry");
    assert!(js.contains("t.auto_sync") && js.contains("t.last_slot"));
    // R-T15b: every [data-id] template carries a data-kind token the observer reads back
    // instead of guessing "task" for everything.
    assert!(js.contains("data-kind=\"task\""), "task rows");
    assert!(js.contains("data-kind=\"approval\""), "deck cards");
    assert!(js.contains("data-kind=\"' + h(n.folder)"), "the drawer carries the note's own kind");
    assert!(!js.contains("getAttribute(\"data-kind\") || \"task\""), "no guessed fallback to task");
    // C3: a flag raised on a deck card is an approval. `submitFlag` reads the kind off the host
    // element the popover is nested in, never the literal "task".
    assert!(!js.contains("ev(\"issue_opened\", targetId, \"task\""), "issue_opened must not hardcode task");
    assert!(js.contains("pop.closest(\"[data-kind]\")"), "submitFlag reads the host element's own kind");
    // R-T15c: watchSeen no longer blindly re-observes everything on every paint — a Set tracks
    // what is currently registered so a stale (detached) target is dropped, not accumulated.
    assert!(js.contains("seenObservedEls") && js.contains(".isConnected") && js.contains("unobserve("));
    assert!(!js.contains("disconnect()"), "R-T15c: never a wholesale disconnect (would reset an in-progress dwell)");
    // Minor: the out-of-range/non-numeric exit is a cancel too, and manual sync/backup swallow
    // an IPC rejection instead of leaving it unhandled.
    assert!(js.matches("ev(\"edit_cancelled\"").count() >= 5, "every no-write exit of editField fires edit_cancelled");
    assert!(js.contains("[data-sync]"));
    let sync_line_start = js.find("[data-sync]").unwrap();
    let backup_line_start = js.find("[data-backup]").unwrap();
    assert!(js[sync_line_start..sync_line_start + 220].contains(".catch(function () {})"), "manual sync swallows an IPC rejection");
    assert!(js[backup_line_start..backup_line_start + 220].contains(".catch(function () {})"), "manual backup swallows an IPC rejection");
}

#[test]
fn the_decisions_view_lists_every_pending_approval_and_acts_like_the_deck() {
    let js = read("console.js");
    assert!(js.contains("function renderDecisionsView("), "the view has its own renderer");
    assert!(js.contains("decisions: renderDecisionsView"), "and it is in VIEW_RENDERERS");
    assert!(js.contains("function decideCard(id, verdict, snoozeUntil, host)"), "decideCard is host-aware");
    assert!(js.contains("hidden_amendments"), "the header says how many amendments the deck hides");
    // F8: the view renders d.cards whole — no urgency filter anywhere in the renderer.
    let view = js.split("function renderDecisionsView(").nth(1).unwrap().split("\n  function ").next().unwrap();
    assert!(!view.contains("urgency !==") && !view.contains("urgency ==="), "no urgency filter in the view");
    assert!(view.contains("data-kind=\"approval\""), "rows are observed as approvals");
    assert!(view.contains("data-verdict=\"approved\"") && view.contains("data-verdict=\"rejected\"") && view.contains("data-verdict=\"snoozed\""));
    // The digest count reads honestly at one.
    assert!(!js.contains("\" events in today's digest\""), "no fixed plural");
    assert!(js.contains("=== 1 ? \" event\" : \" events\""), "singular at one");
    let html = read("index.html");
    assert!(html.contains("id=\"main-decisions\"") && html.contains("id=\"dec-list\""));
    // Final fix wave A1: .row's four tracks are the task-row's; every plan-2 row kind restates
    // its own, or the flag button lands in the 5px pip track and .acts in the 88px one.
    let css = read("console.css");
    assert!(css.contains(".row.dec, .row.gtk-row { grid-template-columns: 5px minmax(0,1fr) auto; }"), "flag / title / acts");
    // A3: the whole .acts strip stays in this view -- the note input included.
    assert!(js.contains("if (e.target.closest(\".row.dec .acts\")) { e.stopPropagation(); }"), "the Decisions .acts must not reach the document handler");
}

// Task 2's half of a test shared with Task 3 (its own step 1 says so); Task 3 adds the
// renderIssuesView/issues/data-kind="issue"/main-issues assertions and the "is not built yet"
// negation once Issues stops being the placeholder. The data-id/data-kind count equality holds
// at both points.
#[test]
fn the_good_to_know_and_issues_views_are_real_and_every_observed_row_names_its_kind() {
    let js = read("console.js");
    for f in ["renderGoodToKnowView", "renderIssuesView"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    assert!(js.contains("\"good-to-know\": renderGoodToKnowView"));
    assert!(js.contains("issues: renderIssuesView"));
    assert!(js.contains("data-kind=\"info\""), "R-T15b: the kind that had no template");
    assert!(js.contains("data-kind=\"issue\""), "R-T15b: issue rows");
    // Every template that stamps data-id stamps data-kind in the same string (R-T15b,
    // re-verified) — the leading space excludes CSS selectors like `[data-id="` and prose
    // mentions inside comments, neither of which is an observed-row template.
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
    // R-P2-3 (rider): every nav view now has a renderer — no view is a placeholder any more.
    assert!(!js.contains("is not built yet"), "no view is a placeholder any more");
    let html = read("index.html");
    assert!(html.contains("id=\"main-gtk\""));
    assert!(html.contains("id=\"main-issues\""));
    let css = read("console.css");
    // Final fix wave A1: an Issues row has no flag button, so it has no 5px first column either.
    assert!(css.contains(".row.iss { grid-template-columns: minmax(0,1fr) auto; }"), "title / acts");
    // A5: the category chips reuse .chip, which was styled only inside the flag popover.
    assert!(css.contains(".flagpop .chip, .row.iss .chip {") && css.contains(".flagpop .chip.on, .row.iss .chip.on {"), "an Issues chip must look like a chip");
    // A2: the document handler has a [data-close-info] branch of its own -- without this the
    // Good to know view's Close button invoked close_info twice per click.
    assert!(js.contains("function bindGoodToKnowView()"), "bindGoodToKnowView");
    let gtk = js.split("function bindGoodToKnowView()").nth(1).unwrap().split("\n  function ").next().unwrap();
    assert!(gtk.contains("e.stopPropagation();"), "one click, one close_info");
    // A3: same rule as Decisions, for the resolution input.
    assert!(js.contains("if (e.target.closest(\".row.iss .acts\")) { e.stopPropagation(); }"), "the Issues .acts must not reach the document handler");
    // A7: showRefusal(el, message, id, field) never reads `el`; the id is what prefixes the toast
    // with the note's title and re-finds the row after applyEnvelope's repaint.
    assert_eq!(js.matches("showRefusal(null, m, id)").count(), 2, "resolveIssue and closeInfoItem both pass the id");
    assert!(!js.contains("showRefusal(btn.closest(\".row\")"), "a detached row element says nothing");
}

#[test]
fn the_picker_is_in_the_page_and_carries_no_data_id() {
    let html = read("index.html");
    assert!(html.contains("id=\"picker\"") && html.contains("id=\"pick-list\"") && html.contains("id=\"pick-adopt\""));
    let js = read("console.js");
    assert!(js.contains("function bootConsole(") && js.contains("function renderPicker("));
    assert!(js.contains("invoke(\"launch_state\""), "the boot asks which window this is");
    // R-T15b's count equality still holds: picker rows are keyed by data-profile, never data-id.
    // **The leading space is the repo's spelling** (`app/tests/static_assets.rs:272`) — it excludes
    // CSS selectors like `[data-id="` and prose in comments. Copy it exactly; the unspaced form
    // counts things that are not row templates and the assertion stops meaning anything.
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
    assert!(js.contains("data-profile=\""));
    // Review round 1, IMPORTANT 1: `.app { display: grid }` beats the UA's `[hidden]`, so
    // `renderPicker`'s `.app.hidden = true` did nothing and the picker rendered below a
    // full-height empty console. This rule is the only thing that hides it.
    let css = read("console.css");
    assert!(css.contains(".app[hidden] { display: none; }"), "hiding .app needs a rule of its own");
    assert!(css.contains(".row.pick .acts { margin-left: auto; }"), "Open belongs at the right of the row");
}

#[test]
fn the_wizard_has_nine_panels_and_the_privacy_words_and_no_live_fetch() {
    let html = read("index.html");
    assert!(html.contains("id=\"wizard\""));
    // Spec §4.2, in order. **No `wiz-backup`, and no folder anywhere**: the app creates
    // `%USERPROFILE%\Knowlu\<name>` and `Backups` beside it (§4.1, §11a).
    for p in ["wiz-welcome", "wiz-account", "wiz-subscribe", "wiz-vault", "wiz-calendars", "wiz-logins", "wiz-gmail", "wiz-slots", "wiz-finish"] {
        assert!(html.contains(&format!("id=\"{p}\"")), "panel {p}");
    }
    assert!(!html.contains("id=\"wiz-backup\""), "the backup-folder panel is gone");
    assert!(!html.contains("id=\"wiz-pick-parent\"") && !html.contains("id=\"wiz-pick-bdir\""), "no folder is picked in the wizard");
    let js = read("console.js");
    for f in ["startWizard", "renderWizard", "wizGo", "wizFinish"] {
        assert!(js.contains(&format!("function {f}(")), "missing {f}");
    }
    assert!(js.contains("var PANELS = [\"welcome\", \"account\", \"subscribe\", \"vault\", \"calendars\", \"logins\", \"gmail\", \"slots\", \"finish\"];"));
    // No live fetch in onboarding: the ICS shape is matched, never requested, by the PAGE. The
    // escape keeps the network-reference rule true.
    assert!(js.contains("/^https:\\/\\/"), "the ICS check is an escaped regex");
    assert!(!js.contains("fetch(\"http"), "the page never fetches");
    // Credentials leave page memory the moment the write returns (spec §5).
    assert!(js.contains("clearCredentialFields("), "the fields are cleared by name");
    assert!(js.contains("window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker, openSettings: openSettings, openReport: openReport }"), "the shots seam");
    assert!(js.contains("retarget_credentials"), "Finish moves the credentials when the name changed");
    // R-C1-55, M5: two pins the brief's replacement body dropped, restored — both still matter.
    // A class `display` beats the UA stylesheet's `[hidden]`, and this rule is the only thing that
    // actually hides `#wiz-code-row`, `#wiz-school-free`, `#wiz-lms-kind` and `#wiz-google-row`.
    assert!(read("console.css").contains(".wiz-row[hidden] { display: none; }"), "a hidden .wiz-row must actually hide");
    // `documents` is the retired launch_state key that named the OneDrive-redirected folder.
    assert!(!js.contains("l.documents"), "the retired `documents` key is gone from the page");
    // D5: class (c) — raw note bodies for model improvement — is not built and has no UI, and the
    // page is where a toggle for it would appear. The pin predates C1 and is kept for exactly that.
    assert!(!js.to_lowercase().contains("telemetry"), "no telemetry toggle: (c) is not built (D5)");
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}

/// Spec §11a and VISION's standing rule: **Knowlu never asks for a campus credential.** The student
/// types into the university's own page, inside a window we opened and then throw away. The page has
/// exactly three password fields and they are all ours: the account's, and the two coursework logins
/// the student explicitly chose to store in Credential Manager (D11).
#[test]
fn the_page_has_no_lms_credential_field_anywhere() {
    let html = read("index.html");
    let js = read("console.js");
    // Every password field on the page is one of ours, by id: the two coursework logins the
    // student chose to store (D11). The account panel and the upgrade overlay carry no password
    // at all any more (spec D4). Counted by allow-list rather than by number, so adding one of
    // ours is fine and adding anybody else's is not.
    const OURS: [&str; 2] = ["wiz-zy-pass", "wiz-vhl-pass"];
    let mut seen = 0usize;
    for (i, _) in html.match_indices("type=\"password\"") {
        let around = &html[i.saturating_sub(200)..(i + 200).min(html.len())];
        assert!(OURS.iter().any(|id| around.contains(&format!("id=\"{id}\""))), "an unknown password field near: {around}");
        seen += 1;
    }
    assert_eq!(seen, 2, "the two coursework logins are the only passwords Knowlu ever asks for");
    for id in ["wiz-zy-pass", "wiz-vhl-pass"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "password field {id}");
    }
    // The LMS panel holds a button, a status line and a paste field — and nothing to type a school
    // password into. Checked over the panel's own markup, so a field added there fails here.
    let panel = html.split("id=\"wiz-calendars\"").nth(1).and_then(|s| s.split("id=\"wiz-logins\"").next()).expect("the calendars panel");
    assert!(!panel.contains("password"), "the calendars panel must never carry a password field");
    assert!(!panel.to_lowercase().contains("username"), "…nor a username field");
    assert!(panel.contains("id=\"wiz-lms-open\"") && panel.contains("id=\"wiz-ics\""), "sign-in button and paste fallback");
    // R-C1-42: the capture is the student's SECOND press, and it is its own button — chained onto the
    // open it would read the identity provider's page, where nobody has signed in yet.
    assert!(panel.contains("id=\"wiz-lms-capture\""), "the capture button");
    assert!(js.contains("\"capture_calendar_link\"") && js.contains("wiz-lms-capture"),
        "the capture fires on the press, not on the open");
    // The campus is asked HERE, on the panel that uses it — not two panels later, where it used to be
    // and where it made every sign-in answer "no sign-in page is known for that school yet".
    // R-OB-4: a search over every US institution, not two radios. The radios are gone from the whole
    // page — a list of two schools was a placeholder that read like a decision.
    assert!(panel.contains("id=\"wiz-school\"") && panel.contains("id=\"wiz-school-hits\""), "the school typeahead");
    assert!(panel.contains("id=\"wiz-school-none\"") && panel.contains("id=\"wiz-school-free\""), "…and the free-text fallback");
    assert!(panel.contains("id=\"wiz-lms-kind\""), "…and the two-button LMS question for an uncurated school");
    assert!(!html.contains("name=\"campus\""), "no campus radios anywhere on the page");
    assert!(!js.contains("input[name=\\\"campus\\\"]"), "…and nothing reads one");
    assert!(js.contains("function schoolHits(") && js.contains("\"campus_search\""), "the typeahead asks Rust");
    // **The page must not fetch the list.** `app/tauri.conf.json`'s CSP names only the IPC origin, so
    // a `fetch` of a bundled asset is refused — and that file is the controller's. The search is a
    // command; the page holds ten rows.
    assert!(!js.contains("campuses.json"), "the page never names the asset; `campus_search` reads it");
    // Spec §11a: **both** calendars, on this one panel, before coursework logins and Gmail — the
    // personal one is what makes today's page know the day is already half full.
    assert!(panel.contains("id=\"wiz-cal-ics\"") && panel.contains("id=\"wiz-cal-note\""), "the personal calendar's field");
    assert!(panel.contains("Secret address in iCal format"), "the panel says where the address is");
    assert!(panel.contains("Reset"), "…and advises resetting it first");
    // C2's Google sign-in is live: the calendar scope only, ordered ahead of Gmail (§11a).
    assert!(panel.contains("id=\"wiz-google\""), "the Google sign-in is on the calendars panel");
    assert!(!panel.contains("id=\"wiz-google\" disabled"), "…and is live from C2 on");
    // R-OB-2: the enrolled classes are confirmed on this panel — captured from the sign-in window if
    // the campus lets us, typed if it does not. Without them a first ingest is 28 tasks with no
    // course, which is the run this section of the plan exists because of.
    assert!(panel.contains("id=\"wiz-courses\"") && panel.contains("id=\"wiz-course-rows\""), "the class list");
    assert!(panel.contains("id=\"wiz-course-add\""), "…and the typed fallback beside it");
    assert!(js.contains("\"capture_courses\"") && js.contains("function renderCourses("), "the capture and its rows");
    // A-6: relabelled to what this actually guards. C2 shipped Google connect (calendar scope
    // only, via `google_connect_url`) — the stale message called that "no Google connect in C1",
    // which stopped being true at C2 and would have kept passing for the wrong reason forever.
    // What is still true, and what this proves: no `gmail.readonly` scope string anywhere on the
    // wizard page (the incremental Gmail consent is a later, separate step — not this branch), and
    // no leftover pre-C2 `connect_google` command name.
    assert!(!js.contains("\"connect_google\"") && !js.contains("gmail.readonly"), "no Gmail scope on the wizard page until C4");
    let slots = html.split("id=\"wiz-slots\"").nth(1).and_then(|s| s.split("id=\"wiz-finish\"").next()).expect("the slots panel");
    assert!(!slots.contains("id=\"wiz-school\""), "the school must not also be on the slots panel");
}

/// C2 final review A-5 (m59+m60): the Google flow's own state lives on `WIZ`, rendered by
/// `renderWizard()` like every other field — never a bare module-level flag a direct DOM write can
/// desync from the next repaint, and never a poll that outlives the panel it started on.
#[test]
fn the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it() {
    let js = read("console.js");
    let html = read("index.html");

    // The flag and its note are WIZ fields, not a bare module-level variable a direct DOM write
    // could desync from the next `renderWizard()` call.
    assert!(js.contains("google: false") && js.contains("googleNote:"), "WIZ carries the Google flow's own state");
    assert!(!js.contains("var WIZ_GOOGLE"), "the old bare flag must be gone, not merely unused");

    // A dedicated note element — never the personal calendar field's `wiz-cal-note`, which the
    // original click handler wrote into by mistake.
    assert!(html.contains("id=\"wiz-google-note\""), "the Google status line has its own element");
    assert!(js.contains("EL(\"wiz-google-note\").textContent = WIZ.googleNote"), "renderWizard paints it");

    // The button's disabled state is computed from WIZ on every render — connected OR mid-poll —
    // not set once, directly, and left for a later render to forget.
    assert!(
        js.contains("EL(\"wiz-google\").disabled = WIZ.google || WIZ.googlePolling"),
        "the button's disabled state is derived from WIZ state on every render"
    );

    // Leaving the calendars panel (step 4) cancels a poll in flight — the same cancellation-token
    // shape `WIZ.schoolSeq` uses for the typeahead — so a stale timer never writes onto a panel the
    // student is no longer looking at.
    assert!(js.contains("googleSeq"), "a cancellation token exists");
    let go = js.find("function wizGo(").map(|i| &js[i..]).expect("wizGo");
    assert!(
        go.find("WIZ.googleSeq").map(|i| i < go.find("function wizFinish(").unwrap_or(usize::MAX)).unwrap_or(false),
        "wizGo itself bumps the token on leaving the panel"
    );

    // wizFinish re-reads the truth with one more `google_connected` call, falling back to the
    // polled flag only if THAT call fails — a consent that finished after the last poll tick must
    // still birth the vault with the `cloud:google` entry.
    let finish = js.find("function wizFinish(").map(|i| &js[i..]).expect("wizFinish");
    let google_connected_call = finish.find("invoke(\"google_connected\"");
    let plan_build = finish.find("google_calendar:");
    assert!(google_connected_call.is_some(), "wizFinish re-checks google_connected");
    assert!(
        google_connected_call.unwrap() < plan_build.expect("the plan is built somewhere in wizFinish"),
        "the re-check happens BEFORE the plan is built, not after"
    );
    assert!(finish.contains("WIZ.google") , "the polled flag is still read, as the fallback");
    assert!(!js.contains("google_calendar: WIZ_GOOGLE"), "the plan no longer reads the old bare flag directly");

    // R2-3: the button is disabled as the FIRST statement of wizFinish, before the `google_connected`
    // await — not after it. Two Finish clicks landing in that window used to start two
    // `retarget_credentials`/`wizRegister` flows racing each other.
    let busy_write = finish.find("WIZ.busy = true").expect("wizFinish latches WIZ.busy");
    assert!(
        busy_write < google_connected_call.unwrap(),
        "wiz-next is disabled before the google_connected await, not after it"
    );
}

/// R-OB-1: the wizard that takes a coursework password must also say what the work is for. Quinn's
/// first slot had both logins stored and `courses: {}` in the config, so the engine answered
/// `zybook UACS100Fall2026 not in config; skipped` and then `0 assignments parsed; treating as
/// failure` — three warnings for one missing sentence.
#[test]
fn the_logins_panel_maps_what_it_finds_to_a_course() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-logins\"").nth(1).and_then(|s| s.split("id=\"wiz-gmail\"").next()).expect("the logins panel");
    assert!(panel.contains("id=\"wiz-map\"") && panel.contains("id=\"wiz-map-rows\""), "the mapping block");
    let js = read("console.js");
    assert!(js.contains("\"discover_coursework\""), "discovery runs after the credentials are stored");
    assert!(js.contains("function renderMapping("), "renderMapping");
    // The mapping travels in the plan, and the SLUGS are made in Rust from the codes — a page that
    // invented vault identifiers would be a page deciding what the engine may know.
    assert!(js.contains("zybooks_courses:") && js.contains("vhl_sections:") && js.contains("course_map:"), "the plan carries the mapping");
    assert!(!js.contains("slugify"), "slugs are `knowlu_engine::ingest::slugify`'s, never the page's");
    // R-C1-55, I3: `discover_coursework` spawns the engine and logs into both vendors. A second Next
    // while the first is in flight starts a SECOND child, and the two `WIZ.map` assignments decide the
    // panel between them. Asserted inside `wizGo` itself, so `wizFinish`'s own disable cannot stand in.
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo");
    assert!(go.contains("WIZ.discovering"), "a second Next must not start a second coursework-discover");
    assert!(go.contains("WIZ.busy = true") && go.contains("WIZ.busy = false"),
        "Next is disabled while discovery is in flight and re-enabled when it settles");
}

/// D5 and D6: the mapping row offers the classes the wizard already captured rather than asking
/// for a code from memory, a row with nothing to offer says what it needs, and leaving one blank is
/// a choice whose consequence the panel states before Next goes on. The first live onboarding left
/// the VHL row blank, wrote `sections: {}` and spent the whole next run warning about it.
#[test]
fn a_mapping_row_offers_the_captured_classes_and_says_what_a_blank_one_costs() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-logins\"").nth(1).and_then(|s| s.split("id=\"wiz-gmail\"").next()).expect("the logins panel");
    assert!(panel.contains("<datalist id=\"wiz-course-codes\">"), "the panel keeps one datalist");
    let js = read("console.js");
    assert!(js.contains("function renderCourseCodes("), "renderCourseCodes fills it");
    assert!(js.contains("list=\"wiz-course-codes\""), "every mapping row's field reads it");
    // The list offers what round-trips to the vault's own name for the course — the human code the
    // capture read, or the slug — never the LMS's opaque key, which would be slugged into a course
    // note nobody has (R-C1c-plan-2).
    assert!(js.contains("c.label || c.slug"), "the datalist offers the human code, and the slug otherwise");
    assert!(js.contains("type the course this belongs to"), "a row with no suggestion says what it needs");
    assert!(js.contains(" of these will be asked about in the app"), "…and Next says what blank rows cost");
    assert!(js.contains("function noteUnmapped("), "noteUnmapped");
    // R-C1c-plan-3: the sentence is read BEFORE the panel goes — the first Next latches and stays,
    // the second goes on. Asserted over the `wizGo`/`wizStep` slice (the split runs to
    // `wizRegister`, so it spans both), because `wizFinish`'s own bookkeeping must not stand in.
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo/wizStep");
    assert!(go.contains("noteUnmapped()"), "the count is written on the way out of the panel");
    assert!(go.contains("WIZ.mapWarned"), "…and the panel stays once, so the student reads it");
    assert!(js.contains("mapWarned: false"), "the latch starts clear on a fresh wizard");
    assert!(
        js.contains("WIZ.discovered = false; WIZ.mapWarned = false"),
        "…and re-typed logins clear it with the discovery they invalidate"
    );
    // R-C1b-exec-10 still holds: the sentence is a note, never a refusal — the second Next goes on
    // whatever the rows say, and nothing writes an error for a blank one.
    assert!(!go.contains("WIZ.error = \"Map"), "a blank row must never block Next");
}

/// Final review, I2: `wizFinish` must put only the rows the student explicitly ticked as ignored
/// into `zybooks_ignore` — never a blank row. A blank zyBooks row left out of both `courses` and
/// `ignore` is `BookRouting::Unmapped` (`route_zybook`, `engine/src/coursework.rs`) and the cloud
/// handler's mirror (`routeZybook`, `cloud/supabase/functions/ingest-coursework/parse_zybooks.ts`)
/// files a coursework-map card for it — the same path a blank VHL row already takes. Putting a
/// blank row in `zybooks_ignore` instead makes it silently skipped forever and makes
/// `noteUnmapped`'s "N of these will be asked about in the app" false for zyBooks.
#[test]
fn a_blank_zybooks_row_is_never_sent_as_ignored() {
    let js = read("console.js");
    let finish = js.find("function wizFinish(")
        .map(|i| &js[i..])
        .and_then(|s| s.split("\n  // The Checkout page").next())
        .expect("wizFinish");
    assert!(
        finish.contains("r.source === \"zybooks\" && r.ignore; }"),
        "zybooks_ignore takes only rows the student ticked as ignored"
    );
    assert!(
        !finish.contains("r.source === \"zybooks\" && (r.ignore || !r.course)"),
        "a blank, un-ignored zyBooks row must not reach zybooks_ignore"
    );
}

/// Final review, I4 (a C1 Task 17 bug predating this branch, undoing part of D4): a captured
/// course already carries its own slug and is covered by `course_fragments`
/// (`app/src/scaffold.rs`) — sending its LMS id into `course_map` a second time with an empty
/// slug lets `create_vault_in` fill it with `slugify(<LMS id>)`, and because `course_map_lines` is
/// first-wins with the page's entries first, that phantom slug wins over the real one. Only a
/// TYPED course (`slug: ""`) belongs in this list.
#[test]
fn a_captured_courses_lms_id_never_gets_a_phantom_slug() {
    let js = read("console.js");
    let finish = js.find("function wizFinish(")
        .map(|i| &js[i..])
        .and_then(|s| s.split("\n  // The Checkout page").next())
        .expect("wizFinish");
    assert!(
        finish.contains("if (c.code && !c.slug)"),
        "only a typed course (no slug of its own) contributes its code to course_map"
    );
}

/// D7 / §6, re-ruled by R-C1c-8: the minute between Finish and the first `rank` is a status view of
/// its own that REPLACES the day. `.app` carries `first-run` while the envelope carries the block,
/// and the stylesheet hides the nav, the rail and everything in `main` but the view. The sentence is
/// the page's, the list fills from the live slot and names five steps in plain words, a Settings
/// button stands in for the hidden topline gear, and a first slot that ended without a day says so.
/// The page asks again every three seconds until the day is there, then goes back to its cadence.
#[test]
fn the_first_run_view_says_what_is_happening_and_polls_until_the_day_arrives() {
    let html = read("index.html");
    let block = html.split("id=\"first-run\"").nth(1).and_then(|s| s.split("id=\"main-today\"").next()).expect("the first-run block");
    assert!(
        block.contains("Knowlu is doing its first run. Your day appears here in about a minute."),
        "the sentence D7 asks for, in the page rather than in a string the engine sends"
    );
    assert!(block.contains("id=\"first-run-steps\""), "…the list the live slot fills");
    assert!(
        block.contains("The first run didn't finish. Knowlu will try again on its own."),
        "…and the line for a first slot that ended without a day"
    );
    // The topline gear is hidden with the rest of `main`, so the view carries its own way to Settings,
    // routed by the document's existing `[data-settings]` delegation.
    let at = block.find("data-settings").expect("a Settings button inside the first-run view");
    let tag = block[..at].rfind("<button").expect("…on a button");
    assert!(!block[tag..at].contains('>'), "data-settings is an attribute of that button");
    // Its own treatment: never dressed as the day's lede or a row's metadata (the Task 5 defect).
    assert!(!block.contains("lede") && !block.contains("class=\"meta\""), "the view borrows no day styles: {block}");

    let js = read("console.js");
    assert!(js.contains("function renderFirstRun("), "renderFirstRun");
    assert!(js.contains("function hideFirstRun("), "hideFirstRun");
    assert!(js.contains("var FIRST_RUN_MS = 3000"), "the first-run cadence is three seconds");
    assert!(js.contains("setInterval(poll, 60000)"), "…and the usual cadence is unchanged");
    assert!(js.contains("EL(\"first-run\").hidden = true"), "…and the block is hidden once the day is there");
    assert!(
        js.contains(".classList.add(\"first-run\")") && js.contains(".classList.remove(\"first-run\")"),
        "the view replaces the day by a class on .app, set with the block and taken off with it"
    );
    for say in [
        "Checking your account",
        "Fetching your coursework",
        "Reading your school calendar",
        "Working out what each task needs",
        "Putting your day in order",
    ] {
        assert!(js.contains(&format!("\"{say}\"")), "the step sentence {say:?}");
    }
    // R-C1c-final2 M2: C3′ makes `sync` a slot step; its sentence is here before the merge, so the
    // first seconds of a merged first run are not a view with nothing in progress.
    assert!(js.contains("sync: \"Syncing with your account\""), "the sync step's sentence");
    let render = js.split("function renderFirstRun(").nth(1).and_then(|s| s.split("function hideFirstRun(").next()).expect("renderFirstRun");
    assert!(render.contains("fr.current"), "the step in progress is the live slot's own");
    // R-C1c-final2 M1: a failed account check lands at code 0 (a network is not a failed slot), and
    // is shown as what it is — the failed mark and a short note — never as a check mark.
    assert!(render.contains("String(s[0]).indexOf(\"entitlement (refresh failed\") === 0"), "the failed account check is recognised by name");
    assert!(render.contains("\"couldn't check — will retry\""), "…and says so in a short note");
    // R-C1c-exec-8a (M2): an ended slot with ANY failed step, listed or not (`engine: …` at -1 is
    // not), says it did not finish.
    assert!(
        render.contains("var failed = (fr.steps || []).some(function (s) { return s[1] !== 0; });"),
        "the didn't-finish line counts every step, listed or not"
    );
    assert!(render.contains("EL(\"first-run-end\").hidden = !(!fr.running && failed);"), "…once the slot has ended");
    assert!(!render.contains("class=\"meta\"") && !render.contains("lede"), "the rows borrow no day styles");
    // R-C1c-exec-8a (I1): leaving the view forgets the displayed day, as a view change does, so the
    // first ranked state paints whole instead of holding the pre-slot order behind "refresh order".
    // Only on the way OUT: clearing on every poll without the block would defeat the hold for good.
    let hide = js.split("function hideFirstRun(").nth(1).and_then(|s| s.split("function poll(").next()).expect("hideFirstRun");
    assert!(hide.contains("if (app.classList.contains(\"first-run\")) {"), "the reset runs only on the transition out");
    assert!(hide.contains("current.state = null; current.revision = null; current.pendingOrder = null;"), "…and forgets the displayed day");
    let poll = js.split("function poll(").nth(1).and_then(|s| s.split("function openDrawer(").next()).expect("poll");
    // R-C1c-plan-1: the view stands on the presence of the block — which is `is_first_run` — and
    // never on a failed state, because `surface::build_state` has no failure path to wait for.
    assert!(poll.contains("if (env.first_run) {"), "the first-run view stands on is_first_run alone");
    assert!(poll.contains("renderFirstRun("), "…poll paints it");
    assert!(poll.contains("hideFirstRun()"), "…and takes it away when the key stops coming");
    // R-C1c-exec-8a (M1): a rejected call re-arms the cadence while the view stands, since the error
    // line it writes is hidden in this mode.
    assert!(
        poll.contains("if (document.querySelector(\".app\").classList.contains(\"first-run\")) { armFirstRun(); }"),
        "a rejected state call keeps the first-run cadence going"
    );

    let css = read("console.css");
    for sel in [".app.first-run > nav", ".app.first-run > aside", ".app.first-run > main > :not(#first-run)"] {
        assert!(css.contains(sel), "{sel} is hidden while the first-run view stands");
    }
}

/// Spec §4.2 step 1 and §9's minors row: one attestation, one acceptance, both linked to the text.
#[test]
fn the_account_panel_leads_with_google_asks_for_no_password_and_still_gates_on_eighteen() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-account\"").nth(1).and_then(|s| s.split("id=\"wiz-subscribe\"").next()).expect("the account panel");
    // Spec D1: the Google button is the FIRST control on the panel, not an alternative buried
    // under a form. Proved by position, because "present" is not the claim.
    let g = panel.find("id=\"wiz-google-signin\"").expect("the Google button");
    let e = panel.find("id=\"wiz-email\"").expect("the email field");
    assert!(g < e, "Continue with Google comes before the email field");
    assert!(panel.contains("Continue with Google"), "…and says so in words");
    // Spec D4: no password, anywhere on this panel or in the overlay.
    assert!(!panel.contains("password"), "the account panel must never carry a password field again");
    assert!(!html.contains("id=\"wiz-pw\"") && !html.contains("id=\"up-pw\""), "both account passwords are gone");
    // One button for the email path. The create/sign-in split went with the password.
    assert!(panel.contains("id=\"wiz-magic\"") && panel.contains("Email me a code"), "one button, and it says what it does");
    assert!(!panel.contains("id=\"wiz-create\"") && !panel.contains("id=\"wiz-signin\""), "no create/sign-in split");
    assert!(panel.contains("id=\"wiz-code-row\"") && panel.contains("id=\"wiz-code-go\""), "the code row stays");
    // §9's minors row and the consent log are unchanged: both boxes, both policies, both gates.
    assert!(panel.contains("id=\"wiz-18\"") && panel.contains("18 or older"));
    assert!(panel.contains("id=\"wiz-terms\"") && panel.contains("terms.html") && panel.contains("privacy.html"));
    let js = read("console.js");
    assert!(js.contains("\"google_sign_in\"") && js.contains("\"send_magic_link\"") && js.contains("\"verify_email_code\""));
    assert!(!js.contains("\"sign_up\"") && !js.contains("\"sign_in\""), "the password commands are gone from the page too");
    // Tauri v2 camel-cases argument keys; `age_attested` here is a rejected invoke and a wizard
    // whose Next never unlocks.
    assert!(js.contains("ageAttested:"), "send_magic_link carries the attestation as ageAttested");
    assert!(js.contains("Tick both boxes"), "the page still says why a sign-in was refused");
}

/// Spec §6, last line: the upgrade overlay gets the same two doors. It is a **second** sign-in
/// surface, in the console window, over an existing vault — and it is not painted by `renderWizard`,
/// so the `WIZ.busy` guard that protects `#wiz-google-signin` does not reach it.
#[test]
fn the_upgrade_overlay_offers_the_same_two_doors_and_no_password() {
    let html = read("index.html");
    let panel = html.split("id=\"upgrade\"").nth(1).and_then(|s| s.split("</aside>").next()).expect("the upgrade overlay");
    let g = panel.find("id=\"up-google\"").expect("the overlay's Google button");
    let e = panel.find("id=\"up-email\"").expect("the overlay's email field");
    assert!(g < e, "Continue with Google comes first here too");
    assert!(!panel.contains("password"), "the overlay must never carry a password field again");
    for id in ["up-magic", "up-code-row", "up-code", "up-code-go"] {
        assert!(panel.contains(&format!("id=\"{id}\"")), "the overlay needs #{id}");
    }
    assert!(!html.contains("id=\"up-create\"") && !html.contains("id=\"up-signin\""), "no create/sign-in split");
    let js = read("console.js");
    let listener = js.split("EL(\"upgrade\").addEventListener(\"click\"").nth(1)
        .and_then(|s| s.split("function finishUpgrade(").next()).expect("the upgrade listener");
    for sel in ["#up-google", "#up-magic", "#up-code-go"] {
        assert!(listener.contains(sel), "the overlay's listener must handle {sel}");
    }
    // Two presses on a button that opens a browser are two listeners, two loopback ports and two
    // tabs. The wizard's guard is `WIZ.busy`, painted by renderWizard; the overlay carries its own.
    assert!(js.contains("function upBusy("), "the overlay has a busy guard of its own");
    assert!(listener.contains("UP_BUSY"), "…and the listener reads it before starting a sign-in");
    // **And the flag is declared OUTSIDE the listener** (review R2). Declared inside, it is
    // re-initialised to `false` on every press and guards nothing — while both assertions above
    // still pass. So the claim is about position: `var UP_BUSY` appears in the text BEFORE
    // `EL("upgrade").addEventListener`, where `UPGRADE_DISMISSED` and `UPGRADE_UNREACHABLE` live.
    let before = js.split("EL(\"upgrade\").addEventListener(\"click\"").next().expect("the file before the listener");
    assert!(before.contains("var UP_BUSY"), "UP_BUSY must be declared at IIFE scope, not inside the click handler");
}

/// **F10.** The overlay got `UP_BUSY` and the wizard's Google button gets `WIZ.busy`, but the
/// wizard's two emailed-code doors got neither — two quick presses on `#wiz-magic` spend two of the
/// twenty emails an hour `config.toml`'s comment calls the whole of the cap. Both doors now get the
/// same guard: refuse a second press while one is in flight, latch `WIZ.busy` before the `invoke`,
/// release it on every outcome, and `renderWizard` paints both as disabled while it is set — the
/// same pattern `#wiz-google-signin` and `#wiz-next` already use.
#[test]
fn the_wizards_email_doors_get_the_same_busy_guard_the_google_button_has() {
    let js = read("console.js");
    let render = js.split("function renderWizard(").nth(1).and_then(|s| s.split("\n  }").next()).expect("renderWizard");
    assert!(render.contains("EL(\"wiz-magic\").disabled = WIZ.busy"), "the email door is painted from WIZ.busy");
    assert!(render.contains("EL(\"wiz-code-go\").disabled = WIZ.busy"), "the code door is painted from WIZ.busy");
    let listener = js.split("EL(\"wizard\").addEventListener(\"click\"").nth(1)
        .and_then(|s| s.split("function renderCourses(").next())
        .expect("the wizard's click listener");
    for id in ["#wiz-magic", "#wiz-code-go"] {
        let marker = format!("e.target.closest(\"{id}\")");
        let handler = listener.split(&marker).nth(1)
            .and_then(|s| s.split("if (e.target.closest(").next())
            .unwrap_or_else(|| panic!("the {id} handler"));
        assert!(handler.contains("if (WIZ.busy) { return; }"), "{id} must refuse a second press while one is in flight");
        assert!(handler.contains("WIZ.busy = true"), "{id} must latch WIZ.busy before its invoke");
        assert!(handler.contains("WIZ.busy = false"), "{id} must release WIZ.busy on the outcome");
    }
}

/// Legal note §9: the report is shown, editable, before anything is sent — and what is sent is what
/// was shown, not something rebuilt after the user looked away.
#[test]
fn the_issue_report_is_previewed_edited_and_sent_verbatim() {
    let html = read("index.html");
    assert!(html.contains("id=\"report\""), "the report overlay");
    assert!(html.contains("id=\"report-text\""), "an editable textarea");
    assert!(html.contains("id=\"report-send\"") && html.contains("id=\"report-cancel\""));
    let js = read("console.js");
    assert!(js.contains("function openReport("), "openReport");
    assert!(js.contains("window.KNOWLU_OPEN_REPORT = openReport"), "the tray's one way in");
    assert!(js.contains("\"report_preview\"") && js.contains("\"report_send\""));
    // The send passes the TEXTAREA's value. A send that passed anything else would be sending
    // something the user never read.
    assert!(js.contains("invoke(\"report_send\", { text: EL(\"report-text\").value })"), "send exactly what is on screen");
}

/// **One string, two languages** — the same pin `PRIVACY` gets, for the same reason. `account.rs`
/// emits every transport failure as `"{UNREACHABLE} ({e})"`, and the upgrade overlay decides whether
/// to stand itself down by testing that the error *starts with* it. A silent drift here does not fail
/// anything: it just quietly stops standing the overlay down, on the one path where a student with no
/// network would otherwise be stuck behind it.
#[test]
fn the_unreachable_clause_is_one_string_on_both_sides() {
    let rust = fs::read_to_string("src/account.rs").expect("src/account.rs");
    let clause = rust
        .split("pub const UNREACHABLE: &str = \"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("account.rs must declare `pub const UNREACHABLE: &str = \"…\";`");
    assert!(!clause.is_empty());
    let js = read("console.js");
    assert!(
        js.contains(&format!("var UNREACHABLE = \"{clause}\";")),
        "console.js's UNREACHABLE must be account.rs's, word for word: {clause:?}"
    );
}

/// The local runtime leaves in C4; the wizard stops offering it now, because §4.2's step list has no
/// such step and a wizard that offers a 2 GB download for a feature that is moving to the cloud is
/// lying to a new user.
#[test]
fn the_wizard_never_offers_a_local_model() {
    let html = read("index.html");
    assert!(!html.contains("id=\"wiz-judge\""), "the finish panel's local-judgment offer is gone");
    assert!(!html.contains("2 GB"), "…and so is its download size");
    let js = read("console.js");
    // **Asserted by position, not by slice.** Whether `openSettings` — which legitimately calls these
    // until C4 — happens to fall inside a text slice is a fact about line ordering, not about the
    // wizard. Each command appears exactly once, and after `renderInference`, which is the settings
    // row's own function; anything the wizard called would appear earlier and twice.
    // The flag the deleted `the_wizard_offers_local_judgment_without_doing_anything` pinned, inverted:
    // the checkbox is gone from the markup, so the plan field it filled must be gone from the page.
    assert!(!js.contains("offer_inference"), "the wizard's local-judgment flag is gone with its checkbox");
    let at = |needle: &str| js.find(needle).unwrap_or_else(|| panic!("{needle} is not in console.js"));
    let inference = at("function renderInference(");
    for cmd in ["\"install_inference_download\"", "\"install_inference_file\"", "\"inference_status\""] {
        assert_eq!(js.matches(cmd).count(), 1, "{cmd} is invoked from more than one place");
        assert!(at(cmd) > inference, "the wizard has nothing to do with the local runtime: {cmd}");
    }
}

/// R-P4a-21: the wizard's privacy paragraph and the site's are ONE sentence, checked against each
/// other rather than against two copies of a literal. The site page is the published promise; the
/// wizard is the same promise made to someone who has not visited it. A word edited on either side
/// fails here.
#[test]
fn the_wizards_privacy_sentence_is_the_sites_privacy_sentence() {
    let site = fs::read_to_string(Path::new("../site/privacy.html")).expect("../site/privacy.html");
    let sentence = site
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("<p>") && l.contains("Your vault stays on this machine"))
        .map(|l| l.trim_start_matches("<p>").trim_end_matches("</p>").to_string())
        .expect("site/privacy.html must carry the privacy sentence in one <p>");
    let js = read("console.js");
    assert!(
        js.contains(&format!("var PRIVACY = \"{sentence}\";")),
        "console.js's PRIVACY must be site/privacy.html's sentence, word for word"
    );
}

/// Plan 4a Task 7: the settings overlay. R-P4a-4 — it is a panel in the page, never a `View` name
/// (a view name would reach `surface::View::parse` on every poll and be refused).
#[test]
fn the_settings_panel_has_its_rows_and_one_way_in() {
    let html = read("index.html");
    assert!(html.contains("id=\"settings\""));
    // D9 reconciled as "five plus Updates" (R-P4a-5) — six rows, and `set-updates` is one of them:
    // the row the updater task wired is as much part of the panel as the five that predate it.
    for row in ["set-name", "set-vault", "set-backup", "set-autostart", "set-updates",
                "set-account", "set-report", "set-delete", "set-diag"] {
        assert!(html.contains(&format!("id=\"{row}\"")), "row {row}");
    }
    // R-C1-55, I1: every control `console.js` binds BY NAME at IIFE top level. `EL()` answers `null`
    // for a missing id, so deleting one of these throws before `launch_state` is ever invoked — in
    // every window, so the console, the picker and the wizard all render as a blank document.
    for id in ["set-portal", "set-report-go", "set-delete-1", "set-delete-2"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "control {id}");
    }
    // R-C1-59 (I3): the button's own label, pinned — `site/terms.html` and the billing-jobs cancel
    // mails all tell the student to click this exact control by this exact name.
    assert!(html.contains("id=\"set-portal\">Manage subscription<"), "the settings button must say Manage subscription, word for word");
    let js = read("console.js");
    assert!(js.contains("function openSettings(") && js.contains("function renderSettings("));
    assert!(js.contains("window.KNOWLU_OPEN_SETTINGS"), "the tray's one way in");
    assert!(js.contains("data-settings"), "the topline gear");
    assert!(js.contains("invoke(\"switch_profile\""), "R-P4a-15: back to the picker");
    assert!(js.contains("c.profile_name"), "S4: the profile label comes from settings_context, not a guess");
    assert!(!js.to_lowercase().contains("channel"), "M8: there is one channel, so the row does not name one");
    // Decision 9: slots, timezone, sources and the course map stay the wizard's.
    for absent in ["set-slots", "set-timezone", "set-sources", "set-coursemap"] {
        assert!(!html.contains(absent), "{absent} is not a settings row in 4a");
    }
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}

/// Plan 4a Task 8: the update offer. The page renders whatever the engine's offer says and nothing
/// more — the mid-run gate is `updates::update_offer` in Rust (R9) — and the release host stays out
/// of the shipped page: it lives in `app/tauri.conf.json` (spec §8, the page rules).
#[test]
fn the_update_offer_is_in_the_page_and_the_endpoint_is_not() {
    let js = read("console.js");
    assert!(js.contains("function renderUpdateOffer("));
    assert!(js.contains("Restart to update"), "the exact offer text");
    assert!(js.contains("invoke(\"check_for_updates\"") && js.contains("invoke(\"install_update\""));
    // The endpoint lives in app/tauri.conf.json, never in the page (spec §8, the page rules).
    for f in ["index.html", "console.css", "console.js"] {
        assert!(!read(f).contains("knowlu.com"), "{f} must not name the release host");
    }
    // R-P4a-5's placeholder is spent: the row's button has something to call now.
    let html = read("index.html");
    assert!(html.contains("id=\"upd\""), "the offer's own element in the topline row");
    assert!(!html.contains("id=\"set-update-check\" disabled"), "the Updates row's button is live");
    // The offer carries no data-id, so R-T15b's equality still holds.
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}

/// Knowlu plan 4a Task 9: the bundle facts the installer needs, pinned so a stray edit to
/// `tauri.conf.json` cannot quietly change what a friend installs. `serde_json` is already an
/// `app/` dependency, and the test's working directory is `app/`, which is why the paths are bare.
#[test]
fn the_bundle_config_is_the_one_the_installer_and_the_updater_need() {
    let conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    assert_eq!(conf["identifier"], "com.knowlu.desktop", "decision 8 - baked into every install");
    assert_eq!(conf["bundle"]["active"], true);
    assert_eq!(conf["bundle"]["targets"][0], "nsis");
    assert_eq!(conf["bundle"]["targets"].as_array().unwrap().len(), 1, "nsis is the only target");
    assert_eq!(conf["bundle"]["externalBin"][0], "binaries/knowlu-engine", "the engine travels as a sidecar (decision 7)");
    assert_eq!(conf["bundle"]["windows"]["webviewInstallMode"]["type"], "embedBootstrapper");
    // Fix round 1, m3: `currentUser` is a LOAD-BEARING premise, not a default worth leaving
    // implicit. `updates::install_staged` does not re-verify the staged bundle's signature across
    // the disk hop, and the argument for that is "anyone who can rewrite the bundle in this user's
    // %LOCALAPPDATA% can equally rewrite knowlu.exe" - which is true only while the exe installs
    // into the user's own profile. `perMachine` or `both` would put the exe somewhere only an
    // administrator can write while the bundle stayed user-writable, and the argument would be
    // void. Spelled out here and in tauri.conf.json so flipping it has to pass this line first.
    // (tauri-utils 2.9.3: `bundle.windows.nsis` is `NsisConfig`, whose `install_mode` field is
    // `#[serde(rename_all = "camelCase")]` as `installMode`, taking `NSISInstallerMode`, itself
    // camelCase - so `"currentUser"`, and `deny_unknown_fields` would reject any other spelling.)
    assert_eq!(conf["bundle"]["windows"]["nsis"]["installMode"], "currentUser",
               "R9's disk-hop reasoning depends on this - see updates::install_staged");
    assert_eq!(conf["version"], "0.1.0", "one version, in one place (Knowlu spec 6)");
    // R-P4a-16: the bundler signs each binary BEFORE it packs, through one wrapper script.
    // tauri-bundler 2.9.4 calls try_sign() on the main binary, on every externalBin, on the
    // resources and on the installer, so all three of the things inside the NSIS package are
    // signed at the moment they are produced - signing the installer afterwards would leave the
    // two binaries within it unsigned.
    let sign = &conf["bundle"]["windows"]["signCommand"];
    assert_eq!(sign["cmd"], "powershell");
    let args: Vec<&str> = sign["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert!(args.contains(&"../scripts/sign.ps1") && args.contains(&"%1"), "{args:?}");
    assert!(Path::new("../scripts/sign.ps1").is_file(), "the wrapper the bundler will run");
    assert!(Path::new("../scripts/release.ps1").is_file(), "the one release command");

    // `createUpdaterArtifacts` and `plugins.updater` are ONE decision, not two, and the test is
    // the coupling rather than a fixed value. tauri-cli 2.11.4's `get_bundle_settings`
    // (src/interface/rust.rs) reads `plugins > updater` whenever createUpdaterArtifacts is
    // anything but `false` and fails the whole build - "failed to get updater configuration:
    // plugins > updater doesn't exist" - when it is absent. So a config with the flag on and the
    // plugin missing cannot build at all. Plan 4a Task 8's key-gated step turned both ON together;
    // this assertion held before that (both off) and holds now (both on).
    let cua = &conf["bundle"]["createUpdaterArtifacts"];
    assert!(!cua.is_null(), "the key is spelled out, so Task 8 flips a value rather than adding one");
    let wants_updater = *cua != false;
    let has_plugin = !conf["plugins"]["updater"].is_null();
    assert_eq!(wants_updater, has_plugin, "createUpdaterArtifacts and plugins.updater land together (Task 8)");
}

/// Plan 4a Task 8, key-gated step: the endpoint and the public key live in `app/tauri.conf.json`
/// and nowhere else. Shape, not value — a test that pinned the host would have to be edited to move
/// the release site, and a test that pinned the key would carry key material into the suite.
///
/// `pubkey` is why this test exists at all: `tauri-plugin-updater` 2.11.0 deserializes its config
/// with NO serde default on that field (`config.rs`), and the plugin's `setup` runs at startup - so
/// an empty or missing pubkey is not a degraded updater, it is a Knowlu that does not launch.
#[test]
fn the_updater_endpoint_lives_in_the_app_config() {
    let conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    let ep = conf["plugins"]["updater"]["endpoints"][0].as_str().unwrap_or("");
    assert!(ep.ends_with("/releases/latest.json"), "the endpoint: {ep}");
    // https, not http: the plugin REFUSES an insecure endpoint in a release build
    // (`validate_endpoints`), and a config that only works in dev is worse than one that fails now.
    assert!(ep.starts_with("https://"), "the endpoint must be https in a release build: {ep}");
    assert!(!conf["plugins"]["updater"]["pubkey"].as_str().unwrap_or("").is_empty(),
            "pubkey is required by the plugin - an empty one panics at startup");
}

/// **Ruling R-P4a-30**: the page is granted NO updater permission, and this test is how it stays
/// that way.
///
/// The app drives `tauri-plugin-updater` from Rust - `updater_builder()` / `updater()` are plain
/// `Manager::state` (the plugin's `lib.rs`), which no capability gates - so the ONLY thing
/// `updater:default` would add is the plugin's four IPC commands. Two of those are
/// `allow-install` and `allow-download-and-install`, and `withGlobalTauri: true` means any script
/// running in the page could then call `invoke("plugin:updater|download_and_install")` and walk
/// straight past `update_offer`'s mid-run gate (R9) and `hold_for_install` (S11). The gate is only
/// a gate if it is the only door: `check_for_updates` and `install_update`.
///
/// The whole set is asserted, not just the absence, so a quiet widening of any grant shows up here.
#[test]
fn the_page_is_granted_no_updater_permission() {
    let cap: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("capabilities/default.json").unwrap()).unwrap();
    let perms: Vec<&str> = cap["permissions"].as_array().unwrap()
        .iter().map(|p| p.as_str().unwrap()).collect();
    assert!(!perms.iter().any(|p| p.starts_with("updater:")),
            "R-P4a-30: no updater permission reaches the page: {perms:?}");
    assert_eq!(perms, ["core:default", "window-state:default", "autostart:default",
                       "clipboard-manager:allow-write-text"]);
    assert_eq!(cap["windows"].as_array().unwrap().len(), 1, "the console window, and only it");
}

/// Knowlu plan 4a Task 9: both release scripts are syntactically valid PowerShell 5.1.
/// `[scriptblock]::Create` PARSES without executing, so this never signs, builds or uploads
/// anything - it only catches a script committed with a syntax error, which would otherwise
/// surface for the first time in the middle of a release.
#[test]
fn the_release_scripts_parse_under_powershell_5_1() {
    for s in ["../scripts/sign.ps1", "../scripts/release.ps1", "../scripts/find-signtool.ps1"] {
        assert!(Path::new(s).is_file(), "{s}");
    }
    let script = "$ErrorActionPreference = 'Stop'; \
        foreach ($f in @('..\\scripts\\sign.ps1', '..\\scripts\\release.ps1', '..\\scripts\\find-signtool.ps1')) { \
          [void][scriptblock]::Create([System.IO.File]::ReadAllText((Resolve-Path $f))) \
        }";
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .expect("powershell.exe");
    assert!(
        out.status.success(),
        "a release script does not parse:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// R-P4a-26: `build.rs` must keep dropping the zero-byte sidecar placeholder, or a plain
/// `cargo build` / `cargo test` stops working on any checkout that has never run
/// `scripts\release.ps1`. `app/binaries/` is git-ignored, and `tauri-build`'s `copy_binaries`
/// treats a missing `externalBin` as a hard error - "resource path ... doesn't exist" - raised
/// before a single test compiles. The guard is invisible when it works, which is exactly why it
/// needs a test: nothing else would notice it being deleted until someone cloned the repo.
#[test]
fn build_rs_still_stages_the_zero_byte_sidecar_placeholder() {
    let build_rs = fs::read_to_string("build.rs").unwrap();
    assert!(build_rs.contains("TARGET"), "the placeholder is named for cargo's TARGET triple");
    assert!(build_rs.contains("binaries/knowlu-engine-"), "the placeholder path");
    assert!(
        build_rs.contains("cargo:rerun-if-changed=binaries/"),
        "staging a real engine later must re-run the build script, not reuse the placeholder run"
    );
    // That path is the THIRD spelling of the sidecar stem, after release.ps1's $sidecar and
    // bundle.externalBin. release.ps1 cross-checks its own against the config; this pins the last
    // one, so plan 2 Task 11's rename (quinn-ops -> knowlu-engine) cannot leave one behind.
    let conf: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    let declared = conf["bundle"]["externalBin"][0].as_str().unwrap();
    let stem = declared.rsplit('/').next().unwrap();
    assert!(
        build_rs.contains(&format!("binaries/{stem}-")),
        "build.rs must name {stem}, the stem bundle.externalBin declares"
    );
}

/// Knowlu plan 3a Task 10: the seventh settings row. Every state the reader can be in has words
/// (R7), the two install doors are both there, and nothing on this page reaches the network — the
/// manifest URL lives in `app/src/inference.rs`, never in the page.
#[test]
fn the_local_judgment_row_has_a_state_for_every_outcome() {
    let html = read("index.html");
    assert!(html.contains("id=\"set-judge\""), "the row");
    for id in ["set-judge-state", "set-judge-file", "set-judge-download", "set-judge-remove"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "{id}");
    }
    let js = read("console.js");
    assert!(js.contains("function renderInference("), "renderInference");
    for cmd in ["\"inference_status\"", "\"install_inference_file\"", "\"install_inference_download\"", "\"remove_inference_model\""] {
        assert!(js.contains(cmd), "{cmd}");
    }
    // The four states the row can be in, in the page's own words. **`"ready"` alone would be a dead
    // assertion** — `already` contains it, and this file is full of `already` (M3) — so the
    // rendered string is asserted with its separator.
    for words in ["runtime not installed", "model not installed", "\"ready — \"", "installing "] {
        assert!(js.contains(words), "the row must have words for: {words}");
    }
    // Global constraint: no http:// or https:// under app/static/ — the endpoint is Rust's.
    assert!(!js.contains("manifest.json"), "the manifest URL is inference.rs's, never the page's");
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
    // R-C1-59 (M6): an entitled cloud vault never runs the local runtime (`scheduler::judge_plan`
    // never passes `--runtime`/`--model` once `config/cloud.yaml` exists), so `renderAccountRow`
    // hides the row rather than offering a multi-gigabyte download that cannot affect anything.
    assert!(js.contains("EL(\"set-judge\").hidden"), "the row must be gated on the account status");
}

/// The wizard's default folders come from `launch_state` — the page never builds a path, and the
/// retired `documents` key (which named the OneDrive-redirected folder) is read nowhere. C1: there is
/// no backups root on the page at all.
#[test]
fn the_wizard_takes_its_default_folders_from_the_launch_state() {
    let js = read("console.js");
    // The vault's parent still comes from `launch_state` (renamed on main 2026-09-09:
    // `documents` -> `default_parent`).
    assert!(js.contains("l.default_parent"), "the parent folder still comes from launch_state");
    // …but nothing reads a backups root any more. There is no backup panel and no folder picker in
    // the wizard at all; `create_vault` puts `Backups` beside the vault (Task 12, spec §4.1).
    assert!(!js.contains("l.default_backup"), "the wizard must not read a backups root it cannot show");
}

/// **R-C1-55, C1 — Tauri v2 lower-camel-cases every argument key** (tauri-macros' `ArgumentCase::Camel`)
/// unless the command opts out with `rename_all = "snake_case"`. `send_magic_link` does not opt out, so
/// a page that sends `age_attested` is rejected *before* the command body runs; the handler's `.catch`
/// then paints `UNREACHABLE` — *the account service could not be reached* — and `wizValid`'s step-1 gate
/// refuses Next forever. **No new student could ever get a code**, and the sentence they were shown
/// blamed the network. C1b moved the trap one command over: `sign_up` is gone with the password, and
/// `send_magic_link(email, age_attested)` is now the crate's multi-word command argument.
#[test]
fn the_code_request_spells_its_argument_the_way_tauri_delivers_it() {
    let rust = fs::read_to_string("src/account.rs").expect("src/account.rs");
    // The signature itself, so a renamed or added parameter fails HERE and not in a student's first
    // five minutes.
    assert!(rust.contains("pub fn send_magic_link(email: String, age_attested: bool)"),
        "account::send_magic_link's signature changed — re-derive the keys the page must send");
    assert!(!rust.contains("rename_all"), "account.rs opts no command out of Tauri's camelCase");
    let js = read("console.js");
    // Either spelling of the assignment, because Task 5 is what moves it from the deleted
    // `args.ageAttested = …` into the `send_magic_link` invoke — and this test has to be green at
    // THIS task gate, not only at the next one. What it owns is the half that never changes: the
    // page says `ageAttested` and never `age_attested`, whichever branch carries it.
    assert!(js.contains("ageAttested"), "the attestation travels camel-cased");
    assert!(!js.contains("age_attested"), "the snake_case spelling must not appear on the page at all");
    // The two consent versions are `account.rs`'s constants and are stamped into the consent call
    // there (spec §9): a page that sent its own could make the consent log wrong.
    assert!(rust.contains("TOS_VERSION") && rust.contains("PRIVACY_VERSION"), "the versions are Rust's");
    for own in ["tos_version", "tosVersion", "privacy_version", "privacyVersion"] {
        assert!(!js.contains(own), "the consent versions are Rust's, never the page's: {own}");
    }
}

/// The standing guard behind C1, over **every** command in the crate: read each `#[tauri::command]`
/// signature, and for every parameter whose name has more than one word, assert the page never sends
/// the spelling Tauri would NOT deliver. Exactly one command opts out (`retarget_credentials`, whose
/// own comment says why), and for that one the rule inverts — which is the whole point of deriving
/// this from the source rather than keeping a list.
#[test]
fn no_multi_word_command_argument_is_sent_in_the_wrong_case() {
    fn camel(s: &str) -> String {
        let mut out = String::new();
        for (i, part) in s.split('_').enumerate() {
            if i == 0 { out.push_str(part); continue; }
            let mut cs = part.chars();
            if let Some(c) = cs.next() { out.extend(c.to_uppercase()); }
            out.push_str(cs.as_str());
        }
        out
    }
    let js = read("console.js");
    let mut checked = 0usize;
    for entry in fs::read_dir("src").expect("app/src") {
        let path = entry.expect("a dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
        let rust = fs::read_to_string(&path).expect("a source file");
        for block in rust.split("#[tauri::command").skip(1) {
            let Some(close) = block.find(']') else { continue };
            let opted_out = block[..close].contains("rename_all = \"snake_case\"");
            // Only a real attribute: the fn must follow it with nothing but whitespace between, which
            // is what tells a `#[tauri::command]` written inside a doc comment apart from the real one.
            let after = block[close + 1..].trim_start();
            let Some(sig) = after.strip_prefix("pub fn ").or_else(|| after.strip_prefix("pub async fn ")) else { continue };
            let name = sig.split('(').next().unwrap_or("").to_string();
            let Some(args) = sig.split('(').nth(1).and_then(|a| a.split(')').next()) else { continue };
            for arg in args.split(',') {
                let ident = arg.split(':').next().unwrap_or("").trim();
                if !ident.contains('_') || ident.starts_with('_') { continue; }
                checked += 1;
                let camelled = camel(ident);
                let wrong = if opted_out { camelled.as_str() } else { ident };
                let right = if opted_out { ident } else { camelled.as_str() };
                for shape in [format!("{wrong}:"), format!(".{wrong} =")] {
                    assert!(!js.contains(&shape),
                        "{}::{name} takes `{ident}`, so Tauri delivers it as `{right}` — console.js sends `{wrong}`",
                        path.file_name().unwrap_or_default().to_string_lossy());
                }
            }
        }
    }
    // A scan that silently stops finding anything is a guard that silently stops guarding.
    assert!(checked >= 6, "only {checked} multi-word command arguments found — the scan stopped working");
}

/// **R-C1-55, I1 — every control this task added, in both directions**: present in `index.html`, and
/// named in `console.js`. Six of them are bound at IIFE top level (`report-cancel`, `set-report-go`,
/// `report-send`, `set-portal`, `set-delete-1`, `set-delete-2`) and `EL()` answers `null` for a missing
/// id, so deleting one throws a `TypeError` **before `launch_state` is ever invoked** — in every
/// window, so the console, the picker and the wizard all render as a blank document, with nothing else
/// in this file failing. The rest are written to on a path somebody reaches by pressing something.
#[test]
fn every_control_this_task_added_is_in_the_markup_and_named_by_the_page() {
    let html = read("index.html");
    let js = read("console.js");
    for id in [
        // The report overlay (legal note §9) and Task 16's two commands behind it.
        "report", "report-text", "report-send", "report-cancel", "report-note",
        // The three new settings rows' controls. The rows themselves are pinned by
        // `the_settings_panel_has_its_rows_and_one_way_in`, which is where rows belong.
        "set-account-state", "set-report-go", "set-delete-1", "set-delete-2", "set-delete-note", "set-portal",
        // R-C1-42's second press, and the panels' own new controls.
        "wiz-lms-capture", "wiz-code-row", "wiz-code", "wiz-school-picked", "wiz-courses-note",
        "wiz-course-add-go", "wiz-map-note", "wiz-sub-note", "wiz-account-note",
        // Task 18's upgrade overlay. `upgrade` and `up-later` are bound at IIFE top level too, so
        // they carry the same "delete one and every window renders blank" weight the six above do.
        "upgrade", "up-email", "up-google", "up-magic", "up-code-row", "up-code", "up-code-go", "up-18", "up-terms",
        "up-subscribe", "up-later", "up-error",
    ] {
        assert!(html.contains(&format!("id=\"{id}\"")), "index.html has no #{id}");
        // Either spelling the page uses: `EL("x")` or a `closest("#x")` selector.
        assert!(js.contains(&format!("\"{id}\"")) || js.contains(&format!("\"#{id}\"")),
            "console.js never names #{id}");
    }
    // …and no id may be shared. The brief gave the Problems row and its button the same `set-report`,
    // so `getElementById` resolved to the row; the house spelling is `set-updates`/`set-update-check`.
    let mut ids: Vec<&str> = html
        .match_indices("id=\"")
        .map(|(i, _)| html[i + 4..].split('"').next().unwrap_or(""))
        .collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "index.html carries a duplicate id");
}

/// I11: *Delete my data* takes **this profile's** snapshots out of the shared backups root, never the
/// root. Two profiles on one machine share `%USERPROFILE%\Knowlu\Backups`, and the other one's only
/// other copy of their work is in there.
#[test]
fn deleting_my_data_leaves_another_profiles_snapshots_alone() {
    // The path arithmetic, driven directly: the command itself needs a `ConsoleState`, an `AppHandle`
    // and a live account, and none of the three is what this is about. The real thing —
    // `account::delete_local_data` over a scratch vault, its backups subtree, its registry row and
    // its credentials — is `app/tests/account.rs`'s
    // `deleting_my_data_removes_this_profiles_things_and_nothing_else`.
    let root = std::env::temp_dir().join(format!("knowlu-backups-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mine = root.join("profile_1111111111").join("vault");
    let theirs = root.join("profile_2222222222").join("vault");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(mine.join("a.md"), "x").unwrap();
    std::fs::write(theirs.join("b.md"), "y").unwrap();
    // What `delete_my_data` does: the root JOINED with this profile's id, and nothing above it.
    std::fs::remove_dir_all(root.join("profile_1111111111")).unwrap();
    assert!(!root.join("profile_1111111111").exists());
    assert!(theirs.join("b.md").is_file(), "another profile's snapshots were destroyed");
    let _ = std::fs::remove_dir_all(&root);
}

/// Spec §11a: an install that predates the account is upgraded **in place**, in the console window,
/// over its own vault — so the console page carries the same account panels the wizard does, and it
/// asks no folder question at all.
#[test]
fn the_console_can_sign_an_existing_install_in_without_re_onboarding_it() {
    let html = read("index.html");
    assert!(html.contains("id=\"upgrade\""), "the upgrade overlay");
    for id in ["up-email", "up-google", "up-magic", "up-code-row", "up-code", "up-code-go", "up-18", "up-terms", "up-subscribe", "up-later", "up-error"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "the upgrade overlay needs {id}");
    }
    let panel = html.split("id=\"upgrade\"").nth(1).and_then(|s| s.split("</aside>").next()).expect("the upgrade overlay");
    assert!(!panel.to_lowercase().contains("folder"), "an existing install is never asked about a folder");
    // **It must be dismissable**, exactly as `#report` is (`report-cancel`): spec §5.1 and D4 both
    // promise that a dead connection never hides today's page, and this is the first launch after C1
    // for every install that already exists.
    assert!(panel.contains("id=\"up-later\""), "the upgrade overlay needs a dismiss control");
    // …and its policy links must be the browser-opening kind, asserted **over this panel's markup**
    // rather than over the whole file: the wizard having them is not the same claim.
    assert_eq!(panel.matches("class=\"policy\"").count(), 2, "both policy links open in the browser");
    let js = read("console.js");
    let listener = js.split("EL(\"upgrade\").addEventListener(\"click\"").nth(1).and_then(|s| s.split("function finishUpgrade(").next()).expect("the upgrade listener");
    assert!(listener.contains("a.policy") && listener.contains("preventDefault()"), "the overlay's own listener must intercept them");
    assert!(js.contains("function maybeUpgrade("), "maybeUpgrade");
    assert!(js.contains("\"attach_account\""), "the upgrade ends by attaching the account to this vault");
    // …it only appears when the vault says it needs one, never on a healthy console…
    assert!(js.contains("s.needs_account"), "the overlay is gated on the account status's own flag");
    // …it stays down once dismissed, and stays down when the service cannot be reached at all.
    assert!(js.contains("UPGRADE_DISMISSED"), "the dismiss must survive the next state poll");
    assert!(js.contains("UPGRADE_UNREACHABLE") && js.contains("function upgradeUnreachable("),
        "a sign-in that cannot reach the service must stand the overlay down, not trap the user behind it");
    // …and the guard tests the clause the Rust side actually emits. It read `indexOf("could not be
    // reached") === 0` once, against an error whose first twenty characters are "the account service
    // ", so it could never fire — which is the failure mode a shared literal exists to prevent.
    assert!(js.contains("indexOf(UNREACHABLE) === 0"), "the guard must test the shared clause, not a fragment of it");
    // …and it is a side panel, not a modal: the console underneath stays usable, which is the whole
    // of D4's promise that a dead connection never hides today's page.
    assert!(html.contains("<aside class=\"setpanel\" id=\"upgrade\""), "the overlay is a setpanel, like #settings");
}

/// **R-C1-57 (I2): a refusal is a sentence, not silence.** Both link commands answer with an
/// envelope — `{ok:false, error}` for a dead session, a refused `check_api_base`, a non-2xx from
/// Stripe or a reply with no link in it — and a page that discards it leaves a student pressing a
/// button that does nothing. *Subscribe* also used to start a two-minute entitlement poll on top of
/// a Checkout page that had never opened.
///
/// **M4/M5, same file, same round:** the overlay is never raised over the settings panel (they share
/// a corner and a z-index, and `#upgrade` is last in the DOM), and Escape closes the topmost panel
/// first — the ordering the previous round added without a test.
#[test]
fn a_refused_link_is_said_out_loud_and_escape_closes_the_topmost_panel() {
    let js = read("console.js");
    // The overlay's Subscribe: read the envelope, paint the sentence, and do not poll on a refusal.
    let sub = js
        .split("if (e.target.closest(\"#up-subscribe\"))")
        .nth(1)
        .and_then(|s| s.split("function finishUpgrade(").next())
        .expect("the subscribe branch");
    let refused = sub.find("if (!r.ok)").expect("Subscribe must read open_checkout's envelope");
    assert!(sub.contains("EL(\"up-error\").textContent = r.error"), "…and paint the reason on the panel");
    let polls = sub.find("setTimeout(tick").expect("the entitlement poll");
    assert!(refused < polls, "the refusal is handled before the poll can start");
    assert!(sub[refused..polls].contains("return"), "a refusal must return, never fall through into the poll");

    // *Manage subscription*, in the settings panel, which had the same shape from Task 17.
    let portal = js
        .split("EL(\"set-portal\").addEventListener")
        .nth(1)
        .and_then(|s| s.split("EL(\"set-delete-1\")").next())
        .expect("the portal listener");
    assert!(portal.contains("r.ok") && portal.contains("r.error"), "Manage subscription must say why nothing opened");

    // M4: opening Settings must not raise the overlay on top of it.
    let mu = js.split("function maybeUpgrade(").nth(1).and_then(|s| s.split("EL(\"up-later\")").next()).expect("maybeUpgrade");
    assert!(mu.contains("EL(\"settings\").hidden"), "the overlay must never be raised over the settings panel");

    // M5: Escape closes the topmost first — `#upgrade`, then `#report`, then `#settings`, which is
    // DOM order among panels sharing a z-index.
    let keys = js
        .split("document.addEventListener(\"keydown\"")
        .nth(1)
        .and_then(|s| s.split("function openReport(").next())
        .expect("the page's keydown handler");
    let up = keys.find("EL(\"upgrade\").hidden").expect("Escape must reach the upgrade overlay");
    let report = keys.find("EL(\"report\").hidden").expect("Escape must reach the report overlay");
    let settings = keys.find("EL(\"settings\").hidden").expect("Escape must reach the settings panel");
    assert!(up < report && report < settings, "Escape must close the topmost panel first");
}

/// **R-C1-57 (M6): the upgrade overlay is dressed like the panel it lives in.** It is the first thing
/// every existing install sees at the C1 cut-over, and without these two rules its password field is
/// a white browser default on a dark panel and its two consent checkboxes run together inline. The
/// wizard's panel has had both rules since Task 17 (`.wiz-panel input[type="password"]`,
/// `.wiz-panel label`); this is the settings-panel half.
#[test]
fn the_upgrade_overlays_field_and_labels_are_styled_like_the_rest_of_the_panel() {
    let css = read("console.css");
    assert!(css.contains(".set-row input[type=\"text\"], .set-row input[type=\"password\"]"),
        "a password field in a settings row must look like the text field beside it");
    assert!(css.contains(".setpanel label {"), "a consent checkbox needs a line of its own");
}

/// Spec §7. Quinn, 2026-09-17: "why are there even back and next buttons if they don't work?"
/// Three answers, and this pins all three.
#[test]
fn the_wizards_nav_is_rendered_state_and_never_a_dead_control() {
    let js = read("console.js");
    let render = js.split("function renderWizard(").nth(1).and_then(|s| s.split("\n  }").next()).expect("renderWizard");
    // (a) Back is never `disabled` — at step 0 it is simply not there, so there is no dead control
    // to press. `hidden` on a button the UA stylesheet hides is enough; `disabled` was the bug.
    assert!(render.contains("EL(\"wiz-back\").hidden = WIZ.step === 0"), "Back is hidden at step 0, never disabled");
    assert!(!render.contains("EL(\"wiz-back\").disabled"), "Back is never disabled anywhere");
    // (b) Next's disabled state is a WIZ field renderWizard paints, like every other wizard field.
    // Set only by a direct DOM write, it survived every re-render that did not re-set it — which is
    // a Finish that resolved without relaunching leaving Next dead forever.
    assert!(render.contains("EL(\"wiz-next\").disabled = WIZ.busy"), "Next's disabled state is rendered from WIZ.busy");
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo");
    let fin = js.split("function wizFinish(").nth(1).and_then(|s| s.split("\n  // The Checkout page").next()).expect("wizFinish");
    for (name, body) in [("wizGo", go), ("wizFinish", fin)] {
        assert!(!body.contains("EL(\"wiz-next\").disabled"), "{name} sets WIZ.busy, never the DOM property directly");
        assert!(body.contains("WIZ.busy"), "{name} still latches re-entry, through WIZ.busy");
    }
    // …and one count over the WHOLE file, because the two slices above do not cover the file
    // (review R5). `credentialsStranded` (`console.js:1617-1626`) sits between `wizGo` and
    // `wizFinish`, so its write at `:1623` is in neither slice — and a missed one is the worst of
    // the six: `renderWizard()` fires on the very next line and repaints `disabled = WIZ.busy`, so a
    // stranded-credentials recovery would re-enable Next and then immediately kill it again.
    assert_eq!(
        js.matches("EL(\"wiz-next\").disabled").count(),
        1,
        "renderWizard is the ONLY writer of Next's disabled state"
    );
    // (c) A refused Next says what is missing, in a sentence, and says it again on a second press —
    // a red line that was already on screen does not read as a new answer.
    // Review M7: the gates themselves, not merely the name — a `wizValid` that still exists and no
    // longer refuses an unsigned-in step is exactly the regression this is here to catch.
    let valid = js.split("function wizValid(").nth(1).and_then(|s| s.split("function wizGo(").next()).expect("wizValid");
    assert!(valid.contains("WIZ.step === 1 && !WIZ.accountId"), "step 1 is still gated on a session");
    assert!(valid.contains("WIZ.step === 2 && !WIZ.entitled"), "…and step 2 on an entitlement");
    assert!(go.contains("!wizValid()"), "wizGo refuses a forward step wizValid refuses");
    assert!(js.contains("flashError("), "a repeated refusal is re-announced, not silently unchanged");
    for sentence in ["Sign in first.", "Finish the payment page in your browser, then come back."] {
        assert!(js.contains(sentence), "the refusal names what is missing: {sentence}");
    }
    // R-C1b-exec-9: a Next off the subscribe panel now asks the service once before it refuses,
    // rather than trusting a WIZ.entitled the two-minute poll below may already have given up on.
    assert!(
        go.contains("WIZ.step === 2 && !WIZ.entitled"),
        "wizGo's own pre-ask is gated on the same panel and the same field wizValid is"
    );
    assert!(go.contains("checkEntitled("), "wizGo re-asks the service before it refuses");
    // The subscribe presses ask first too — never a second Checkout page for an account the service
    // already calls entitled.
    let wiz_sub = js
        .find("if (e.target.closest(\"#wiz-sub-month\")")
        .map(|i| &js[i..])
        .and_then(|s| s.split("if (e.target.closest(\"#wiz-lms-open\")").next())
        .expect("the wiz-sub-month/year handler");
    let up_sub = js
        .find("if (e.target.closest(\"#up-subscribe\")")
        .map(|i| &js[i..])
        .and_then(|s| s.split("function finishUpgrade(").next())
        .expect("the up-subscribe handler");
    for (name, handler) in [("#wiz-sub-month", wiz_sub), ("#up-subscribe", up_sub)] {
        let ask = handler.find("checkEntitled(").expect("checkEntitled appears in the subscribe handler");
        let open = handler.find("open_checkout").expect("open_checkout is still reachable on a no");
        assert!(ask < open, "{name} asks the service before it ever opens a second Checkout page");
    }
    // Both expiries name the way forward instead of dead-ending on a re-ask nobody can trigger.
    for sentence in [
        "Still not subscribed. When the payment page is done, press Next.",
        "Still not subscribed. When the payment page is done, press Subscribe again.",
    ] {
        assert!(js.contains(sentence), "the expiry names the way forward: {sentence}");
    }
    // The entitlement status test lives in exactly one place — `checkEntitled` — so a status the
    // service adds later needs one edit, not three.
    assert_eq!(
        js.matches("status === \"trialing\"").count(),
        1,
        "checkEntitled is the ONLY copy of the entitlement status test"
    );
    let poll = js
        .split("function pollEntitlement(")
        .nth(1)
        .and_then(|s| s.split("EL(\"wizard\").addEventListener(").next())
        .expect("pollEntitlement");
    assert!(
        poll.contains("WIZ.step !== 2"),
        "pollEntitlement's tick stands down once the student has left panel 2"
    );
    // Round-4 re-review, BLOCKING: both stand-down checks must run again AFTER checkEntitled()
    // resolves, not only before it starts — a reply that lands after the student moved on must not
    // act on a panel nobody is looking at.
    let ce_in_poll = poll.find("checkEntitled(").expect("pollEntitlement calls checkEntitled");
    let guards: Vec<_> = poll.match_indices("WIZ.step !== 2").map(|(i, _)| i).collect();
    assert_eq!(guards.len(), 2, "pollEntitlement's stand-down is checked before AND after checkEntitled()");
    assert!(guards[1] > ce_in_poll, "the second WIZ.step !== 2 check runs after checkEntitled() resolves");
    let up_sub = js
        .find("if (e.target.closest(\"#up-subscribe\")")
        .map(|i| &js[i..])
        .and_then(|s| s.split("function finishUpgrade(").next())
        .expect("the up-subscribe handler");
    let ce_in_up = up_sub.find("checkEntitled(").expect("the up-subscribe handler calls checkEntitled");
    let hidden_checks: Vec<_> = up_sub.match_indices("EL(\"upgrade\").hidden").map(|(i, _)| i).collect();
    assert_eq!(hidden_checks.len(), 2, "the overlay's stand-down is checked before AND after checkEntitled()");
    assert!(hidden_checks[1] > ce_in_up, "the second hidden check runs after checkEntitled() resolves");
}

/// R-C1b-exec-10: an empty coursework discovery — no rows, for any reason — must not trap the
/// student on the logins panel, and must say something rather than show an empty div.
#[test]
fn an_empty_coursework_discovery_lets_next_proceed_and_says_so() {
    let js = read("console.js");
    let step = js
        .split("function wizStep(")
        .nth(1)
        .and_then(|s| s.split("function wizRegister(").next())
        .expect("wizStep");
    assert!(
        step.contains("WIZ.map.length || WIZ.discovered"),
        "a finished discovery, rows or none, lets Next proceed"
    );
    let mapping = js
        .split("function renderMapping(")
        .nth(1)
        .and_then(|s| s.split("\n  }").next())
        .expect("renderMapping");
    assert!(mapping.contains("!WIZ.discovered"), "the mapping panel stays up for a finished-but-empty discovery");
    assert!(
        js.contains("You can go on — Knowlu will try again on its first run."),
        "an empty discovery names the way forward rather than dead-ending"
    );
    let onboarding_rs = std::fs::read_to_string("src/onboarding.rs").expect("src/onboarding.rs");
    for stale in ["fill them in below", "fill those in below"] {
        assert!(!js.contains(stale), "console.js no longer tells the student to fill in a panel with nothing on it: {stale}");
        assert!(!onboarding_rs.contains(stale), "onboarding.rs no longer tells the student to fill in a panel with nothing on it: {stale}");
    }
    let html = read("index.html");
    let map_div = html
        .find("id=\"wiz-map\"")
        .map(|i| &html[i..])
        .and_then(|s| s.split("</div>").next())
        .expect("#wiz-map");
    assert!(map_div.contains("id=\"wiz-map-heading\""), "the mapping heading has an id renderMapping can hide");
}

/// The version constant and the page's own date are one fact in two files (`account.rs`'s rule).
#[test]
fn the_privacy_version_constant_is_the_published_pages_date() {
    let rust = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    let version = rust
        .split("pub const PRIVACY_VERSION: &str = \"").nth(1)
        .and_then(|s| s.split('"').next())
        .expect("account.rs must declare `pub const PRIVACY_VERSION: &str = \"…\";`");
    let page = std::fs::read_to_string("../site/privacy.html").expect("site/privacy.html");
    assert!(page.contains(&format!("Effective {version}.")), "the page's Effective date is {version}");
    assert!(page.contains(&format!("This page is version <strong>{version}</strong>")), "…and so is its version line");
    // Spec §10: the policy must not describe a password the product no longer has.
    assert!(!page.contains("password hash"), "the password-hash clause is gone");
    assert!(!page.contains("your password, which Supabase holds"), "…and the account-clause's own password mention is gone");
    assert!(page.contains("There is no password on a Knowlu account at all"), "…and the page says so");
    assert!(page.contains("Signing in with Google tells us three things"), "Google sign-in is disclosed");
    // F3: the "Your account" collection entry must not be narrower than the page's own Google
    // paragraph — both must name the same three things Google hands over at sign-in.
    let account_entry = page.split("<dt>Your account</dt>").nth(1).and_then(|s| s.split("<dt>").next())
        .expect("the Your account entry");
    assert!(account_entry.contains("the Google account id that identifies it"), "{account_entry}");
    assert!(account_entry.contains("your name") && account_entry.contains("a link to your profile picture"),
        "the Your account entry must agree with the Google paragraph: {account_entry}");
}

/// Phase 2 of the commitment model (spec D6, D7, D8, §4): the Schedule view headed "Your week",
/// the window editor with its 400 ms preview, the today view's moved line, and `schedule` never
/// reaching the read model as a view name.
#[test]
fn the_schedule_view_the_window_editor_and_the_moved_line_are_there() {
    let html = read("index.html");
    assert!(html.contains("<a href=\"#schedule\" data-view=\"schedule\"><span class=\"dot\"></span>Schedule<span class=\"ct\"></span></a>"), "D6: a Schedule link");
    assert!(html.contains("<section id=\"main-schedule\" hidden>") && html.contains("<h2>Your week</h2>"), "D6: headed Your week");
    for id in ["sched-list", "sched-oh", "sched-window", "sched-save", "sched-say", "sched-moved", "sched-items", "moved"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    let js = read("console.js");
    assert!(js.contains("schedule: renderScheduleView"), "in VIEW_RENDERERS");
    for f in ["stateView", "renderMoved", "renderScheduleView", "windowEditorHtml", "windowSequence", "bindWindowEditor", "runPreview", "confirmWeek", "bindScheduleView"] {
        assert!(js.contains(&format!("function {f}(")), "missing function {f}");
    }
    assert!(js.contains("var PREVIEW_MS = 400;") && js.contains("setTimeout(runPreview, PREVIEW_MS)"), "§4: the preview is debounced 400 ms");
    assert!(js.contains("invoke(\"preview_window\", { window: seq })"));
    assert!(js.contains("\"No change to today's plan\""));
    assert!(js.contains("items.slice(0, 5)"), "§4: the first five items of the previewed day");
    assert!(js.contains("el.textContent = m ? m.text : \"\";"), "D7: the today view prints moved.text");
    assert!(js.contains("data-same-as-monday"), "§4: the same-as-Monday shortcut");
    assert!(js.contains("invoke(\"commitments_confirm\", { view: stateView(), confirm: payload })"));
    // Q10-a: `schedule` never reaches `surface::View::parse`. ui_event is the one call that names
    // the page's own view.
    assert_eq!(js.matches("view: current.view").count(), 1, "only ui_event sends current.view");
    assert!(js.contains("{ action: action, view: current.view,"), "…and it is ui_event");
    assert!(js.contains("runs: 1, schedule: 1 }"), "route() accepts schedule");
    assert!(!js.contains("data-remove"), "D8: no remove control in phase 2");
    let css = read("console.css");
    assert!(css.contains(".row.sched { grid-template-columns: minmax(0,1fr) auto; }"), "a Schedule row restates its tracks");
    assert!(css.contains(".moved[hidden] { display: none; }"));
}
