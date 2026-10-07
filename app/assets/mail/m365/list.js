// Microsoft 365, Outlook on the web: read the message rows the list shows, in the form the fingerprint
// uses (spec 4.6 step 7, 4.11 step 3; S2 recorded the form). The caller has the mail view open, after a
// search or on a folder. Like every step script it is the body of an async function of `args`:
// `args.wait_ms` (default 5000) bounds each wait for a conversation to draw or put away its messages. It
// returns
//   { result: "ok", total: number, drawn: number, left: number,
//     rows: [{ sender, subject, received, received_at, conversation?, own? }] }
//   { result: "not found", control: "list" | "list row" }
// `sender` is the address from the sender element's title attribute, lower case: the row's visible text is
// a display name about as often as an address, so the text is never read. `subject` is the first title after
// the sender's that is not the time's and not inside a button (live rows put button titles among their
// own, T18c). `received` is the time element's title attribute, the full date and time as the page writes
// it, and `received_at` is the instant that names (the page's own reading of it, so its locale and zone), or
// null. `total` is the list's aria-setsize (live, on the first row only): the list is virtualised, so the
// rows are only the rows drawn, and `drawn` counts the list's own rows read (a conversation is one).
// A conversation (live T18c, 2026-10-07: the inbox is in conversation view) is a row with an "Expand
// conversation" button; its row names no address. As Quinn ruled for Gmail threads (2026-10-02), it is
// expanded, each message drawn in it is read (a message whose row shows no subject has the conversation's),
// and it is collapsed again. Its messages carry `conversation`, the conversation's number in this read, and
// a message from the mailbox's own address (the folder pane's, as signin.js reads it) carries `own: true`.
// A conversation that cannot be read whole (a message unreadable, no messages drawn, no mailbox address on
// the page) is counted in `left` and never fails the read. A single row it cannot read fails the whole read,
// to the guided fallback, rather than leaving a message out. The text of a message is never read, and
// never leaves the page.
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const EMAIL = /^[^\s@<>"]+@[^\s@<>"]+\.[^\s@<>"]+$/;
const CONVERSATION = /^(expand|collapse) conversation$/i;
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
// A deadline on the clock, never a count of turns: a hidden window may throttle its timers (T18c review).
const until = async (ready, ms) => {
  const end = Date.now() + ms;
  while (!ready() && Date.now() < end) await sleep(Math.min(50, end - Date.now()));
  return ready();
};
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;

const lists = [...document.querySelectorAll('[role="listbox"]')];
const list = lists.find((el) => /message list/i.test(el.getAttribute("aria-label") || "")) || lists[0];
if (!list) return { result: "not found", control: "list" };
const options = () => [...list.querySelectorAll('[role="option"]')];

// A row's own titles, in order, leaving out every title on or inside a button.
const titles = (row) =>
  [...row.querySelectorAll("[title]")]
    .filter((el) =>
      !el.closest('button, [role="button"]') || !row.contains(el.closest('button, [role="button"]'))
    )
    .map((el) => (el.getAttribute("title") || "").trim());
// A message row's three fields; `fallback` is a conversation's subject for a message row that shows none.
const read = (row, fallback) => {
  const t = titles(row);
  const at = t.findIndex((v) => ADDRESS.test(v));
  if (at < 0) return null;
  const isTime = (v) => !ADDRESS.test(v) && TIMED.test(v);
  const received = t.slice(at + 1).find(isTime);
  const subject = t.slice(at + 1).find((v) => v !== "" && !ADDRESS.test(v) && !isTime(v)) ?? fallback;
  if (!received || subject === undefined) return null;
  const when = new Date(received);
  return {
    sender: t[at].toLowerCase(),
    subject: subject.replace(/\s+/g, " "),
    received: received.replace(/\s+/g, " "),
    received_at: isNaN(when.getTime()) ? null : when.toISOString(),
  };
};
const toggle = (row) =>
  [...row.querySelectorAll('button, [role="button"]')].find((b) =>
    CONVERSATION.test((b.getAttribute("aria-label") || "").trim())
  );
// A conversation's subject: its first title that is not the time's (its row names no address).
const topic = (row) => titles(row).find((v) => v !== "" && !ADDRESS.test(v) && !TIMED.test(v));

// The mailbox's own address, from the folder pane only, as signin.js reads it.
const own = (() => {
  for (const tree of document.querySelectorAll('[role="tree"]')) {
    for (const item of tree.querySelectorAll('[role="treeitem"], [title], [aria-label]')) {
      for (const value of [item.getAttribute("title"), item.getAttribute("aria-label")]) {
        const v = String(value || "").trim();
        if (EMAIL.test(v)) return v.toLowerCase();
      }
    }
  }
  return "";
})();

const expanded = (row) => /^collapse/i.test((toggle(row).getAttribute("aria-label") || "").trim());
// A conversation the page already shows open is put away first: its messages' rows are not the list's own.
for (const row of options().filter((r) => toggle(r) && expanded(r))) {
  const was = options().length;
  toggle(row).click();
  await until(() => !expanded(row) && options().length < was, waitMs);
}

// First every single row, before anything is expanded (an expanded list may draw other rows).
const rows = [];
const convos = [];
const tops = options();
for (const row of tops) {
  if (toggle(row)) {
    convos.push({ label: row.getAttribute("aria-label") || "", nth: convos.length });
    continue;
  }
  const one = read(row);
  if (!one) return { result: "not found", control: "list row" };
  rows.push(one);
}
const sizes = tops.map((el) => Number(el.getAttribute("aria-setsize")))
  .concat([Number(list.getAttribute("aria-setsize"))])
  .filter((n) => Number.isInteger(n) && n >= 0);
const total = sizes.length ? Math.max(...sizes) : tops.length;

// Then each conversation, found again by its label (the node may have been redrawn), one at a time.
const convoRows = () => options().filter((row) => toggle(row));
// Waits for rows that were not there before to appear, and for their number to hold still a moment.
const drawnSince = async (was) => {
  const fresh = () => options().filter((o) => !was.has(o));
  if (!(await until(() => fresh().length > 0, waitMs))) return [];
  const end = Date.now() + waitMs;
  for (let n = fresh().length; Date.now() < end; n = fresh().length) {
    await sleep(150);
    if (fresh().length === n) break;
  }
  return fresh();
};
let left = 0;
for (const c of convos) {
  const found = convoRows();
  const row = found.find((r, i) => i === c.nth && (r.getAttribute("aria-label") || "") === c.label) ||
    found.find((r) => (r.getAttribute("aria-label") || "") === c.label);
  if (!row || !own) {
    left++;
    continue;
  }
  if (expanded(row)) {
    toggle(row).click();
    await until(() => !expanded(row), waitMs);
  }
  const subject = topic(row);
  const was = new Set(options());
  toggle(row).click();
  const kids = await drawnSince(was);
  const got = kids.map((kid) => read(kid, subject));
  if (kids.length === 0 || got.some((one) => one === null)) {
    left++;
  } else {
    for (const one of got) {
      one.conversation = c.nth + 1;
      if (one.sender === own) one.own = true;
      rows.push(one);
    }
  }
  // Put away again, so the list is as it was found for the next read or forward.
  if (document.contains(row) && toggle(row) && expanded(row)) {
    toggle(row).click();
    await until(() => kids.every((kid) => !document.contains(kid)), waitMs);
  }
}
return { result: "ok", total, drawn: tops.length, left, rows };
