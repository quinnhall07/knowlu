// Gmail, Settings > Forwarding and POP/IMAP: add Knowlu's address to the forwarding addresses (spec 4.3 step
// 3). Press "Add a forwarding address", type the address into the dialog's email field and press Next (and
// Proceed, if the dialog asks). Google then asks "verify it's you" in a popup, which the host allows; the
// phone tap is the student's. This script ends when it has pressed the button that opens the popup; once the
// popup has closed the host runs forwarding_add_done.js. A body of an async function of `args`:
// `args.address` is the Knowlu forwarding address; `args.replace` is true only when the student answered
// "forward to Knowlu instead"; `args.wait_ms` and `args.reloaded` are as in forwarding_read.js. It returns
//   { result: "ok", stage: "asked", changed: true }    (the popup has been asked for)
//   { result: "ok", stage: "listed", changed: false }  (the address is already a confirmed one: no dialog)
// On a Gmail with no forwarding address yet the section has only the Add button and no forwarding radios;
// that is the first state, not an error: Add is pressed. An address added but not yet verified is not an
// option of the forward-to list, so it is not "listed" and Add is pressed again.
//   { result: "elsewhere" }   (forwarding is on to another address and `args.replace` is not true: asked first)
//   { result: "reloading" }
//   { result: "not found", control: "section" | "forwarding radio" | "email field" | "next" }
//   { result: "failed", reason: "no address" }
// The section's buttons are <input type=button value=...>, found by their value. The dialogs' buttons are
// <button> elements named by their text on live Gmail (2026-10-18; older layouts drew inputs), so inside a
// dialog either form is accepted, and only inside a dialog.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  if (el.id) {
    for (const l of document.querySelectorAll("label")) if (l.getAttribute("for") === el.id) return text(l);
  }
  const wrap = el.closest ? el.closest("label") : null;
  if (wrap) return text(wrap);
  // Live Gmail (2026-10-18): the "Forward a copy of incoming mail to" radio has no label of any kind; its
  // text is in the next cell of the same table row, the row that also holds the two selects.
  const cell = el.closest ? el.closest("td") : null;
  const next = cell ? cell.nextElementSibling : null;
  return next && String(next.tagName).toLowerCase() === "td" ? text(next) : "";
};
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return !!ready();
};
const SECTION = "#settings/fwdandpop";
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 3000;
const buttons = (root) => [...(root || document).querySelectorAll('input[type="button"]')];
const byValue = (root, pattern) =>
  buttons(root).find((b) => pattern.test(String(b.getAttribute("value") || "").trim()));
const addEl = () => byValue(document, /^add a forwarding address$/i);
const ensureSection = async () => {
  if (location.hash !== SECTION) location.hash = SECTION;
  if (await until(addEl, waitMs)) return "ok";
  location.hash = "#inbox";
  await sleep(Math.min(waitMs, 500));
  location.hash = SECTION;
  if (await until(addEl, waitMs)) return "ok";
  if (args.reloaded === true) return "missing";
  location.reload();
  return "reload";
};
const radio = (pattern) =>
  [...document.querySelectorAll('input[type="radio"]')].find((r) => pattern.test(nameOf(r)));
const radioOn = () => radio(/forward a copy of incoming mail/i);
const radioOff = () => radio(/^disable forwarding$/i);
const selectsOf = () => {
  const r = radioOn();
  let n = r ? r.parentElement : null;
  while (n && !n.querySelector("select")) n = n.parentElement;
  return n ? [...n.querySelectorAll("select")] : [];
};
const isKeep = (o) => /keep gmail.s copy in the inbox/i.test(text(o));
const dispSelect = () => selectsOf().find((s) => [...s.options].some(isKeep));
const toSelect = () => selectsOf().find((s) => s !== dispSelect());
const selectedOf = (sel) => [...sel.options].find((o) => o.selected) || sel.options[0] || null;
const optionIs = (o, a) => norm(o.value) === a || norm(text(o)) === a;
const dialogs = () => [...document.querySelectorAll('[role="dialog"]')];
const dialogButton = (d, pattern) => {
  const input = [...d.querySelectorAll('input[type="button"], input[type="submit"]')].find((b) =>
    pattern.test(String(b.getAttribute("value") || "").trim())
  );
  if (input) return input;
  return [...d.querySelectorAll('button, [role="button"]')].find((b) =>
    pattern.test((b.getAttribute("aria-label") || "").trim() || text(b))
  );
};
const inDialog = (pattern) => {
  for (const d of dialogs()) {
    const b = dialogButton(d, pattern);
    if (b) return b;
  }
  return null;
};
// The type a browser gives an input: an absent or unknown type attribute reads as "text" (T18c's audit:
// live Outlook drew a box with no type attribute, which a `[type=...]` selector never matches).
const kind = (el) => String(el.type || el.getAttribute("type") || "text").toLowerCase();
const emailField = () => {
  for (const d of dialogs()) {
    const f = d.querySelector('input[type="email"]');
    if (f) return f;
    // Else the dialog's one text box, typed or not.
    const boxes = [...d.querySelectorAll("input")].filter((el) =>
      kind(el) === "text" || kind(el) === "email"
    );
    if (boxes.length === 1) return boxes[0];
  }
  return null;
};
const setValue = (el, v) => {
  if (el.focus) el.focus();
  const d = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), "value");
  if (d && d.set) d.set.call(el, v);
  else el.value = v;
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
};

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };
const drew = await ensureSection();
if (drew === "reload") return { result: "reloading" };
if (drew === "missing") return { result: "not found", control: "section" };

const ron = radioOn();
// A Gmail with no forwarding address yet draws only the Add button and no forwarding radios at all (live
// 2026-10-18): that is the fresh state, so the checks on what is set and listed are skipped and Add is pressed.
// One radio without the other is a page this script does not know: not found.
const fresh = !ron && !radioOff();
if (!fresh && (!ron || !radioOff())) return { result: "not found", control: "forwarding radio" };
const list = fresh ? null : toSelect();
if (!fresh && ron.checked === true && list) {
  const now = selectedOf(list);
  if (args.replace !== true && now && norm(now.value || text(now)) !== "" && !optionIs(now, address)) {
    return { result: "elsewhere" };
  }
}
if (list && [...list.options].some((o) => optionIs(o, address))) {
  return { result: "ok", stage: "listed", changed: false };
}

addEl().click();
if (!(await until(emailField, waitMs))) return { result: "not found", control: "email field" };
setValue(emailField(), String(args.address).trim());
const next = inDialog(/^next$/i);
if (!next) {
  const cancel = inDialog(/^cancel$/i);
  if (cancel) cancel.click();
  return { result: "not found", control: "next" };
}
next.click();
// Some dialogs ask again before the popup; the popup itself is the host's.
if (await until(() => inDialog(/^proceed$/i), waitMs)) inDialog(/^proceed$/i).click();
return { result: "ok", stage: "asked", changed: true };
