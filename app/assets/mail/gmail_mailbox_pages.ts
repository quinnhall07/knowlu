// A synthetic Gmail mailbox for the Gmail mailbox scripts (plan T10.3b). It is modelled on the spikes'
// selectors (research note 6, S2): the list is a `table[role=grid]` of `tr[role=row]`, each with a
// `div[role=checkbox]`, the sender as a `span[email]` whose `email` attribute is the address (the visible
// text is a display name or the address), the subject as a `span.bog`, and the time as a `span[title]`
// whose title is the full date and time. The pager reads "1–50 of 123" with Newer/Older/Newest/Oldest
// buttons; the toolbar has Delete and More. The page is a model of what the scripts rely on, not a copy of
// Gmail; the live checks (T11, T18c) say whether the real page agrees. Every name, address and subject here
// is synthetic.
import { El, load, Page } from "./test_dom.ts";

export const OURS = "box-1@in.example.invalid";
export const NONCE = "bk7f3a9c2e41";

export type Folder = "Inbox" | "Spam" | "Trash" | "Sent";

export interface Msg {
  id: string;
  /** The sender's address, or, in Sent, the recipient's. */
  who: string;
  /** What the row shows: a display name, or the bare address. */
  shown?: string;
  subject: string;
  snippet?: string;
  /** The time element's title. */
  at: string;
  /** The day, as the search's `after:` compares it. */
  day: string;
  folder: Folder;
  /** The size of the message as an attachment, in bytes. */
  size: number;
  /** Other participants of a conversation: the row lists them too. */
  also?: string[];
}

export interface Opts {
  /** The folder the page shows: the Inbox, or Sent. */
  view?: "inbox" | "sent";
  /** Rows to a page (Gmail's is 50). */
  size?: number;
  /** What a Send does. */
  send?: "ok" | "banner" | "banner-closes" | "ndr";
  /** The More menu has no "Forward as attachment". */
  noAttachmentItem?: boolean;
  /** The compose window opens already addressed to someone else. */
  prefill?: string;
  /** "Forward as attachment" opens a compose with no attachment. */
  bare?: boolean;
  /** The compose carries one attachment fewer than the rows selected. */
  short?: boolean;
  /** The page never answers the search. */
  deaf?: boolean;
  /** The compose offers to send Google Drive links, whatever the sizes shown. */
  driveOffer?: boolean;
  /** The pager has no Oldest button. */
  noOldest?: boolean;
  /** The row of `id` shows the message `as` once it is ticked. */
  swap?: { id: string; as: string };
  /** The page shows no "1–50 of 123" range. */
  blind?: boolean;
}

export interface Sent {
  to: string[];
  subject: string;
  /** The ids of the messages attached. */
  attached: string[];
}

export interface Box {
  page: Page;
  msgs: Msg[];
  sent: Sent[];
  queries: string[];
  deleted: string[];
  discards: number;
  el(id: string): El;
  rows(): El[];
  checked(): string[];
  range(): string;
}

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
export const when = (n: number) => `${DAYS[new Date(2026, 9, n).getDay()]}, Oct ${n}, 2026, 3:04 PM`;

export function msg(
  id: string,
  who: string,
  subject: string,
  day: number,
  folder: Folder = "Inbox",
  size = 20_000,
): Msg {
  return {
    id,
    who,
    subject,
    at: when(day),
    day: `2026-10-${String(day).padStart(2, "0")}`,
    folder,
    size,
    shown: id.endsWith("a") ? who : who.split("@")[0].replace(/\W+/g, " "),
  };
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
const sized = (n: number) => n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : `${Math.max(1, Math.round(n / 1e3))}K`;
const LIMIT = 25_000_000;

export function mailbox(all: Msg[], o: Opts = {}): Box {
  const size = o.size ?? 50;
  const page = load(`
    <div role="navigation"><a href="#inbox">Inbox</a><a href="#sent">Sent</a></div>
    <div role="search"><input type="text" id="q" name="q" aria-label="Search mail"></div>
    <div role="toolbar" id="bar">
      <div role="button" id="del" aria-label="Delete">Delete</div>
      <div role="button" id="more" aria-label="More" aria-haspopup="true">More</div>
      <span id="rangebox"></span>
      <div role="button" id="newest" aria-label="Newest">Newest</div>
      <div role="button" id="newer" aria-label="Newer">Newer</div>
      <div role="button" id="older" aria-label="Older">Older</div>
      ${o.noOldest ? "" : `<div role="button" id="oldest" aria-label="Oldest">Oldest</div>`}
    </div>
    <table role="grid" id="grid"><tbody id="list"></tbody></table><div id="over"></div>`);
  const d = page.document;
  const loc = { hash: o.view === "sent" ? "#sent" : "#inbox", host: "mail.google.com", reload() {} };
  page.location = loc;
  const b: Box = {
    page,
    msgs: all.map((m) => ({ ...m })),
    sent: [],
    queries: [],
    deleted: [],
    discards: 0,
    el: (id) => d.getElementById(id),
    rows: () => [...d.querySelectorAll('tr[role="row"]')],
    checked: () =>
      [...d.querySelectorAll('tr[role="row"] [role="checkbox"][aria-checked="true"]')].map((c: El) =>
        c.closest("tr").getAttribute("data-id")
      ),
    range: () => (d.getElementById("rangebox").textContent || "").replace(/\s+/g, " ").trim(),
  };
  let view: { kind: "inbox" | "sent" | "search"; since: string; out: string[] } = {
    kind: o.view ?? "inbox",
    since: "",
    out: [],
  };
  let at = 0;
  let swapped = false;
  const shown = () => {
    const keep = (m: Msg) =>
      view.kind === "inbox"
        ? m.folder === "Inbox"
        : view.kind === "sent"
        ? m.folder === "Sent"
        : m.day >= view.since && !view.out.includes(m.folder);
    return b.msgs.filter(keep).sort((x, y) => (x.day < y.day ? 1 : x.day > y.day ? -1 : 0));
  };
  const pages = () => Math.max(1, Math.ceil(shown().length / size));
  const fill = () => {
    const all = shown();
    at = Math.min(at, pages() - 1);
    const rows = all.slice(at * size, at * size + size);
    d.getElementById("list").innerHTML = all.length === 0
      ? `<tr><td>No messages matched your search.</td></tr>`
      : rows.map((m) => `
      <tr role="row" data-id="${m.id}"><td><div role="checkbox" aria-checked="false"></div></td>
        <td><span email="${esc(m.who)}">${esc(m.shown ?? m.who)}</span>${
        (m.also ?? []).map((a) => `, <span email="${esc(a)}">${esc(a.split("@")[0])}</span>`).join("")
      }</td>
        <td><span class="bog">${esc(m.subject)}</span><span class="y2"> - ${
        esc(m.snippet ?? "a preview")
      }</span></td>
        <td class="xW"><span title="${m.at}">Oct ${Number(m.day.slice(8))}</span></td></tr>`).join("");
    const n = (v: number) => `<span class="ts">${v}</span>`;
    d.getElementById("rangebox").innerHTML = all.length === 0 || o.blind
      ? ""
      : `<span class="Dj">${n(at * size + 1)}–${n(at * size + rows.length)} of ${n(all.length)}</span>`;
    for (
      const [id, off] of [["newest", at === 0], ["newer", at === 0], ["older", at >= pages() - 1], [
        "oldest",
        at >= pages() - 1,
      ]] as [string, boolean][]
    ) {
      b.el(id)?.setAttribute("aria-disabled", String(off));
    }
    for (const c of d.querySelectorAll('[role="checkbox"]')) {
      c.addEventListener("click", () => {
        c.setAttribute("aria-checked", String(c.getAttribute("aria-checked") !== "true"));
        const row = c.closest("tr");
        const to = o.swap && row.getAttribute("data-id") === o.swap.id && !swapped &&
          b.msgs.find((m) => m.id === o.swap!.as);
        if (!to) return;
        // The page draws another message in this row the moment it is ticked.
        swapped = true;
        row.querySelector("span[email]").setAttribute("email", to.who);
        row.querySelector(".bog").textContent = to.subject;
      });
    }
  };
  const go = (id: string, to: () => number) =>
    b.el(id)?.addEventListener("click", () => {
      if (b.el(id).getAttribute("aria-disabled") === "true") return;
      at = to();
      setTimeout(fill, 20);
    });
  go("newest", () => 0);
  go("newer", () => at - 1);
  go("older", () => at + 1);
  go("oldest", () => pages() - 1);
  fill();

  b.el("q").addEventListener("keydown", (e: El) => {
    if (e.key !== "Enter") return;
    const q = b.el("q").value;
    b.queries.push(q);
    if (o.deaf) return;
    const since = /after:(\d{4})\/(\d{2})\/(\d{2})/.exec(q);
    const out = [["spam", "Spam"], ["trash", "Trash"], ["sent", "Sent"]].filter(([k]) =>
      q.includes(`-in:${k}`)
    );
    view = {
      kind: "search",
      since: since ? `${since[1]}-${since[2]}-${since[3]}` : "0000-00-00",
      out: out.map((x) => x[1]),
    };
    loc.hash = `#search/${encodeURIComponent(q)}`;
    at = 0;
    setTimeout(fill, 20);
  });
  b.el("del").addEventListener("click", () => {
    const ids = b.checked();
    if (ids.length === 0) return;
    for (const m of b.msgs) if (ids.includes(m.id)) m.folder = "Trash";
    b.deleted.push(...ids);
    fill();
  });
  b.el("more").addEventListener("click", () => {
    if (b.checked().length === 0) return;
    const item = o.noAttachmentItem ? "" : `<div role="menuitem" id="m-att">Forward as attachment</div>`;
    b.el("over").innerHTML = `<div role="menu"><div role="menuitem">Mark as read</div>${item}</div>`;
    b.el("m-att")?.addEventListener("click", () => compose(b.checked()));
  });

  function compose(ids: string[]) {
    const picked = b.msgs.filter((m) => ids.includes(m.id));
    const files = o.bare ? [] : o.short ? picked.slice(1) : picked;
    const total = files.reduce((n, m) => n + m.size, 0);
    const chip = (v: string) => `<span email="${esc(v)}">${esc(v)}</span>`;
    b.el("over").innerHTML = `<div role="dialog" aria-label="New Message">
      <div id="chips">${o.prefill ? chip(o.prefill) : ""}</div>
      <input type="text" id="to" aria-label="To recipients">
      <input type="text" id="subj" name="subjectbox" aria-label="Subject" value="Fwd: ${
      esc(picked[0]?.subject ?? "")
    }">
      ${
      files.map((m) =>
        `<div class="aZo"><a aria-label="Attachment: ${esc(m.subject)}.eml">${
          esc(m.subject)
        }.eml</a><span> (${sized(m.size)})</span></div>`
      ).join("")
    }
      ${
      o.driveOffer || total > LIMIT
        ? `<div role="alertdialog">The files you attached are over 25 MB. Send them as Google Drive links?</div>`
        : ""
    }
      <div role="button" aria-label="Send (Ctrl-Enter)">Send</div>
      <div role="button" aria-label="Discard draft (Ctrl-Shift-D)">Discard</div></div>`;
    b.el("to").addEventListener("keydown", (e: El) => {
      const v = b.el("to").value.trim();
      if (e.key !== "Enter" || !v) return;
      b.el("chips").insertAdjacentHTML("beforeend", chip(v));
      b.el("to").value = "";
    });
    const button = (label: string) =>
      [...d.querySelectorAll('[role="button"]')].find((x: El) =>
        (x.getAttribute("aria-label") || "").startsWith(label)
      );
    button("Discard").addEventListener("click", () => {
      b.discards++;
      b.el("over").innerHTML = "";
    });
    button("Send").addEventListener("click", () => {
      const to = [...d.querySelectorAll("#chips [email]")].map((x: El) => x.getAttribute("email"));
      const subject = b.el("subj")?.value ?? "";
      const banner = () =>
        d.getElementById("over").insertAdjacentHTML(
          "beforeend",
          `<div role="alert">Message not sent. Your message could not be sent right now.</div>`,
        );
      if (o.send === "banner") return banner();
      b.el("over").innerHTML = "";
      if (o.send === "banner-closes") return banner();
      b.sent.push({ to, subject, attached: files.map((m) => m.id) });
      b.msgs.push({ ...msg(`s${b.sent.length}`, to[0] ?? "", subject, 2, "Sent") });
      if (o.send === "ndr") {
        b.msgs.push({
          ...msg("ndr", "mailer-daemon@googlemail.com", "Delivery Status Notification (Failure)", 2),
          snippet: `Your message wasn't delivered. Subject: ${subject}`,
        });
      }
      fill();
    });
  }
  return b;
}
