// Gmail, Settings > Forwarding and POP/IMAP: turn forwarding off ("Disable forwarding"), press Save Changes
// and read the section back after a hash round trip (spec 4.12 step 1, 4.13). A body of an async function of
// `args`: `args.address` is this mailbox's Knowlu forwarding address; `args.wait_ms`, `args.settle_ms` and
// `args.reloaded` are as in forwarding_set.js. It returns one of
//   { result: "ok", forwarding: "off", changed: boolean }   (changed is false when it was already off)
//   { result: "elsewhere" }   (forwarding goes to another address: that is the student's and is left on)
//   { result: "reloading" }
//   { result: "not found", control: "section" | "forwarding radio" | "address list" | "save" }
//      (address list: forwarding is on and the list that says where to cannot be found, so it cannot be told
//      whether the student's own is there)
//   { result: "failed", reason: "no address" | "save did not settle" | "read back differed" }
// It removes nothing from the list of addresses (forwarding_remove_address.js does that, after this).
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
const disabled = (el) => el.disabled === true || el.getAttribute("aria-disabled") === "true";
const settleMs = Number(args && args.settle_ms) >= 0 ? Number(args.settle_ms) : 5000;
const saveEl = () =>
  [...document.querySelectorAll('button, input[type="button"]')].find((b) =>
    /^save changes$/i.test(text(b) || String(b.getAttribute("value") || "").trim())
  );
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
const roff = radioOff();
if (!ron || !roff) return { result: "not found", control: "forwarding radio" };
if (ron.checked !== true) return { result: "ok", forwarding: "off", changed: false };
const list = toSelect();
if (!list) return { result: "not found", control: "address list" };
const now = selectedOf(list);
if (now && norm(now.value || text(now)) !== "" && !optionIs(now, address)) return { result: "elsewhere" };
if (!saveEl()) return { result: "not found", control: "save" };

roff.click();
await until(() => !disabled(saveEl()), waitMs);
saveEl().click();
if (!(await until(() => disabled(saveEl()), settleMs))) {
  return { result: "failed", reason: "save did not settle" };
}
if (!(await roundTrip())) return { result: "failed", reason: "read back differed" };
const back = radioOn();
if (!back || back.checked === true) return { result: "failed", reason: "read back differed" };
return { result: "ok", forwarding: "off", changed: true };
