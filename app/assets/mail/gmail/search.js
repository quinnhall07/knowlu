// Gmail: search the mailbox for mail received on or after a day, leaving out Spam and Trash, and read the
// number of results from the pager's range, "1–50 of 123" (spec 4.3 step 7, 4.11 step 3). The caller has
// Gmail's mail view open. Like every step script it is the body of an async function of `args`:
// `args.since` is a day, `YYYY-MM-DD`; `args.wait_ms` (default 10000) bounds the wait for the page to answer
// and each page of a walk; `args.max_ms` (default 75000, inside the host's 90 s step) bounds the whole call.
// It returns one of
//   { result: "ok", count: number, exact: true }   (the range's exact total, never the rows drawn; the page
//      is left where the search drew it)
//   { result: "ok", count: number, exact: true, at_oldest: true }   (the total was "many": the script walked
//      to the last page, the oldest, and read its exact total; the page is LEFT on the oldest page)
//   { result: "ok", count: number, exact: false, at_oldest: boolean }   (the walk ran out of `max_ms`, the
//      page left where it stopped and `at_oldest` false; or the last page still read "many", `at_oldest`
//      true. `count` is the rows counted on the pages walked: a lower bound, never a total)
//   { result: "not found", control: "search" | "result count" | "pager" }   ("search": no list ever drew;
//      "pager": "many" and no "Next results" or "Older" to walk with)
//   { result: "failed", reason: "bad date" | "search did not settle" | "page did not change" }
// The query is `after:<yyyy/mm/dd> -in:spam -in:trash -in:sent -in:drafts`: Gmail's default search already
// leaves out Spam and Trash and the terms say so, and Sent and Drafts are the student's own mail, not mail
// received, so they are never listed for a forward. Whether Gmail reads every term is a live check (plan
// T11, T18c), not a thing the synthetic page can say.
// The search runs by URL, not by typing: live 2026-10-18 Gmail's box is labelled "Ask Gmail" (no input is
// named "Search mail"), and it may route to Gemini rather than search. Setting
// `location.hash = "#search/" + encodeURIComponent(query)` (spaces as `+`) ran the real search. The hash is
// set at once, so the search has answered only when the shown grid has redrawn (a new first row, or "no
// messages matched" newly drawn) or the shown range text has changed.
// Live 2026-10-18/19 (WebView2 155): Gmail keeps the previous view's grid and range in the document, hidden
// (offsetParent null), and draws the search's own after them, so only a SHOWN grid and a SHOWN range are
// ever read. A search's range reads "1–45 of many" on every page but the last, which is exact ("1,451–1,493
// of 1,493"); pages hold 43 to 50 rows. A page is never reached by URL (`#search/<q>/p30` drew page 1's rows
// under a p30 range): the walk presses "Next results" (a folder's "Older") a page at a time until it is
// aria-disabled, each press waiting for the shown grid's first row to change. Gmail's pager ignores
// `click()`; a press is pointerdown, mousedown, pointerup, mouseup and click on the button. A hash already on
// this search does not change, so Gmail does not redraw: the page is read as it is.
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

const since = String((args && args.since) || "");
const parts = /^(\d{4})-(\d{2})-(\d{2})$/.exec(since);
const real = parts && new Date(Date.UTC(+parts[1], +parts[2] - 1, +parts[3]));
if (
  !real || real.getUTCFullYear() !== +parts[1] || real.getUTCMonth() !== +parts[2] - 1 ||
  real.getUTCDate() !== +parts[3]
) {
  return { result: "failed", reason: "bad date" };
}
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 10000;
const maxMs = Number(args.max_ms) >= 0 ? Number(args.max_ms) : 75000;

const RANGE = /^([\d,]+)\s*[-–]\s*([\d,]+)\s+of\s+(.+)$/i;
const grid = () => [...document.querySelectorAll('table[role="grid"]')].filter(shown).pop() || null;
const rows = () => (grid() ? [...grid().querySelectorAll('tr[role="row"]')] : []);
const nones = () =>
  (grid() ? [...grid().querySelectorAll("td")] : []).filter((el) => /^no messages matched/i.test(text(el)));
const rangeSpan = () =>
  [...document.querySelectorAll("span")].filter(shown).map(text).find((t) => RANGE.test(t)) || "";
const was = new Set(rows());
const wasNone = new Set(nones());
const wasRange = rangeSpan();

const query = `after:${parts[1]}/${parts[2]}/${parts[3]} -in:spam -in:trash -in:sent -in:drafts`;
const target = "#search/" + encodeURIComponent(query).replace(/%20/g, "+");
const already = String(location.hash || "") === target;
if (!already) location.hash = target;

const onSearch = () => String(location.hash || "").startsWith("#search/");
const answered = () =>
  already && grid() !== null ||
  onSearch() && (
      (rows().length > 0 && !was.has(rows()[0])) ||
      nones().some((el) => !wasNone.has(el)) ||
      (rangeSpan() !== "" && rangeSpan() !== wasRange)
    );
await until(answered, waitMs);
if (!answered()) {
  return grid()
    ? { result: "failed", reason: "search did not settle" }
    : { result: "not found", control: "search" };
}
if (nones().length > 0 && rows().length === 0) return { result: "ok", count: 0, exact: true };

// The rows may redraw before the range text does: give the range the rest of the wait to be drawn and to
// differ from the previous view's. A search that gives the very same range just reads it after the wait.
if (!already) await until(() => rangeSpan() !== "" && rangeSpan() !== wasRange, Math.min(waitMs, 3000));
const exact = (t) => {
  const m = RANGE.exec(t);
  return m && /^[\d,]+$/.test(m[3]) ? Number(m[3].replace(/,/g, "")) : null;
};
if (!RANGE.test(rangeSpan())) return { result: "not found", control: "result count" };
if (exact(rangeSpan()) !== null) return { result: "ok", count: exact(rangeSpan()), exact: true };

// "of many": walk to the last page, where the range is exact.
const pager = () =>
  [...document.querySelectorAll('[role="button"]')].filter(shown).find((b) =>
    /^(next results|older)$/i.test(nameOf(b))
  );
let counted = rows().length;
for (;;) {
  const total = exact(rangeSpan());
  if (total !== null) return { result: "ok", count: total, exact: true, at_oldest: true };
  const next = pager();
  if (!next) return { result: "not found", control: "pager" };
  if (next.getAttribute("aria-disabled") === "true") {
    return { result: "ok", count: counted, exact: false, at_oldest: true };
  }
  if (Date.now() - started >= maxMs) return { result: "ok", count: counted, exact: false, at_oldest: false };
  // A new first row, or the same row element drawn with another message (Gmail may reuse its rows).
  const first = rows()[0];
  const firstText = first ? text(first) : "";
  const range = rangeSpan();
  press(next);
  const changed = () => rows().length > 0 && (rows()[0] !== first || text(rows()[0]) !== firstText);
  if (!(await until(changed, waitMs))) {
    return { result: "failed", reason: "page did not change" };
  }
  counted += rows().length;
  await until(() => rangeSpan() !== range, Math.min(waitMs, 3000));
}
