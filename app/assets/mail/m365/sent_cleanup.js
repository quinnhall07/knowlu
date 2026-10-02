// Microsoft 365, Outlook on the web: delete from Sent Items the copies Knowlu's own forwards left there,
// and nothing else (spec 4.2 step 5, 4.11 step 4, risk 13, R4). The caller has opened Sent Items. Like
// every step script it is the body of an async function of `args`: `args.address` is this mailbox's Knowlu
// forwarding address, `args.nonce` the batch's nonce, `args.max` (default 100) the most it deletes in one
// call and `args.wait_ms` (default 5000) bounds each wait. It returns one of
//   { result: "ok", deleted: number }
//   { result: "not found", control: "sent items" | "delete" }
//   { result: "failed", reason: "no address" | "no nonce" | "row not selected" | "delete did not settle",
//     deleted: number }   (the last two say how many went before it stopped)
// A row is one of ours only when BOTH hold: its subject is exactly `Knowlu <nonce>` (what forward.js sets)
// and its recipients are exactly Knowlu's address, with no one else. A copy with the nonce to someone
// else, a copy to Knowlu without the nonce, a copy to Knowlu and to someone else, and every other item
// stay. It does nothing at all unless the selected folder is Sent Items, and before each Delete it
// checks the one row selected is the row it means.
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  const by = el.getAttribute("aria-labelledby");
  if (by) {
    const t = by.split(/\s+/).map((id) => document.getElementById(id)).filter(Boolean).map(text).join(" ");
    if (t) return t;
  }
  return "";
};
const norm = (v) => String(v || "").trim().toLowerCase();
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const until = async (ready, ms) => {
  for (let waited = 0; !ready() && waited < ms; waited += 50) await sleep(50);
  return ready();
};
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;

const address = norm(args && args.address);
if (!ADDRESS.test(address)) return { result: "failed", reason: "no address", deleted: 0 };
const nonce = String((args && args.nonce) || "").trim();
if (!nonce || /\s/.test(nonce)) return { result: "failed", reason: "no nonce", deleted: 0 };
const waitMs = Number(args.wait_ms) >= 0 ? Number(args.wait_ms) : 5000;
const max = Number(args.max) > 0 ? Math.floor(Number(args.max)) : 100;

const folder = [...document.querySelectorAll('[role="treeitem"]')].find((t) =>
  t.getAttribute("aria-selected") === "true"
);
if (!folder || !/^sent items$/i.test(nameOf(folder) || text(folder))) {
  return { result: "not found", control: "sent items" };
}

// A row's recipients and subject, from its title attributes (the sender slot of Sent Items holds who it went to).
const read = (row) => {
  const recipients = new Set();
  let subject = null;
  for (const el of row.querySelectorAll("[title]")) {
    const t = (el.getAttribute("title") || "").trim();
    const parts = t.split(/\s*[;,]\s*/).filter((p) => p !== "");
    if (parts.length > 0 && parts.every((p) => ADDRESS.test(p))) {
      parts.forEach((p) => recipients.add(p.toLowerCase()));
    } else if (!(TIMED.test(t) && /\d/.test(t)) && subject === null && t !== "") {
      subject = t.replace(/\s+/g, " ");
    }
  }
  return { recipients: [...recipients], subject };
};
const ours = (row) => {
  const f = read(row);
  return f.subject === `Knowlu ${nonce}` && f.recipients.length === 1 && f.recipients[0] === address;
};
const mine = [...document.querySelectorAll('[role="listbox"] [role="option"]')].filter(ours).slice(0, max);
const del = () =>
  [...document.querySelectorAll('button, [role="button"]')].find((b) =>
    /^delete$/i.test(nameOf(b) || text(b))
  );
if (mine.length > 0 && !del()) return { result: "not found", control: "delete" };

let deleted = 0;
for (const row of mine) {
  if (!document.contains(row)) continue;
  row.click();
  const picked = () =>
    row.getAttribute("aria-selected") === "true" &&
    document.querySelectorAll('[role="option"][aria-selected="true"]').length === 1;
  if (!(await until(picked, waitMs))) return { result: "failed", reason: "row not selected", deleted };
  if (!del()) return { result: "not found", control: "delete" };
  del().click();
  if (!(await until(() => !document.contains(row), waitMs))) {
    return { result: "failed", reason: "delete did not settle", deleted };
  }
  deleted++;
}
return { result: "ok", deleted };
