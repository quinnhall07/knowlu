// Spec test 35, the Microsoft 365 forwarding half (plan T10.2a). Each test runs the compiled-in script
// text from `m365/` against a synthetic Settings > Mail > Forwarding page. The page is modelled on the
// spikes' selectors (research note 6: the switch is `input[role=switch]`, read by `checked`); the rest
// of its controls are found by their visible names, which the page here carries. No address is real.
import { assert, assertEquals } from "@std/assert";
import { El, load, Page, run } from "./test_dom.ts";

const OURS = "box-1@in.example.invalid";
const OTHER = "someone@elsewhere.example.invalid";

interface Opts {
  on?: boolean;
  to?: string;
  keep?: boolean;
  /** aria-checked as the page left it, which is not what the switch holds. */
  aria?: "true" | "false";
  without?: "switch" | "address" | "keep" | "save";
  /** The Save button stays after a click: the page never says it saved. */
  sticky?: boolean;
  /** How long after the page loads the dialog draws its controls (T18c: Outlook draws it lazily). */
  draw_ms?: number;
  /** How long after the switch turns on the address field and keep-a-copy are drawn. */
  reveal_ms?: number;
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
  /** Every checkbox click, in order: which box, and what the page held just before it. */
  clicks: Array<{ id: string; before: Snap }>;
}

// The page as live Outlook draws it (T18c, 2026-10-05, read on a university tenant): a Settings dialog with
// a "Forwarding" heading, drawn after the page loads; the switch is `input type=checkbox role=switch`
// labelled "Enable forwarding" by a <label>; the address field and keep-a-copy are drawn only while the
// switch is on; and there is **no Save button until something is changed**, so a clean page, on or off,
// has none. A save takes the change and the Save button goes away.
function forwarding(o: Opts = {}): Forwarding {
  const w = o.without;
  const page = load(`
    <div role="dialog"><div role="heading">Forwarding</div>
      <button type="button">Layout</button><button type="button">Compose and reply</button>
      <button type="button">clear</button><button type="button" aria-label="Close settings">x</button>
      <div><input id="convo" type="checkbox" role="switch" aria-checked="true" checked>
        <label for="convo">Group messages by conversation</label></div>
      <div id="body"></div><div id="foot"></div></div>`);
  const d = page.document;
  const f: Forwarding = {
    page,
    saves: 0,
    clicks: [],
    saved: null,
    el: (id) => d.getElementById(id),
    snap: () => ({ on: !!f.el("fwd")?.checked, to: f.el("to")?.value ?? "", keep: !!f.el("keep")?.checked }),
  };
  let held: Snap = { on: o.on ?? false, to: o.to ?? "", keep: o.keep ?? false };
  let dirty = false;
  const changed = () => {
    dirty = true;
    footer();
  };
  const footer = () => {
    if (w === "save") return;
    if (dirty && !f.el("save")) {
      f.el("foot").insertAdjacentHTML("beforeend", `<button id="save" type="button">Save</button>`);
      f.el("save").addEventListener("click", () => {
        f.saves++;
        f.saved = f.snap();
        held = f.saved;
        if (o.sticky) return;
        dirty = false;
        f.el("save").remove();
      });
    }
  };
  const box = (el: El) =>
    el.addEventListener("click", () => {
      f.clicks.push({ id: el.id, before: f.snap() });
      // What a browser does with a click on a checkbox: it toggles `checked` and leaves aria-checked alone.
      el.checked = !el.checked;
      if (el.id === "fwd") reveal();
      changed();
    });
  const reveal = () => {
    if (!f.el("fwd").checked) {
      f.el("rest")?.remove();
      return;
    }
    setTimeout(() => {
      if (!f.el("fwd")?.checked || f.el("rest")) return;
      const field = w === "address" ? "" : `<div><label for="to">Forward my email to:</label>
        <input id="to" type="text"></div>`;
      const keep = w === "keep" ? "" : `<div><input id="keep" type="checkbox">
        <label for="keep">Keep a copy of forwarded messages</label></div>`;
      f.el("body").insertAdjacentHTML("beforeend", `<div id="rest">${field}${keep}</div>`);
      if (f.el("to")) {
        f.el("to").value = held.to;
        f.el("to").addEventListener("input", changed);
      }
      if (f.el("keep")) {
        f.el("keep").checked = held.keep;
        box(f.el("keep"));
      }
    }, o.reveal_ms ?? 30);
  };
  box(f.el("convo"));
  if (w !== "switch") {
    setTimeout(() => {
      const aria = o.aria ?? (held.on ? "true" : "false");
      f.el("body").insertAdjacentHTML(
        "afterbegin",
        `<div><input id="fwd" type="checkbox" role="switch" aria-checked="${aria}">
          <label for="fwd">Enable forwarding</label></div>`,
      );
      f.el("fwd").checked = held.on;
      box(f.el("fwd"));
      if (held.on) reveal();
    }, o.draw_ms ?? 30);
  }
  return f;
}

/** Waits until the synthetic page has drawn what it draws on load. */
const drawn = () => new Promise((done) => setTimeout(done, 120));

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

Deno.test("m365_the_live_page_has_no_save_until_the_switch_is_turned_on", async () => {
  // T18c: Outlook shows no Save while forwarding is off and nothing has changed. The switch is all a
  // script may require up front; Save is waited for once the change is made.
  const f = forwarding({ on: false });
  await drawn();
  assertEquals(f.el("save"), null, "a clean page has no Save");
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS });
  assertEquals(r, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: true });
  assertEquals(f.saves, 1);
  assertEquals(f.saved, { on: true, to: OURS, keep: true });
});

Deno.test("m365_a_dialog_drawn_late_is_waited_for", async () => {
  // The scripts start as soon as the document is complete; the Settings dialog draws after that.
  const a = forwarding({ on: false, draw_ms: 400, reveal_ms: 300 });
  const set = await run("m365/forwarding_set.js", a.page, { address: OURS });
  assertEquals(set, { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: true });
  const b = forwarding({ on: true, to: OURS, keep: true, draw_ms: 400, reveal_ms: 300 });
  const read = await run("m365/forwarding_read.js", b.page, { address: OURS });
  assertEquals(read, { result: "ok", forwarding: "on", to: "ours", keep_copy: true });
  const c = forwarding({ on: true, to: OURS, keep: true, draw_ms: 400, reveal_ms: 300 });
  assertEquals(await run("m365/forwarding_off.js", c.page, { address: OURS }), {
    result: "ok",
    forwarding: "off",
    changed: true,
  });
  assertEquals(c.saved?.on, false);
});

Deno.test("m365_forwarding_off_waits_for_save_after_the_change_and_puts_the_switch_back_without_one", async () => {
  // On, ours and clean: no Save yet. Turning it off draws one.
  const f = forwarding({ on: true, to: OURS, keep: true });
  await drawn();
  assertEquals(f.el("save"), null, "a clean page has no Save");
  // A page that never draws Save: the switch is put back, nothing is left unsaved, nothing is saved.
  const g = forwarding({ on: true, to: OURS, keep: true, without: "save" });
  await drawn();
  const before = g.snap();
  assertEquals(await run("m365/forwarding_off.js", g.page, { address: OURS, wait_ms: 100 }), NOT_FOUND("save"));
  await drawn();
  assertEquals(g.snap(), before);
  assertEquals(g.saves, 0);
});

Deno.test("m365_forwarding_set_puts_back_every_change_when_save_never_comes", async () => {
  // Forwarding on to another address, the student said replace: the address and keep-copy are changed,
  // then Save never draws. Both are put back, so the page holds no unsaved change.
  const f = forwarding({ on: true, to: OTHER, keep: false, without: "save" });
  await drawn();
  const before = f.snap();
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS, replace: true, wait_ms: 100 });
  assertEquals(r, NOT_FOUND("save"));
  assertEquals(f.snap(), before);
  assertEquals(f.saves, 0);
});

Deno.test("m365_forwarding_set_from_off_puts_back_every_change_switch_last_when_save_never_comes", async () => {
  // Off: the switch is flipped, the address typed and keep-a-copy ticked, then Save never draws. Every
  // change is put back, keep-a-copy and the address while they are still on the page, the switch last.
  const f = forwarding({ on: false, keep: false, without: "save" });
  await drawn();
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS, wait_ms: 100 });
  assertEquals(r, NOT_FOUND("save"));
  assertEquals(f.saves, 0);
  assertEquals(f.clicks.map((c) => c.id), ["fwd", "keep", "keep", "fwd"]);
  const last = f.clicks[3].before;
  assertEquals(last, { on: true, to: "", keep: false }, "the address and keep-a-copy were put back before the switch");
  assertEquals(f.snap(), { on: false, to: "", keep: false });
});

Deno.test("m365_forwarding_set_never_replaces_an_address_not_yet_drawn", async () => {
  // A switch that is on always has an address: a field still blank after the wait is not drawn yet, and
  // may hold the student's own forwarding. Without the student's yes nothing is changed.
  const f = forwarding({ on: true, to: "", keep: false });
  await drawn();
  const before = f.snap();
  const r = await run("m365/forwarding_set.js", f.page, { address: OURS, wait_ms: 100 });
  assertEquals(r, NOT_FOUND("address"));
  assertEquals(f.saves, 0);
  assertEquals(f.clicks.length, 0);
  assertEquals(f.snap(), before);
});

Deno.test("m365_the_waits_are_bounded_by_wall_time_not_by_turns", async () => {
  // A hidden WebView2 may throttle timers: each 50 ms sleep can take far longer. The bound is a deadline
  // on the clock, so a page with no switch is given up on at draw_ms, not after draw_ms / 50 slow turns.
  const real = globalThis.setTimeout;
  // deno-lint-ignore no-explicit-any
  (globalThis as any).setTimeout = (fn: () => void, ms?: number) => real(fn, Math.max(ms ?? 0, 0) * 10);
  try {
    for (const script of ["forwarding_read.js", "forwarding_set.js", "forwarding_off.js"]) {
      const started = Date.now();
      const r = await run(`m365/${script}`, { ...load("<p>Oops</p>") }, { address: OURS, draw_ms: 300 });
      const took = Date.now() - started;
      assertEquals(r, NOT_FOUND("switch"), script);
      // Turn-counted, six throttled sleeps of 500 ms each: 3 s. On the clock: one or two sleeps.
      assert(took < 1600, `${script} waited ${took} ms for a 300 ms bound`);
    }
  } finally {
    globalThis.setTimeout = real;
  }
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
    // The switch is waited for (the dialog draws late); `draw_ms` bounds the wait on a page that has none.
    assertEquals(await run(`m365/${script}`, f.page, { address: OURS, draw_ms: 200 }), NOT_FOUND("switch"), script);
    assertEquals(f.saves, 0, script);
  }
  // A control only a write needs: the script says which, and leaves the page as it found it.
  for (
    const [without, control] of [["address", "address"], ["keep", "keep copy"], ["save", "save"]] as const
  ) {
    const f = forwarding({ on: false, without });
    await drawn();
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
  await drawn();
  const kept = g.snap();
  assertEquals(await run("m365/forwarding_off.js", g.page, { address: OURS, wait_ms: 50 }), NOT_FOUND("address"));
  assertEquals(g.saves, 0);
  assertEquals(g.snap(), kept);
  // An empty page (a sign-in or error page that answered on the mail origin) has no control either.
  assertEquals(
    await run("m365/forwarding_read.js", { ...load("<p>Oops</p>") }, { address: OURS, draw_ms: 200 }),
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
  await drawn();
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
