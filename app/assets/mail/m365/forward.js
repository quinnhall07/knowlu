// Microsoft 365, Outlook on the web: forward ONE listed message as an attachment to Knowlu's address,
// under a subject that carries the batch's nonce, and read the page for a refusal (spec 4.2 step 5,
// 4.11 step 4, risk 13). The caller has the mail view open on a list (list.js) and passes the row as
// list.js read it. Like every step script it is the body of an async function of `args`:
// `args.address` is this mailbox's Knowlu forwarding address, `args.nonce` the open batch's nonce,
// `args.sender`, `args.subject` and `args.received` the row's three fields, `args.in_conversation` true
// when list.js read the message inside a conversation, and `args.wait_ms` (default 5000) bounds each wait.
// It returns one of
//   { result: "ok", sent: true }
//   { result: "refused", reason: "send failure" | "non-delivery report" }   (the routine stops: nothing
//      is sent again until the next day)
//   { result: "not found", control: "message" | "forward menu" | "forward as attachment" | "send" |
//      "to" | "subject" }
//   { result: "failed", reason: "no address" | "no nonce" | "row not selected" | "no attachment" |
//      "recipient not accepted" | "subject not set" | "send did not settle" }
// It sends one message to that one address and nothing else: the subject is exactly `Knowlu <nonce>`
// (sent_cleanup.js deletes by that same subject), the original's words are not in it, and a compose
// window with no attachment, another recipient or the wrong subject is never sent. It looks for a refusal
// before it acts, so a report already on the page stops the next forward, and again after the Send.
// Nothing here retries: "send did not settle" is not a reason to click Send again.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  const by = el.getAttribute("aria-labelledby");
  if (by) {
    const t = by.split(/\s+/).map((id) => document.getElementById(id)).filter(Boolean).map(text).join(" ");
    if (t) return t;
  }
  if (el.id) {
    for (const l of document.querySelectorAll("label")) if (l.getAttribute("for") === el.id) return text(l);
  }
  return "";
};
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
// A deadline on the clock, never a count of turns: a hidden window may throttle its timers (T18c review).
const until = async (ready, ms) => {
  const end = Date.now() + ms;
  while (!ready() && Date.now() < end) await sleep(Math.min(50, end - Date.now()));
  return ready();
};
const setValue = (el, v) => {
  if (el.focus) el.focus();
  if (!("value" in el)) {
    el.textContent = v;
  } else {
    const d = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), "value");
    if (d && d.set) d.set.call(el, v);
    else el.value = v;
  }
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
// The page's own buttons, never one inside a list row (live rows carry buttons of their own, T18c).
const buttons = () =>
  [...document.querySelectorAll('button, [role="button"]')].filter((b) => !b.closest('[role="option"]'));
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const FAILED =
  /couldn.?t send|could not send|wasn.?t sent|message not sent|failed to send|unable to send|can.?t send|sending limit/i;
const REPORT =
  /undeliverable|delivery (has )?failed|delivery status notification|wasn.?t delivered|couldn.?t be delivered|not delivered/i;

const address = norm(args && args.address);
if (!ADDRESS.test(address)) return { result: "failed", reason: "no address" };
const nonce = String((args && args.nonce) || "").trim();
if (!nonce || /\s/.test(nonce)) return { result: "failed", reason: "no nonce" };
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const want = {
  sender: norm(args.sender),
  subject: String(args.subject || "").replace(/\s+/g, " ").trim(),
  received: String(args.received || "").replace(/\s+/g, " ").trim(),
};

// A send failure in an alert, or a non-delivery report for this batch's nonce in a list.
const refusal = () => {
  for (const a of document.querySelectorAll('[role="alert"]')) {
    if (FAILED.test(text(a))) return "send failure";
  }
  for (const r of document.querySelectorAll('[role="option"]')) {
    const t = text(r);
    if (t.includes(nonce) && REPORT.test(t)) return "non-delivery report";
  }
  return null;
};
const before = refusal();
if (before) return { result: "refused", reason: before };

// The row, by the three fields list.js read from it, read as list.js reads them: the subject is the first
// title after the sender's that is not the time's and not inside a button (T18c's live rows).
const titles = (row) =>
  [...row.querySelectorAll("[title]")]
    .filter((el) =>
      !el.closest('button, [role="button"]') || !row.contains(el.closest('button, [role="button"]'))
    )
    .map((el) => (el.getAttribute("title") || "").trim());
const read = (row, fallback) => {
  const t = titles(row);
  const at = t.findIndex((v) => ADDRESS.test(v));
  if (at < 0) return null;
  const isTime = (v) => !ADDRESS.test(v) && TIMED.test(v);
  const received = t.slice(at + 1).find(isTime);
  const subject = t.slice(at + 1).find((v) => v !== "" && !ADDRESS.test(v) && !isTime(v)) ?? fallback;
  if (!received || subject === undefined) return null;
  return {
    sender: norm(t[at]),
    subject: subject.replace(/\s+/g, " "),
    received: received.replace(/\s+/g, " "),
  };
};
const matches = (f) =>
  !!f && f.sender === want.sender && f.subject === want.subject && f.received === want.received;
const options = () => [...document.querySelectorAll('[role="listbox"] [role="option"]')];
const toggle = (row) =>
  [...row.querySelectorAll('button, [role="button"]')].find((b) =>
    /^(expand|collapse) conversation$/i.test((b.getAttribute("aria-label") || "").trim())
  );
const expanded = (row) => /^collapse/i.test((toggle(row).getAttribute("aria-label") || "").trim());
const topic = (row) => titles(row).find((v) => v !== "" && !ADDRESS.test(v) && !TIMED.test(v));
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
const putAway = async (conv) => {
  if (!conv || !document.contains(conv) || !toggle(conv) || !expanded(conv)) return;
  toggle(conv).click();
  await until(() => !expanded(conv), waitMs);
};

let row = options().find((r) => !toggle(r) && matches(read(r)));
// A message list.js read inside a conversation: each conversation is opened in turn, as list.js opens it, and
// put away again unless it holds the message (then after the forward).
let opened = null;
if (!row && args.in_conversation === true) {
  for (const conv of options().filter((r) => toggle(r))) {
    if (!document.contains(conv) || !toggle(conv)) continue;
    await putAway(conv);
    const subject = topic(conv);
    const was = new Set(options());
    toggle(conv).click();
    row = (await drawnSince(was)).find((kid) => matches(read(kid, subject)));
    if (row) {
      opened = conv;
      break;
    }
    await putAway(conv);
  }
}
if (!row) return { result: "not found", control: "message" };
const result = await (async () => {
  row.click();
  const picked = () =>
    row.getAttribute("aria-selected") === "true" &&
    [...document.querySelectorAll('[role="option"][aria-selected="true"]')].length === 1;
  if (!(await until(picked, waitMs))) return { result: "failed", reason: "row not selected" };

  const menu = buttons().find((b) =>
    /forward/i.test(nameOf(b) || text(b)) && b.getAttribute("aria-haspopup")
  );
  if (!menu) return { result: "not found", control: "forward menu" };
  menu.click();
  const item = () =>
    [...document.querySelectorAll('[role="menuitem"]')].find((i) =>
      /^forward as attachment$/i.test(text(i) || nameOf(i))
    );
  if (!(await until(item, waitMs))) return { result: "not found", control: "forward as attachment" };
  item().click();

  const send = () => buttons().find((b) => /^send$/i.test(nameOf(b) || text(b)));
  if (!(await until(send, waitMs))) return { result: "not found", control: "send" };
  const sendButton = send();
  const compose = sendButton.closest('[role="dialog"], [role="form"], form') || document.body;
  const field = (pattern) =>
    [...compose.querySelectorAll('input, textarea, [role="textbox"], [contenteditable="true"]')]
      .find((el) => pattern.test(nameOf(el)));
  const to = field(/^to$/i);
  if (!to) return { result: "not found", control: "to" };
  const subject = field(/subject/i);
  if (!subject) return { result: "not found", control: "subject" };
  const attached = [compose, ...compose.querySelectorAll("*")].some((el) =>
    /\.eml\b/i.test(el.getAttribute("aria-label") || "") ||
    (el.children.length === 0 && /\.eml\b/i.test(text(el)))
  );
  if (!attached) return { result: "failed", reason: "no attachment" };

  const recipients = () => [
    ...new Set(
      [...compose.querySelectorAll("[title], [aria-label]")]
        .flatMap((el) => [el.getAttribute("title"), el.getAttribute("aria-label")])
        .map((v) => norm(v)).filter((v) => ADDRESS.test(v)),
    ),
  ];
  setValue(to, String(args.address).trim());
  press(to, "Enter");
  await until(() => recipients().length > 0, waitMs);
  const got = recipients();
  if (got.length !== 1 || got[0] !== address) return { result: "failed", reason: "recipient not accepted" };

  const wanted = `Knowlu ${nonce}`;
  setValue(subject, wanted);
  if (String("value" in subject ? subject.value : text(subject)).trim() !== wanted) {
    return { result: "failed", reason: "subject not set" };
  }

  sendButton.click();
  await until(() => !document.contains(sendButton) || refusal() !== null, waitMs);
  const after = refusal();
  if (after) return { result: "refused", reason: after };
  if (document.contains(sendButton)) return { result: "failed", reason: "send did not settle" };
  return { result: "ok", sent: true };
})();
await putAway(opened);
return result;
