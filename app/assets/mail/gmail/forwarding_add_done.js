// Gmail, Settings > Forwarding and POP/IMAP: after Google's verify popup has closed, the opener usually shows a
// dialog saying the confirmation link was sent; press its OK (spec 4.3 step 3). Knowlu confirms the address
// itself afterwards (step 4). A body of an async function of `args`: `args.wait_ms` (default 5000) bounds the
// wait for the dialog to show and for it to go; `args.address` is the Knowlu forwarding address and
// `args.reloaded` is as in forwarding_read.js. It returns one of
//   { result: "ok", sent: true }
//   { result: "reloading" }   (only on the read-back below, when the section did not draw)
//   { result: "not found", control: "confirmation dialog" | "ok" }
//   { result: "failed", reason: "dialog stayed" }
// The button is found only inside that dialog: a <button> named "OK" on live Gmail (2026-10-18), or an
// <input type=button value="OK"> on older layouts.
// Live 2026-10-18 the popup can do the whole confirmation itself (accounts.google.com, SetSID, SetOSID, then
// the mailbox, and it closes) and the opener draws no dialog at all, yet the address WAS added. So when no
// dialog shows within the wait, the section is read back: a row of the Forwarding section that starts with
// "Verify" and holds `args.address` (case-blind, trimmed) means the verification email is on its way, and an
// address that is already an option of the forward-to list is already confirmed; both are ok. Anything else
// stays "not found / confirmation dialog". The read-back never presses Re-send email or Remove address, or
// anything else: it only reads.
// Live 2026-10-18: the forward-to list's options are matched by the one address their text names (values are
// indexes; Gmail appends " (in use)" once forwarding is on to an address; "--" and "Remove ..." options are
// never targets).
const text = (el) => ((el && el.textContent) || "").replace(/\s+/g, " ").trim();
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return !!ready();
};
const SECTION = "#settings/fwdandpop";
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const sentDialog = () =>
  [...document.querySelectorAll('[role="dialog"]')].find((d) =>
    /confirmation/i.test(text(d)) && /sent/i.test(text(d))
  );

// The section is drawn when its Add button is (as in forwarding_add.js).
const drawn = () =>
  [...document.querySelectorAll('input[type="button"]')].some((b) =>
    /^add a forwarding address$/i.test(String(b.getAttribute("value") || "").trim())
  );
const address = norm(args && args.address);
const verifyRow = () =>
  [...document.querySelectorAll("tr")].some((r) =>
    /^verify\b/i.test(text(r)) && norm(text(r)).includes(address)
  );
// Live Gmail (2026-10-18): the forward-to list's option values are indexes; the text is the bare address, or
// "<address> (in use)" once forwarding is on to it, then a "--" separator and "Remove <address>" options. An
// option is matched by the one address its text names (trailing parenthesised status ignored); "--" and Remove
// options are not addresses. A value that is the address still matches (older layouts).
const label = (o) => text(o).replace(/(\s*\([^)]*\))+$/, "").trim();
const isTarget = (o) => !/^remove\s/i.test(label(o)) && !/^[-\u2013\u2014]+$/.test(label(o));
const addressIn = (s) => {
  const found = String(s).match(/[^\s<>()"',;]+@[^\s<>()"',;]+/g);
  return found && found.length === 1 ? norm(found[0]) : "";
};
const listed = () =>
  [...document.querySelectorAll("select option")].some((o) =>
    isTarget(o) && (addressIn(label(o)) === address || norm(o.value) === address)
  );
const readBack = async () => {
  if (!address) return "none";
  if (location.hash !== SECTION) location.hash = SECTION;
  if (!(await until(drawn, waitMs))) {
    location.hash = "#inbox";
    await sleep(Math.min(waitMs, 500));
    location.hash = SECTION;
    if (!(await until(drawn, waitMs))) {
      if (args.reloaded === true) return "none";
      location.reload();
      return "reload";
    }
  }
  return verifyRow() || listed() ? "found" : "none";
};

if (!(await until(sentDialog, waitMs))) {
  const back = await readBack();
  if (back === "reload") return { result: "reloading" };
  if (back === "found") return { result: "ok", sent: true };
  return { result: "not found", control: "confirmation dialog" };
}
const dialog = sentDialog();
const isOk = (s) => /^ok$/i.test(s);
const ok =
  [...dialog.querySelectorAll('input[type="button"], input[type="submit"]')].find((b) =>
    isOk(String(b.getAttribute("value") || "").trim())
  ) || [...dialog.querySelectorAll('button, [role="button"]')].find((b) =>
    isOk((b.getAttribute("aria-label") || "").trim() || text(b))
  );
if (!ok) return { result: "not found", control: "ok" };
ok.click();
if (!(await until(() => !document.contains(dialog), waitMs))) {
  return { result: "failed", reason: "dialog stayed" };
}
return { result: "ok", sent: true };
