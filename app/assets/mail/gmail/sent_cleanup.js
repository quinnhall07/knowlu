// Gmail: delete from Sent the copies Knowlu's own forwards left there, and nothing else (spec 4.3 step 7,
// 4.11 step 4, risk 13, R4). The caller has opened Sent (`#sent`). Like every step script it is the body of
// an async function of `args`: `args.address` is this mailbox's Knowlu forwarding address, `args.nonce` the
// batch's nonce, `args.max` (default 100) the most it deletes in one call and `args.wait_ms` (default 5000)
// bounds each wait. It returns one of
//   { result: "ok", deleted: number }
//   { result: "not found", control: "sent items" | "delete" }
//   { result: "failed", reason: "no address" | "no nonce" | "row not selected" | "delete did not settle",
//     deleted: number }   (the last two say how many went before it stopped)
// A row is one of ours only when BOTH hold: its subject is exactly `Knowlu <nonce>` (what forward.js sets)
// and its recipients are exactly Knowlu's address, with no one else. A copy with the nonce to someone else,
// a copy to Knowlu without the nonce, a copy to Knowlu and to someone else, and every other item stay. It
// does nothing at all unless the page is Sent, and it takes one row at a time: Gmail draws the list again
// after every Delete, so each round finds a row of ours afresh, checks the one box ticked is that row's, and
// reads the row again right before the Delete (a row that shows another message by then is let go
// and skipped). Gmail's Delete moves a copy to Trash, where it stays until
// Gmail empties it (a live check, plan T11, T18c).
// Live 2026-10-18/19 (WebView2 155): only the SHOWN grid's rows are read (Gmail keeps a previous view's grid
// in the document, hidden, offsetParent null); a row's checkbox toggles on `click()`; Delete is a
// `[role=button]` that ignores `click()` and is pressed: pointerdown, mousedown, pointerup, mouseup, click.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => (el.getAttribute("aria-label") || el.getAttribute("data-tooltip") || "").trim();
// Shown: live, a hidden view's elements have no offsetParent.
const shown = (el) => !!el && el.offsetParent !== null;
const fire = (el, type) => {
  const C = type.startsWith("pointer") && typeof PointerEvent === "function"
    ? PointerEvent
    : typeof MouseEvent === "function"
    ? MouseEvent
    : Event;
  const init = {
    bubbles: true,
    cancelable: true,
    composed: true,
    button: 0,
    buttons: type.endsWith("down") ? 1 : 0,
    pointerId: 1,
    pointerType: "mouse",
    isPrimary: true,
  };
  if (typeof window !== "undefined") init.view = window;
  const e = new C(type, init);
  for (const k of ["button", "buttons"]) {
    if (e[k] !== init[k]) Object.defineProperty(e, k, { value: init[k] });
  }
  el.dispatchEvent(e);
};
const press = (el) => {
  for (const type of ["pointerdown", "mousedown", "pointerup", "mouseup", "click"]) fire(el, type);
};
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;

const address = norm(args && args.address);
if (!ADDRESS.test(address)) return { result: "failed", reason: "no address", deleted: 0 };
const nonce = String((args && args.nonce) || "").trim();
if (!nonce || /\s/.test(nonce)) return { result: "failed", reason: "no nonce", deleted: 0 };
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const max = Number(args.max) > 0 ? Math.floor(Number(args.max)) : 100;

if (!/^#sent(\/p\d+)?$/.test(String(location.hash || ""))) {
  return { result: "not found", control: "sent items" };
}

const grid = () => [...document.querySelectorAll('table[role="grid"]')].filter(shown).pop() || null;
const rowEls = () => (grid() ? [...grid().querySelectorAll('tr[role="row"]')] : []);
const boxOf = (row) => row.querySelector('[role="checkbox"]');
const ticked = () => rowEls().filter((r) => boxOf(r) && boxOf(r).getAttribute("aria-checked") === "true");
// A row's recipients and subject (Sent's sender slot holds who it went to).
const ours = (row) => {
  const recipients = new Set(
    [...row.querySelectorAll("span[email]")].map((el) => norm(el.getAttribute("email"))),
  );
  const subject = row.querySelector(".bog");
  return !!boxOf(row) && !!subject && text(subject) === `Knowlu ${nonce}` && recipients.size === 1 &&
    recipients.has(address);
};
const del = () =>
  [...document.querySelectorAll('[role="button"], button')].filter(shown).find((b) =>
    /^delete$/i.test(nameOf(b) || text(b))
  );
const next = () => rowEls().find(ours);
if (next() && !del()) return { result: "not found", control: "delete" };

let deleted = 0;
for (let tries = 0; deleted < max && tries < max + 5; tries++) {
  const row = next();
  if (!row) break;
  boxOf(row).click();
  const picked = () => ticked().length === 1 && ticked()[0] === row;
  if (!(await until(picked, waitMs))) {
    for (const r of ticked()) boxOf(r).click();
    return { result: "failed", reason: "row not selected", deleted };
  }
  // Read again right before the Delete: the row ticked is still one of ours, and it is the only one.
  if (!ours(row)) {
    boxOf(row).click();
    continue;
  }
  if (!del()) {
    boxOf(row).click();
    return { result: "not found", control: "delete" };
  }
  press(del());
  if (!(await until(() => !document.contains(row) || !ours(row), waitMs))) {
    boxOf(row).click();
    return { result: "failed", reason: "delete did not settle", deleted };
  }
  deleted++;
}
return { result: "ok", deleted };
