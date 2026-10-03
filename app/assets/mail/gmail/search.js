// Gmail: search the mailbox for mail received on or after a day, leaving out Spam and Trash, and read the
// number of results from the pager's "1-50 of 123" (spec 4.3 step 7, 4.11 step 3). The caller has Gmail's
// mail view open. Like every step script it is the body of an async function of `args`: `args.since` is a
// day, `YYYY-MM-DD`; `args.wait_ms` (default 10000) bounds the wait for the page to answer. It returns one of
//   { result: "ok", count: number }          (the pager's total, never the number of rows drawn)
//   { result: "not found", control: "search" | "result count" }
//   { result: "failed", reason: "bad date" | "search did not settle" }
// The query is `after:<yyyy/mm/dd> -in:spam -in:trash -in:sent -in:drafts`: Gmail's default search already
// leaves out Spam and Trash and the terms say so, and Sent and Drafts are the student's own mail, not mail
// received, so they are never listed for a forward. Whether Gmail reads every term is a live check (plan
// T11, T18c), not a thing the synthetic page can say.
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
const press = (el, key) => {
  const K = typeof KeyboardEvent === "function" ? KeyboardEvent : Event;
  for (const type of ["keydown", "keypress", "keyup"]) {
    const e = new K(type, { key, code: key, keyCode: 13, which: 13, bubbles: true, cancelable: true });
    if (e.key !== key) Object.defineProperty(e, "key", { value: key });
    el.dispatchEvent(e);
  }
};

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

const box = [...document.querySelectorAll('input[type="text"], input[type="search"], input:not([type])')]
  .find((el) => /^search mail$/i.test(nameOf(el)));
if (!box) return { result: "not found", control: "search" };

const rows = () => [...document.querySelectorAll('table[role="grid"] tr[role="row"]')];
const none = () =>
  [...document.querySelectorAll('table[role="grid"] td')].some((el) =>
    /^no messages matched/i.test(text(el))
  );
const was = new Set(rows());
const hadNone = none();

setValue(box, `after:${parts[1]}/${parts[2]}/${parts[3]} -in:spam -in:trash -in:sent -in:drafts`);
press(box, "Enter");

const answered = () => (none() && !hadNone) || (rows().length > 0 && !was.has(rows()[0]));
await until(answered, waitMs);
if (!answered() && !(hadNone && none())) return { result: "failed", reason: "search did not settle" };
if (none() && rows().length === 0) return { result: "ok", count: 0 };

const RANGE = /^([\d,]+)\s*[-–]\s*([\d,]+)\s+of\s+(.+)$/i;
const range = [...document.querySelectorAll("span")].map(text).map((t) => RANGE.exec(t)).find(Boolean);
const total = range && /^[\d,]+$/.test(range[3]) ? Number(range[3].replace(/,/g, "")) : null;
if (total === null) return { result: "not found", control: "result count" };
return { result: "ok", count: total };
