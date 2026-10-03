// Spec test 35, the Microsoft 365 forwarding half (plan T10.2a). Each test runs the compiled-in script
// text from `m365/` against a synthetic Settings > Mail > Forwarding page. The page is modelled on the
// spikes' selectors (research note 6: the switch is `input[role=switch]`, read by `checked`); the rest
// of its controls are found by their visible names, which the page here carries. No address is real.
import { assert, assertEquals } from "@std/assert";
import { disable, El, load, Page, run } from "./test_dom.ts";

const OURS = "box-1@in.example.invalid";
const OTHER = "someone@elsewhere.example.invalid";

interface Opts {
  on?: boolean;
  to?: string;
  keep?: boolean;
  /** aria-checked as the page left it, which is not what the switch holds. */
  aria?: "true" | "false";
  without?: "switch" | "address" | "keep" | "save";
  /** The Save button stays enabled after a click: the page never says it saved. */
  sticky?: boolean;
}

interface Snap {
  on: boolean;
  to: string;
  keep: boolean;
}

interface Forwarding {
  page: Page;
  saves: number;
  saved: Snap | null;
  snap(): Snap;
  el(id: string): El;
}

function forwarding(o: Opts = {}): Forwarding {
  const on = o.on ?? false;
  const w = o.without;
  const aria = o.aria ?? (on ? "true" : "false");
  const page = load(`
    <div role="main"><h2>Forwarding</h2>
      <div><input id="convo" type="checkbox" role="switch" aria-checked="true" checked>
        <label for="convo">Group messages by conversation</label></div>
      ${
    w === "switch"
      ? ""
      : `<div><input id="fwd" type="checkbox" role="switch" aria-checked="${aria}" ${on ? "checked" : ""}>
        <label for="fwd">Enable forwarding</label></div>`
  }
      ${
    w === "address" ? "" : `<div><label for="to">Forward my email to:</label>
        <input id="to" type="text" value="${o.to ?? ""}"></div>`
  }
      ${
    w === "keep" ? "" : `<div><input id="keep" type="checkbox" ${o.keep ? "checked" : ""}>
        <label for="keep">Keep a copy of forwarded messages</label></div>`
  }
      ${w === "save" ? "" : `<button id="save" type="button">Save</button>`}
      <button id="discard" type="button">Discard</button></div>`);
  const d = page.document;
  const f: Forwarding = {
    page,
    saves: 0,
    saved: null,
    el: (id) => d.getElementById(id),
    snap: () => ({
      on: !!f.el("fwd")?.checked,
      to: f.el("to")?.value ?? "",
      keep: !!f.el("keep")?.checked,
    }),
  };
  let dirty = false;
  const sync = () => {
    const live = !!f.el("fwd")?.checked;
    if (f.el("to")) disable(f.el("to"), !live);
    if (f.el("keep")) disable(f.el("keep"), !live && !f.el("keep").checked);
    if (f.el("save")) disable(f.el("save"), !dirty);
  };
  // What a browser does with a click on a checkbox: it toggles `checked` and leaves aria-checked alone.
  for (const box of d.querySelectorAll('input[type="checkbox"]')) {
    box.addEventListener("click", () => {
      box.checked = !box.checked;
      dirty = true;
      sync();
    });
  }
  f.el("to")?.addEventListener("input", () => {
    dirty = true;
    sync();
  });
  f.el("save")?.addEventListener("click", () => {
    if (f.el("save").hasAttribute("disabled")) return;
    f.saves++;
    f.saved = f.snap();
    dirty = o.sticky ? dirty : false;
    sync();
  });
  sync();
  return f;
}

const NOT_FOUND = (control: string) => ({ result: "not found", control });

Deno.test("m365_forwarding_set_turns_it_on_with_keep_copy_and_reads_it_back", async () => {
  const f = forwarding({ on: false, keep: false });
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS });
  assertEquals(r, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: true });
  assertEquals(f.saves, 1);
  assertEquals(f.saved, { on: true, to: OURS, keep: true });
  // The decoy switch on the same page is not the forwarding switch and is left alone.
  assert(f.el("convo").checked);
});

Deno.test("m365_forwarding_set_on_a_page_already_set_saves_nothing", async () => {
  const f = forwarding({ on: true, to: OURS.toUpperCase(), keep: true });
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS });
  assertEquals(r, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: false });
  assertEquals(f.saves, 0);
});

Deno.test("m365_forwarding_set_fixes_a_missing_keep_copy_when_it_is_already_ours", async () => {
  const f = forwarding({ on: true, to: OURS, keep: false });
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS });
  assertEquals(r, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: true });
  assertEquals(f.saved, { on: true, to: OURS, keep: true });
});

Deno.test("m365_the_switch_is_read_by_checked_not_aria_checked", async () => {
  // aria-checked says on, the switch holds off.
  const off = forwarding({ on: false, aria: "true" });
  const a = await run("m365/forwarding_read.js", off.page, { address: OURS });
  assertEquals(a, { result: "ok", forwarding: "off", to: "none", keep_copy: false });
  // aria-checked says off, the switch holds on.
  const on = forwarding({ on: true, aria: "false", to: OURS, keep: true });
  const b = await run("m365/forwarding_read.js", on.page, { address: OURS });
  assertEquals(b, { result: "ok", forwarding: "on", to: "ours", keep_copy: true });
  // Setting turns a switch whose aria-checked is stale on, by clicking it once.
  const stale = forwarding({ on: false, aria: "true" });
  const r = await run("m365/forwarding_set.js", stale.page, { address: OURS });
  assertEquals(r.result, "ok");
  assertEquals(stale.saved?.on, true);
});

Deno.test("m365_forwarding_elsewhere_is_reported_not_replaced", async () => {
  for (const script of ["forwarding_set.js", "forwarding_off.js"]) {
    const f = forwarding({ on: true, to: OTHER, keep: false });
    const r = await run(`m365/${script}`, f.page, { address: OURS });
    assertEquals(r, { result: "elsewhere" }, script);
    assertEquals(f.saves, 0, script);
    assertEquals(f.snap(), { on: true, to: OTHER, keep: false }, script);
  }
  // A read names it as elsewhere and never returns the other address.
  const f = forwarding({ on: true, to: OTHER });
  const r = await run("m365/forwarding_read.js", f.page, { address: OURS });
  assertEquals(r, { result: "ok", forwarding: "on", to: "elsewhere", keep_copy: false });
  assert(!JSON.stringify(r).includes("elsewhere.example"));
});

Deno.test("m365_a_missing_control_returns_not_found", async () => {
  for (const script of ["forwarding_read.js", "forwarding_set.js", "forwarding_off.js"]) {
    const f = forwarding({ without: "switch" });
    assertEquals(await run(`m365/${script}`, f.page, { address: OURS }), NOT_FOUND("switch"), script);
    assertEquals(f.saves, 0, script);
  }
  // A control only a write needs: the script says which, and leaves the page as it found it.
  for (
    const [without, control] of [["address", "address"], ["keep", "keep copy"], ["save", "save"]] as const
  ) {
    const f = forwarding({ on: false, without });
    const before = f.snap();
    assertEquals(
      await run("m365/forwarding_set.js", f.page, { address: OURS, wait_ms: 50 }),
      NOT_FOUND(control),
      without,
    );
    assertEquals(f.snap(), before, `${without}: the switch is put back`);
    assertEquals(f.saves, 0, without);
  }
  // Forwarding is on but the address field cannot be found (a renamed label): turning it off could switch
  // off forwarding that points elsewhere, so the script says what it cannot find and clicks nothing.
  const g = forwarding({ on: true, to: OTHER, without: "address" });
  const kept = g.snap();
  assertEquals(await run("m365/forwarding_off.js", g.page, { address: OURS }), NOT_FOUND("address"));
  assertEquals(g.saves, 0);
  assertEquals(g.snap(), kept);
  // An empty page (a sign-in or error page that answered on the mail origin) has no control either.
  assertEquals(
    await run("m365/forwarding_read.js", { ...load("<p>Oops</p>") }, { address: OURS }),
    NOT_FOUND("switch"),
  );
});

Deno.test("m365_forwarding_set_replaces_elsewhere_only_with_the_students_answer", async () => {
  // Without the answer the other address is left alone.
  const a = forwarding({ on: true, to: OTHER, keep: false });
  assertEquals(await run("m365/forwarding_set.js", a.page, { address: OURS }), { result: "elsewhere" });
  assertEquals(await run("m365/forwarding_set.js", a.page, { address: OURS, replace: false }), {
    result: "elsewhere",
  });
  assertEquals(a.saves, 0);
  assertEquals(a.snap(), { on: true, to: OTHER, keep: false });
  // With it, the address is replaced, keep-copy is turned on and the page is read back.
  const b = forwarding({ on: true, to: OTHER, keep: false });
  const r = await run("m365/forwarding_set.js", b.page, { address: OURS, replace: true });
  assertEquals(r, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: true });
  assertEquals(b.saves, 1);
  assertEquals(b.saved, { on: true, to: OURS, keep: true });
  // Only a literal true counts.
  const c = forwarding({ on: true, to: OTHER });
  assertEquals(await run("m365/forwarding_set.js", c.page, { address: OURS, replace: "yes" }), {
    result: "elsewhere",
  });
  assertEquals(c.saves, 0);
});

Deno.test("m365_forwarding_off_turns_it_off_and_reads_it_back", async () => {
  const f = forwarding({ on: true, to: OURS, keep: true });
  const r = await run("m365/forwarding_off.js", f.page, { address: OURS });
  assertEquals(r, { result: "ok", forwarding: "off", changed: true });
  assertEquals(f.saves, 1);
  assertEquals(f.saved?.on, false);
  // Already off: nothing to do and nothing saved.
  const g = forwarding({ on: false });
  assertEquals(await run("m365/forwarding_off.js", g.page, { address: OURS }), {
    result: "ok",
    forwarding: "off",
    changed: false,
  });
  assertEquals(g.saves, 0);
});

Deno.test("m365_forwarding_off_leaves_forwarding_whose_address_is_empty", async () => {
  // Checkpoint C, R4: the switch is on and the "Forward to" field is drawn empty (Outlook fills it in
  // after it draws). Forwarding may go to the student's own address, so nothing is clicked.
  const f = forwarding({ on: true, to: "", keep: true });
  const before = f.snap();
  assertEquals(await run("m365/forwarding_off.js", f.page, { address: OURS }), NOT_FOUND("address"));
  assertEquals(f.saves, 0);
  assertEquals(f.snap(), before);
});

Deno.test("m365_a_save_that_does_not_settle_is_reported_failed", async () => {
  const f = forwarding({ on: false, sticky: true });
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS, settle_ms: 200 });
  assertEquals(r, { result: "failed", reason: "save did not settle" });
});

Deno.test("m365_a_script_without_an_address_changes_nothing", async () => {
  for (const script of ["forwarding_read.js", "forwarding_set.js", "forwarding_off.js"]) {
    const f = forwarding({ on: false });
    assertEquals(await run(`m365/${script}`, f.page, {}), { result: "failed", reason: "no address" }, script);
    assertEquals(f.saves, 0);
  }
});
