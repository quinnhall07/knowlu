// A synthetic Gmail Settings > Forwarding and POP/IMAP page for the Gmail forwarding scripts (plan T10.3a,
// spec test 35). Modelled on the spikes' selectors (research note 6): Gmail's buttons are
// `<input type=button value=...>`, and the section sometimes does not draw until a hash round trip. The page
// is a small model: what a control shows is drawn from `model`, a click edits the controls, and Save commits
// them, so a hash round trip re-draws what was saved and drops what was not. No address here is real.
import { disable, El, load, Page } from "./test_dom.ts";

export const SECTION = "#settings/fwdandpop";

export interface Model {
  /** The confirmed forwarding addresses, in the list's order. */
  addresses: string[];
  on: boolean;
  /** The address forwarding goes to while `on`. */
  to: string;
  disp: "keep" | "read" | "archive" | "trash";
}

export interface Opts extends Partial<Model> {
  /** Where the page is when the script starts; the section's own hash when absent. */
  at?: string;
  /** The section does not draw at first: until a hash round trip, until a reload, or never. */
  lateDraw?: "hash" | "reload" | "never";
  /** A control the page lacks. */
  without?: "add" | "radios" | "to" | "disp" | "save" | "remove" | "next" | "ok" | "email";
  /** After Next, the dialog asks "Proceed" before the popup opens. */
  proceed?: boolean;
  /** Save never settles: the button stays enabled. */
  sticky?: boolean;
  /** The add dialog's address box has no type attribute (T18c's audit: a type-less input reads as text). */
  untypedEmail?: boolean;
  /**
   * Live 2026-10-18: a Gmail with no forwarding address yet draws only the Add button, no forwarding radios
   * and no Remove button (the POP section is still there).
   */
  fresh?: boolean;
  /** An address added but not yet verified: it is not an option of the list; the page shows a verify text. */
  unverified?: string;
  /**
   * The older markup, where each radio sits inside a wrapping <label> in one block. The live page is the
   * default: a table row per radio, the "Forward a copy" radio with no label of its own.
   */
  labelled?: boolean;
}

export interface Gmail {
  page: Page;
  model: Model;
  saves: number;
  /** Addresses typed into the add dialog and sent with Next, in order. */
  typed: string[];
  popups: number;
  oks: number;
  reloads: number;
  hashes: string[];
  /** Addresses the page's Remove button was confirmed on, in order. */
  removed: string[];
  /** Clicks on look-alike controls that are not input buttons. */
  decoy: number;
  /** Clicks on any control the page draws, enabled or not: a script that must click nothing leaves it 0. */
  clicks: number;
  /** Google's verify popup closes and the opener shows its "confirmation link sent" dialog. */
  verified(): void;
  el(id: string): El;
  snap(): { on: boolean; to: string; disp: string; addresses: string[] };
}

const DISP: Record<string, string> = {
  keep: "keep Gmail's copy in the Inbox",
  read: "mark Gmail's copy as read",
  archive: "archive Gmail's copy",
  trash: "delete Gmail's copy",
};

export function gmail(o: Opts = {}): Gmail {
  const model: Model = {
    addresses: [...(o.addresses ?? [])],
    on: o.on ?? false,
    to: o.to ?? "",
    disp: o.disp ?? "keep",
  };
  const page = load(`<div id="root"></div>`);
  const d = page.document;
  const g: Gmail = {
    page,
    model,
    saves: 0,
    typed: [],
    popups: 0,
    oks: 0,
    reloads: 0,
    hashes: [],
    removed: [],
    decoy: 0,
    clicks: 0,
    verified: () =>
      dialog(
        `<p>Confirmation link sent</p><p>A confirmation link was sent to the address.</p>${
          input("OK", "ok")
        }`,
      ),
    el: (id) => d.getElementById(id),
    snap: () => ({ on: model.on, to: model.to, disp: model.disp, addresses: [...model.addresses] }),
  };
  const w = o.without;
  const input = (
    value: string,
    id: string,
  ) => (w && id === w ? "" : `<input type="button" value="${value}" id="${id}">`);
  let hash = o.at ?? SECTION;
  let drawn = !o.lateDraw;
  let dirty = false;
  let pendingRemove = "";
  const wired = new WeakSet();

  const options = () =>
    model.addresses.map((a) => `<option value="${a}"${a === model.to ? " selected" : ""}>${a}</option>`).join(
      "",
    );
  const dispOptions = () =>
    Object.entries(DISP).map(([k, t]) =>
      `<option value="${k}"${k === model.disp ? " selected" : ""}>${t}</option>`
    ).join("");
  const section = () => {
    const none = model.addresses.length === 0 ? " disabled" : "";
    const live = !o.labelled;
    const rows = w === "radios" || o.fresh ? "" : live
      // The live shape: each radio in its own first TD. "Disable forwarding" has a label[for]; the "Forward a
      // copy" radio has none, its text sits in the next TD of the row, which also holds both selects.
      ? `<tr><td><input type="radio" name="sx_em" id="r-off"${
        model.on ? "" : " checked"
      }></td><td><label for="r-off">Disable forwarding</label></td></tr>
        <tr><td><input type="radio" name="sx_em" id="r-on"${model.on ? " checked" : ""}${none}></td><td>
          Forward a copy of incoming mail to
          ${w === "to" ? "" : `<select id="to"${none}>${options()}</select>`}
          and ${w === "disp" ? "" : `<select id="disp"${none}>${dispOptions()}</select>`}</td></tr>`
      : `<tr><td>&nbsp;</td><td><div><label><input type="radio" name="fwd" id="r-off"${
        model.on ? "" : " checked"
      }> Disable forwarding</label></div>
        <div><label><input type="radio" name="fwd" id="r-on"${
        model.on ? " checked" : ""
      }${none}> Forward a copy of incoming mail to</label>
          ${w === "to" ? "" : `<select id="to"${none}>${options()}</select>`}
          and ${w === "disp" ? "" : `<select id="disp"${none}>${dispOptions()}</select>`}</div></td></tr>`;
    const verify = o.unverified
      ? `<div>A verification email was sent to ${o.unverified}. It is not yet verified.</div>`
      : "";
    return `<div id="fwdandpop"><table>
      <tr><td>Forwarding:</td><td>
        ${input("Add a forwarding address", "add")}${
      o.fresh ? "" : input("Remove forwarding address", "remove")
    }
        ${verify}
      </td></tr>
      ${rows}
      <tr><td>POP download:</td><td><label><input type="radio" name="pop" id="p-off" checked> Disable POP</label>
        <label><input type="radio" name="pop" id="p-on"> Enable POP for all mail</label></td></tr>
      <tr><td></td><td>When messages are accessed with POP
        <select id="pop-disp">${dispOptions()}</select></td></tr></table>
      ${w === "save" ? "" : `<button id="save" type="button" disabled>Save Changes</button>`}
      <button id="cancel" type="button">Cancel</button></div>
      <button id="decoy-next" type="button">Next</button>
      <div role="button" id="decoy-add">Add a forwarding address</div>`;
  };
  const dialog = (html: string) => {
    g.el("dlg")?.remove();
    d.getElementById("root").insertAdjacentHTML("beforeend", `<div role="dialog" id="dlg">${html}</div>`);
    wire();
  };
  const closeDialog = () => g.el("dlg")?.remove();

  const wire = () => {
    // Each element is wired once: a re-wire after a dialog opens must not stack a second handler.
    const fresh = (el: El) => {
      if (!el || wired.has(el)) return null;
      wired.add(el);
      return el;
    };
    const on = (id: string, fn: () => void) =>
      fresh(g.el(id))?.addEventListener("click", () => {
        g.clicks++;
        if (g.el(id).hasAttribute("disabled")) return;
        fn();
      });
    const edit = () => {
      dirty = true;
      sync();
    };
    for (const id of ["r-off", "r-on"]) {
      on(id, () => {
        g.el("r-off").checked = id === "r-off";
        g.el("r-on").checked = id === "r-on";
        edit();
      });
    }
    for (const id of ["to", "disp"]) fresh(g.el(id))?.addEventListener("change", edit);
    on("decoy-next", () => g.decoy++);
    on("decoy-add", () => g.decoy++);
    on(
      "add",
      () =>
        dialog(
          `<p>Add a forwarding address</p>${
            w === "email" ? "" : o.untypedEmail ? `<input id="email">` : `<input type="email" id="email">`
          }
      ${input("Next", "next")}${input("Cancel", "cancel-dlg")}`,
        ),
    );
    on("next", () => {
      g.typed.push(g.el("email")?.value ?? "");
      if (o.proceed) {
        dialog(
          `<p>A confirmation will be sent to the address. Proceed?</p>${input("Proceed", "proceed")}${
            input("Cancel", "cancel-dlg")
          }`,
        );
      } else {
        g.popups++;
      }
    });
    on("proceed", () => g.popups++);
    on("cancel-dlg", closeDialog);
    on("ok", () => {
      g.oks++;
      closeDialog();
    });
    on("save", () => {
      g.saves++;
      model.on = !!g.el("r-on").checked;
      if (model.on && g.el("to")) model.to = g.el("to").value;
      if (g.el("disp")) model.disp = g.el("disp").value;
      dirty = o.sticky ? dirty : false;
      sync();
    });
    on("remove", () => {
      pendingRemove = g.el("to")?.value ?? "";
      dialog(
        `<p>Are you sure you want to remove this forwarding address?</p>${input("OK", "confirm-remove")}${
          input("Cancel", "cancel-dlg")
        }`,
      );
    });
    on("confirm-remove", () => {
      g.removed.push(pendingRemove);
      model.addresses = model.addresses.filter((a) => a !== pendingRemove);
      if (model.to === pendingRemove) Object.assign(model, { on: false, to: model.addresses[0] ?? "" });
      render();
    });
    sync();
  };
  const sync = () => {
    if (g.el("save")) disable(g.el("save"), !dirty);
  };
  // linkedom's <option>.selected and <select>.value are not a browser's: give the page's own.
  const browserSelects = () => {
    for (const r of d.querySelectorAll('input[type="radio"]')) r.checked = r.hasAttribute("checked");
    for (const sel of d.querySelectorAll("select")) {
      for (const opt of sel.options) {
        Object.defineProperty(opt, "selected", {
          configurable: true,
          get: () => opt.hasAttribute("selected"),
          set: (on: boolean) => on ? opt.setAttribute("selected", "") : opt.removeAttribute("selected"),
        });
      }
      Object.defineProperty(sel, "value", {
        configurable: true,
        get: () => ([...sel.options].find((o: El) => o.selected) ?? sel.options[0])?.value ?? "",
        set: (v: string) => {
          for (const o of sel.options) o.selected = o.value === v;
        },
      });
    }
  };
  const render = () => {
    dirty = false;
    d.getElementById("root").innerHTML = hash !== SECTION
      ? `<div>Inbox</div>`
      : drawn
      ? section()
      : `<h2>Settings</h2><div>Loading...</div>`;
    browserSelects();
    wire();
  };

  const loc = {
    host: "mail.google.com",
    get hash() {
      return hash;
    },
    set hash(v: string) {
      const was = hash;
      hash = v;
      g.hashes.push(v);
      if (v === SECTION && was !== SECTION && o.lateDraw === "hash") drawn = true;
      render();
    },
    reload() {
      g.reloads++;
      if (o.lateDraw === "reload") drawn = true;
      render();
    },
  };
  page.location = loc;
  render();
  return g;
}
