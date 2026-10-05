// Microsoft 365, Settings > Mail > Forwarding: turn forwarding on to Knowlu's address with "Keep a copy
// of forwarded messages" on, Save, and read the page back (spec 4.2 step 3). The caller has navigated
// to the Forwarding page; it re-reads the setting after navigating away and back (forwarding_read.js).
// A body of an async function of `args`: `args.address` is the Knowlu forwarding address; `args.draw_ms`
// (default 15000) bounds the wait for the dialog to draw its switch; `args.wait_ms` (default 5000) bounds
// the wait for a control the switch or a change reveals; `args.settle_ms` (default 5000) bounds the wait
// for the page to take the Save (its Save button goes away or goes disabled). It returns one of
//   { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: boolean }
//   { result: "elsewhere" }       (forwarding is on to another address and `args.replace` is not true: nothing is
//      changed, the student is asked)
//   { result: "not found", control: "switch" | "address" | "keep copy" | "save" }
//   { result: "failed", reason: "no address" | "save did not settle" | "read back differed" }
// It replaces another forwarding address only when `args.replace === true`, which the caller sets only
// from the student's answer to the "forward to Knowlu instead" question (spec 4.2 step 3).
// Live Outlook (T18c) draws the dialog after the document is complete, draws the address field and
// keep-a-copy only while the switch is on, and shows no Save until something is changed: only the switch
// is required up front. A control that does not come after the change is answered with every change put
// back (keep-a-copy, the address, then the switch), so no unsaved change is left on the page.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  const by = el.getAttribute("aria-labelledby");
  if (by) {
    const t = by.split(/\s+/).map((id) => document.getElementById(id)).filter(Boolean).map(text).join(" ");
    if (t) return t;
  }
  if (el.id) {
    for (const l of document.querySelectorAll("label")) if (l.getAttribute("for") === el.id) return text(l);
  }
  const wrap = el.closest ? el.closest("label") : null;
  return wrap ? text(wrap) : "";
};
const named = (selector, pattern) =>
  [...document.querySelectorAll(selector)].find((el) => pattern.test(nameOf(el)));
const norm = (v) => String(v || "").trim().toLowerCase();
const disabled = (el) => el.disabled === true || el.getAttribute("aria-disabled") === "true";
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
const setValue = (el, v) => {
  if (el.focus) el.focus();
  const d = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), "value");
  if (d && d.set) d.set.call(el, v);
  else el.value = v;
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
};
const switchEl = () => named('input[role="switch"]', /forward/i);
const fieldEl = () => named('input[type="text"], input[type="email"], input:not([type])', /forward.*\bto\b/i);
const keepEl = () => named('input[type="checkbox"]', /keep a copy/i);
const saveEl = () =>
  [...document.querySelectorAll('button, [role="button"]')].find((b) => /^save$/i.test(nameOf(b) || text(b)));

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };
const drawMs = Number(args.draw_ms) >= 0 ? Number(args.draw_ms) : 15000;
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const settleMs = Number(args.settle_ms) >= 0 ? Number(args.settle_ms) : 5000;
const replace = args.replace === true;

if (!(await until(() => switchEl(), drawMs))) return { result: "not found", control: "switch" };
const sw = switchEl();
if (sw.checked === true) {
  // A field drawn empty may be filled in a moment later with the student's own address: give it the wait.
  await until(() => fieldEl() && norm(fieldEl().value) !== "", waitMs);
  const there = fieldEl();
  if (!there) return { result: "not found", control: "address" };
  if (!replace && norm(there.value) !== "" && norm(there.value) !== address) return { result: "elsewhere" };
}

let flipped = false;
let typed = null;
let ticked = false;
if (sw.checked !== true) {
  sw.click();
  flipped = true;
}
const putBack = () => {
  if (ticked && keepEl()) keepEl().click();
  if (typed !== null && fieldEl()) setValue(fieldEl(), typed);
  if (flipped && switchEl()) switchEl().click();
};
const revealed = () => fieldEl() && keepEl() && !disabled(fieldEl()) && !disabled(keepEl());
const missing = () => (!fieldEl() ? "address" : !keepEl() ? "keep copy" : null);
await until(revealed, waitMs);
const lost = missing();
if (lost) {
  putBack();
  return { result: "not found", control: lost };
}
if (norm(fieldEl().value) !== address) {
  typed = fieldEl().value;
  setValue(fieldEl(), String(args.address).trim());
}
if (keepEl().checked !== true) {
  keepEl().click();
  ticked = true;
}

const changed = flipped || typed !== null || ticked;
if (changed) {
  if (!(await until(() => saveEl() && !disabled(saveEl()), waitMs))) {
    putBack();
    return { result: "not found", control: "save" };
  }
  saveEl().click();
  const saved = await until(() => !saveEl() || disabled(saveEl()), settleMs);
  if (!saved) return { result: "failed", reason: "save did not settle" };
}

const back = switchEl();
const ours = fieldEl() && norm(fieldEl().value) === address;
const kept = keepEl() && keepEl().checked === true;
if (!back || back.checked !== true || !ours || !kept) {
  return { result: "failed", reason: "read back differed" };
}
return { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed };
