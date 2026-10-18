// Spec test 35, the Gmail forwarding half (plan T10.3a, spec 4.3 and 4.12 step 1). Each test runs the
// compiled-in script text from `gmail/` against a synthetic Settings > Forwarding and POP/IMAP page
// (gmail_forwarding_page.ts). No address is real.
import { assert, assertEquals } from "@std/assert";
import { gmail } from "./gmail_forwarding_page.ts";
import { run } from "./test_dom.ts";

const OURS = "box-1@in.example.invalid";
const OTHER = "someone@elsewhere.example.invalid";
const THIRD = "third@elsewhere.example.invalid";
const FAST = { wait_ms: 50, settle_ms: 200 };

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
    await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST }),
    NOT_FOUND("address list"),
  );
  assertEquals(f.saves, 0);
  const g = gmail();
  assertEquals(
    await run("gmail/forwarding_set.js", g.page, { address: OURS, ...FAST }),
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
    ["radios", ["read", "set", "off"], "forwarding radio"],
    ["to", ["read"], "address list"],
    ["to", ["set"], "address list"],
    ["disp", ["set"], "keep copy"],
    ["save", ["set", "off"], "save"],
  ] as const;
  for (const [without, scripts, control] of rows) {
    for (const script of scripts) {
      const f = gmail({ addresses: [OURS], on: script === "off" || script === "read", to: OURS, without });
      const before = f.snap();
      const r = await run(`gmail/forwarding_${script}.js`, f.page, { address: OURS, ...FAST });
      assertEquals(r, NOT_FOUND(control), `${script} without ${without}`);
      assertEquals(f.snap(), before, `${script} without ${without}: nothing changed`);
      assertEquals(f.saves, 0);
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
  assertEquals(f.el("to").value, OTHER, "the list's selection is left as it was");
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
    await run("gmail/forwarding_set.js", f.page, { address: OURS, ...FAST }),
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
