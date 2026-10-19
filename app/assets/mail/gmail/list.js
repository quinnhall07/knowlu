// Gmail: read the message rows the page shows, in the form the fingerprint uses (spec 4.3 step 7, 4.11 step
// 3; S2 recorded the form), and move between result pages oldest first. The caller has the mail view open,
// after a search or on a folder. Like every step script it is the body of an async function of `args`:
// `args.go` is optional, one of "oldest" | "newer" | "older" | "newest" (the pager's own buttons, moved before
// the read; the routine reads the oldest page first, then "newer" a page at a time), `args.wait_ms`
// (default 5000) bounds the wait for each page to redraw, and `args.max_ms` (default 75000, inside the host's
// 90 s step) bounds a walk. It returns one of
//   { result: "ok", page: { from, to, total }, moved: boolean, at_oldest?: boolean,
//     rows: [{ sender, subject, received, received_at, thread }] }   (rows are OLDEST FIRST: Gmail draws
//      a page newest first, so the page's rows are read in reverse. `moved` is false when `go` had nowhere
//      to go, and the page read is the one already showing. `total` is null while a search's range reads
//      "of many". `at_oldest` comes with `go: "oldest"` only: false when the walk ran out of `max_ms`
//      short of the oldest page, and the page read is where it stopped; the caller asks again)
//   { result: "not found", control: "list" | "list row" | "pager" | "page range" }
//   { result: "failed", reason: "bad direction" | "page did not change" }
// The pager, as live 2026-10-18/19 (WebView2 155): a folder has "Newer"/"Older" (and "Newest"/"Oldest" when
// drawn); a search has "Previous results"/"Next results" and no Oldest or Newest. "older" is "Older" or "Next
// results", "newer" is "Newer" or "Previous results"; "oldest" is "Oldest" when drawn, else a walk of
// "older" until it is aria-disabled, inside `max_ms`; "newest" is "Newest" when drawn, else a walk of
// "newer" (to the page as far as `max_ms` allows). The end is the exact total when the range has one, else
// the button's aria-disabled. A page is never reached by URL (`#search/<q>/p30` draws page 1's rows under a
// p30 range). The pager ignores `click()`: a press is pointerdown, mousedown, pointerup, mouseup and click on
// the button. Gmail keeps a previous view's grid and range in the document, hidden (offsetParent null): only
// the SHOWN grid and range are read.
// `sender` is the address from the sender element's `email` attribute, lower case: the row's visible text is
// a display name about as often as an address, so the text is never read. `received` is the time element's
// title attribute, the full date and time as the page writes it, and `received_at` is the instant that
// names (the page's own reading of it, so its locale and zone), or null. A row that is a conversation
// lists several senders, or shows a message count beside the sender names (live: digits alone in a
// `span.bx0`, "Ana, Bo 2"; an older form, "Name (3)", is read too): the last sender (the newest) is read,
// `thread` is true, and `messages` is the count shown, or null when the row shows none. Two senders naming
// one address are one sender. (Non-thread rows have no `messages`.) The routine opens a thread row and reads
// every message in it (thread.js, plan T11.2). A row it cannot read fails the whole read, to the guided
// fallback, rather than leaving a message out. The text of a message is never read, and never leaves the
// page.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => (el.getAttribute("aria-label") || el.getAttribute("data-tooltip") || "").trim();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
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
const started = Date.now();
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const RANGE = /^([\d,]+)\s*[-–]\s*([\d,]+)\s+of\s+(.+)$/i;
const BUTTONS = {
  oldest: ["Oldest"],
  older: ["Older", "Next results"],
  newer: ["Newer", "Previous results"],
  newest: ["Newest"],
};
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const maxMs = Number(args && args.max_ms) >= 0 ? Number(args.max_ms) : 75000;

const grid = () => [...document.querySelectorAll('table[role="grid"]')].filter(shown).pop() || null;
if (!grid()) return { result: "not found", control: "list" };
const rowEls = () => (grid() ? [...grid().querySelectorAll('tr[role="row"]')] : []);
const rangeText = () =>
  [...document.querySelectorAll("span")].filter(shown).map(text).find((t) => RANGE.test(t)) || "";
const readRange = () => {
  const m = RANGE.exec(rangeText());
  const n = (v) => Number(String(v).replace(/,/g, ""));
  if (!m) return null;
  return { from: n(m[1]), to: n(m[2]), total: /^[\d,]+$/.test(m[3]) ? n(m[3]) : null };
};
const button = (dir) =>
  [...document.querySelectorAll('[role="button"]')].filter(shown).find((b) =>
    BUTTONS[dir].some((label) => nameOf(b).toLowerCase() === label.toLowerCase())
  );
const off = (b) => b.getAttribute("aria-disabled") === "true";
// Nothing to move to: no results, or the end that direction is already showing.
const atEnd = (dir) => {
  const here = readRange();
  if (!here) return true;
  if (dir === "newer" || dir === "newest") return here.from <= 1;
  if (here.total !== null) return here.to >= here.total;
  const b = button("older");
  return !!b && off(b);
};
// One press of `b`: the shown grid's first row changes (a new row, or the same row drawn with another
// message), then the range is given a moment to follow.
const step = async (b) => {
  const first = rowEls()[0];
  const firstText = first ? text(first) : "";
  const range = rangeText();
  press(b);
  const changed = () => rowEls().length > 0 && (rowEls()[0] !== first || text(rowEls()[0]) !== firstText);
  if (!(await until(changed, waitMs))) return false;
  await until(() => rangeText() !== range, Math.min(waitMs, 3000));
  return true;
};
const NOT_CHANGED = { result: "failed", reason: "page did not change" };

let moved = false;
let atOldest;
if (args && args.go !== undefined && args.go !== null) {
  const go = String(args.go);
  if (!BUTTONS[go]) return { result: "failed", reason: "bad direction" };
  const toward = go === "oldest" || go === "older" ? "older" : "newer";
  if (!atEnd(go)) {
    const jump = (go === "oldest" || go === "newest") && button(go);
    if (jump || go === "older" || go === "newer") {
      const b = jump || button(go);
      if (!b || off(b)) return { result: "not found", control: "pager" };
      if (!(await step(b))) return NOT_CHANGED;
      moved = true;
    } else {
      // No Oldest (or Newest) drawn: walk a page at a time, inside the budget.
      while (!atEnd(go) && Date.now() - started < maxMs) {
        const b = button(toward);
        if (!b || off(b)) {
          if (moved) break;
          return { result: "not found", control: "pager" };
        }
        if (!(await step(b))) return NOT_CHANGED;
        moved = true;
      }
    }
  }
  if (go === "oldest") atOldest = atEnd("oldest");
}

const rows = [];
for (const row of rowEls()) {
  if (row.querySelector('[role="checkbox"]') === null) continue;
  const senders = [...row.querySelectorAll("span[email]")].map((el) =>
    (el.getAttribute("email") || "").trim().toLowerCase()
  );
  const sender = senders[senders.length - 1];
  const stamp = [...row.querySelectorAll("[title]")].map((el) => (el.getAttribute("title") || "").trim())
    .find((t) => TIMED.test(t) && /\d/.test(t));
  const subjectEl = row.querySelector(".bog");
  if (!sender || !ADDRESS.test(sender) || !stamp || !subjectEl) {
    return { result: "not found", control: "list row" };
  }
  const at = new Date(stamp);
  // The message count Gmail draws after the sender names: live, digits alone in a span.bx0; else "(3)" at
  // the end of the cell that holds them.
  const bx = row.querySelector("span.bx0");
  const cell = row.querySelector("span[email]").closest("td");
  const older = cell && /\((\d+)\)\s*$/.exec(text(cell));
  const count = bx && /^\d+$/.test(text(bx)) ? Number(text(bx)) : older ? Number(older[1]) : null;
  const thread = new Set(senders).size > 1 || (count !== null && count > 1);
  const item = {
    sender,
    subject: text(subjectEl),
    received: stamp.replace(/\s+/g, " "),
    received_at: isNaN(at.getTime()) ? null : at.toISOString(),
    thread,
  };
  if (thread) item.messages = count;
  rows.push(item);
}
const page = readRange();
const end = atOldest === undefined ? {} : { at_oldest: atOldest };
if (!page) {
  return rows.length === 0
    ? { result: "ok", page: { from: 0, to: 0, total: 0 }, moved, ...end, rows }
    : { result: "not found", control: "page range" };
}
return { result: "ok", page, moved, ...end, rows: rows.reverse() };
