// Gmail, Settings > Forwarding and POP/IMAP: after Google's verify popup has closed, the opener shows a
// dialog saying the confirmation link was sent; press its OK (spec 4.3 step 3). Knowlu confirms the address
// itself afterwards (step 4). A body of an async function of `args`: `args.wait_ms` (default 5000) bounds the
// wait for the dialog to show and for it to go. It returns one of
//   { result: "ok", sent: true }
//   { result: "not found", control: "confirmation dialog" | "ok" }
//   { result: "failed", reason: "dialog stayed" }
// The button is found only inside that dialog: a <button> named "OK" on live Gmail (2026-10-18), or an
// <input type=button value="OK"> on older layouts.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return !!ready();
};
const waitMs = Number(args && args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const sentDialog = () =>
  [...document.querySelectorAll('[role="dialog"]')].find((d) =>
    /confirmation/i.test(text(d)) && /sent/i.test(text(d))
  );

if (!(await until(sentDialog, waitMs))) return { result: "not found", control: "confirmation dialog" };
const dialog = sentDialog();
const isOk = (s) => /^ok$/i.test(s);
const ok = [...dialog.querySelectorAll('input[type="button"], input[type="submit"]')].find((b) =>
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
