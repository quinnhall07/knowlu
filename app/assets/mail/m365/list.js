// Microsoft 365, Outlook on the web: read the message rows the list shows, in the form the fingerprint
// uses (spec 4.6 step 7, 4.11 step 3; S2 recorded the form). The caller has the mail view open, after a
// search or on a folder. Like every step script it is the body of an async function of `args`. It returns
//   { result: "ok", total: number, rows: [{ sender, subject, received, received_at }] }
//   { result: "not found", control: "list" | "list row" }
// `sender` is the address from the sender element's title attribute, lower case: the row's visible text is
// a display name about as often as an address, so the text is never read. `received` is the time
// element's title attribute, the full date and time as the page writes it, and `received_at` is the
// instant that names (the page's own reading of it, so its locale and zone), or null. `total` is the
// list's aria-setsize: the list is virtualised, so `rows` are only the rows drawn. A row it cannot read
// fails the whole read, to the guided fallback, rather than leaving a message out. The text of a message
// is never read, and never leaves the page.
const ADDRESS = /^[^\s@<>;,"']+@[^\s@<>;,"']+$/;
const TIMED = /\d{1,2}:\d{2}/;

const lists = [...document.querySelectorAll('[role="listbox"]')];
const list = lists.find((el) => /message list/i.test(el.getAttribute("aria-label") || "")) || lists[0];
if (!list) return { result: "not found", control: "list" };

const rows = [];
for (const row of list.querySelectorAll('[role="option"]')) {
  const titles = [...row.querySelectorAll("[title]")].map((el) => (el.getAttribute("title") || "").trim());
  const sender = titles.find((t) => ADDRESS.test(t));
  const received = titles.find((t) => t !== sender && !ADDRESS.test(t) && TIMED.test(t) && /\d/.test(t));
  const subject = titles.find((t) => t !== sender && t !== received && t !== "");
  if (!sender || !received || subject === undefined) return { result: "not found", control: "list row" };
  const at = new Date(received);
  rows.push({
    sender: sender.toLowerCase(),
    subject: subject.replace(/\s+/g, " "),
    received: received.replace(/\s+/g, " "),
    received_at: isNaN(at.getTime()) ? null : at.toISOString(),
  });
}
const sizes = [list, ...list.querySelectorAll('[role="option"]')]
  .map((el) => Number(el.getAttribute("aria-setsize"))).filter((n) => Number.isInteger(n) && n >= 0);
return { result: "ok", total: sizes.length ? Math.max(...sizes) : rows.length, rows };
