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
fn the_wizard_has_seven_panels_the_privacy_words_and_no_live_fetch() {
    let html = read("index.html");
    assert!(html.contains("id=\"wizard\""));
    for p in ["wiz-welcome", "wiz-vault", "wiz-backup", "wiz-lms", "wiz-logins", "wiz-slots", "wiz-finish"] {
        assert!(html.contains(&format!("id=\"{p}\"")), "panel {p}");
    }
    let js = read("console.js");
    for f in ["startWizard", "renderWizard", "wizGo", "wizFinish"] {
        assert!(js.contains(&format!("function {f}(")), "missing {f}");
    }
    // Decision 10: no telemetry, and no toggle to argue about.
    assert!(!js.to_lowercase().contains("telemetry"));
    // The privacy paragraph, exactly, and the same words the site carries (Task 10 pins the pair).
    assert!(js.contains("Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates."));
    // No live fetch in onboarding: the ICS URL is matched by shape, never requested. The regex is
    // escaped so the page still carries no `https://` literal (the network-reference rule).
    assert!(js.contains("/^https:\\/\\/"), "the ICS check is an escaped regex");
    assert!(!js.contains("fetch(\"http"), "onboarding never fetches");
    // Credentials leave page memory the moment the write returns (spec §5).
    assert!(js.contains("clearCredentialFields("), "the fields are cleared by name");
    // S13: the one seam the headless checks drive the panels through.
    // All three panels the scripts put on screen without a backend — `console-shots.py` calls each
    // of these by name, so a rename here is a script that shoots nothing and says nothing.
    assert!(js.contains("window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker, openSettings: openSettings }"), "the shots seam, spelled as scripts/console-shots.py calls it");
    // R-P4a-11: a new vault is <parent>\<name>, and the page says where before Finish.
    assert!(js.contains("function dest()") && js.contains("wiz-name") && js.contains("wiz-pick-parent"));
    // The `.app[hidden]` lesson again (Task 2, review round 1): `.wiz-row` is a flex container
    // and a class rule beats the UA stylesheet's `[hidden]`, so the adopt and restore rows sat
    // on screen while creating a vault until this rule existed.
    assert!(read("console.css").contains(".wiz-row[hidden] { display: none; }"), "a hidden .wiz-row must actually hide");
    // R-P4a-23: a rename after panel 5 MOVES the entries rather than orphaning them — the page
    // never leaves a login keyed to a path it is not going to create.
    assert!(js.contains("retarget_credentials"), "Finish moves the credentials when the path changed");
    // R-T15b's equality still holds: the wizard's own markup carries ids and no data-id at all,
    // so the page-wide count of data-id against data-kind is untouched by it.
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
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
        .find(|l| l.starts_with("<p>") && l.contains("Everything stays on this machine"))
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
    for row in ["set-name", "set-vault", "set-backup", "set-autostart", "set-updates", "set-diag"] {
        assert!(html.contains(&format!("id=\"{row}\"")), "row {row}");
    }
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
}

/// The wizard offers local judgment and never performs it: "Nothing is fetched now" is the wizard's
/// promise, and a two-gigabyte download during onboarding would break it (spec §5.3, D8).
#[test]
fn the_wizard_offers_local_judgment_without_doing_anything() {
    let html = read("index.html");
    assert!(html.contains("id=\"wiz-judge\""), "the finish panel's offer");
    assert!(html.contains("Nothing is fetched now."), "the wizard's promise still stands");
    assert!(!html.contains("Gmail proposals, event verdicts and enrichment are not here yet"),
        "enrichment ships in this plan — that sentence is now false");
    let js = read("console.js");
    assert!(js.contains("offer_inference"), "the plan carries the checkbox to apply_profile_settings");
    // D8, asserted where it can actually fail: neither install command may be reachable from the
    // wizard's own code. **Task 10 review, m1**: `startWizard`'s body alone is init-only —
    // `wizFinish` and `wizRegister`, where a "helpful" download would actually be added, sit in
    // later top-level functions the original slice never reached. Widened to span every wizard
    // function: from `startWizard`'s open brace to the wizard's own click listener, the one
    // `EL("wizard").addEventListener("click", …)` in the file, which is the line right after
    // `wizFinish` closes and before which every wizard-only helper (`renderWizard`, `dest`,
    // `within`, `wizGo`, `wizRegister`, `wizFinish`) is declared.
    let wizard = js
        .split("function startWizard(")
        .nth(1)
        .and_then(|s| s.split("EL(\"wizard\").addEventListener(\"click\"").next())
        .expect("startWizard..the wizard's click listener");
    for cmd in ["install_inference_download", "install_inference_file"] {
        assert!(!wizard.contains(cmd), "the wizard offers and never installs: {cmd}");
    }
}
