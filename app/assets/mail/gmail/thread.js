// Gmail: open ONE conversation row and read every message in it in the form the fingerprint uses, then go
// back to the list (plan T11.2, Quinn's ruling of 2026-10-02: a row that shows a message count is opened and
// each message fingerprinted; spec 4.11 step 3). The caller has the mail view open on a page of results
// (list.js) and passes the row as list.js read it. Like every step script it is the body of an async
// function of `args`: `args.sender`, `args.subject` and `args.received` the row's three fields,
// `args.messages` (optional) the count list.js read from the row, and `args.wait_ms` (default 5000) bounds
// each wait. Every answer carries `presses`, the page loads this call made, failures too (review of c8806b3:
// the routine charges them to the day's page budget): the row's open, each "Expand all" and the back press. It
// returns one of
//   { result: "ok", messages: [{ sender, subject, received, received_at, own? }] }   (oldest first; the page is
//      back on its list)
//   { result: "not found", control: "list" | "message" | "back" | "account" }
//   { result: "failed", reason: "conversation did not open" | "list did not return" | "no messages" |
//      "not every message drawn" }   (fewer messages drawn than the row's count: a partial read would
//      call a held message missing, so nothing is returned)
// `sender` is the address from the message header's `email` attribute, lower case: the visible text is a
// display name about as often as an address, so the text is never read. `received` is the time element's
// title attribute, the full date and time as the page writes it, and `received_at` is the instant that names
// (the page's own reading of it), or null. `subject` is the conversation's subject (the page shows one for
// all of it; the routine also tries the "Re: " form of it for a later message). A conversation shows only
// its first and newest messages open: "Expand all" is pressed until every message is drawn (a conversation
// of two shows "Collapse all" instead). The text of a message is never read, and never leaves the page. It
// clicks the row, never its checkbox, and ticks, sends and deletes nothing.
// Live 2026-10-18/19 (WebView2 155): only the SHOWN grid is read (Gmail keeps a previous view's grid in the
// document, hidden, offsetParent null). The row opens on `click()`. The back control is a `[role=button]`
// titled "Back to Search Results" (on a folder, "Back to <folder>") with no aria-label, matched by its
// aria-label or title; it and "Expand all" ignore `click()` and are pressed: pointerdown, mousedown,
// pointerup, mouseup and click on the button.
// A message the signed-in account itself sent (a reply, a draft) carries `own: true` and no other message
// carries the key: the service never receives the student's own mail, so the routine must not call it
// missing. The account is the address in the account chip's label ("Google Account: <name> (<address>), Google membership": the last
// parenthesised address anywhere in the label);
// with no chip to read it, nothing is opened and the answer is { result: "not found", control: "account" }.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => (el.getAttribute("aria-label") || "").trim();
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
const buttons = () => [...document.querySelectorAll('[role="button"], button')].filter(shown);
let presses = 0;
const answer = (r) => ({ ...r, presses });
// Shown: live, a hidden view's elements have no offsetParent.
function shown(el) {
  return !!el && el.offsetParent !== null;
}
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
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const want = {
  sender: norm(args && args.sender),
  subject: String((args && args.subject) || "").replace(/\s+/g, " ").trim(),
  received: String((args && args.received) || "").replace(/\s+/g, " ").trim(),
};

const shownGrid = () => [...document.querySelectorAll('table[role="grid"]')].filter(shown).pop() || null;
const grid = shownGrid();
const listShowing = () => {
  const g = shownGrid();
  return !!g && g.querySelectorAll('tr[role="row"]').length > 0;
};
if (!grid) return answer({ result: "not found", control: "list" });

// Whose mailbox this is: the last parenthesised address of the account chip's label.
const own = (() => {
  for (const el of document.querySelectorAll('[aria-label^="Google Account"]')) {
    // Live (2026-10-18): "Google Account: <name> (<address>), Google membership"; the address does not end it.
    const found = [...(el.getAttribute("aria-label") || "").matchAll(/\(([^()\s]+@[^()\s]+)\)/g)].pop();
    if (found) return norm(found[1]);
  }
  return "";
})();
if (!own) return answer({ result: "not found", control: "account" });

// The row, by the three fields list.js read from it (the newest sender, as list.js reads it).
const read = (row) => {
  const senders = [...row.querySelectorAll("span[email]")].map((el) => norm(el.getAttribute("email")));
  const stamp = [...row.querySelectorAll("[title]")].map((el) => (el.getAttribute("title") || "").trim())
    .find((t) => TIMED.test(t) && /\d/.test(t));
  const subject = row.querySelector(".bog");
  if (row.querySelector('[role="checkbox"]') === null || senders.length === 0 || !stamp || !subject) {
    return null;
  }
  return {
    sender: senders[senders.length - 1],
    subject: text(subject),
    received: stamp.replace(/\s+/g, " "),
  };
};
const row = [...grid.querySelectorAll('tr[role="row"]')].find((r) => {
  const f = read(r);
  return f && f.sender === want.sender && f.subject === want.subject && f.received === want.received;
});
if (!row) return answer({ result: "not found", control: "message" });

const messageEls = () => [...document.querySelectorAll('[role="main"] [data-message-id]')];
row.click();
presses++;
if (!(await until(() => messageEls().length > 0, waitMs))) {
  return answer({ result: "failed", reason: "conversation did not open" });
}

// Open every message: press "Expand all" while it is offered and the count is still growing.
const expand = () => buttons().find((b) => /^expand all$/i.test(nameOf(b) || text(b)));
for (let round = 0; round < 3 && expand(); round++) {
  const seen = messageEls().length;
  press(expand());
  presses++;
  await until(() => messageEls().length > seen, Math.min(waitMs, 1000));
}

const messages = [];
const subjectEl = document.querySelector('[role="main"] h2');
const subject = subjectEl ? text(subjectEl) : want.subject;
for (const m of messageEls()) {
  const who = m.querySelector("span[email]");
  const sender = who ? norm(who.getAttribute("email")) : "";
  const stamp = [...m.querySelectorAll("[title]")].map((el) => (el.getAttribute("title") || "").trim())
    .find((t) => TIMED.test(t) && /\d/.test(t));
  if (!ADDRESS.test(sender) || !stamp) return await back({ result: "not found", control: "message" });
  const at = new Date(stamp);
  const one = {
    sender,
    subject,
    received: stamp.replace(/\s+/g, " "),
    received_at: isNaN(at.getTime()) ? null : at.toISOString(),
  };
  if (sender === own) one.own = true;
  messages.push(one);
}

async function back(result) {
  const button = buttons().find((b) =>
    /^back to /i.test(nameOf(b)) || /^back to /i.test((b.getAttribute("title") || "").trim())
  );
  if (!button) return answer({ result: "not found", control: "back" });
  press(button);
  presses++;
  if (!(await until(listShowing, waitMs))) return answer({ result: "failed", reason: "list did not return" });
  return answer(result);
}
if (messages.length === 0) return await back({ result: "failed", reason: "no messages" });
const counted = Number(args.messages);
if (Number.isInteger(counted) && counted > messages.length) {
  return await back({ result: "failed", reason: "not every message drawn" });
}
return await back({ result: "ok", messages });
