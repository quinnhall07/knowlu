// Microsoft 365, Outlook on the web: search the mailbox for mail received on or after a day, leaving out
// Junk Email and Deleted Items, and read the number of results from the list's aria-setsize (spec 4.2
// step 5, 4.11 step 3). The caller has Outlook's mail view open. Like every step script it is the body of
// an async function of `args`: `args.since` is a day, `YYYY-MM-DD`; `args.wait_ms` (default 10000) bounds
// the wait for the page to answer. It returns one of
//   { result: "ok", count: number }          (the list is virtualised: the count is aria-setsize, never
//                                             the number of rows drawn)
//   { result: "not found", control: "search" | "result count" }
//   { result: "failed", reason: "bad date" | "search did not settle" }
// The query is `received>=<since>` with `NOT folder:"Junk Email" NOT folder:"Deleted Items"`, so the
// results are the mailbox's own folders and nothing else. Whether Outlook reads that syntax is a live
// check (plan T11, T18c), not a thing the synthetic page can say.
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

const box = [
  ...document.querySelectorAll('input[type="search"], input[role="searchbox"], input[type="text"]'),
]
  .find((el) => /search/i.test(nameOf(el)));
if (!box) return { result: "not found", control: "search" };
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

setValue(box, `received>=${since} NOT folder:"Junk Email" NOT folder:"Deleted Items"`);
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
