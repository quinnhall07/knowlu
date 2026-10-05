// Microsoft 365, Settings > Mail > Forwarding: turn forwarding off, Save, and read the page back (spec
// 4.12 step 1, 4.13). The caller has navigated to the Forwarding page. A body of an async function of
// `args`: `args.address` is this mailbox's Knowlu forwarding address; `args.draw_ms`, `args.wait_ms` and
// `args.settle_ms` are as in forwarding_set.js. It returns one of
//   { result: "ok", forwarding: "off", changed: boolean }   (changed is false when it was already off)
//   { result: "elsewhere" }      (forwarding goes to another address: that is the student's and is left on)
//   { result: "not found", control: "switch" | "address" | "save" }   (address: forwarding is on and the
//      field that says where to cannot be found or is empty, so it cannot be told whether the student's own
//      is there; save: no Save came after the switch was turned off, which is then turned back on)
//   { result: "failed", reason: "no address" | "save did not settle" | "read back differed" }
// Live Outlook (T18c) draws the dialog after the document is complete and shows no Save until something
// is changed, so Save is looked for only after the switch is turned off.
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
const switchEl = () => named('input[role="switch"]', /forward/i);
const fieldEl = () => named('input[type="text"], input[type="email"], input:not([type])', /forward.*\bto\b/i);
const saveEl = () =>
  [...document.querySelectorAll('button, [role="button"]')].find((b) => /^save$/i.test(nameOf(b) || text(b)));

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };
const drawMs = Number(args.draw_ms) >= 0 ? Number(args.draw_ms) : 15000;
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const settleMs = Number(args.settle_ms) >= 0 ? Number(args.settle_ms) : 5000;

if (!(await until(() => switchEl(), drawMs))) return { result: "not found", control: "switch" };
const sw = switchEl();
if (sw.checked !== true) return { result: "ok", forwarding: "off", changed: false };
await until(() => fieldEl() && norm(fieldEl().value) !== "", waitMs);
const there = fieldEl();
if (!there) return { result: "not found", control: "address" };
// Only our address, read, is turned off: a field still empty after the wait (Outlook fills it in after it
// draws) may hide the student's own, so it is named and nothing is clicked (Checkpoint C, R4).
if (norm(there.value) === "") return { result: "not found", control: "address" };
if (norm(there.value) !== address) return { result: "elsewhere" };

sw.click();
if (!(await until(() => saveEl() && !disabled(saveEl()), waitMs))) {
  if (switchEl() && switchEl().checked !== true) switchEl().click();
  return { result: "not found", control: "save" };
}
saveEl().click();
const saved = await until(() => !saveEl() || disabled(saveEl()), settleMs);
if (!saved) return { result: "failed", reason: "save did not settle" };
const back = switchEl();
if (!back || back.checked === true) return { result: "failed", reason: "read back differed" };
return { result: "ok", forwarding: "off", changed: true };
