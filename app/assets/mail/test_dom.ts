// The DOM shim for the step scripts (plan P14, spec test 35). A step script is the body of an async
// function of `args`, compiled into the app with `include_str!`; this file runs that same text, byte
// for byte, against a synthetic page parsed by linkedom (a DOM for Deno). The tests never ship. Every
// page, address and name in them is synthetic.
import { parseHTML } from "linkedom";

// deno-lint-ignore no-explicit-any
export type El = any;

export interface Page {
  // deno-lint-ignore no-explicit-any
  document: any;
  // deno-lint-ignore no-explicit-any
  Event: any;
  /** The page's `location`, for the scripts that navigate by hash or reload; a test's page may bring its own. */
  // deno-lint-ignore no-explicit-any
  location?: any;
}

const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

// A deterministic layout, so the scripts' visibility and pointer reads mean the same here as live.
// An element is hidden when it or an ancestor carries `hidden` or `style="display: none"`: its
// `offsetParent` is null and its box is empty, as in a browser. A shown element's box is a band in document
// order that holds its descendants' bands: the element at pre-order index i with s descendants spans
// y = 10i to 10(i + s + 1), x = 0 to 100. `elementFromPoint` is the innermost shown element whose band holds
// the point (its band is the latest in document order to hold it).
const hiddenHere = (el: El) =>
  el.hasAttribute?.("hidden") || /display\s*:\s*none/i.test(el.getAttribute?.("style") || "");
function isShown(el: El): boolean {
  if (!el.ownerDocument?.contains(el)) return false;
  for (let e = el; e && e.nodeType === 1; e = e.parentNode) if (hiddenHere(e)) return false;
  return true;
}
function boxOf(el: El) {
  const zero = { x: 0, y: 0, left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
  if (!isShown(el)) return zero;
  const i = [...el.ownerDocument.querySelectorAll("*")].indexOf(el);
  const s = el.querySelectorAll("*").length;
  const top = 10 * i, bottom = 10 * (i + s + 1);
  return { x: 0, y: top, left: 0, top, right: 100, bottom, width: 100, height: bottom - top };
}
let laidOut = false;
function layOut(document: El) {
  if (!laidOut) {
    let proto = Object.getPrototypeOf(document.body);
    while (proto && proto.constructor?.name !== "Element") proto = Object.getPrototypeOf(proto);
    Object.defineProperty(proto, "offsetParent", {
      configurable: true,
      get(this: El) {
        return isShown(this) ? this.parentElement ?? null : null;
      },
    });
    Object.defineProperty(proto, "getBoundingClientRect", {
      configurable: true,
      value(this: El) {
        return boxOf(this);
      },
    });
    laidOut = true;
  }
  document.elementFromPoint = (x: number, y: number) => {
    const all = [...document.querySelectorAll("*")].reverse();
    return all.find((el: El) => {
      const r = boxOf(el);
      return r.height > 0 && x >= r.left && x < r.right && y >= r.top && y < r.bottom;
    }) ?? null;
  };
}

/** Parses `html` into a page. The shim has no form behaviour of its own, so `behave` is the page. */
export function load(html: string): Page {
  const win = parseHTML(`<!doctype html><html><body>${html}</body></html>`) as unknown as Page;
  const { document, Event } = win;
  layOut(document);
  for (const box of document.querySelectorAll('input[type="checkbox"]')) {
    box.checked = box.hasAttribute("checked");
  }
  return { document, Event };
}

/** Whether a mouse event carries a point inside `el`'s box (a menu item that answers only at its point). */
export function atPoint(el: El, e: El): boolean {
  const r = el.getBoundingClientRect();
  return typeof e.clientX === "number" && typeof e.clientY === "number" && r.height > 0 &&
    e.clientX >= r.left && e.clientX < r.right && e.clientY >= r.top && e.clientY < r.bottom;
}

/**
 * Runs the script at `path` (relative to this folder) on `page`, as the app does: the text is the body
 * of an async function of `args`, and what it returns is the result. The script sees the page's
 * `document`, `Event` and `location`, and nothing else of the page.
 */
export async function run(path: string, page: Page, args: Record<string, unknown> = {}) {
  const source = await Deno.readTextFile(new URL(path, import.meta.url));
  const body = new AsyncFunction("args", "document", "Event", "location", source);
  return await body(args, page.document, page.Event, page.location ?? { hash: "", host: "", reload() {} });
}

export function disable(el: El, on: boolean) {
  el.disabled = on;
  if (on) el.setAttribute("disabled", "");
  else el.removeAttribute("disabled");
}
