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
  // Forwarding is off and the list shows another address: the script selects ours before it presses
  // Remove, so Remove acts on ours and on nothing else (§10, R4).
  const f = gmail({ addresses: [OTHER, OURS, THIRD], on: false, to: OTHER, disp: "read" });
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
  const h = gmail({ addresses: [OTHER, OURS], without: "remove" });
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
