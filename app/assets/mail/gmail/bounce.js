// Gmail: look in the Inbox for a non-delivery report of this batch's forwards (spec 4.2 step 5, risk 15;
// Quinn's ruling 2026-10-19 on the T18c gate "Gmail non-delivery report between sends"). A bounce lands in the
// Inbox, which the page left open after a search does not list, so the routine opens the Inbox every few sends
// and runs this. Like every step script it is the body of an async function of `args`: `args.address` is this
// mailbox's Knowlu forwarding address, `args.nonce` the open batch's nonce, `args.since` an ISO instant, when
// the batch opened (optional), and `args.wait_ms` (default 5000) bounds the wait for the list to draw. It
// returns one of
//   { result: "ok", rows: number }   (no report: `rows` is how many rows of the shown list it read)
//   { result: "refused", reason: "non-delivery report" }   (the routine stops: nothing is sent again until
//      the next day)
//   { result: "not found", control: "inbox" }   (no list was drawn in time)
//   { result: "failed", reason: "no address" | "no nonce" }
// A row is a report of ours when its text holds a delivery-failure phrase (forward.js's own) AND EITHER it holds
// the batch's nonce (Gmail quotes the sent subject, `Knowlu <nonce>`) OR it names Knowlu's address and its time
// is at or after `since` (a report that quotes no subject; the minute the batch opened counts, because a row's
// time is only as fine as a minute). With no `since`, only the nonce counts; a row whose time cannot be read
// never counts by its address. A report from an earlier batch, or any other row, is not ours. It reads the
// rows and nothing else: it clicks and opens nothing, and returns counts and fixed words only, never a row's
// text, an address or a subject.
// Live 2026-10-18/19 (WebView2 155): only the SHOWN grid's rows are read (Gmail keeps a previous view's grid
// in the document, hidden, offsetParent null), as list.js reads them; a row's time is the `title` of the
// element beside it that holds a clock time.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
// Shown: live, a hidden view's elements have no offsetParent.
const shown = (el) => !!el && el.offsetParent !== null;
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const REPORT =
  /undeliverable|delivery status notification|delivery (has )?failed|wasn.?t delivered|couldn.?t be delivered|not delivered|address not found/i;

const address = norm(args && args.address);
if (!ADDRESS.test(address)) return { result: "failed", reason: "no address" };
const nonce = String((args && args.nonce) || "").trim().toLowerCase();
if (!nonce || /\s/.test(nonce)) return { result: "failed", reason: "no nonce" };
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
// The minute the batch opened: a row's time carries no seconds.
const opened = args.since ? new Date(String(args.since)) : null;
const since = opened && !isNaN(opened.getTime()) ? Math.floor(opened.getTime() / 60000) * 60000 : null;

const grid = () => [...document.querySelectorAll('table[role="grid"]')].filter(shown).pop() || null;
const rowEls = () =>
  grid() ? [...grid().querySelectorAll('tr[role="row"]')].filter((r) => r.querySelector('[role="checkbox"]')) : [];
if (!(await until(grid, waitMs))) return { result: "not found", control: "inbox" };
// The list may be drawn before its rows are.
await until(() => rowEls().length > 0, waitMs);

// The instant a row's time names, as list.js reads `received_at`: null when the page's time cannot be read.
const timeOf = (row) => {
  const stamp = [...row.querySelectorAll("[title]")].map((el) => (el.getAttribute("title") || "").trim())
    .find((t) => TIMED.test(t) && /\d/.test(t));
  const at = stamp ? new Date(stamp).getTime() : NaN;
  return isNaN(at) ? null : at;
};

const rows = rowEls();
for (const row of rows) {
  const t = text(row);
  if (!REPORT.test(t)) continue;
  const lower = t.toLowerCase();
  if (lower.includes(nonce)) return { result: "refused", reason: "non-delivery report" };
  const at = timeOf(row);
  if (since !== null && at !== null && at >= since && lower.includes(address)) {
    return { result: "refused", reason: "non-delivery report" };
  }
}
return { result: "ok", rows: rows.length };
