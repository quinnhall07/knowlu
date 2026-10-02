// Microsoft 365, Settings > Mail > Forwarding: read the forwarding setting and change nothing (spec 4.2
// step 3's check, 4.11 step 2). The caller has already navigated to the Forwarding page (never a reload).
// Like every step script it is the body of an async function of `args`; `args.address` is this mailbox's
// Knowlu forwarding address. It returns one of
//   { result: "ok", forwarding: "on" | "off", to: "none" | "ours" | "elsewhere", keep_copy: boolean }
//   { result: "not found", control: "switch" | "address" }   (the page lacks a control: guided steps)
//   { result: "failed", reason: "no address" }
// The switch is `input[role=switch]` and is read by its `checked` property, never `aria-checked`, which
// the page leaves stale. An address that is not ours is never returned, only named "elsewhere".
const text = (el) => (el.textContent || "").replace(/\s+/g, " ").trim();
const nameOf = (el) => {
  const aria = el.getAttribute("aria-label");
  if (aria) return aria.trim();
  const by = el.getAttribute("aria-labelledby");
  if (by) {
    const t = by.split(/\s+/).map((id) => document.getElementById(id)).filter(Boolean).map(text).join(" ");
    if (t) return t;
  }
  if (el.id) {
    for (const l of document.querySelectorAll("label")) if (l.getAttribute("for") === el.id) return text(l);
  }
  const wrap = el.closest ? el.closest("label") : null;
  return wrap ? text(wrap) : "";
};
const named = (selector, pattern) =>
  [...document.querySelectorAll(selector)].find((el) => pattern.test(nameOf(el)));
const norm = (v) => String(v || "").trim().toLowerCase();

const address = norm(args && args.address);
if (!address) return { result: "failed", reason: "no address" };

const sw = named('input[role="switch"]', /forward/i);
if (!sw) return { result: "not found", control: "switch" };
const on = sw.checked === true;
const field = named('input[type="text"], input[type="email"], input:not([type])', /forward.*\bto\b/i);
if (on && !field) return { result: "not found", control: "address" };
const typed = field ? norm(field.value) : "";
const keep = named('input[type="checkbox"]', /keep a copy/i);
return {
  result: "ok",
  forwarding: on ? "on" : "off",
  to: typed === "" ? "none" : typed === address ? "ours" : "elsewhere",
  keep_copy: keep ? keep.checked === true : false,
};
