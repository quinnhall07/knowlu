// Gmail: search the mailbox for mail received on or after a day, leaving out Spam and Trash, and read the
// number of results from the pager's "1-50 of 123" (spec 4.3 step 7, 4.11 step 3). The caller has Gmail's
// mail view open. Like every step script it is the body of an async function of `args`: `args.since` is a
// day, `YYYY-MM-DD`; `args.wait_ms` (default 10000) bounds the wait for the page to answer. It returns one of
//   { result: "ok", count: number }          (the pager's total, never the number of rows drawn)
//   { result: "not found", control: "search" | "result count" }   ("search": no list ever drew)
//   { result: "failed", reason: "bad date" | "search did not settle" }
// The query is `after:<yyyy/mm/dd> -in:spam -in:trash -in:sent -in:drafts`: Gmail's default search already
// leaves out Spam and Trash and the terms say so, and Sent and Drafts are the student's own mail, not mail
// received, so they are never listed for a forward. Whether Gmail reads every term is a live check (plan
// T11, T18c), not a thing the synthetic page can say.
// The search runs by URL, not by typing: live 2026-10-18 Gmail's box is labelled "Ask Gmail" (no input is
// named "Search mail"), and it may route to Gemini rather than search. Setting
// `location.hash = "#search/" + encodeURIComponent(query)` (spaces as `+`) ran the real search: 50 rows and
// "1-50 of 2,819". The hash is set at once, so the search has answered only when the grid has redrawn (a
// new first row, or "no messages matched" newly drawn) or the range text has changed; the old view's range
// is never read as this search's count. A hash already on this search does not change, so Gmail does not
// redraw: the page is read as it is.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
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

const RANGE = /^([\d,]+)\s*[-–]\s*([\d,]+)\s+of\s+(.+)$/i;
const grid = () => document.querySelector('table[role="grid"]');
const rows = () => [...document.querySelectorAll('table[role="grid"] tr[role="row"]')];
const nones = () =>
  [...document.querySelectorAll('table[role="grid"] td')].filter((el) =>
    /^no messages matched/i.test(text(el))
  );
const rangeSpan = () => [...document.querySelectorAll("span")].map(text).find((t) => RANGE.test(t)) || "";
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
      rangeSpan() !== wasRange
    );
await until(answered, waitMs);
if (!answered()) {
  return grid()
    ? { result: "failed", reason: "search did not settle" }
    : { result: "not found", control: "search" };
}
if (nones().length > 0 && rows().length === 0) return { result: "ok", count: 0 };

// The rows may redraw before the range text does: give the range the rest of the wait to change from the
// previous view's, so that view's total is not read as this search's. A search that gives the very same
// range (a degenerate repeat of the view) just reads it after the wait.
if (!already) await until(() => rangeSpan() !== wasRange, Math.min(waitMs, 3000));
const range = RANGE.exec(rangeSpan());
const total = range && /^[\d,]+$/.test(range[3]) ? Number(range[3].replace(/,/g, "")) : null;
if (total === null) return { result: "not found", control: "result count" };
return { result: "ok", count: total };
