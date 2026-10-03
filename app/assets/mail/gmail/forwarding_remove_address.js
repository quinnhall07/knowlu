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
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  if (el.id) {
    for (const l of document.querySelectorAll("label")) if (l.getAttribute("for") === el.id) return text(l);
  }
  const wrap = el.closest ? el.closest("label") : null;
  return wrap ? text(wrap) : "";
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
const roundTrip = async () => {
  location.hash = "#inbox";
  await sleep(Math.min(waitMs, 500));
  location.hash = SECTION;
  return await until(addEl, waitMs);
};
const names = (sel) => [...sel.options].map((o) => norm(o.value || text(o)));

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
const shown = selectedOf(list);
if (shown !== ours && list.options.length !== 1) {
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
    const b = byValue(d, /^(ok|remove)$/i);
    if (b && /remov/i.test(text(d))) return b;
  }
  return null;
};
if (await until(confirm, Math.min(waitMs, 2000))) confirm().click();
if (!(await roundTrip())) return { result: "failed", reason: "read back differed" };
const back = toSelect();
const now = back ? names(back) : null;
if (!now || now.includes(address) || !others.every((n) => now.includes(n))) {
  return { result: "failed", reason: "read back differed" };
}
return { result: "ok", removed: true, changed: true };
