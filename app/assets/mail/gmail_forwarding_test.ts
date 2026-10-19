// Spec test 35, the Gmail forwarding half (plan T10.3a, spec 4.3 and 4.12 step 1). Each test runs the
// compiled-in script text from `gmail/` against a synthetic Settings > Forwarding and POP/IMAP page
// (gmail_forwarding_page.ts). No address is real.
import { assert, assertEquals } from "@std/assert";
import { type Gmail, gmail, SECTION } from "./gmail_forwarding_page.ts";
import { run } from "./test_dom.ts";

const OURS = "box-1@in.example.invalid";
const OTHER = "someone@elsewhere.example.invalid";
const THIRD = "third@elsewhere.example.invalid";
const FAST = { wait_ms: 50, settle_ms: 200 };

// The text of the forward-to list's selected option (its value is an index on the live shape).
const selectedText = (f: Gmail) =>
  ([...f.el("to").options].find((o: { selected: boolean }) => o.selected) ?? f.el("to").options[0])
    .textContent;
const optionTexts = (f: Gmail) => [...f.el("to").options].map((o: { textContent: string }) => o.textContent);

const NOT_FOUND = (control: string) => ({ result: "not found", control });
const KEPT = { result: "ok", forwarding: "on", to: "ours", keep_copy: true };

Deno.test("gmail_add_address_clicks_input_buttons_by_value", async () => {
  const f = gmail();
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", stage: "asked", changed: true });
  assertEquals(f.typed, [OURS]);
  assertEquals(f.popups, 1);
  // The look-alikes on the page (a <button> named Next, a div named Add) are never clicked.
  assertEquals(f.decoy, 0);
  // Google's popup closes; the opener shows its dialog, and the script presses OK on it.
  f.verified();
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, FAST), { result: "ok", sent: true });
  assertEquals(f.oks, 1);
  assertEquals(f.el("dlg"), null);
  assertEquals(f.decoy, 0);
});

Deno.test("gmail_add_address_finds_an_address_box_with_no_type_attribute", async () => {
  // T18c (2026-10-07): live Outlook drew its search box with no type attribute, which `[type=...]` never
  // matches. The audit found this dialog's box was taken only as `input[type="email"]`: an untyped box (a
  // browser reads its type as "text"), the dialog's one text box, is the address box too.
  const f = gmail({ untypedEmail: true });
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", stage: "asked", changed: true });
  assertEquals(f.typed, [OURS]);
});

Deno.test("gmail_add_address_presses_proceed_when_the_dialog_asks", async () => {
  const f = gmail({ proceed: true });
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", stage: "asked", changed: true });
  assertEquals(f.popups, 1);
  assertEquals(f.typed, [OURS]);
});

Deno.test("gmail_add_address_on_a_list_that_has_it_opens_no_dialog", async () => {
  const f = gmail({ addresses: [OTHER, OURS.toUpperCase()] });
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", stage: "listed", changed: false });
  assertEquals(f.typed, []);
  assertEquals(f.el("dlg"), null);
});

Deno.test("gmail_the_confirmation_dialog_is_not_found_when_google_showed_none", async () => {
  const f = gmail();
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, FAST), NOT_FOUND("confirmation dialog"));
  assertEquals(f.oks, 0);
});

Deno.test("gmail_a_section_that_did_not_draw_is_redrawn_by_a_hash_round_trip", async () => {
  // The section does not draw on the first visit; leaving the hash and coming back draws it.
  const f = gmail({ lateDraw: "hash", addresses: [OURS], on: true, to: OURS });
  const r = await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, listed: true });
  assertEquals(f.reloads, 0);
  assert(f.hashes.length >= 2 && f.hashes.at(-1) === "#settings/fwdandpop", "it came back to the section");
  assert(f.hashes[0] !== "#settings/fwdandpop", "it left the section first");
  // A page elsewhere is taken to the section by its hash.
  const g = gmail({ at: "#inbox", addresses: [OURS] });
  const s = await run("gmail/forwarding_read.js", g.page, { address: OURS, ...FAST });
  assertEquals(s, { result: "ok", forwarding: "off", to: "none", keep_copy: true, listed: true });
  assertEquals(g.hashes, ["#settings/fwdandpop"]);
});

Deno.test("gmail_a_section_that_a_hash_round_trip_cannot_draw_is_reloaded_once_then_not_found", async () => {
  // Only a reload draws it: the script reloads and says so, and the caller runs it again with reloaded.
  const f = gmail({ lateDraw: "reload", addresses: [OURS] });
  const first = await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST });
  assertEquals(first, { result: "reloading" });
  assertEquals(f.reloads, 1);
  const second = await run("gmail/forwarding_read.js", f.page, { address: OURS, reloaded: true, ...FAST });
  assertEquals(second.result, "ok");
  assertEquals(f.reloads, 1, "it reloads once, never twice");
  // A page that never draws: after the one reload the section is not found, so the step is guided.
  for (const script of ["read", "set", "off", "add", "remove_address"]) {
    const g = gmail({ lateDraw: "never", addresses: [OURS] });
    const args = { address: OURS, reloaded: true, ...FAST };
    assertEquals(await run(`gmail/forwarding_${script}.js`, g.page, args), NOT_FOUND("section"), script);
    assertEquals(g.reloads, 0, `${script}: told it already reloaded`);
    assertEquals(g.saves + g.popups + g.removed.length, 0, script);
  }
});

Deno.test("gmail_forwarding_on_keeps_gmails_copy_in_the_inbox_and_reads_it_back", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: false, disp: "archive" });
  const r = await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { ...KEPT, changed: true });
  assertEquals(f.saves, 1);
  // It picked our address in the list, not the first one, and Gmail's copy stays in the Inbox.
  assertEquals(f.model, { addresses: [OTHER, OURS], on: true, to: OURS, disp: "keep" });
  // Set again, nothing to change and nothing saved.
  const again = await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST });
  assertEquals(again, { ...KEPT, changed: false });
  assertEquals(f.saves, 1);
  // Forwarding already ours with a copy that is not kept: only the copy is fixed.
  const g = gmail({ addresses: [OURS], on: true, to: OURS, disp: "trash" });
  assertEquals(await run("gmail/forwarding_set.js", g.page, { address: OURS, ...FAST }), {
    ...KEPT,
    changed: true,
  });
  assertEquals(g.model.disp, "keep");
});

Deno.test("gmail_forwarding_set_before_the_address_is_confirmed_is_not_found", async () => {
  const f = gmail({ addresses: [OTHER] });
  assertEquals(
    await run("gmail/forwarding_set.js", f.page, { address: OURS, reloaded: true, ...FAST }),
    NOT_FOUND("address list"),
  );
  assertEquals(f.saves, 0);
  const g = gmail();
  assertEquals(
    await run("gmail/forwarding_set.js", g.page, { address: OURS, reloaded: true, ...FAST }),
    NOT_FOUND("address list"),
  );
  assertEquals(g.saves, 0);
});

Deno.test("gmail_a_save_that_does_not_settle_is_reported_failed", async () => {
  const f = gmail({ addresses: [OURS], sticky: true });
  const r = await run("gmail/forwarding_set.js", f.page, { address: OURS, wait_ms: 50, settle_ms: 200 });
  assertEquals(r, { result: "failed", reason: "save did not settle" });
});

Deno.test("gmail_forwarding_elsewhere_is_reported_not_replaced", async () => {
  for (const script of ["set", "off", "add"]) {
    const f = gmail({ addresses: [OTHER, OURS], on: true, to: OTHER, disp: "read" });
    const before = f.snap();
    const r = await run(`gmail/forwarding_${script}.js`, f.page, { address: OURS, ...FAST });
    assertEquals(r, { result: "elsewhere" }, script);
    assertEquals(f.saves + f.popups + f.typed.length, 0, script);
    assertEquals(f.snap(), before, script);
  }
  // A read names it as elsewhere and never returns the other address.
  const f = gmail({ addresses: [OTHER], on: true, to: OTHER });
  const r = await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", forwarding: "on", to: "elsewhere", keep_copy: true, listed: false });
  assert(!JSON.stringify(r).includes("elsewhere.example"));
  // With the student's answer, forwarding is pointed at Knowlu instead; only a literal true counts.
  const g = gmail({ addresses: [OTHER, OURS], on: true, to: OTHER, disp: "read" });
  assertEquals(await run("gmail/forwarding_set.js", g.page, { address: OURS, replace: "yes", ...FAST }), {
    result: "elsewhere",
  });
  assertEquals(g.saves, 0);
  const h = await run("gmail/forwarding_set.js", g.page, { address: OURS, replace: true, ...FAST });
  assertEquals(h, { ...KEPT, changed: true });
  assertEquals(g.model, { addresses: [OTHER, OURS], on: true, to: OURS, disp: "keep" });
});

Deno.test("gmail_a_missing_control_returns_not_found", async () => {
  // A control a read needs, then the controls only a write needs: the script names the one it cannot find
  // and changes nothing, because it looks for every control before it clicks any.
  const rows = [
    ["radios", ["read", "set", "off", "add"], "forwarding radio"],
    ["to", ["read"], "address list"],
    ["to", ["set"], "address list"],
    ["disp", ["set"], "keep copy"],
    ["save", ["set", "off"], "save"],
  ] as const;
  for (const [without, scripts, control] of rows) {
    for (const script of scripts) {
      const f = gmail({ addresses: [OURS], on: script === "off" || script === "read", to: OURS, without });
      const before = f.snap();
      const r = await run(`gmail/forwarding_${script}.js`, f.page, {
        address: OURS,
        reloaded: true,
        ...FAST,
      });
      assertEquals(r, NOT_FOUND(control), `${script} without ${without}`);
      assertEquals(f.snap(), before, `${script} without ${without}: nothing changed`);
      assertEquals(f.saves, 0);
      assertEquals(f.clicks + f.popups + f.typed.length, 0, `${script} without ${without}: nothing pressed`);
    }
  }
  // The add dialog's own controls, and the confirmation's OK.
  for (const [without, control] of [["email", "email field"], ["next", "next"]] as const) {
    const f = gmail({ without });
    assertEquals(
      await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST }),
      NOT_FOUND(control),
    );
    assertEquals(f.popups, 0, without);
  }
  const g = gmail({ without: "ok" });
  g.verified();
  assertEquals(await run("gmail/forwarding_add_done.js", g.page, FAST), NOT_FOUND("ok"));
  // A page that is not Gmail's settings at all (a sign-in or error page that answered on the origin).
  const h = gmail({ lateDraw: "never" });
  assertEquals(
    await run("gmail/forwarding_read.js", h.page, { address: OURS, reloaded: true, ...FAST }),
    NOT_FOUND("section"),
  );
});

Deno.test("gmail_forwarding_off_turns_it_off_and_reads_it_back", async () => {
  const f = gmail({ addresses: [OURS], on: true, to: OURS });
  const r = await run("gmail/forwarding_off.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", forwarding: "off", changed: true });
  assertEquals(f.saves, 1);
  assertEquals(f.model.on, false);
  // Already off: nothing to do and nothing saved.
  const g = gmail({ addresses: [OURS] });
  assertEquals(await run("gmail/forwarding_off.js", g.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "off",
    changed: false,
  });
  assertEquals(g.saves, 0);
});

Deno.test("gmail_forwarding_off_leaves_forwarding_whose_address_it_cannot_read", async () => {
  // Checkpoint C, R4: forwarding is on but the list shows no address (no option, or a blank one drawn
  // before it is filled in). It may be the student's own, so nothing is clicked and the list is named.
  for (const addresses of [[], ["", OURS]]) {
    const f = gmail({ addresses, on: true, to: "" });
    const before = f.snap();
    const r = await run("gmail/forwarding_off.js", f.page, { address: OURS, ...FAST });
    assertEquals(r, NOT_FOUND("address list"), JSON.stringify(addresses));
    assertEquals(f.clicks, 0);
    assertEquals(f.saves, 0);
    assertEquals(f.snap(), before);
  }
});

Deno.test("gmail_remove_address_removes_it", async () => {
  const f = gmail({ addresses: [OURS] });
  const r = await run("gmail/forwarding_remove_address.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", removed: true, changed: true });
  assertEquals(f.removed, [OURS]);
  assertEquals(f.model.addresses, []);
  // Gone already: nothing to remove and nothing clicked.
  const g = gmail({ addresses: [OTHER] });
  assertEquals(await run("gmail/forwarding_remove_address.js", g.page, { address: OURS, ...FAST }), {
    result: "ok",
    removed: false,
    changed: false,
  });
  assertEquals(g.removed, []);
});

Deno.test("gmail_remove_address_leaves_every_other_forwarding_address", async () => {
  // Forwarding is off (forwarding_off.js has run) and the list already shows ours: Remove acts on ours and
  // on nothing else (§10, R4).
  const f = gmail({ addresses: [OTHER, OURS, THIRD], on: false, to: OURS, disp: "read" });
  const r = await run("gmail/forwarding_remove_address.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", removed: true, changed: true });
  assertEquals(f.removed, [OURS]);
  assertEquals(f.model, { addresses: [OTHER, THIRD], on: false, to: OTHER, disp: "read" });
  assertEquals(f.saves, 0);
  // Ours is the one forwarding goes to: the others are still listed.
  const g = gmail({ addresses: [OTHER, OURS, THIRD], on: true, to: OURS });
  await run("gmail/forwarding_remove_address.js", g.page, { address: OURS, ...FAST });
  assertEquals(g.removed, [OURS]);
  assertEquals(g.model.addresses, [OTHER, THIRD]);
  // Remove's button cannot be found: nothing is clicked, nothing is removed.
  const h = gmail({ addresses: [OTHER, OURS], to: OURS, without: "remove" });
  assertEquals(
    await run("gmail/forwarding_remove_address.js", h.page, { address: OURS, ...FAST }),
    NOT_FOUND("remove"),
  );
  assertEquals(h.model.addresses, [OTHER, OURS]);
});

Deno.test("gmail_remove_address_does_nothing_while_forwarding_goes_to_another_address", async () => {
  // Checkpoint C, R4: that Remove acts on the list's selected, unsaved address is unverified on real Gmail
  // (plan, live checks for T11 and T18c). While forwarding is saved to another address the script names a
  // needs-you and clicks nothing, so the student's own forward-to address cannot be deleted by a guess.
  const f = gmail({ addresses: [OTHER, OURS, THIRD], on: true, to: OTHER, disp: "read" });
  const before = f.snap();
  const r = await run("gmail/forwarding_remove_address.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "needs you", reason: "forwarding goes to another address" });
  assertEquals(f.removed, []);
  assertEquals(f.saves, 0);
  assertEquals(f.snap(), before);
});

Deno.test("gmail_remove_address_never_changes_the_selection_before_remove", async () => {
  // Checkpoint C, R4: until the T18c live run passes, it is unverified whether Remove acts on the list's
  // selected option or on the saved forward-to address. So the script acts only when ours is already the
  // selected option, or the only one; it never selects ours itself. Forwarding is off and the student's
  // own address is the one the list shows (setup added ours but failed before turning forwarding on, or
  // the student switched back and turned forwarding off): needs-you, and nothing is clicked or selected.
  const f = gmail({ addresses: [OTHER, OURS, THIRD], on: false, to: OTHER, disp: "read" });
  const before = f.snap();
  const r = await run("gmail/forwarding_remove_address.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "needs you", reason: "another address is selected" });
  assertEquals(f.clicks, 0);
  assertEquals(f.removed, []);
  assertEquals(f.el("dlg"), null);
  assertEquals(selectedText(f), OTHER, "the list's selection is left as it was");
  assertEquals(f.snap(), before);
  // Nothing saved and the list shows its first option, which is not ours: the same.
  const g = gmail({ addresses: [OTHER, OURS], on: false });
  assertEquals(await run("gmail/forwarding_remove_address.js", g.page, { address: OURS, ...FAST }), {
    result: "needs you",
    reason: "another address is selected",
  });
  assertEquals(g.clicks, 0);
  assertEquals(g.model.addresses, [OTHER, OURS]);
  // Ours is the only option: Remove can act on nothing else, so it is removed.
  const h = gmail({ addresses: [OURS], on: false });
  assertEquals(await run("gmail/forwarding_remove_address.js", h.page, { address: OURS, ...FAST }), {
    result: "ok",
    removed: true,
    changed: true,
  });
  assertEquals(h.removed, [OURS]);
});

Deno.test("gmail_a_script_without_an_address_changes_nothing", async () => {
  for (const script of ["read", "add", "set", "off", "remove_address"]) {
    const f = gmail({ addresses: [OURS], on: true, to: OURS });
    const before = f.snap();
    assertEquals(await run(`gmail/forwarding_${script}.js`, f.page, {}), {
      result: "failed",
      reason: "no address",
    });
    assertEquals(f.snap(), before, script);
    assertEquals(f.saves + f.popups + f.removed.length, 0, script);
  }
});

// Live 2026-10-18 (T18c step 4), read from Gmail's real Settings > Forwarding and POP/IMAP page.

Deno.test("gmail_add_address_on_a_fresh_gmail_with_only_the_add_button_presses_add", async () => {
  // No forwarding address yet: the section shows only the Add button, no forwarding radios.
  const f = gmail({ fresh: true });
  assertEquals(f.page.document.querySelectorAll('input[name="sx_em"]').length, 0);
  assert(f.el("add"), "the Add button is there");
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", stage: "asked", changed: true });
  assertEquals(f.typed, [OURS]);
  assertEquals(f.popups, 1);
  assertEquals(f.decoy, 0);
  f.verified();
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, FAST), { result: "ok", sent: true });
});

Deno.test("gmail_the_other_scripts_find_no_forwarding_radio_on_a_fresh_gmail_and_change_nothing", async () => {
  for (const script of ["read", "set", "off"]) {
    const f = gmail({ fresh: true });
    const r = await run(`gmail/forwarding_${script}.js`, f.page, { address: OURS, ...FAST });
    assertEquals(r, NOT_FOUND("forwarding radio"), script);
    assertEquals(f.saves + f.popups + f.removed.length, 0, script);
  }
});

Deno.test("gmail_the_forward_a_copy_radio_is_found_by_its_row_text_on_the_live_shape", async () => {
  // The "Forward a copy" radio has no label[for], no aria-label and no wrapping label: its text is in the next
  // TD of its row, which also holds both selects. The POP section has a select with the same options.
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OURS, disp: "read" });
  const on = f.el("r-on");
  assertEquals(on.getAttribute("name"), "sx_em");
  assertEquals(on.getAttribute("aria-label"), null);
  assertEquals(on.closest("label"), null);
  assertEquals(f.page.document.querySelectorAll('label[for="r-on"]').length, 0);
  // read: on, ours, the copy is not kept (the POP select's "keep" is never taken for the row's).
  assertEquals(await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "on",
    to: "ours",
    keep_copy: false,
    listed: true,
  });
  // set fixes the copy; off turns it off; remove then takes the address out. All on the live shape.
  assertEquals(await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST }), {
    ...KEPT,
    changed: true,
  });
  assertEquals(f.model.disp, "keep");
  assertEquals(await run("gmail/forwarding_off.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "off",
    changed: true,
  });
  assertEquals(f.model.on, false);
  assertEquals(await run("gmail/forwarding_remove_address.js", f.page, { address: OTHER, ...FAST }), {
    result: "needs you",
    reason: "another address is selected",
  });
  const g = gmail({ addresses: [OURS], on: true, to: OURS });
  await run("gmail/forwarding_off.js", g.page, { address: OURS, ...FAST });
  const r = await run("gmail/forwarding_remove_address.js", g.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", removed: true, changed: true });
  assertEquals(g.removed, [OURS]);
  // The add script reads the same radio: forwarding is on to another address.
  const h = gmail({ addresses: [OTHER, OURS], on: true, to: OTHER });
  assertEquals(await run("gmail/forwarding_add.js", h.page, { address: OURS, ...FAST }), {
    result: "elsewhere",
  });
});

Deno.test("gmail_an_address_added_but_not_verified_is_not_listed", async () => {
  // The forward-to select has 0 options; the page shows a verify text instead.
  const f = gmail({ unverified: OURS });
  assertEquals(f.page.document.querySelectorAll("#to option").length, 0);
  const r = await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", forwarding: "off", to: "none", keep_copy: true, listed: false });
  // Set cannot pick an address that is not an option: it is not found, nothing saved.
  assertEquals(
    await run("gmail/forwarding_set.js", f.page, { address: OURS, reloaded: true, ...FAST }),
    NOT_FOUND("address list"),
  );
  assertEquals(f.saves, 0);
});

Deno.test("gmail_the_older_wrapping_label_shape_is_still_read", async () => {
  const f = gmail({ labelled: true, addresses: [OURS], on: true, to: OURS });
  assertEquals(await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "on",
    to: "ours",
    keep_copy: true,
    listed: true,
  });
});

// Live Gmail (2026-10-18): the dialogs' buttons are <button> elements named by their text. The page option
// `buttonDialogs` draws them so; the section's own controls stay <input type=button>.
Deno.test("gmail_add_address_presses_button_elements_in_the_dialogs", async () => {
  for (const proceed of [false, true]) {
    const f = gmail({ buttonDialogs: true, proceed });
    // The dialog as it is drawn: <button> elements named by their text, no <input type=button> (the page's
    // hidden dialog is a decoy, so only the drawn one, #dlg, is looked at).
    f.el("add").click();
    assertEquals(f.page.document.querySelectorAll('#dlg input[type="button"]').length, 0);
    assertEquals(
      [...f.page.document.querySelectorAll("#dlg button")].map((b: { textContent: string }) => b.textContent),
      ["Next", "Cancel"],
    );
    const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
    assertEquals(r, { result: "ok", stage: "asked", changed: true }, `proceed ${proceed}`);
    assertEquals(f.typed, [OURS]);
    assertEquals(f.popups, 1);
    // The look-alike <button>Next</button> outside the dialog is never clicked.
    assertEquals(f.decoy, 0);
  }
});

Deno.test("gmail_add_address_cancels_a_button_dialog_when_next_is_missing", async () => {
  const f = gmail({ buttonDialogs: true, without: "next" });
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, NOT_FOUND("next"));
  assertEquals(f.popups, 0);
  assert(f.el("dlg") === null, "Cancel closed the dialog");
  assertEquals(f.decoy, 0);
});

Deno.test("gmail_add_address_cancels_an_input_dialog_when_next_is_missing", async () => {
  const f = gmail({ without: "next" });
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, NOT_FOUND("next"));
  assert(f.el("dlg") === null, "Cancel closed the dialog");
});

Deno.test("gmail_add_done_presses_a_button_element_ok", async () => {
  const f = gmail({ buttonDialogs: true });
  f.verified();
  assertEquals(f.page.document.querySelectorAll('#dlg input[type="button"]').length, 0);
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, FAST), { result: "ok", sent: true });
  assertEquals(f.oks, 1);
  assert(f.el("dlg") === null, "OK closed the dialog");
  // A confirmation dialog with no OK of either kind is still not found.
  const g = gmail({ buttonDialogs: true, without: "ok" });
  g.verified();
  assertEquals(await run("gmail/forwarding_add_done.js", g.page, FAST), NOT_FOUND("ok"));
});

Deno.test("gmail_remove_address_confirms_a_button_element_dialog", async () => {
  const f = gmail({ buttonDialogs: true, addresses: [OURS] });
  const r = await run("gmail/forwarding_remove_address.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", removed: true, changed: true });
  assertEquals(f.removed, [OURS]);
  assertEquals(f.model.addresses, []);
});

// Review of ceae879/35c7738 (T18c, 2026-10-18).

Deno.test("gmail_add_address_on_a_page_with_radios_it_does_not_know_is_not_fresh_and_presses_nothing", async () => {
  // No Remove control, no Forward-a-copy radio, but a radio of the section that is not one of ours: that is
  // not the fresh state (which has no radio, no select and no Remove control), so Add is not pressed.
  const f = gmail({ fresh: true, strangeRadios: true });
  assertEquals(
    await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST }),
    NOT_FOUND("forwarding radio"),
  );
  assertEquals(f.clicks + f.popups + f.typed.length, 0);
  assert(f.el("dlg") === null, "no dialog was opened");
  // Both radios named, only one of them: still a page the script does not know.
  const g = gmail({ addresses: [OURS], strangeRadios: true, without: "radios" });
  assertEquals(
    await run("gmail/forwarding_add.js", g.page, { address: OURS, ...FAST }),
    NOT_FOUND("forwarding radio"),
  );
  assertEquals(g.clicks + g.popups + g.typed.length, 0);
});

Deno.test("gmail_add_address_on_an_unverified_address_presses_add_again", async () => {
  // The address was added but not verified: it is not an option of the forward-to list (0 options), so it is
  // not listed, and Add is pressed again to ask for the popup (header comment of forwarding_add.js).
  for (const buttonDialogs of [false, true]) {
    const f = gmail({ unverified: OURS, buttonDialogs });
    assertEquals(f.page.document.querySelectorAll("#to option").length, 0);
    const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
    assertEquals(r, { result: "ok", stage: "asked", changed: true }, `buttonDialogs ${buttonDialogs}`);
    assertEquals(f.typed, [OURS]);
    assertEquals(f.popups, 1);
    assertEquals(f.decoy, 0);
  }
});

Deno.test("gmail_add_address_presses_buttons_only_in_the_dialog_it_opened", async () => {
  // A [role=dialog] with its own Next, Cancel and Proceed sits on the page before the add dialog, ahead of it
  // in the DOM. It is never clicked: Next and Cancel are the add dialog's, and Proceed is a dialog that
  // appeared after Add and names forwarding or confirming.
  const hidden = (f: ReturnType<typeof gmail>) => f.page.document.getElementById("decoy-dlg");
  for (const buttonDialogs of [false, true]) {
    for (const proceed of [false, true]) {
      const f = gmail({ buttonDialogs, proceed });
      const before = hidden(f);
      assert(before !== null, "the page has a dialog of its own before Add");
      assert(before.compareDocumentPosition(f.el("add")) & 4, "and it is ahead of the section in the DOM");
      const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
      const why = `buttonDialogs ${buttonDialogs} proceed ${proceed}`;
      assertEquals(r, { result: "ok", stage: "asked", changed: true }, why);
      assertEquals(f.typed, [OURS], why);
      assertEquals(f.popups, 1, why);
      assertEquals(f.decoy, 0, why);
      assert(hidden(f) === before, `${why}: the page's own dialog is untouched`);
    }
  }
  // Next missing from the add dialog: its own Cancel is pressed, and the decoy's Next and Cancel are not.
  for (const buttonDialogs of [false, true]) {
    const f = gmail({ buttonDialogs, without: "next" });
    assertEquals(await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST }), NOT_FOUND("next"));
    assertEquals(f.popups, 0);
    assert(f.el("dlg") === null, "the add dialog was cancelled");
    assertEquals(f.decoy, 0);
    assert(hidden(f) !== null, "the page's own dialog is still there");
  }
});

Deno.test("gmail_add_done_reads_back_the_verify_row_when_the_popup_confirmed_with_no_dialog", async () => {
  // Live 2026-10-18: Google's popup did the whole confirmation itself and the opener drew no dialog; the
  // address was added, and Gmail's Forwarding section shows a "Verify <address>" row with Re-send and Remove.
  const f = gmail({ silentPopup: true });
  const r = await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", stage: "asked", changed: true });
  assertEquals(f.popups, 1);
  assert(f.el("dlg") === null, "no dialog is drawn");
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    sent: true,
  });
  // Re-send email and Remove address are never pressed, and nothing else is.
  assertEquals(f.verifyClicks, 0);
  assertEquals(f.oks, 0);
  assertEquals(f.removed, []);
  assertEquals(f.decoy, 0);
});

Deno.test("gmail_add_done_readback_matches_the_address_blind_to_case_and_spaces", async () => {
  const f = gmail({ verifyRow: OURS.toUpperCase() });
  const args = { address: `  ${OURS}  `, ...FAST };
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, args), { result: "ok", sent: true });
  assertEquals(f.verifyClicks, 0);
});

Deno.test("gmail_add_done_readback_ignores_a_verify_row_for_a_different_address", async () => {
  const f = gmail({ verifyRow: OTHER });
  const r = await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, NOT_FOUND("confirmation dialog"));
  assertEquals(f.verifyClicks, 0);
  assertEquals(f.clicks, 0);
});

Deno.test("gmail_add_done_readback_ignores_text_that_names_the_address_but_is_not_a_verify_row", async () => {
  // The older "verification email was sent" sentence is not a Verify row, and an address in no row is nothing.
  const f = gmail({ unverified: OURS });
  assertEquals(
    await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST }),
    NOT_FOUND("confirmation dialog"),
  );
  const g = gmail();
  assertEquals(
    await run("gmail/forwarding_add_done.js", g.page, { address: OURS, ...FAST }),
    NOT_FOUND("confirmation dialog"),
  );
  assertEquals(g.clicks, 0);
});

Deno.test("gmail_add_done_readback_accepts_an_address_that_is_already_confirmed", async () => {
  const f = gmail({ addresses: [OTHER, OURS.toUpperCase()] });
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    sent: true,
  });
  assertEquals(f.clicks, 0);
});

Deno.test("gmail_add_done_prefers_the_dialog_when_it_shows_even_with_an_address", async () => {
  const f = gmail({ buttonDialogs: true });
  f.verified();
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    sent: true,
  });
  assertEquals(f.oks, 1);
  assert(f.el("dlg") === null, "OK closed the dialog");
});

Deno.test("gmail_add_done_readback_goes_to_the_section_and_follows_the_reload_contract", async () => {
  // The opener is elsewhere: the read-back goes to the Forwarding section by the hash.
  const f = gmail({ at: "#inbox", verifyRow: OURS });
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    sent: true,
  });
  assert(f.hashes.includes(SECTION), "it went to the section");
  // The section does not draw until a reload: the script asks for it, then reads after it.
  const g = gmail({ lateDraw: "reload", verifyRow: OURS });
  assertEquals(await run("gmail/forwarding_add_done.js", g.page, { address: OURS, ...FAST }), {
    result: "reloading",
  });
  assertEquals(g.reloads, 1);
  assertEquals(
    await run("gmail/forwarding_add_done.js", g.page, { address: OURS, reloaded: true, ...FAST }),
    {
      result: "ok",
      sent: true,
    },
  );
  // A section that never draws is not found once reloaded.
  const h = gmail({ lateDraw: "never", verifyRow: OURS });
  assertEquals(
    await run("gmail/forwarding_add_done.js", h.page, { address: OURS, reloaded: true, ...FAST }),
    NOT_FOUND("confirmation dialog"),
  );
  assertEquals(h.reloads, 0);
});

// Live Gmail (2026-10-18, real mailbox, structure only): the forward-to list's option values are indexes; the
// text is the bare address, then "--", then "Remove <address>" per address; once forwarding is on to an address
// its option reads "<address> (in use)" and its Remove option "Remove <address> (in use)". An option is matched
// by the one address its text names.
Deno.test("gmail_the_page_draws_the_live_forward_to_list_with_in_use_on_the_address_forwarding_goes_to", () => {
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OURS });
  assertEquals(optionTexts(f), [
    OTHER,
    `${OURS} (in use)`,
    "--",
    `Remove ${OTHER}`,
    `Remove ${OURS} (in use)`,
  ]);
  assertEquals([...f.el("to").options].map((o: { value: string }) => o.value), ["0", "1", "2", "3", "4"]);
  assertEquals(optionTexts(gmail({ addresses: [OURS] })), [OURS, "--", `Remove ${OURS}`]);
});

Deno.test("gmail_forwarding_set_turns_on_and_reads_back_ok_with_the_in_use_suffix", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: false, disp: "archive" });
  assertEquals(await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST }), {
    ...KEPT,
    changed: true,
  });
  assertEquals(f.model, { addresses: [OTHER, OURS], on: true, to: OURS, disp: "keep" });
  assertEquals(selectedText(f), `${OURS} (in use)`, "the read-back saw the suffix");
  // Again, ours is the one in use: nothing to change, nothing saved.
  assertEquals(await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST }), {
    ...KEPT,
    changed: false,
  });
  assertEquals(f.saves, 1);
  // The address is matched blind to case.
  const g = gmail({ addresses: [OURS.toUpperCase()], on: true, to: OURS.toUpperCase(), disp: "trash" });
  assertEquals(await run("gmail/forwarding_set.js", g.page, { address: OURS, ...FAST }), {
    ...KEPT,
    changed: true,
  });
});

Deno.test("gmail_forwarding_read_reports_ours_and_listed_when_forwarding_is_on_to_ours_in_use", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OURS });
  assertEquals(await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "on",
    to: "ours",
    keep_copy: true,
    listed: true,
  });
  // Off, the list still names ours (not in use): listed, to none.
  const g = gmail({ addresses: [OURS], on: false });
  assertEquals(await run("gmail/forwarding_read.js", g.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "off",
    to: "none",
    keep_copy: true,
    listed: true,
  });
});

Deno.test("gmail_forwarding_read_reports_elsewhere_for_a_different_address_in_use_and_still_lists_ours", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OTHER });
  const r = await run("gmail/forwarding_read.js", f.page, { address: OURS, ...FAST });
  assertEquals(r, { result: "ok", forwarding: "on", to: "elsewhere", keep_copy: true, listed: true });
  // set, off and add all stop at elsewhere and touch nothing.
  for (const script of ["set", "off", "add"]) {
    const before = f.snap();
    assertEquals(await run(`gmail/forwarding_${script}.js`, f.page, { address: OURS, ...FAST }), {
      result: "elsewhere",
    }, script);
    assertEquals(f.snap(), before, script);
    assertEquals(f.saves + f.popups + f.typed.length, 0, script);
  }
});

Deno.test("gmail_forwarding_add_returns_listed_when_ours_is_confirmed_and_in_use_and_presses_no_add", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OURS });
  assertEquals(await run("gmail/forwarding_add.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    stage: "listed",
    changed: false,
  });
  assertEquals(f.clicks + f.popups + f.typed.length, 0);
  assert(f.el("dlg") === null, "no dialog opened");
});

Deno.test("gmail_forwarding_off_turns_it_off_when_the_option_reads_in_use", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OURS });
  assertEquals(await run("gmail/forwarding_off.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    forwarding: "off",
    changed: true,
  });
  assertEquals(f.saves, 1);
  assertEquals(f.model.on, false);
});

Deno.test("gmail_forwarding_remove_address_removes_ours_when_its_remove_option_reads_in_use", async () => {
  const f = gmail({ addresses: [OTHER, OURS, THIRD], on: true, to: OURS });
  assert(optionTexts(f).includes(`Remove ${OURS} (in use)`));
  assertEquals(await run("gmail/forwarding_remove_address.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    removed: true,
    changed: true,
  });
  assertEquals(f.removed, [OURS]);
  assertEquals(f.model.addresses, [OTHER, THIRD]);
  assert(!optionTexts(f).some((t: string) => t.includes(OURS)), "no option names ours any longer");
  // Ours is the only address: the list has its Remove and separator options too, yet it is the only target.
  const g = gmail({ addresses: [OURS], on: true, to: OURS });
  assertEquals(await run("gmail/forwarding_remove_address.js", g.page, { address: OURS, ...FAST }), {
    result: "ok",
    removed: true,
    changed: true,
  });
  assertEquals(g.removed, [OURS]);
  // Another address in use: the script still asks the student and clicks nothing.
  const h = gmail({ addresses: [OTHER, OURS], on: true, to: OTHER });
  assertEquals(await run("gmail/forwarding_remove_address.js", h.page, { address: OURS, ...FAST }), {
    result: "needs you",
    reason: "forwarding goes to another address",
  });
  assertEquals(h.clicks, 0);
});

Deno.test("gmail_a_separator_or_a_remove_option_is_never_a_forward_target", async () => {
  // Ours is not a confirmed address; a "--" option carries it as its value and a "Remove" option names it.
  const f = gmail({ addresses: [OTHER] });
  f.el("to").insertAdjacentHTML(
    "beforeend",
    `<option value="${OURS}">--</option><option value="9">Remove ${OURS}</option>` +
      `<option value="${OURS}">Remove ${OURS} (in use)</option>`,
  );
  const args = { address: OURS, reloaded: true, ...FAST };
  assertEquals((await run("gmail/forwarding_read.js", f.page, args)).listed, false);
  assertEquals(await run("gmail/forwarding_set.js", f.page, args), NOT_FOUND("address list"));
  assertEquals(await run("gmail/forwarding_remove_address.js", f.page, args), {
    result: "ok",
    removed: false,
    changed: false,
  });
  assertEquals(f.saves + f.removed.length, 0);
  // add does not take it for confirmed: it presses Add.
  assertEquals(await run("gmail/forwarding_add.js", f.page, args), {
    result: "ok",
    stage: "asked",
    changed: true,
  });
  assertEquals(f.typed, [OURS]);
});

Deno.test("gmail_add_done_readback_accepts_an_address_that_is_confirmed_and_in_use", async () => {
  const f = gmail({ addresses: [OTHER, OURS], on: true, to: OURS });
  assertEquals(await run("gmail/forwarding_add_done.js", f.page, { address: OURS, ...FAST }), {
    result: "ok",
    sent: true,
  });
  assertEquals(f.clicks, 0);
  // A Remove option naming it is not a confirmation.
  const g = gmail({ addresses: [OTHER] });
  g.el("to").insertAdjacentHTML("beforeend", `<option value="9">Remove ${OURS}</option>`);
  assertEquals(
    await run("gmail/forwarding_add_done.js", g.page, { address: OURS, ...FAST }),
    NOT_FOUND("confirmation dialog"),
  );
});

Deno.test("gmail_forwarding_set_reloads_once_when_the_forward_to_list_predates_the_confirmation", async () => {
  // Live 2026-10-18: the page drew its list before the server confirmed our address; a hash round trip does not
  // redraw it. The first run reloads and clicks nothing; the rerun, told it reloaded, turns forwarding on.
  const f = gmail({ addresses: [OTHER, OURS], staleList: OURS, disp: "archive" });
  assertEquals(optionTexts(f), [OTHER, "--", `Remove ${OTHER}`]);
  assertEquals(await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST }), {
    result: "reloading",
  });
  assertEquals(f.reloads, 1);
  assertEquals(f.clicks + f.saves, 0, "nothing is clicked before the reload");
  assertEquals(f.model.on, false);
  assertEquals(await run("gmail/forwarding_set.js", f.page, { address: OURS, reloaded: true, ...FAST }), {
    ...KEPT,
    changed: true,
  });
  assertEquals(f.model, { addresses: [OTHER, OURS], on: true, to: OURS, disp: "keep" });
  assertEquals(f.reloads, 1, "it reloads once, never twice");
});

Deno.test("gmail_forwarding_set_with_a_list_still_stale_after_the_reload_is_not_found", async () => {
  // The reload brought no fresh list (the address is not confirmed after all): told it reloaded, it stops.
  const f = gmail({ addresses: [OTHER] });
  assertEquals(
    await run("gmail/forwarding_set.js", f.page, { address: OURS, reloaded: true, ...FAST }),
    NOT_FOUND("address list"),
  );
  assertEquals(f.reloads, 0);
  assertEquals(f.clicks + f.saves, 0);
});
