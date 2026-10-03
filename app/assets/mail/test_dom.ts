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

/** Parses `html` into a page. The shim has no form behaviour of its own, so `behave` is the page. */
export function load(html: string): Page {
  const win = parseHTML(`<!doctype html><html><body>${html}</body></html>`) as unknown as Page;
  const { document, Event } = win;
  for (const box of document.querySelectorAll('input[type="checkbox"]')) {
    box.checked = box.hasAttribute("checked");
  }
  return { document, Event };
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
