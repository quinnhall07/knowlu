// The free-text policy (prompt-injection suite plan, section 4.3 and its amendments): what a model
// answer's free text may carry to a student's device. It keeps links, markup and line breaks that a
// hostile email or page planted out of `why`, `importance_reason`, `evidence`, `title`, `item.title`
// and `target`. `judge_validate.ts` runs steps 1 and 2 (the line rule, `oneLine`); this file is steps
// 3 to 5, for model-written text only. Template claims keep the line rule alone.
//
// Every rule matches on the value's MATCHING FORM: NFKC, every Default_Ignorable_Code_Point removed,
// the dot look-alikes U+3002, U+FF0E and U+FF61 read as ".", then lower-cased. The form is for
// matching only: a value that breaks nothing is sent byte for byte.
//
// The suite's oracle (`cloud/eval/redteam/detect.ts`) states the same rules on its own and shares no
// code with this file; `output_policy_test.ts` holds the two together. The one intended difference:
// the oracle reads any TLD-shaped final label as a TLD, this file only the dated IANA snapshot in
// `tlds.ts` plus the four reserved names.
//
// `oneLine` is imported from `judge_validate.ts`, which imports this file: the cycle is safe because
// neither module uses the other's exports while it loads, only inside functions (as `judge_due.ts`).
import { oneLine } from "./judge_validate.ts";
import { IANA_TLDS, IDN_TLDS, RESERVED_TLDS } from "./tlds.ts";

/** The model-written free-text fields the policy covers. */
export type FreeTextField = "why" | "importance_reason" | "evidence" | "title" | "item.title" | "target";

interface FieldRule {
  /** Explanations are neutralised on a breach; names are refused (ruling Q1). */
  neutralise: boolean;
  /** The bound, in characters, applied after neutralising (step 5). */
  max: number;
  /** Evidence has always been trimmed after its cut; the other fields keep today's `oneLine` clip. */
  trim: boolean;
}

const FIELDS: Record<FreeTextField, FieldRule> = {
  why: { neutralise: true, max: 140, trim: false },
  importance_reason: { neutralise: true, max: 140, trim: false },
  evidence: { neutralise: true, max: 140, trim: true },
  title: { neutralise: false, max: 200, trim: false },
  "item.title": { neutralise: false, max: 200, trim: false },
  target: { neutralise: false, max: 200, trim: false },
};

/** The policy's closed extension list, written out here on its own. */
const EXTENSIONS: ReadonlySet<string> = new Set(
  (
    "pdf doc docx ppt pptx xls xlsx csv txt md rtf odt zip tar gz 7z py ipynb java c cpp h js ts rs " +
    "sh pl r m sql html css json xml png jpg jpeg gif svg mp3 mp4 mov wav ai psd"
  ).split(" "),
);

/** Exactly the scrub's six tokens, exempt from the markup rule (removed, one pass, after lowercasing). */
const SCRUB_TOKEN = /<(?:email|url|user|note|secret|token)>/g;

const LINK_REMOVED = "[link removed]";
const MARKUP_REMOVED = "[markup removed]";
const PLACEHOLDERS = [LINK_REMOVED, MARKUP_REMOVED];

const DOT_LOOK_ALIKES: ReadonlySet<string> = new Set(["\u3002", "\uFF0E", "\uFF61"]);
/** The division slash, the fraction slash, the big solidus and the box-drawing diagonal read as "/"
 * (T2b review and re-review rulings). */
const SLASH_LOOK_ALIKES: ReadonlySet<string> = new Set(["\u2215", "\u2044", "\u29F8", "\u2571"]);

/**
 * A bidi embedding, override or isolate control (U+202A to U+202E, U+2066 to U+2069) can make a host
 * display other than it reads: one anywhere in the value as given is a markup breach (T2b review
 * rulings). It is default-ignorable, so it is looked for before normalising drops it, and neutralising
 * removes it with the other default-ignorables.
 */
const BIDI_CONTROL = /[\u202A-\u202E\u2066-\u2069]/;

/**
 * Cyrillic, Greek and other letters that look like Latin ones, upper and lower case, each to the Latin letter
 * it looks like (T2b review rulings). Applied to a final label only, before the TLD lookup.
 */
const LOOK_ALIKES: ReadonlyMap<string, string> = new Map(
  [
    // Cyrillic, lower then upper.
    ["\u0430", "a"],
    ["\u0435", "e"],
    ["\u043E", "o"],
    ["\u0440", "p"],
    ["\u0441", "c"],
    ["\u0443", "y"],
    ["\u0445", "x"],
    ["\u0456", "i"],
    ["\u0458", "j"],
    ["\u0455", "s"],
    ["\u0501", "d"],
    ["\u04CF", "l"],
    ["\u04BB", "h"],
    ["\u051B", "q"],
    ["\u051D", "w"],
    ["\u0410", "a"],
    ["\u0415", "e"],
    ["\u041E", "o"],
    ["\u0420", "p"],
    ["\u0421", "c"],
    ["\u0423", "y"],
    ["\u0425", "x"],
    ["\u0406", "i"],
    ["\u0408", "j"],
    ["\u0405", "s"],
    ["\u0500", "d"],
    ["\u04C0", "l"],
    ["\u04BA", "h"],
    ["\u051A", "q"],
    ["\u051C", "w"],
    // Cyrillic capitals in their own right (re-review): each looks like a Latin capital its lower case
    // does not (capital Ve reads B, capital En reads H).
    ["\u0412", "b"],
    ["\u041A", "k"],
    ["\u041C", "m"],
    ["\u041D", "h"],
    ["\u0422", "t"],
    // Greek, lower then upper.
    ["\u03BF", "o"],
    ["\u03B1", "a"],
    ["\u03B5", "e"],
    ["\u03B9", "i"],
    ["\u03BA", "k"],
    ["\u03BD", "v"],
    ["\u03C1", "p"],
    ["\u03C4", "t"],
    ["\u03C5", "u"],
    ["\u03C7", "x"],
    ["\u039F", "o"],
    ["\u0391", "a"],
    ["\u0395", "e"],
    ["\u0399", "i"],
    ["\u039A", "k"],
    ["\u039D", "n"],
    ["\u03A1", "p"],
    ["\u03A4", "t"],
    ["\u03A5", "y"],
    ["\u03A7", "x"],
    // Greek capitals in their own right (re-review).
    ["\u0392", "b"],
    ["\u0396", "z"],
    ["\u0397", "h"],
    ["\u039C", "m"],
    // Others (re-review). NFKC runs first: lunate sigma U+03F2 arrives as final sigma U+03C2, and capital
    // lunate sigma U+03F9 as capital sigma U+03A3, so those two carry the "c".
    ["\u03F2", "c"],
    ["\u03C2", "c"],
    ["\u03F9", "c"],
    ["\u03A3", "c"],
    ["\u03F3", "j"],
    ["\u0261", "g"],
    ["\u0131", "i"],
    ["\u0585", "o"],
  ],
);

/** Step 3's normalised text: NFKC, default-ignorables removed, case kept. */
function normalised(value: string): string {
  return value.normalize("NFKC").replace(/\p{Default_Ignorable_Code_Point}/gu, "");
}

/** A normalised text's matching form, with each form code unit's source range in that text. */
interface Form {
  form: string;
  start: number[];
  end: number[];
}

/**
 * Dots and slashes mapped and lower-cased one character at a time, so every form position maps back to
 * the text. This equals lower-casing the whole string except for a final sigma (read as a medial one),
 * and no rule or listed TLD turns on that difference.
 */
function matchingForm(text: string): Form {
  let form = "";
  const start: number[] = [];
  const end: number[] = [];
  let at = 0;
  for (const ch of text) {
    const mapped = DOT_LOOK_ALIKES.has(ch) ? "." : SLASH_LOOK_ALIKES.has(ch) ? "/" : ch.toLowerCase();
    for (let k = 0; k < mapped.length; k++) {
      start.push(at);
      end.push(at + ch.length);
    }
    form += mapped;
    at += ch.length;
  }
  return { form, start, end };
}

/**
 * A final label's lookup key (T2b review rulings): NFD, combining marks removed, look-alikes mapped to
 * Latin, then lower-cased. So a mark (`c` U+0336 `om`), an accent (`c\u00F3m`) or a Cyrillic or Greek
 * letter (`c` U+043E `m`) does not hide a listed TLD.
 */
function tldKey(label: string): string {
  let key = "";
  for (const ch of label.normalize("NFD").replace(/\p{M}/gu, "")) key += LOOK_ALIKES.get(ch) ?? ch;
  return key.toLowerCase();
}

/** Every listed TLD by its lookup key; each internationalised name is listed in both forms. */
const TLDS: ReadonlySet<string> = new Set(
  [...IANA_TLDS, ...RESERVED_TLDS, ...IDN_TLDS.map(([, unicode]) => unicode)].map((name) =>
    tldKey(matchingForm(normalised(name)).form)
  ),
);

/** A breaking span on the matching form, `[from, to)`; `link` when a URL rule found any of it. */
interface Span {
  from: number;
  to: number;
  link: boolean;
}

interface Scan {
  line: boolean;
  spans: Span[];
  matching: Form;
}

function breaksLine(form: string): boolean {
  for (const ch of form) {
    const n = ch.codePointAt(0)!;
    if (n <= 0x1f || n === 0x7f || n === 0x85 || n === 0x2028 || n === 0x2029) return true;
  }
  return false;
}

/**
 * URL prefixes; a backslash reads as a slash. `scheme://`; `javascript:`, `vbscript:`, `http:`,
 * `https:` and `mailto:` anywhere; a bracketed IPv6 literal (hex digits, two or more colons); after a
 * word boundary (no letter, mark, digit or `_` before), `data:` before `,`, `;`, `base64` or a MIME
 * shape and `file:` before a slash; `www.` anywhere; `localhost` before a port or a path. Each span
 * runs from the match to the next whitespace.
 */
const URL_PREFIXES: readonly RegExp[] = [
  // The lookbehind only skips starts inside a run, which never begin the leftmost match; it keeps a
  // long run of letters from being rescanned from every position.
  /(?<![\p{L}\p{N}])[\p{L}\p{N}][\p{L}\p{N}+.\-]*:[\/\\]{2}/gu,
  /(?:java|vb)script:/g,
  /https?:/g,
  /mailto:/g,
  /\[[0-9a-f]*:[0-9a-f]*:[0-9a-f:]*\]/g,
  /(?<![\p{L}\p{M}\p{N}_])data:(?:[,;]|base64|[a-z0-9][a-z0-9!#$&^_.+\-]*[\/\\][a-z0-9])/gu,
  /(?<![\p{L}\p{M}\p{N}_])file:[\/\\]/gu,
  /www\./g,
  /(?<![\p{L}\p{M}\p{N}_\-])localhost(?::[0-9]|[\/\\])/gu,
];

/** Two or more dot-joined labels. The lookbehind, as above, only skips starts inside a label. */
const HOST_SHAPED = /(?<![\p{L}\p{M}\p{N}_\-])[\p{L}\p{M}\p{N}_\-]+(?:\.[\p{L}\p{M}\p{N}_\-]+)+/gu;

/** The index of the first whitespace at or after `from`, else the end of `text`. */
function runEnd(text: string, from: number): number {
  const ws = /\s/g;
  ws.lastIndex = from;
  return ws.exec(text)?.index ?? text.length;
}

/** `/` or `\`, or `?` or `#` before a non-space character, at the start of `rest`. */
function pathOrQuery(rest: string): boolean {
  return /^(?:[\/\\]|[?#]\S)/.test(rest);
}

/** Four groups of one to three digits, each at most 255. */
function dottedQuad(token: string): boolean {
  const groups = token.split(".");
  return groups.length === 4 && groups.every((g) => /^[0-9]{1,3}$/.test(g) && Number(g) <= 255);
}

/**
 * Whether a host-shaped token, followed by `rest`, is a host, and whether a URL tail follows it.
 * On an extension-list final label only a tail makes a host (one optional `.` first; `:` and digits
 * count only before a path or query, so `main.rs:42` is a file and line). Otherwise a dotted quad or
 * a listed TLD is a host, tail or not.
 */
function hostTail(token: string, rest: string, cased: string): { host: boolean; tail: boolean } {
  const label = token.slice(token.lastIndexOf(".") + 1);
  const after = rest.startsWith(".") ? rest.slice(1) : rest;
  if (EXTENSIONS.has(label)) {
    const port = /^:\d+/.exec(after);
    const host = pathOrQuery(after) || (port !== null && pathOrQuery(after.slice(port[0].length)));
    return { host, tail: host };
  }
  // The label as written is keyed too, so an upper-case look-alike maps by its own shape (Greek capital
  // nu reads as "n", where its lower case would read as "v").
  const host = dottedQuad(token) || TLDS.has(tldKey(label)) || TLDS.has(tldKey(cased));
  return { host, tail: host && (pathOrQuery(after) || /^:[0-9]/.test(after)) };
}

/** `]` then `(`: from the `[` that opens it (if one does) to the `)` that closes it (if one does). */
function markdownLink(text: string, at: number): [number, number] {
  let from = at;
  for (let i = at - 1; i >= 0 && text[i] !== "]"; i--) {
    if (text[i] === "[") {
      from = i;
      break;
    }
  }
  const close = /^[^\s)]*\)/.exec(text.slice(at + 2));
  return [from, at + 2 + (close?.[0].length ?? 0)];
}

/** The markup spans of `form`, found with the scrub's tokens removed and mapped back to `form`. */
function markupSpans(form: string): Span[] {
  let kept = "";
  const index: number[] = [];
  let last = 0;
  const copy = (to: number) => {
    for (let i = last; i < to; i++) {
      kept += form[i];
      index.push(i);
    }
  };
  for (const token of form.matchAll(SCRUB_TOKEN)) {
    copy(token.index);
    last = token.index + token[0].length;
  }
  copy(form.length);
  const found: Array<[number, number]> = [];
  for (const m of kept.matchAll(/<[\p{L}\/!?]/gu)) {
    const close = kept.indexOf(">", m.index + m[0].length);
    found.push([m.index, close === -1 ? runEnd(kept, m.index + m[0].length) : close + 1]);
  }
  for (const m of kept.matchAll(/\]\(/g)) found.push(markdownLink(kept, m.index));
  for (const m of kept.matchAll(/!\[/g)) found.push([m.index, m.index + 2]);
  return found.map(([from, to]) => ({ from: index[from], to: index[to - 1] + 1, link: false }));
}

/** Every rule on one matching form: the line rule's verdict and every URL and markup span. */
function scan(text: string): Scan {
  const matching = matchingForm(text);
  const { form, start, end: until } = matching;
  const spans: Span[] = [];
  for (const rule of URL_PREFIXES) {
    for (const m of form.matchAll(rule)) {
      spans.push({ from: m.index, to: runEnd(form, m.index + m[0].length), link: true });
    }
  }
  for (const m of form.matchAll(HOST_SHAPED)) {
    const end = m.index + m[0].length;
    const cased = text.slice(start[m.index + m[0].lastIndexOf(".") + 1], until[end - 1]);
    const { host, tail } = hostTail(m[0], form.slice(end), cased);
    if (host) spans.push({ from: m.index, to: tail ? runEnd(form, end) : end, link: true });
  }
  spans.push(...markupSpans(form));
  return { line: breaksLine(form), spans, matching };
}

/** Whether `value` breaks any rule: a bidi control in it as given, or any rule on its matching form. */
function breaks(value: string): boolean {
  if (BIDI_CONTROL.test(value)) return true;
  const { line, spans } = scan(normalised(value));
  return line || spans.length > 0;
}

/** Overlapping or touching spans become one; it is a link if any part of it was. */
function merged(spans: Span[]): Span[] {
  const out: Span[] = [];
  for (const span of [...spans].sort((a, b) => a.from - b.from || a.to - b.to)) {
    const last = out[out.length - 1];
    if (last !== undefined && span.from <= last.to) {
      last.to = Math.max(last.to, span.to);
      last.link ||= span.link;
    } else out.push({ ...span });
  }
  return out;
}

/**
 * Step 3 on a breach: the normalised text, each breaking span replaced by its placeholder, then
 * `oneLine`'s substitutions again (NFKC can bring back a character `oneLine` had replaced). Null when
 * the value breaks the line rule, which has no placeholder.
 */
function neutralised(value: string): string | null {
  const text = normalised(value);
  const { line, spans, matching: { start, end } } = scan(text);
  if (line) return null;
  let out = "";
  let at = 0;
  for (const span of merged(spans)) {
    out += text.slice(at, start[span.from]) + (span.link ? LINK_REMOVED : MARKUP_REMOVED);
    at = end[span.to - 1];
  }
  return oneLine(out + text.slice(at), Number.POSITIVE_INFINITY);
}

/** Step 5: the first `max` characters; a cut that would split a placeholder falls before it. */
function clipped(value: string, max: number): string {
  const chars = [...value];
  if (chars.length <= max) return value;
  let cut = chars.slice(0, max).join("").length;
  for (const placeholder of PLACEHOLDERS) {
    for (let i = value.indexOf(placeholder); i !== -1 && i < cut; i = value.indexOf(placeholder, i + 1)) {
      if (i + placeholder.length > cut) cut = i;
    }
  }
  return value.slice(0, cut);
}

/** A value longer than this many times its bound is refused before any scan (T2b review rulings). */
const LENGTH_FACTOR = 4;

/** Whether `value` holds more than `limit` characters, counted no further than `limit + 1`. */
function tooLong(value: string, limit: number): boolean {
  if (value.length <= limit) return false;
  let count = 0;
  for (const _ of value) if (++count > limit) return true;
  return false;
}

/**
 * Steps 3 to 5 for one model-written free-text field, on its value after steps 1 and 2. Returns the
 * value to send, or null when the field refuses: a value over four times its bound (before any scan;
 * every field, explanations included, as for a value that cannot be neutralised), a breaking name
 * (`title`, `item.title`, `target`), a
 * neutralised explanation that still breaks (a placeholder forming markup with its neighbours), or a
 * clip that leaves a breach (a cut that ends in a host). A clean value comes back byte for byte,
 * clipped to the field's bound.
 */
export function enforceFreeText(field: FreeTextField, value: string): string | null {
  const rule = FIELDS[field];
  if (tooLong(value, LENGTH_FACTOR * rule.max)) return null;
  let sent = value;
  if (breaks(value)) {
    if (!rule.neutralise) return null;
    const replaced = neutralised(value);
    if (replaced === null || breaks(replaced)) return null;
    sent = replaced;
  }
  sent = clipped(sent, rule.max);
  if (rule.trim) sent = sent.trim();
  return breaks(sent) ? null : sent;
}
