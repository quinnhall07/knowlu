// Gmail, Settings > Forwarding and POP/IMAP: turn forwarding on to Knowlu's address, with "keep Gmail's copy
// in the Inbox", press Save Changes and read the section back after a hash round trip (spec 4.3 step 5). The
// address must already be confirmed (step 4: Knowlu confirms it server-side) and so be in the list. A body of
// an async function of `args`: `args.address` is the Knowlu forwarding address; `args.replace` is true only
// when the student answered "forward to Knowlu instead"; `args.wait_ms`, `args.settle_ms` (default 5000, the
// wait for Save to take) and `args.reloaded` are as in forwarding_read.js. It returns one of
//   { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed: boolean }
//   { result: "elsewhere" }   (forwarding is on to another address and `args.replace` is not true: nothing is
//      changed, the student is asked)
//   { result: "reloading" }
//   { result: "not found", control: "section" | "forwarding radio" | "address list" | "keep copy" | "save" }
//      (address list: our address is not among the confirmed ones)
//   { result: "failed", reason: "no address" | "save did not settle" | "read back differed" }
// It looks for every control before it clicks any, so a page missing one is left as it was found.
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
const disabled = (el) => el.disabled === true || el.getAttribute("aria-disabled") === "true";
const settleMs = Number(args && args.settle_ms) >= 0 ? Number(args.settle_ms) : 5000;
const saveEl = () =>
  [...document.querySelectorAll('button, input[type="button"]')].find((b) =>
    /^save changes$/i.test(text(b) || String(b.getAttribute("value") || "").trim())
  );
const choose = (sel, option) => {
  for (const o of sel.options) o.selected = o === option;
  sel.value = option.value;
  sel.dispatchEvent(new Event("change", { bubbles: true }));
};
// Gmail draws the saved settings when the section is drawn again, so a round trip is the read-back.
const roundTrip = async () => {
  location.hash = "#inbox";
  await sleep(Math.min(waitMs, 500));
  location.hash = SECTION;
  return await until(addEl, waitMs);
};

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };
const drew = await ensureSection();
if (drew === "reload") return { result: "reloading" };
if (drew === "missing") return { result: "not found", control: "section" };

const ron = radioOn();
if (!ron || !radioOff()) return { result: "not found", control: "forwarding radio" };
const list = toSelect();
const ours = list ? [...list.options].find((o) => optionIs(o, address)) : null;
if (!ours) return { result: "not found", control: "address list" };
const disp = dispSelect();
const keep = disp ? [...disp.options].find(isKeep) : null;
if (!keep) return { result: "not found", control: "keep copy" };
if (!saveEl()) return { result: "not found", control: "save" };
if (ron.checked === true) {
  const now = selectedOf(list);
  if (args.replace !== true && now && shown(now) !== "" && !optionIs(now, address)) {
    return { result: "elsewhere" };
  }
}

let changed = false;
if (ron.checked !== true) {
  ron.click();
  changed = true;
}
if (selectedOf(list) !== ours) {
  choose(list, ours);
  changed = true;
}
if (selectedOf(disp) !== keep) {
  choose(disp, keep);
  changed = true;
}
if (changed) {
  await until(() => !disabled(saveEl()), waitMs);
  saveEl().click();
  if (!(await until(() => disabled(saveEl()), settleMs))) {
    return { result: "failed", reason: "save did not settle" };
  }
  if (!(await roundTrip())) return { result: "failed", reason: "read back differed" };
}

const backList = toSelect();
const backDisp = dispSelect();
const back = backList && backDisp && radioOn() && radioOn().checked === true &&
  optionIs(selectedOf(backList), address) && isKeep(selectedOf(backDisp));
if (!back) return { result: "failed", reason: "read back differed" };
return { result: "ok", forwarding: "on", to: "ours", keep_copy: true, changed };
