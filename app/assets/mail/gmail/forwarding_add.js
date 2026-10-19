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
// dialog either form is accepted, and only inside a dialog: Next and Cancel in the dialog that holds the
// address box, Proceed in that dialog or a follow-up one that names forwarding or confirming, never in a
// dialog that was already on the page before Add was pressed.
// Fresh (the Forwarding section has no radio, no select and no Remove control) is positive: radios the script
// cannot name are { result: "not found", control: "forwarding radio" }, and nothing is pressed.
// Live 2026-10-18: the forward-to list's options are matched by the one address their text names (values are
// indexes; Gmail appends " (in use)" once forwarding is on to an address; "--" and "Remove ..." options are
// never targets).
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
// Live Gmail (2026-10-18, real mailbox): the forward-to list's option values are indexes, never the address. The
// options are the confirmed addresses (text = the bare address), a "--" separator, then one "Remove <address>"
// option per address. Once forwarding is on to an address Gmail appends " (in use)" to its option and to its
// Remove option. So an option is matched by the one address its text names, case-blind, with any trailing
// parenthesised status ignored; a value that is the address still matches (older layouts); "--" and Remove
// options are never forward targets.
const label = (o) => text(o).replace(/(\s*\([^)]*\))+$/, "").trim();
const removeOf = (o) => /^remove\s/i.test(label(o));
const isTarget = (o) => !removeOf(o) && !/^[-\u2013\u2014]+$/.test(label(o));
const ADDR = /[^\s<>()"',;]+@[^\s<>()"',;]+/g;
const addressIn = (s) => {
  const found = String(s).match(ADDR);
  return found && found.length === 1 ? norm(found[0]) : "";
};
const addressOf = (o) => {
  if (!isTarget(o)) return "";
  const v = norm(o.value);
  return addressIn(label(o)) || (v.includes("@") ? v : "");
};
const optionIs = (o, a) => addressOf(o) === a;
// What the option shows, for "is anything selected": its address, else its label ("" for a blank option).
const shown = (o) => addressOf(o) || norm(label(o));
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
// The type a browser gives an input: an absent or unknown type attribute reads as "text" (T18c's audit:
// live Outlook drew a box with no type attribute, which a `[type=...]` selector never matches).
const kind = (el) => String(el.type || el.getAttribute("type") || "text").toLowerCase();
// The address box and the dialog that holds it. Next and Cancel are looked for in that dialog only: a page
// keeps other dialogs of its own, and their buttons are never ours to press.
const emailBox = () => {
  for (const d of dialogs()) {
    const f = d.querySelector('input[type="email"]');
    if (f) return { dialog: d, field: f };
    // Else the dialog's one text box, typed or not.
    const boxes = [...d.querySelectorAll("input")].filter((el) =>
      kind(el) === "text" || kind(el) === "email"
    );
    if (boxes.length === 1) return { dialog: d, field: boxes[0] };
  }
  return null;
};
// The Forwarding section is the table the Add button sits in; a row about POP or IMAP is not part of it.
const aboutPopOrImap = (el) => /\b(pop|imap)\b/i.test(text(el.closest("tr") || el));
const isFresh = () => {
  const add = addEl();
  const section = add ? add.closest("table") || add.parentElement : null;
  if (!section) return false;
  // Fresh is positive: the section draws no radio, no select and no Remove control (live 2026-10-18). A radio
  // this script cannot name is a page it does not know, not a fresh one.
  const own = [...section.querySelectorAll('input[type="radio"], select')].filter((el) =>
    !aboutPopOrImap(el)
  );
  const remove = [...section.querySelectorAll('input[type="button"], button, [role="button"], a, span')].some(
    (el) => /\bremove\b/i.test(el.getAttribute("value") || text(el)),
  );
  return own.length === 0 && !remove;
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
// A Gmail with no forwarding address yet draws only the Add button in its Forwarding section: no radio, no
// select and no Remove control (live 2026-10-18). That is the fresh state, so the checks on what is set and
// listed are skipped and Add is pressed. Fresh is positive: a section with a radio this script cannot name is
// not fresh, and neither is one with only one of the two radios. Both are a page it does not know: not found.
const fresh = !ron && !radioOff() && isFresh();
if (!fresh && (!ron || !radioOff())) return { result: "not found", control: "forwarding radio" };
const list = fresh ? null : toSelect();
if (!fresh && ron.checked === true && list) {
  const now = selectedOf(list);
  if (args.replace !== true && now && shown(now) !== "" && !optionIs(now, address)) {
    return { result: "elsewhere" };
  }
}
if (list && [...list.options].some((o) => optionIs(o, address))) {
  return { result: "ok", stage: "listed", changed: false };
}

// A page keeps dialogs of its own (hidden ones, other prompts): the ones there before Add is pressed are
// never searched for Next, Cancel or Proceed.
const before = new Set(dialogs());
addEl().click();
if (!(await until(emailBox, waitMs))) return { result: "not found", control: "email field" };
const box = emailBox();
setValue(box.field, String(args.address).trim());
const next = dialogButton(box.dialog, /^next$/i);
if (!next) {
  const cancel = dialogButton(box.dialog, /^cancel$/i);
  if (cancel) cancel.click();
  return { result: "not found", control: "next" };
}
next.click();
// Some dialogs ask again before the popup (a follow-up dialog that names forwarding or confirming, or the add
// dialog itself); the popup itself is the host's.
const proceed = () => {
  for (const d of dialogs()) {
    if (d !== box.dialog && (before.has(d) || !/forward|confirm/i.test(text(d)))) continue;
    const b = dialogButton(d, /^proceed$/i);
    if (b) return b;
  }
  return null;
};
if (await until(proceed, waitMs)) proceed().click();
return { result: "ok", stage: "asked", changed: true };
