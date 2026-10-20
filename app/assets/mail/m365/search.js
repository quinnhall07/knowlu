// Microsoft 365, Outlook on the web: search the mailbox for mail received on or after a day, leaving out
// Junk Email, Deleted Items, Sent Items and Drafts, and read the number of results from the list's
// aria-setsize (spec 4.2 step 5, 4.11 step 3). The caller has Outlook's mail view open. Like every step
// script it is the body of an async function of `args`: `args.since` is a day, `YYYY-MM-DD`;
// `args.until` is optional, a later day, `YYYY-MM-DD`, before which the mail must have been received (so
// `[since, until)`: the routine narrows a window to a span the virtualised list can draw whole, T11.2);
// `args.draw_ms` (default 15000) bounds the wait for the search box to be drawn; `args.wait_ms` (default
// 10000) bounds the wait for the page to answer. It returns one of
//   { result: "ok", count: number }          (the list is virtualised: the count is aria-setsize, never
//                                             the number of rows drawn)
//   { result: "not found", control: "search" | "result count" }
//   { result: "failed", reason: "bad date" | "search did not settle" }
// The query is `received>=<since>` (and `received<<until>` when given) with `NOT folder:"Junk Email"
// NOT folder:"Deleted Items" NOT folder:"Sent Items" NOT folder:"Drafts"`, so the results are mail the mailbox received and nothing
// else: Exchange gives a Sent Items row a received time, and the student's own replies, or a Knowlu copy a
// failed cleanup left there, must never be forwarded as "missing". Whether Outlook reads that syntax is a live
// check (plan T11, T18c), not a thing the synthetic page can say.
// Live Outlook (T18c, 2026-10-07) draws the box as `<input role="combobox">` with NO type attribute (its
// `.type` is "text"), and redirects outlook.office.com to outlook.cloud.microsoft mid-load, so the box is
// briefly absent: any input whose type is text or search (an untyped one included) is a candidate, and the
// box is waited for.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  const by = el.getAttribute("aria-labelledby");
  if (by) {
    const t = by.split(/\s+/).map((id) => document.getElementById(id)).filter(Boolean).map(text).join(" ");
    if (t) return t;
  }
  return "";
};
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
// A deadline on the clock, never a count of turns: a hidden window may throttle its timers (T18c review).
const until = async (ready, ms) => {
  const end = Date.now() + ms;
  while (!ready() && Date.now() < end) await sleep(Math.min(50, end - Date.now()));
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
const press = (el, key) => {
  const K = typeof KeyboardEvent === "function" ? KeyboardEvent : Event;
  for (const type of ["keydown", "keypress", "keyup"]) {
    const e = new K(type, { key, code: key, keyCode: 13, which: 13, bubbles: true, cancelable: true });
    if (e.key !== key) Object.defineProperty(e, "key", { value: key });
    el.dispatchEvent(e);
  }
};

const isDay = (v) => {
  const parts = /^(\d{4})-(\d{2})-(\d{2})$/.exec(v);
  const real = parts && new Date(Date.UTC(+parts[1], +parts[2] - 1, +parts[3]));
  return !!real && real.getUTCFullYear() === +parts[1] && real.getUTCMonth() === +parts[2] - 1 &&
    real.getUTCDate() === +parts[3];
};
const since = String((args && args.since) || "");
const before = args && args.until !== undefined && args.until !== null ? String(args.until) : "";
if (!isDay(since) || (before !== "" && (!isDay(before) || before <= since))) {
  return { result: "failed", reason: "bad date" };
}
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 10000;
const drawMs = Number(args.draw_ms) >= 0 ? Number(args.draw_ms) : 15000;

// The type a browser gives the input: an absent or unknown type attribute reads as "text".
const kind = (el) => String(el.type || el.getAttribute("type") || "text").toLowerCase();
const boxEl = () =>
  [...document.querySelectorAll("input")].find((el) =>
    (kind(el) === "text" || kind(el) === "search") && /search/i.test(nameOf(el))
  );
if (!(await until(boxEl, drawMs))) return { result: "not found", control: "search" };
const box = boxEl();
const go = [...document.querySelectorAll('button, [role="button"]')].find((b) =>
  /^search$/i.test(nameOf(b) || text(b))
);

const rows = () => [...document.querySelectorAll('[role="listbox"] [role="option"]')];
const none = () =>
  [...document.querySelectorAll('[role="status"], [role="alert"]')].some((el) =>
    /^(no results|we didn.?t find anything|no items match)/i.test(text(el))
  );
const was = new Set(rows());
const hadNone = none();

const query = `received>=${since}${before ? ` received<${before}` : ""} NOT folder:"Junk Email"` +
  ` NOT folder:"Deleted Items" NOT folder:"Sent Items" NOT folder:"Drafts"`;
setValue(box, query);
if (go) go.click();
else press(box, "Enter");

const answered = () => (none() && !hadNone) || (rows().length > 0 && !was.has(rows()[0]));
await until(answered, waitMs);
if (!answered() && !(hadNone && none())) return { result: "failed", reason: "search did not settle" };
if (none() && rows().length === 0) return { result: "ok", count: 0 };

const sizes = [...document.querySelectorAll('[role="listbox"], [role="listbox"] [role="option"]')]
  .map((el) => Number(el.getAttribute("aria-setsize"))).filter((n) => Number.isInteger(n) && n >= 0);
if (sizes.length === 0) return { result: "not found", control: "result count" };
return { result: "ok", count: Math.max(...sizes) };
