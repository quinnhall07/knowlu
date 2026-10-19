// Gmail: forward the listed messages on the page as attachments in ONE send to Knowlu's address, under a
// subject that carries the batch's nonce, and read the page for a refusal (spec 4.3 step 7, 4.11 step 4, risk
// 13). The caller has the mail view open on a page of results (list.js) and passes that page's rows as
// list.js read them. Like every step script it is the body of an async function of `args`:
// `args.address` is this mailbox's Knowlu forwarding address, `args.nonce` the open batch's nonce,
// `args.rows` the rows to send ([{ sender, subject, received, messages? }], at most 100, all on this page;
// `messages` is a conversation row's message count from list.js, which "Forward as attachment" is expected
// to attach as that many attachments, T11.2),
// `args.max_bytes` (default 25000000) the most the attachments may add up to, `args.wait_ms` (default
// 5000) bounds each wait, and `args.upload_ms` (default 30000, never less than `wait_ms`) bounds the wait for
// the attachments to upload. It returns one of
//   { result: "ok", sent: true, count: number }   (count: the attachments sent, a conversation row's
//      messages each)
//   { result: "refused", reason: "send failure" | "non-delivery report" }   (the routine stops: nothing is
//      sent again until the next day)
//   { result: "not found", control: "message" | "more menu" | "forward as attachment" | "send" | "to" |
//      "subject" }
//   { result: "failed", reason: "no address" | "no nonce" | "no rows" | "too many rows" |
//      "rows already selected" | "rows not selected" | "attachment failed" | "no attachment" |
//      "attachments do not match the rows" | "attachment size unreadable" | "over 25 MB" |
//      "recipient not accepted" | "subject not set" | "send did not settle" }   ("attachments do not match
//      the rows" also carries `attached` and `expected`, the counts it compared)
// It sends one message to that one address and nothing else: the subject is exactly `Knowlu <nonce>`
// (sent_cleanup.js deletes by that same subject), the originals' words are not in it, and a compose window
// with no attachment, a count of attachments that is not the rows', more than the size limit, Gmail's offer
// of Google Drive links, another recipient or the wrong subject is never sent: the draft is discarded and the
// selection let go. Over the limit is the caller's to halve; a send is never split here. It looks for a
// refusal before it acts, so a report already on the page stops the next forward, and again after the Send.
// Nothing here retries: "send did not settle" is not a reason to press Send again.
// Live 2026-10-18/19 (WebView2 155): only the SHOWN grid's rows are read (Gmail keeps a previous view's grid
// in the document, hidden, offsetParent null); a row's checkbox toggles on `click()`. The toolbar's More is
// aria-label "More email options" (aria-haspopup "false") and opens on a press (pointerdown, mousedown,
// pointerup, mouseup, click), never `click()`. A menu item answers only events carrying its centre's
// clientX/clientY sent to the innermost element at that point (`pointAt`: pointerover, mouseover,
// pointermove, mousemove, then the press); Send and Discard are pressed the same way. An attachment chip is
// a `div.dL`: "Uploading of attachment: <name>" while it uploads (waited for, inside `upload_ms`), "Uploading
// of attachment: <name> failed. Press enter to retry" when the upload failed ("attachment failed": the
// draft is discarded; so is a chip still uploading when `upload_ms` runs out), and "Attachment: <name>" when attached, its own text "<name>.eml (6K)" with the size
// a leaf inside it: the size is read after the name, never from it (a subject may hold "(3)").
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => (el.getAttribute("aria-label") || el.getAttribute("data-tooltip") || "").trim();
// Shown: live, a hidden view's elements have no offsetParent.
const shown = (el) => !!el && el.offsetParent !== null;
const fire = (el, type, at) => {
  const C = type.startsWith("pointer") && typeof PointerEvent === "function"
    ? PointerEvent
    : typeof MouseEvent === "function"
    ? MouseEvent
    : Event;
  const init = {
    bubbles: true, cancelable: true, composed: true, button: 0, buttons: type.endsWith("down") ? 1 : 0,
    pointerId: 1, pointerType: "mouse", isPrimary: true, clientX: at ? at.x : 0, clientY: at ? at.y : 0,
  };
  if (typeof window !== "undefined") init.view = window;
  const e = new C(type, init);
  for (const k of ["button", "buttons", "clientX", "clientY"]) {
    if (e[k] !== init[k]) Object.defineProperty(e, k, { value: init[k] });
  }
  el.dispatchEvent(e);
};
const PRESS = ["pointerdown", "mousedown", "pointerup", "mouseup", "click"];
const tap = (el) => {
  for (const type of PRESS) fire(el, type);
};
// The whole pointer sequence at the centre of `el`, sent to the innermost element there.
const pointAt = (el) => {
  const r = el.getBoundingClientRect();
  const at = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  const hit = typeof document.elementFromPoint === "function" ? document.elementFromPoint(at.x, at.y) : null;
  const target = hit && el.contains(hit) ? hit : el;
  for (const type of ["pointerover", "mouseover", "pointermove", "mousemove", ...PRESS]) fire(target, type, at);
};
const norm = (v) => String(v || "").trim().toLowerCase();
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
const buttons = () => [...document.querySelectorAll('[role="button"], button')].filter(shown);
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;
const FAILED =
  /message not sent|couldn.?t send|could not be sent|couldn.?t be sent|failed to send|unable to send|can.?t send|sending limit/i;
const REPORT =
  /undeliverable|delivery status notification|delivery (has )?failed|wasn.?t delivered|couldn.?t be delivered|not delivered/i;
const UNITS = { "": 1, K: 1e3, M: 1e6, G: 1e9 };

const address = norm(args && args.address);
if (!ADDRESS.test(address)) return { result: "failed", reason: "no address" };
const nonce = String((args && args.nonce) || "").trim();
if (!nonce || /\s/.test(nonce)) return { result: "failed", reason: "no nonce" };
const wanted = Array.isArray(args.rows) ? args.rows : [];
if (wanted.length === 0) return { result: "failed", reason: "no rows" };
if (wanted.length > 100) return { result: "failed", reason: "too many rows" };
// A plain row is one attachment; a conversation row is as many as the messages list.js counted in it.
const expected = wanted.reduce((n, w) => n + (Number.isInteger(w && w.messages) && w.messages > 1 ? w.messages : 1), 0);
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const uploadMs = Number(args.upload_ms) >= 0 ? Number(args.upload_ms) : Math.max(waitMs, 30000);
const maxBytes = Number(args.max_bytes) > 0 ? Number(args.max_bytes) : 25000000;

// A send failure in an alert, or a non-delivery report for this batch's nonce in a list.
const refusal = () => {
  for (const a of document.querySelectorAll('[role="alert"]')) {
    if (FAILED.test(text(a))) return "send failure";
  }
  for (const r of document.querySelectorAll('table[role="grid"] tr[role="row"]')) {
    const t = text(r);
    if (t.includes(nonce) && REPORT.test(t)) return "non-delivery report";
  }
  return null;
};
const before = refusal();
if (before) return { result: "refused", reason: before };

const grid = () => [...document.querySelectorAll('table[role="grid"]')].filter(shown).pop() || null;
const rowEls = () => (grid() ? [...grid().querySelectorAll('tr[role="row"]')] : []);
const boxOf = (row) => row.querySelector('[role="checkbox"]');
const ticked = () => rowEls().filter((r) => boxOf(r) && boxOf(r).getAttribute("aria-checked") === "true");
// The row for each wanted message, by the three fields list.js read from it; two wanted messages never
// share a row.
const read = (row) => {
  const senders = [...row.querySelectorAll("span[email]")].map((el) => norm(el.getAttribute("email")));
  const stamp = [...row.querySelectorAll("[title]")].map((el) => (el.getAttribute("title") || "").trim())
    .find((t) => TIMED.test(t) && /\d/.test(t));
  const subject = row.querySelector(".bog");
  if (!boxOf(row) || senders.length === 0 || !stamp || !subject) return null;
  return {
    sender: senders[senders.length - 1],
    subject: text(subject),
    received: stamp.replace(/\s+/g, " "),
  };
};
const targets = [];
for (const w of wanted) {
  const want = {
    sender: norm(w && w.sender),
    subject: String((w && w.subject) || "").replace(/\s+/g, " ").trim(),
    received: String((w && w.received) || "").replace(/\s+/g, " ").trim(),
  };
  const row = rowEls().find((r) => {
    const f = !targets.includes(r) && read(r);
    return f && f.sender === want.sender && f.subject === want.subject && f.received === want.received;
  });
  if (!row) return { result: "not found", control: "message" };
  targets.push(row);
}
if (ticked().length > 0) return { result: "failed", reason: "rows already selected" };

// Whatever happens next, a failure lets go of the selection and discards the draft it opened.
const bail = async (result) => {
  const discard = buttons().find((b) => /^discard draft/i.test(nameOf(b)));
  if (discard) pointAt(discard);
  for (const r of ticked()) boxOf(r).click();
  await until(() => ticked().length === 0, waitMs);
  return result;
};
for (const r of targets) boxOf(r).click();
const picked = () => ticked().length === targets.length && targets.every((r) => ticked().includes(r));
if (!(await until(picked, waitMs))) return await bail({ result: "failed", reason: "rows not selected" });

const more = buttons().find((b) => /^more( email options)?$/i.test(nameOf(b) || text(b)));
if (!more) return await bail({ result: "not found", control: "more menu" });
tap(more);
const item = () =>
  // A menu may be fixed-position (offsetParent null though drawn): its box says it is drawn.
  [...document.querySelectorAll('[role="menuitem"]')].filter((i) =>
    shown(i) || i.getBoundingClientRect().height > 0
  ).find((i) =>
    /^forward as attachment$/i.test(text(i))
  );
if (!(await until(item, waitMs))) {
  return await bail({ result: "not found", control: "forward as attachment" });
}
pointAt(item());

const send = () => buttons().find((b) => /^send\b/i.test(nameOf(b) || text(b)));
if (!(await until(send, waitMs))) return await bail({ result: "not found", control: "send" });
const sendButton = send();
const compose = sendButton.closest('[role="dialog"]') || document.body;
const field = (pattern) =>
  [...compose.querySelectorAll('input, textarea, [role="textbox"]')].find((el) =>
    pattern.test(nameOf(el) || el.getAttribute("name") || "")
  );
const to = field(/^to( recipients)?$/i);
if (!to) return await bail({ result: "not found", control: "to" });
const subject = field(/^(subject|subjectbox)$/i);
if (!subject) return await bail({ result: "not found", control: "subject" });

// The chips upload first: wait while one is still uploading; a failed upload is never sent. Live
// (2026-10-19) Send is drawn before any chip, so the wait lasts until the rows' chips have all attached.
const chipLabels = () => [...compose.querySelectorAll("[aria-label]")].map(nameOf);
const uploadFailed = () => chipLabels().some((l) => /^uploading of attachment:.*failed/i.test(l));
const uploading = () => chipLabels().some((l) => /^uploading of attachment:/i.test(l) && !/failed/i.test(l));
const attached = () => [...compose.querySelectorAll('[aria-label^="Attachment:"]')];
// Uploads have their own bound: live (2026-10-19) several rows' chips attach one at a time, seconds apart,
// for longer than a step's wait. A compose that attached fewer is known only when the bound runs out: a
// count holding steady for a while is not the end of it.
await until(() => uploadFailed() || (!uploading() && attached().length >= expected), uploadMs);
// A chip still uploading at the bound is neither counted nor sized: it is never sent.
if (uploadFailed() || uploading()) return await bail({ result: "failed", reason: "attachment failed" });

const attachments = attached();
if (attachments.length === 0 && !uploading()) return await bail({ result: "failed", reason: "no attachment" });
if (attachments.length !== expected) {
  return await bail({
    result: "failed",
    reason: "attachments do not match the rows",
    attached: attachments.length,
    expected,
  });
}
let total = 0;
const SIZE = /\((\d+(?:\.\d+)?)\s*([KMG]?)B?\)/gi;
for (const a of attachments) {
  // The size is the chip's own text after the name, "<name>.eml (6K)", never the name itself (a subject
  // may hold "(3)"): the name is the label's, and the last size after it counts.
  const name = nameOf(a).replace(/^attachment:\s*/i, "");
  const own = text(a);
  const after = name && own.startsWith(name) ? own.slice(name.length) : own.replace(/^.*\.eml\b/i, "");
  const m = [...after.matchAll(SIZE)].pop();
  if (!m) return await bail({ result: "failed", reason: "attachment size unreadable" });
  total += Number(m[1]) * UNITS[m[2].toUpperCase()];
}
const offer = [...compose.querySelectorAll('[role="alert"], [role="alertdialog"]')].some((a) =>
  /google drive|25 ?MB/i.test(text(a))
);
if (total >= maxBytes || offer) return await bail({ result: "failed", reason: "over 25 MB" });

// A recipient Gmail accepted: live (2026-10-19) a `div[role=option]` naming the address in data-hovercard-id;
// older composes drew an `[email]` element.
const recipients = () => [
  ...new Set(
    [
      ...[...compose.querySelectorAll("[email]")].map((el) => el.getAttribute("email")),
      ...[...compose.querySelectorAll('[role="option"][data-hovercard-id]')].map((el) =>
        el.getAttribute("data-hovercard-id")
      ),
    ].map(norm).filter(Boolean),
  ),
];
setValue(to, String(args.address).trim());
press(to, "Enter");
await until(() => recipients().length > 0, waitMs);
const got = recipients();
if (got.length !== 1 || got[0] !== address) {
  return await bail({ result: "failed", reason: "recipient not accepted" });
}

const line = `Knowlu ${nonce}`;
setValue(subject, line);
if (String(subject.value).trim() !== line) return await bail({ result: "failed", reason: "subject not set" });

pointAt(sendButton);
await until(() => !document.contains(sendButton) || refusal() !== null, waitMs);
const after = refusal();
if (after) return { result: "refused", reason: after };
if (document.contains(sendButton)) return { result: "failed", reason: "send did not settle" };
// Live (2026-10-19) the forwarded rows stay ticked after the send; the next forward starts from none.
for (const r of ticked()) boxOf(r).click();
await until(() => ticked().length === 0, waitMs);
return { result: "ok", sent: true, count: attachments.length };
