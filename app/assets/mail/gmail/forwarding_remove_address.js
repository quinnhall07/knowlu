// Gmail, Settings > Forwarding and POP/IMAP: remove Knowlu's address from the list of forwarding addresses
// (spec 4.12 step 1; no re-authentication was needed, X3b). Run after forwarding_off.js, which leaves our
// address the list's selected one. The script never changes the list's selection: it presses Remove only
// when our address is already selected, or is the only one listed, and then confirms; every other
// forwarding address is left in the list (spec 10, R4). A body of an async function of `args`:
// `args.address` is the Knowlu forwarding address; `args.wait_ms` and `args.reloaded` are as in
// forwarding_read.js. It returns one of
//   { result: "ok", removed: true, changed: true }     (it is gone and every other address is still listed)
//   { result: "ok", removed: false, changed: false }   (it was not in the list: nothing clicked)
//   { result: "needs you", reason: "forwarding goes to another address" | "another address is selected" }
//      (nothing clicked or selected, see below)
//   { result: "reloading" }
//   { result: "not found", control: "section" | "address list" | "remove" }
//   { result: "failed", reason: "no address" | "read back differed" }
// Live 2026-10-18: the forward-to list's options are matched by the one address their text names (values are
// indexes; Gmail appends " (in use)" once forwarding is on to an address; "--" and "Remove ..." options are
// never targets).
const text = (el) => ((el && el.textContent) || "").replace(/\s+/g, " ").trim();
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
// Live Gmail (2026-10-18): a dialog's buttons are <button> elements named by their text; older layouts drew
// <input type=button value=...>. Either form is accepted, inside a dialog only.
const dialogButton = (d, pattern) =>
  byValue(d, pattern) ||
  [...d.querySelectorAll('button, [role="button"]')].find((b) =>
    pattern.test((b.getAttribute("aria-label") || "").trim() || text(b))
  );
const addEl = () => byValue(document, /^add a forwarding address$/i);
// Live 2026-10-18: right after a reload the section draws (its Add button is there) before Gmail fills the
// forward-to list, so the select is there with no options. With the forwarding radios drawn, wait for the list to
// have an option; a list that stays empty is carried on with as it is, and the script's own not-found and reload
// paths decide.
const listFilled = async () => {
  await until(() => {
    if (!radioOn()) return true;
    const sel = toSelect();
    return !!sel && sel.options.length > 0;
  }, waitMs);
  return "ok";
};
const ensureSection = async () => {
  if (location.hash !== SECTION) location.hash = SECTION;
  if (await until(addEl, waitMs)) return await listFilled();
  location.hash = "#inbox";
  await sleep(Math.min(waitMs, 500));
  location.hash = SECTION;
  if (await until(addEl, waitMs)) return await listFilled();
  if (args.reloaded === true) return "missing";
  location.reload();
  return "reload";
};
const radio = (pattern) =>
  [...document.querySelectorAll('input[type="radio"]')].find((r) => pattern.test(nameOf(r)));
const radioOn = () => radio(/forward a copy of incoming mail/i);
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
  if (!o || !isTarget(o)) return "";
  const v = norm(o.value);
  return addressIn(label(o)) || (v.includes("@") ? v : "");
};
const optionIs = (o, a) => addressOf(o) === a;
// The Remove options name the confirmed addresses too ("Remove <address>", with " (in use)" on the one in use).
const removesOf = (sel) =>
  [...sel.options].filter(removeOf).map((o) => addressIn(label(o).replace(/^remove\s+/i, "")));
const roundTrip = async () => {
  location.hash = "#inbox";
  await sleep(Math.min(waitMs, 500));
  location.hash = SECTION;
  if (!(await until(addEl, waitMs))) return false;
  await listFilled();
  return true;
};
const names = (sel) => [...sel.options].filter(isTarget).map((o) => addressOf(o));

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };
const drew = await ensureSection();
if (drew === "reload") return { result: "reloading" };
if (drew === "missing") return { result: "not found", control: "section" };

const list = toSelect();
if (!list) return { result: "not found", control: "address list" };
const ours = [...list.options].find((o) => optionIs(o, address));
if (!ours) return { result: "ok", removed: false, changed: false };
// Whether Remove acts on the list's selected-but-unsaved address, or on the saved forward-to address, is
// not verified on real Gmail (plan, live checks for T11 and T18c). A wrong guess would delete the student's
// own address (and, while forwarding is on, switch their forwarding off), so until a scratch-mailbox run has
// passed the selection is never changed before Remove (Checkpoint C, R4). The script acts only when ours is
// already the list's selected option, read before anything is touched, or is the only option; otherwise
// nothing is clicked or selected and the student is asked.
const radioNow = radioOn();
const selected = selectedOf(list);
if (selected !== ours && [...list.options].filter(isTarget).length !== 1) {
  const on = radioNow && radioNow.checked === true;
  return {
    result: "needs you",
    reason: on ? "forwarding goes to another address" : "another address is selected",
  };
}
const remove = byValue(document, /^remove forwarding address$/i);
if (!remove) return { result: "not found", control: "remove" };
const others = names(list).filter((n) => n !== address);
remove.click();
// Gmail asks to confirm; the confirmation names the removal, and it is pressed only if it does.
const confirm = () => {
  for (const d of document.querySelectorAll('[role="dialog"]')) {
    const b = dialogButton(d, /^(ok|remove)$/i);
    if (b && /remov/i.test(text(d))) return b;
  }
  return null;
};
if (await until(confirm, Math.min(waitMs, 2000))) confirm().click();
if (!(await roundTrip())) return { result: "failed", reason: "read back differed" };
const back = toSelect();
const now = back ? names(back) : null;
if (
  !now || now.includes(address) || removesOf(back).includes(address) || !others.every((n) => now.includes(n))
) {
  return { result: "failed", reason: "read back differed" };
}
return { result: "ok", removed: true, changed: true };
