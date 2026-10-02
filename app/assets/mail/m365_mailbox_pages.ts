// A synthetic Outlook on the web mailbox for the Microsoft 365 mailbox scripts (plan T10.2b). It is
// modelled on the spikes' selectors (research note 6, S2): message rows are `role=option` in a
// `role=listbox`, each carrying `aria-setsize`; the sender element's `title` attribute is the address
// (the visible text is a display name or a bare address); the time element's `title` is the full date and
// time. Toolbar buttons are found by their names. The page is a model of what the scripts rely on, not a
// copy of Outlook; the live checks (T11, T18c) say whether the real page agrees. Every name, address and
// subject here is synthetic.
import { El, load, Page } from "./test_dom.ts";

export const OURS = "box-1@in.example.invalid";
export const NONCE = "bk7f3a9c2e41";

export type Folder = "Inbox" | "Junk Email" | "Deleted Items" | "Sent Items";

export interface Msg {
  id: string;
  /** The sender's address, or, in Sent Items, the recipient's. */
  who: string;
  /** What the row shows: a display name, or the bare address. */
  shown?: string;
  subject: string;
  /** The time element's title. */
  at: string;
  /** The day, as the search's `received>=` compares it. */
  day: string;
  folder: Folder;
}

export interface Opts {
  /** The folder the page is showing. */
  folder?: Folder;
  /** Rows drawn at once: the list is virtualised, `aria-setsize` says how many there are. */
  drawn?: number;
  /** What a Send does. */
  send?: "ok" | "banner" | "banner-closes" | "ndr";
  /** The Forward menu has no "Forward as attachment". */
  noAttachmentItem?: boolean;
  /** The compose window opens already addressed to someone else. */
  prefill?: string;
  /** "Forward as attachment" opens a compose with no attachment. */
  bare?: boolean;
  /** The page never answers the search. */
  deaf?: boolean;
}

export interface Sent {
  to: string[];
  subject: string;
  /** The id of the message attached, or null when the forward carried none. */
  attached: string | null;
}

export interface Box {
  page: Page;
  msgs: Msg[];
  sent: Sent[];
  queries: string[];
  deleted: string[];
  el(id: string): El;
  rows(): El[];
}

const when = (n: number) => `Thu 10/${n}/2026 3:04 PM`;

export function msg(id: string, who: string, subject: string, day: number, folder: Folder = "Inbox"): Msg {
  return {
    id,
    who,
    subject,
    at: when(day),
    day: `2026-10-${String(day).padStart(2, "0")}`,
    folder,
    shown: id.endsWith("a") ? who : who.split("@")[0].replace(/\W+/g, " "),
  };
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");

export function mailbox(all: Msg[], o: Opts = {}): Box {
  const folder = o.folder ?? "Inbox";
  const page = load(`
    <div role="tree" aria-label="Folders">
      ${
    ["Inbox", "Sent Items", "Junk Email"].map((f) =>
      `<div role="treeitem" aria-label="${f}" aria-selected="${f === folder}">${f}</div>`
    ).join("")
  }</div>
    <div role="search"><input type="search" id="q" aria-label="Search mail"><button type="button" id="go" aria-label="Search">Search</button></div>
    <div id="bar"><button type="button" id="fwd" aria-label="Forward" aria-haspopup="menu">Forward</button>
      <button type="button" id="del" aria-label="Delete">Delete</button></div>
    <div role="listbox" id="list" aria-label="Message list"></div><div id="over"></div>`);
  const d = page.document;
  const b: Box = {
    page,
    msgs: all.map((m) => ({ ...m })),
    sent: [],
    queries: [],
    deleted: [],
    el: (id) => d.getElementById(id),
    rows: () => [...d.querySelectorAll('[role="option"]')],
  };
  let selected: string | null = null;
  const fill = (shown: Msg[]) => {
    selected = null;
    const drawn = shown.slice(0, o.drawn ?? shown.length);
    const list = b.el("list");
    list.innerHTML = shown.length === 0 ? `<div role="status">No results</div>` : drawn.map((m, i) => `
      <div role="option" id="r-${m.id}" aria-selected="false" aria-setsize="${shown.length}" aria-posinset="${
      i + 1
    }">
        <span title="${esc(m.who)}">${esc(m.shown ?? m.who)}</span>
        <span title="${esc(m.subject)}">${esc(m.subject)}</span>
        <span title="${m.at}">3:04 PM</span><span>a preview of the body</span></div>`).join("");
    for (const row of b.rows()) {
      row.addEventListener("click", () => {
        for (const r of b.rows()) r.setAttribute("aria-selected", String(r === row));
        selected = row.getAttribute("id").slice(2);
      });
    }
  };
  const inFolder = () => b.msgs.filter((m) => m.folder === folder);
  fill(inFolder());

  b.el("go").addEventListener("click", () => {
    const q = b.el("q").value;
    b.queries.push(q);
    if (o.deaf) return;
    const since = /received>=(\d{4}-\d{2}-\d{2})/.exec(q)?.[1] ?? "0000-00-00";
    const out = [...q.matchAll(/NOT folder:"([^"]+)"/g)].map((m) => m[1]);
    setTimeout(() => fill(b.msgs.filter((m) => m.day >= since && !out.includes(m.folder))), 20);
  });
  b.el("del").addEventListener("click", () => {
    if (!selected) return;
    b.deleted.push(selected);
    b.msgs = b.msgs.filter((m) => m.id !== selected);
    b.el(`r-${selected}`)?.remove();
    selected = null;
  });
  b.el("fwd").addEventListener("click", () => {
    if (!selected) return;
    const attach = o.noAttachmentItem ? "" : `<div role="menuitem" id="m-att">Forward as attachment</div>`;
    b.el("over").innerHTML =
      `<div role="menu"><div role="menuitem" id="m-plain">Forward</div>${attach}</div>`;
    b.el("m-plain").addEventListener("click", () => compose(null));
    b.el("m-att")?.addEventListener("click", () => compose(selected));
  });

  const alertBanner = () => {
    d.getElementById("over").insertAdjacentHTML(
      "beforeend",
      `<div role="alert">We couldn't send your message. Try again later.</div>`,
    );
  };
  function compose(attached: string | null) {
    const m = b.msgs.find((x) => x.id === attached);
    b.el("over").innerHTML = `<div role="dialog" aria-label="Message compose">
      <input type="text" id="to" aria-label="To"><span id="pills">${
      o.prefill ? `<span title="${esc(o.prefill)}">${esc(o.prefill)}</span>` : ""
    }</span>
      <input type="text" id="subj" aria-label="Add a subject" value="FW: ${esc(m?.subject ?? "x")}">
      ${
      attached && !o.bare
        ? `<div id="att" aria-label="Attachment: ${esc(m?.subject ?? "")}.eml">${
          esc(m?.subject ?? "")
        }.eml</div>`
        : ""
    }
      <button type="button" aria-label="Send">Send</button><button type="button" aria-label="Discard">Discard</button></div>`;
    b.el("to").addEventListener("keydown", (e: El) => {
      const v = b.el("to").value.trim();
      if (e.key !== "Enter" || !v) return;
      b.el("pills").insertAdjacentHTML("beforeend", `<span title="${esc(v)}">${esc(v)}</span>`);
      b.el("to").value = "";
    });
    const send = [...d.querySelectorAll("button")].find((x: El) => x.getAttribute("aria-label") === "Send");
    send.addEventListener("click", () => {
      const to = [...d.querySelectorAll("#pills [title]")].map((x: El) => x.getAttribute("title"));
      const subject = b.el("subj")?.value ?? "";
      if (o.send === "banner") return alertBanner();
      b.el("over").innerHTML = "";
      if (o.send === "banner-closes") return alertBanner();
      b.sent.push({ to, subject, attached });
      b.msgs.push({ ...msg(`s${b.sent.length}`, to[0] ?? "", "sent", 2, "Sent Items") });
      if (o.send === "ndr") {
        b.msgs.push(
          msg("ndr", "postmaster@school.example.invalid", `Undeliverable: ${b.sent.at(-1)!.subject}`, 2),
        );
        if (folder === "Inbox") fill(inFolder());
      }
    });
  }
  return b;
}
