// A synthetic Gmail mailbox for the Gmail mailbox scripts (plan T10.3b). It is modelled on the spikes'
// selectors (research note 6, S2): the list is a `table[role=grid]` of `tr[role=row]`, each with a
// `div[role=checkbox]`, the sender as a `span[email]` whose `email` attribute is the address (the visible
// text is a display name or the address), the subject as a `span.bog`, and the time as a `span[title]`
// whose title is the full date and time. A conversation row draws its message count as digits alone in a
// `span.bx0` after the sender names ("Ana, Bo 2"). The toolbar has Delete and "More email options".
// Setting `location.hash` to `#search/<query>` (the query percent-encoded, spaces as plus signs) runs a
// search, as live; the search box is labelled "Ask Gmail" and does nothing here (live 2026-10-18).
// As observed live over CDP (2026-10-18/19, WebView2 155):
// - A view is its range text, its pager and its grid. A search keeps the previous view in the document,
//   hidden (`display: none`, so `offsetParent` is null), and draws its own view after it.
// - A folder's range is exact, "1–50 of 2,819", with Newest/Newer/Older/Oldest. A search's reads "of many"
//   on every page but the last, which is exact ("1,451–1,493 of 1,493"), with "Previous results", "Next
//   results" and "Show more messages"; the button at its end is aria-disabled. A page moves the hash to
//   `.../p2`.
// - Pager buttons, Delete, More, Send, Discard, "Expand all" and the conversation's titled back button answer
//   a press (a mousedown then a mouseup), never `click()`. A menu item answers only a mousedown and a click
//   carrying a point inside its box. A row and a row's checkbox answer `click()`.
// - An attachment chip is a `div.dL`: "Uploading of attachment: <name>" while it uploads, "... failed.
//   Press enter to retry" when it fails, "Attachment: <name>" when it is attached, its size a leaf inside it.
// The page is a model of what the scripts rely on, not a copy of Gmail; the live checks (T11, T18c) say
// whether the real page agrees. Every name, address and subject here is synthetic.
import { atPoint, El, load, Page } from "./test_dom.ts";

export const OURS = "box-1@in.example.invalid";
export const NONCE = "bk7f3a9c2e41";
/** The signed-in account the synthetic page's account chip names. */
export const SELF = "me.student@school.example.invalid";

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
  /** The day, as the search's `after:` and `before:` compare it (`before:` excludes its own day). */
  day: string;
  folder: Folder;
  /** The size of the message as an attachment, in bytes. */
  size: number;
  /** Other participants of a conversation: the row lists them too. */
  also?: string[];
  /** The message count Gmail draws after the sender names, digits alone in a `span.bx0`: "Name 3". */
  count?: number;
  /** The conversation's messages, oldest first, newest last (the row is the newest). Opening the row draws
   * them; a conversation of more than two shows only the first and the newest until "Expand all". */
  convo?: { who: string; at: string }[];
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
  /** The To line draws no recipient for the first address committed to it (live, ef28 2026-10-19: none on a
   * page's first forward): its first Enter is taken and the text kept ("enter"), or the text taken and no
   * recipient drawn ("lost"); or the field is drawn again when the attachments go in, the one found before gone
   * ("redrawn", with `upload`); "never": no recipient is ever drawn. */
  toMiss?: "enter" | "lost" | "redrawn" | "never";
  /** "Forward as attachment" opens a compose with no attachment. */
  bare?: boolean;
  /** The compose carries one attachment fewer than the rows selected. */
  short?: boolean;
  /** The page never answers the search. */
  deaf?: boolean;
  /** The compose offers to send Google Drive links, whatever the sizes shown. */
  driveOffer?: boolean;
  /** The folder pager has no Oldest button. */
  noOldest?: boolean;
  /** The pager's buttons take a press and draw nothing. */
  stuckPager?: boolean;
  /** Attachments upload before they attach ("slow"), the upload fails ("fails"), or the compose draws no chip
   * at first and the attached chips a moment later ("late", live 2026-10-19). */
  upload?: "slow" | "fails" | "late";
  /** How long an upload takes, in ms (default 100). */
  uploadMs?: number;
  /** The compose draws no chip at first, then one attached chip every this many ms (live 2026-10-19). */
  staggerMs?: number;
  /** One more chip than the rows', still uploading and never finishing, beside the attached ones. */
  stuckChip?: boolean;
  /** The message with this id never draws a chip (live, ef25 2026-10-19: six rows, five attached). */
  noChip?: string;
  /** Discard closes the compose this many ms after it is pressed (default: at once). */
  discardMs?: number;
  /** Discard is pressed and the compose never closes. */
  stuckDiscard?: boolean;
  /** A compose is already open when the script starts (a failed try's draft that did not go). */
  openCompose?: boolean;
  /** A compose already open holding this recipient and subject (an earlier try's draft of Knowlu's, or not). */
  leftover?: { to?: string; subject?: string };
  /** The compose already open is hidden (`display: none`): it is not drawn. */
  hiddenCompose?: boolean;
  /** A dialog of Google's own is drawn first in the page, with a "Send feedback" button and no To or Subject. */
  feedback?: boolean;
  /** The row of `id` shows the message `as` once it is ticked. */
  swap?: { id: string; as: string };
  /** The page shows no "1–50 of 123" range. */
  blind?: boolean;
  /** After a search the range text updates later than the rows, the previous view's range standing till then. */
  slowRange?: boolean;
  /** "Forward as attachment" on a conversation row attaches only its newest message. */
  newestOnly?: boolean;
  /** An opened conversation has no "Back to ..." button. */
  noBack?: boolean;
  /** "Expand all" draws nothing more. */
  stuck?: boolean;
  /** The account chip names this address (default `SELF`); `null` draws no chip. */
  account?: string | null;
  /** The toolbar draws Delete only while a row is ticked (live 2026-10-19). */
  deleteOnSelect?: boolean;
  /** With `deleteOnSelect`, Delete is drawn this many ms after the tick. */
  deleteAfterMs?: number;
  /** `swap` happens this many ms after the tick, not at once. */
  swapAfterMs?: number;
  /** The first view's rows draw this many ms after the page opens (live 2026-10-19, Sent after a pass). */
  lateRowsMs?: number;
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
  /** Presses of the pager's enabled buttons (live, each one is a page load), whether or not the page moved. */
  presses: number;
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
const num = (v: number) => v.toLocaleString("en-US");

/** A Gmail button: it acts on a mousedown then a mouseup (a press), and `click()` alone does nothing. */
export function onPress(el: El, act: () => void) {
  let armed = false;
  el.addEventListener("mousedown", (e: El) => (armed = (e.button ?? 0) === 0));
  el.addEventListener("mouseup", () => {
    if (!armed) return;
    armed = false;
    act();
  });
}

/** A Gmail menu item: it acts only on a mousedown and then a click that carry a point inside its box. */
function onPoint(el: El, act: () => void) {
  let armed = false;
  el.addEventListener("mousedown", (e: El) => (armed = atPoint(el, e)));
  el.addEventListener("click", (e: El) => {
    if (!armed || !atPoint(el, e)) return;
    armed = false;
    act();
  });
}

type Kind = "inbox" | "sent" | "search";
const PAGER: Record<Kind, [string, string][]> = {
  inbox: [["newest", "Newest"], ["newer", "Newer"], ["older", "Older"], ["oldest", "Oldest"]],
  sent: [["newest", "Newest"], ["newer", "Newer"], ["older", "Older"], ["oldest", "Oldest"]],
  search: [["newer", "Previous results"], ["older", "Next results"], ["first", "Show more messages"]],
};
const BACK: Record<Kind, string> = {
  inbox: "Back to Inbox",
  sent: "Back to Sent Mail",
  search: "Back to Search Results",
};

export function mailbox(all: Msg[], o: Opts = {}): Box {
  const size = o.size ?? 50;
  const account = o.account === undefined ? SELF : o.account;
  // Gmail's account chip, as read live 2026-10-18: "Google Account: <name> (<address>), Google membership".
  // The address is in parentheses but does not end the label.
  const chip = account === null
    ? ""
    : `<a href="#account" aria-label="Google Account: Student Name (${account}), Google membership">S</a>`;
  const page = load(`
    ${chip}
    <div role="navigation"><a href="#inbox">Inbox</a><a href="#sent">Sent</a></div>
    <div role="search"><input type="text" id="q" name="q" aria-label="Ask Gmail"></div>
    <div role="toolbar" id="bar">
      <div role="button" id="del" aria-label="Delete" data-tooltip="Delete">Delete</div>
      <div role="button" id="more" aria-label="More email options" data-tooltip="More email options"
        aria-haspopup="false">More</div>
    </div>
    <div id="views"></div><div id="over"></div>
    <div role="main" id="convo"></div>`);
  const d = page.document;
  // Setting the hash to `#search/...` is a search, as live (2026-10-18); setting it to what it already is
  // does nothing, as in a browser (no hashchange). Paging moves the hash to `<view>/p<n>` without a search.
  let base = o.view === "sent" ? "#sent" : "#inbox";
  let hash = base;
  const loc = {
    get hash() {
      return hash;
    },
    set hash(v: string) {
      if (v === hash) return;
      hash = v;
      onHash(v);
    },
    host: "mail.google.com",
    reload() {},
  };
  page.location = loc;
  // The view showing: the last view drawn (earlier ones stay in the document, hidden).
  const cur = () => [...d.querySelectorAll("#views > .vw")].pop();
  const b: Box = {
    page,
    msgs: all.map((m) => ({ ...m })),
    sent: [],
    queries: [],
    deleted: [],
    discards: 0,
    presses: 0,
    // "grid" and "rangebox" are the showing view's; every other id is the page's own.
    el: (id) =>
      id === "grid"
        ? cur()?.querySelector('table[role="grid"]') ?? null
        : id === "rangebox"
        ? cur()?.querySelector(".rangebox") ?? null
        : d.getElementById(id),
    rows: () => [...(cur()?.querySelectorAll('tr[role="row"]') ?? [])],
    checked: () =>
      [...(cur()?.querySelectorAll('tr[role="row"] [role="checkbox"][aria-checked="true"]') ?? [])].map((
        c: El,
      ) => c.closest("tr").getAttribute("data-id")),
    range: () => (b.el("rangebox")?.textContent || "").replace(/\s+/g, " ").trim(),
  };
  let view: { kind: Kind; since: string; until: string; out: string[] } = {
    kind: o.view ?? "inbox",
    since: "",
    until: "",
    out: [],
  };
  let at = 0;
  let swapped = false;
  // A new view: the one showing is hidden and kept, as live, and the new one draws after it.
  const newView = () => {
    const was = cur();
    if (was) was.setAttribute("style", "display: none");
    const buttons = PAGER[view.kind]
      .filter(([go]) => !(o.noOldest && go === "oldest"))
      .map(([go, label]) =>
        `<div role="button" data-go="${go}" aria-label="${label}" data-tooltip="${label}">${label}</div>`
      ).join("");
    d.getElementById("views").insertAdjacentHTML(
      "beforeend",
      `<div class="vw"><div class="pager"><span class="rangebox"></span>${buttons}</div>
        <table role="grid"><tbody></tbody></table></div>`,
    );
    const v = cur();
    const pager = (go: string, to: () => number) => {
      const btn = v.querySelector(`[data-go="${go}"]`);
      if (!btn) return;
      onPress(btn, () => {
        if (btn.getAttribute("aria-disabled") === "true") return;
        b.presses++;
        if (o.stuckPager) return;
        at = to();
        hash = at === 0 ? base : `${base}/p${at + 1}`;
        setTimeout(fill, 20);
      });
    };
    pager("newest", () => 0);
    pager("newer", () => at - 1);
    pager("older", () => at + 1);
    pager("oldest", () => pages() - 1);
  };
  const shown = () => {
    const keep = (m: Msg) =>
      view.kind === "inbox"
        ? m.folder === "Inbox"
        : view.kind === "sent"
        ? m.folder === "Sent"
        : m.day >= view.since && (view.until === "" || m.day < view.until) && !view.out.includes(m.folder);
    return b.msgs.filter(keep).sort((x, y) => (x.day < y.day ? 1 : x.day > y.day ? -1 : 0));
  };
  const pages = () => Math.max(1, Math.ceil(shown().length / size));
  const fill = () => {
    const v = cur();
    const all = shown();
    at = Math.min(at, pages() - 1);
    const rows = all.slice(at * size, at * size + size);
    v.querySelector("tbody").innerHTML = all.length === 0
      ? `<tr><td>No messages matched your search.</td></tr>`
      : rows.map((m) => `
      <tr role="row" data-id="${m.id}"><td><div role="checkbox" aria-checked="false"></div></td>
        <td><span email="${esc(m.who)}">${esc(m.shown ?? m.who)}</span>${
        (m.also ?? []).map((a) => `, <span email="${esc(a)}">${esc(a.split("@")[0])}</span>`).join("")
      }${m.count ? ` <span class="bx0">${m.count}</span>` : ""}</td>
        <td><span class="bog">${esc(m.subject)}</span><span class="y2"> - ${
        esc(m.snippet ?? "a preview")
      }</span></td>
        <td class="xW"><span title="${m.at}">Oct ${Number(m.day.slice(8))}</span></td></tr>`).join("");
    const n = (x: number) => `<span class="ts">${num(x)}</span>`;
    // A search's total is "many" until its last page, as live.
    const last = at >= pages() - 1;
    const of = view.kind === "search" && !last ? `<span class="ts">many</span>` : n(all.length);
    const drawRange = () => {
      v.querySelector(".rangebox").innerHTML = all.length === 0 || o.blind
        ? ""
        : `<span class="Dj">${n(at * size + 1)}–${n(at * size + rows.length)} of ${of}</span>`;
    };
    // `slowRange`: after a search the rows redraw first and the range text follows.
    if (o.slowRange && view.kind === "search") setTimeout(drawRange, 80);
    else drawRange();
    for (const [go, off] of [["newest", at === 0], ["newer", at === 0], ["older", last], ["oldest", last]]) {
      v.querySelector(`[data-go="${go}"]`)?.setAttribute("aria-disabled", String(off));
    }
    for (const tr of v.querySelectorAll('tr[role="row"]')) {
      tr.addEventListener("click", (e: El) => {
        if (e.target?.closest?.('[role="checkbox"]')) return;
        open(tr.getAttribute("data-id"));
      });
    }
    for (const c of v.querySelectorAll('[role="checkbox"]')) {
      c.addEventListener("click", () => {
        c.setAttribute("aria-checked", String(c.getAttribute("aria-checked") !== "true"));
        showDelete();
        const row = c.closest("tr");
        const to = o.swap && row.getAttribute("data-id") === o.swap.id && !swapped &&
          b.msgs.find((m) => m.id === o.swap!.as);
        if (!to) return;
        // The page draws another message in this row the moment it is ticked (or a moment later).
        swapped = true;
        // The row is that message now: a Delete takes it.
        const draw = () => {
          row.setAttribute("data-id", to.id);
          row.querySelector("span[email]").setAttribute("email", to.who);
          row.querySelector(".bog").textContent = to.subject;
        };
        if (o.swapAfterMs) setTimeout(draw, o.swapAfterMs);
        else draw();
      });
    }
  };
  // Opening a conversation row: the list hides and the conversation draws, first and newest only when it
  // has more than two messages, until "Expand all" (two show "Collapse all"); the back button, titled and
  // with no aria-label, returns to the list.
  function open(id: string) {
    const m = b.msgs.find((x) => x.id === id);
    if (!m?.convo) return;
    const list = b.el("grid");
    list.setAttribute("hidden", "");
    const all = m.convo;
    const draw = (full: boolean) => {
      const shownMsgs = full || all.length <= 2 ? all : [all[0], all[all.length - 1]];
      const toggle = all.length > 2 && !full
        ? `<div role="button" id="expand" aria-label="Expand all" data-tooltip="Expand all">Expand all</div>`
        : `<div role="button" id="collapse" aria-label="Collapse all">Collapse all</div>`;
      b.el("convo").innerHTML = `<h2 class="hP">${esc(m.subject)}</h2>${toggle}
        ${
        shownMsgs.map((x, i) =>
          `<div data-message-id="${m.id}-${i}"><span class="gD" email="${esc(x.who)}">${
            esc(x.who.split("@")[0])
          }</span><span class="g3" title="${x.at}">Oct</span></div>`
        ).join("")
      }
        ${
        o.noBack
          ? ""
          : `<div role="button" id="back" title="${BACK[view.kind]}"><div class="asa"></div></div>`
      }`;
      if (b.el("expand")) onPress(b.el("expand"), () => !o.stuck && draw(true));
      if (b.el("back")) {
        onPress(b.el("back"), () => {
          b.el("convo").innerHTML = "";
          list.removeAttribute("hidden");
        });
      }
    };
    draw(false);
  }
  // Live (2026-10-19) Delete is drawn only while a row is ticked.
  function showDelete() {
    if (!o.deleteOnSelect) return;
    const style = b.checked().length > 0 ? "" : "display: none";
    if (style === "" && o.deleteAfterMs) setTimeout(() => b.el("del").setAttribute("style", style), o.deleteAfterMs);
    else b.el("del").setAttribute("style", style);
  }
  newView();
  if (o.lateRowsMs) setTimeout(fill, o.lateRowsMs);
  else fill();
  showDelete();

  // The box does nothing here: live, "Ask Gmail" may go to Gemini, so no script types into it. A search
  // keeps the view it leaves, hidden, and draws its own after it, a moment later.
  function onHash(h: string) {
    const hit = /^#search\/([^/]*)/.exec(h);
    if (!hit) return;
    const q = decodeURIComponent(hit[1].replace(/\+/g, " "));
    b.queries.push(q);
    if (o.deaf) return;
    const since = /after:(\d{4})\/(\d{2})\/(\d{2})/.exec(q);
    const until = /before:(\d{4})\/(\d{2})\/(\d{2})/.exec(q);
    const out = [["spam", "Spam"], ["trash", "Trash"], ["sent", "Sent"]].filter(([k]) =>
      q.includes(`-in:${k}`)
    );
    setTimeout(() => {
      view = {
        kind: "search",
        since: since ? `${since[1]}-${since[2]}-${since[3]}` : "0000-00-00",
        until: until ? `${until[1]}-${until[2]}-${until[3]}` : "",
        out: out.map((x) => x[1]),
      };
      base = `#search/${hit[1]}`;
      at = 0;
      newView();
      fill();
    }, 20);
  }
  onPress(b.el("del"), () => {
    const ids = b.checked();
    if (ids.length === 0) return;
    for (const m of b.msgs) if (ids.includes(m.id)) m.folder = "Trash";
    b.deleted.push(...ids);
    fill();
    showDelete();
  });
  onPress(b.el("more"), () => {
    if (b.checked().length === 0) return;
    const item = o.noAttachmentItem
      ? ""
      : `<div role="menuitem" id="m-att"><div class="J-N-Jz">Forward as attachment</div></div>`;
    b.el("over").innerHTML = `<div role="menu"><div role="menuitem">Mark as read</div>${item}</div>`;
    if (b.el("m-att")) onPoint(b.el("m-att"), () => compose(b.checked()));
  });

  function compose(ids: string[]) {
    const picked = b.msgs.filter((m) => ids.includes(m.id));
    // A conversation row attaches every message in it (or only the newest, `newestOnly`).
    const attach = picked.flatMap((m) =>
      m.convo && !o.newestOnly ? m.convo.map((_, i) => ({ ...m, id: `${m.id}#${i}` })) : [m]
    );
    const files = o.bare ? [] : o.short ? attach.slice(1) : attach.filter((m) => m.id !== o.noChip);
    const total = files.reduce((n, m) => n + m.size, 0);
    // Live 2026-10-19: an accepted recipient is a `div[role=option]` naming the address in data-hovercard-id.
    const chip = (v: string) => `<div role="option" data-hovercard-id="${esc(v)}"><span>${esc(v)}</span></div>`;
    // Live chips: the name and the size are leaves inside the chip, "<name>.eml (6K)".
    const attached = (m: Msg) =>
      `<div class="dL" aria-label="Attachment: ${esc(m.subject)}.eml"><a class="dO"><div class="vI">${
        esc(m.subject)
      }.eml</div><div class="SaH2Ve">(${sized(m.size)})</div></a></div>`;
    const uploading = (m: Msg) =>
      `<div class="dL" aria-label="Uploading of attachment: ${esc(m.subject)}.eml"><div class="vI">${
        esc(m.subject)
      }.eml</div></div>`;
    const failed = (m: Msg) =>
      `<div class="dL" aria-label="Uploading of attachment: ${
        esc(m.subject)
      }.eml failed. Press enter to retry"><div class="vI">${
        esc(m.subject)
      }.eml</div> Attachment failed. <span>Retry</span> <span>Help</span></div>`;
    b.el("over").innerHTML = `<div role="dialog" aria-label="New Message">
      <div id="chips">${o.prefill ? chip(o.prefill) : ""}</div>
      <input type="text" id="to" aria-label="To recipients">
      <input type="text" id="subj" name="subjectbox" aria-label="Subject" value="Fwd: ${
      esc(picked[0]?.subject ?? "")
    }">
      <div id="files">${o.upload === "late" || o.staggerMs ? "" : files.map(o.upload ? uploading : attached).join("")}${
        o.stuckChip && files[0] ? uploading(files[0]) : ""
      }</div>
      ${
      o.driveOffer || total > LIMIT
        ? `<div role="alertdialog">The files you attached are over 25 MB. Send them as Google Drive links?</div>`
        : ""
    }
      <div role="button" aria-label="Send ‪(Ctrl-Enter)‬" data-tooltip="Send">Send</div>
      <div role="button" aria-label="Discard draft ‪(Ctrl-Shift-D)‬">Discard</div></div>`;
    // Enter on the To line draws the typed address as a recipient (`toMiss`: not the first time, or never).
    let misses = o.toMiss === "enter" || o.toMiss === "lost" ? 1 : 0;
    const accepts = (input: El) =>
      input.addEventListener("keydown", (e: El) => {
        const v = input.value.trim();
        // A field drawn over is not the page's any more: nothing typed into it reaches the compose.
        if (e.key !== "Enter" || !v || o.toMiss === "never" || !d.contains(input)) return;
        if (misses > 0) {
          misses--;
          if (o.toMiss === "lost") input.value = "";
          return;
        }
        b.el("chips").insertAdjacentHTML("beforeend", chip(v));
        input.value = "";
      });
    accepts(b.el("to"));
    const redrawTo = () => {
      if (o.toMiss !== "redrawn" || !b.el("to")) return;
      const fresh = d.createElement("input");
      for (const [k, v] of [["type", "text"], ["id", "to"], ["aria-label", "To recipients"]]) {
        fresh.setAttribute(k, v);
      }
      b.el("to").replaceWith(fresh);
      accepts(fresh);
    };
    if (o.staggerMs) {
      files.forEach((m, i) =>
        setTimeout(() => b.el("files")?.insertAdjacentHTML("beforeend", attached(m)), o.staggerMs! * (i + 1))
      );
    } else if (o.upload) {
      setTimeout(() => {
        const box = b.el("files");
        if (box) box.innerHTML = files.map(o.upload === "fails" ? failed : attached).join("");
        redrawTo();
      }, o.uploadMs ?? 100);
    }
    const button = (label: string) =>
      [...b.el("over").querySelectorAll('[role="button"]')].find((x: El) =>
        (x.getAttribute("aria-label") || "").startsWith(label)
      );
    onPress(button("Discard"), () => {
      b.discards++;
      if (o.stuckDiscard) return;
      if (o.discardMs) setTimeout(() => (b.el("over").innerHTML = ""), o.discardMs);
      else b.el("over").innerHTML = "";
    });
    onPress(button("Send"), () => {
      const to = [...d.querySelectorAll('#chips [role="option"]')].map((x: El) => x.getAttribute("data-hovercard-id"));
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
      // Live 2026-10-19: after a send the rows that were forwarded stay ticked.
      const kept = b.checked();
      fill();
      for (const id of kept) {
        cur()?.querySelector(`tr[role="row"][data-id="${id}"] [role="checkbox"]`)?.setAttribute("aria-checked", "true");
      }
    });
  }
  if (o.openCompose || o.leftover || o.hiddenCompose) compose([]);
  if (o.leftover?.to) {
    b.el("chips").insertAdjacentHTML(
      "beforeend",
      `<div role="option" data-hovercard-id="${esc(o.leftover.to)}"><span>${esc(o.leftover.to)}</span></div>`,
    );
  }
  if (o.leftover?.subject !== undefined) b.el("subj").value = o.leftover.subject;
  if (o.hiddenCompose) b.el("over").firstElementChild.setAttribute("style", "display: none");
  if (o.feedback) {
    d.body.insertAdjacentHTML(
      "afterbegin",
      `<div role="dialog" aria-label="Send feedback to Google"><textarea aria-label="Describe your issue"></textarea>
        <div role="button" aria-label="Send feedback">Send feedback</div></div>`,
    );
  }
  return b;
}

/**
 * A non-delivery report as Gmail draws it in the Inbox: from "Mail Delivery Subsystem", titled "Delivery Status
 * Notification (Failure)", its preview "Address not found Your message wasn't delivered to <address> because the
 * address couldn't be found" (live 2026-10-19: the report carries the address, not the sent subject).
 */
export function report(id: string, day: number, over: Partial<Msg> = {}): Msg {
  return {
    ...msg(id, "mailer-daemon@mail.example.invalid", "Delivery Status Notification (Failure)", day),
    shown: "Mail Delivery Subsystem",
    snippet:
      `Address not found Your message wasn't delivered to ${OURS} because the address couldn't be found`,
    ...over,
  };
}
