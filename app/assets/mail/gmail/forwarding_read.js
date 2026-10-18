// Gmail, Settings > Forwarding and POP/IMAP: read the forwarding setting and change nothing (spec 4.3 step
// 5's check, 4.11 step 2). A body of an async function of `args`: `args.address` is this mailbox's Knowlu
// forwarding address; `args.wait_ms` (default 3000) bounds each wait for the section to draw; `args.reloaded`
// is true when the caller is running the script again after the reload it asked for. It goes to the section
// by hash; a section that did not draw is redrawn by a hash round trip, and if that fails it reloads the page
// once. It returns one of
//   { result: "ok", forwarding: "on" | "off", to: "none" | "ours" | "elsewhere", keep_copy: boolean,
//     listed: boolean }   (listed: our address is among the confirmed forwarding addresses)
//   { result: "reloading" }   (the page was reloaded: run it again with `reloaded: true`)
//   { result: "not found", control: "section" | "forwarding radio" | "address list" }
//   { result: "failed", reason: "no address" }
// An address that is not ours is never returned, only named "elsewhere". Gmail's buttons are
// <input type=button value=...>, found by their value.
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

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };
const drew = await ensureSection();
if (drew === "reload") return { result: "reloading" };
if (drew === "missing") return { result: "not found", control: "section" };

if (!radioOn() || !radioOff()) return { result: "not found", control: "forwarding radio" };
const on = radioOn().checked === true;
const list = toSelect();
if (on && !list) return { result: "not found", control: "address list" };
const there = on && list ? selectedOf(list) : null;
const disp = dispSelect();
const kept = disp ? selectedOf(disp) : null;
return {
  result: "ok",
  forwarding: on ? "on" : "off",
  to: !there || norm(there.value || text(there)) === ""
    ? "none"
    : optionIs(there, address)
    ? "ours"
    : "elsewhere",
  keep_copy: !!kept && isKeep(kept),
  listed: !!list && [...list.options].some((o) => optionIs(o, address)),
};
