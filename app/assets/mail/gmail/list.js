// Gmail: read the message rows the page shows, in the form the fingerprint uses (spec 4.3 step 7, 4.11 step
// 3; S2 recorded the form), and move between result pages oldest first. The caller has the mail view open,
// after a search or on a folder. Like every step script it is the body of an async function of `args`:
// `args.go` is optional, one of "oldest" | "newer" | "older" | "newest" (the pager's own buttons, moved before
// the read; the routine reads the oldest page first, then "newer" a page at a time), and `args.wait_ms`
// (default 5000) bounds the wait for the page to redraw. It returns one of
//   { result: "ok", page: { from, to, total }, moved: boolean,
//     rows: [{ sender, subject, received, received_at, thread }] }   (rows are OLDEST FIRST: Gmail draws
//      a page newest first, so the page's rows are read in reverse. `moved` is false when `go` had nowhere
//      to go, and the page read is the one already showing)
//   { result: "not found", control: "list" | "list row" | "pager" | "page range" }
//   { result: "failed", reason: "bad direction" | "page did not change" }
// `sender` is the address from the sender element's `email` attribute, lower case: the row's visible text is
// a display name about as often as an address, so the text is never read. `received` is the time element's
// title attribute, the full date and time as the page writes it, and `received_at` is the instant that
// names (the page's own reading of it, so its locale and zone), or null. A row that is a conversation
// lists several senders: the last (the newest) is read and `thread` is true. A row it cannot read fails the
// whole read, to the guided fallback, rather than leaving a message out. The text of a message is never read,
// and never leaves the page.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => (el.getAttribute("aria-label") || "").trim();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const RANGE = /^([\d,]+)\s*[-–]\s*([\d,]+)\s+of\s+(.+)$/i;
const BUTTONS = { oldest: "Oldest", newer: "Newer", older: "Older", newest: "Newest" };
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;

const grid = document.querySelector('table[role="grid"]');
if (!grid) return { result: "not found", control: "list" };
const rowEls = () => [...grid.querySelectorAll('tr[role="row"]')];
const readRange = () => {
  const m = [...document.querySelectorAll("span")].map(text).map((t) => RANGE.exec(t)).find(Boolean);
  const n = (v) => Number(String(v).replace(/,/g, ""));
  if (!m || !/^[\d,]+$/.test(m[3])) return null;
  return { from: n(m[1]), to: n(m[2]), total: n(m[3]) };
};

let moved = false;
if (args && args.go !== undefined && args.go !== null) {
  const label = BUTTONS[String(args.go)];
  if (!label) return { result: "failed", reason: "bad direction" };
  const here = readRange();
  // Nothing to move to: no results, or the page that direction is already showing.
  const atEnd = !here ||
    (args.go === "oldest" || args.go === "older" ? here.to >= here.total : here.from <= 1);
  if (!atEnd) {
    const button = [...document.querySelectorAll('[role="button"]')].find((b) => nameOf(b) === label);
    if (!button || button.getAttribute("aria-disabled") === "true") {
      return { result: "not found", control: "pager" };
    }
    const was = new Set(rowEls());
    button.click();
    if (!(await until(() => rowEls().length > 0 && !was.has(rowEls()[0]), waitMs))) {
      return { result: "failed", reason: "page did not change" };
    }
    moved = true;
  }
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
  rows.push({
    sender,
    subject: text(subjectEl),
    received: stamp.replace(/\s+/g, " "),
    received_at: isNaN(at.getTime()) ? null : at.toISOString(),
    thread: new Set(senders).size > 1,
  });
}
const page = readRange();
if (!page) {
  return rows.length === 0
    ? { result: "ok", page: { from: 0, to: 0, total: 0 }, moved, rows }
    : { result: "not found", control: "page range" };
}
return { result: "ok", page, moved, rows: rows.reverse() };
