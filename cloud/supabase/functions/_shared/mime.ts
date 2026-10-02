/**
 * MIME for the mail functions (email plan T2c; spec §4.6 steps 5 and 7, §4.7 step 1, D20). Three reads,
 * each of a message held only in memory:
 *
 * - `parseHeaders`: From, Date, Subject, Message-ID, `List-*`, `Precedence`, the topmost `Received`
 *   time, and every field in order with its exact bytes (the signature checks, T2d, canonicalise
 *   those bytes, so nothing here re-encodes them).
 * - `bodyText` (`mail-process`, §4.7 step 1): the first `text/plain` part, else the first `text/html`
 *   part reduced to text. A `message/rfc822` part or a part marked `attachment` is never read.
 * - `attachedMessages` (intake's batch send, §4.6 step 5): each `message/rfc822` part, as its own bytes.
 *   No other part is decoded, and a message inside an attached message is not one of the results.
 *
 * **Every input is hostile.** Nothing here throws on a malformed message, logs, or puts any part of a
 * message in an error: a part it cannot read is skipped. Nesting stops at `MAX_DEPTH` and a message is
 * walked for at most `MAX_PARTS` parts, so a crafted structure costs a bounded walk; the object itself
 * is already capped by the S3 client (`s3.ts`).
 *
 * Bytes are held as a "binary string" (one char per byte, 0–255), so a header's `raw` and a part's bytes
 * round-trip exactly; text is decoded from bytes only at the edge (a charset, RFC 2047, RFC 6532).
 */

export interface HeaderField {
  /** The field name as written. */
  name: string;
  /** Unfolded and trimmed; raw UTF-8 (RFC 6532) decoded; RFC 2047 words left as written. */
  value: string;
  /** The field's exact bytes (one char per byte), folding included, without its final line break. */
  raw: string;
}

export interface MailHeaders {
  headers: HeaderField[];
  /** The first From mailbox's display name, decoded; `null` when it has none. */
  fromName: string | null;
  /** The first From mailbox's address, lower-cased; `null` when there is no `local@domain`. */
  fromAddress: string | null;
  /** The Date header as written (unfolded, trimmed). */
  date: string | null;
  /** The Date as epoch milliseconds; `null` when it is not a real RFC 5322 date-time. */
  dateAt: number | null;
  subject: string | null;
  /** The first msg-id without its angle brackets or whitespace; `null` when absent or empty. */
  messageId: string | null;
  /** Each `List-*` field (first occurrence), keyed by its lower-cased name. */
  list: Record<string, string>;
  /** `Precedence`, lower-cased. */
  precedence: string | null;
  /** The time stamped by the topmost `Received` field; `null` when absent or unreadable. */
  receivedAt: number | null;
  /** §4.6 step 7's delivery time: `receivedAt`, else `dateAt`. A lower `Received` is never read. */
  deliveredAt: number | null;
}

export type MailInput = Uint8Array | string;

export const MAX_DEPTH = 8;
export const MAX_PARTS = 500;

// ---------------------------------------------------------------- bytes

/** A message as a binary string. A JS string input is taken as text and encoded as UTF-8 first. */
function toBinary(input: MailInput): string {
  const bytes = typeof input === "string" ? new TextEncoder().encode(input) : input;
  let out = "";
  for (let i = 0; i < bytes.length; i += 0x8000) {
    out += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return out;
}

function toBytes(binary: string): Uint8Array {
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i) & 0xff;
  return out;
}

/** Decode bytes in a declared charset; an unknown or missing label reads as UTF-8. Never throws. */
function decodeCharset(bytes: Uint8Array, charset: string | undefined): string {
  let decoder: TextDecoder;
  try {
    decoder = new TextDecoder((charset ?? "utf-8").trim() || "utf-8");
  } catch {
    decoder = new TextDecoder("utf-8");
  }
  return decoder.decode(bytes);
}

/** Raw 8-bit header text (RFC 6532) is UTF-8; pure ASCII passes through untouched. */
function headerText(binary: string): string {
  return /[\x80-\xff]/.test(binary) ? decodeCharset(toBytes(binary), "utf-8") : binary;
}

const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_INDEX = new Map([...B64].map((c, i) => [c, i]));

/** Lenient base64: characters outside the alphabet are skipped; a trailing partial group is dropped. */
function base64ToBinary(text: string): string {
  const out: number[] = [];
  let acc = 0;
  let bits = 0;
  for (const ch of text) {
    if (ch === "=") break;
    const v = B64_INDEX.get(ch);
    if (v === undefined) continue;
    acc = (acc << 6) | v;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      out.push((acc >> bits) & 0xff);
    }
  }
  return binaryFromNumbers(out);
}

function binaryFromNumbers(nums: number[]): string {
  let s = "";
  for (let i = 0; i < nums.length; i += 0x8000) s += String.fromCharCode(...nums.slice(i, i + 0x8000));
  return s;
}

/** Quoted-printable: soft breaks removed, `=XX` decoded, anything else kept as written. */
function qpToBinary(text: string, underscoreIsSpace = false): string {
  let s = text.replace(/=[ \t]*\r?\n/g, "");
  if (underscoreIsSpace) s = s.replaceAll("_", " ");
  return s.replace(/=([0-9A-Fa-f]{2})/g, (_m, h: string) => String.fromCharCode(parseInt(h, 16)));
}

/** A part's body as bytes, by its `Content-Transfer-Encoding`. */
function decodeTransfer(body: string, cte: string): string {
  if (cte === "base64") return base64ToBinary(body);
  if (cte === "quoted-printable") return qpToBinary(body);
  return body;
}

// ---------------------------------------------------------------- entities and fields

/** An entity's header block and body, split at the first empty line (CRLF or LF). */
function splitEntity(entity: string): { head: string; body: string } {
  if (entity.startsWith("\r\n")) return { head: "", body: entity.slice(2) };
  if (entity.startsWith("\n")) return { head: "", body: entity.slice(1) };
  const crlf = entity.indexOf("\n\r\n");
  const lf = entity.indexOf("\n\n");
  if (crlf < 0 && lf < 0) return { head: entity, body: "" };
  if (lf < 0 || (crlf >= 0 && crlf < lf)) {
    return { head: entity.slice(0, crlf), body: entity.slice(crlf + 3) };
  }
  return { head: entity.slice(0, lf), body: entity.slice(lf + 2) };
}

const FIELD_NAME = /^[\x21-\x39\x3b-\x7e]+$/;

/** The fields of a header block, in order. A line that is neither a field nor a fold is dropped,
 * and so are the folds that follow it. */
function parseFields(head: string): HeaderField[] {
  const fields: HeaderField[] = [];
  let current: string[] | null = null;
  const flush = () => {
    if (current === null) return;
    const raw = current.join("\n");
    const colon = raw.indexOf(":");
    const name = raw.slice(0, colon).replace(/[ \t]+$/, "");
    if (FIELD_NAME.test(name)) {
      const value = headerText(raw.slice(colon + 1).replace(/\r?\n/g, "")).trim();
      fields.push({ name, value, raw: raw.replace(/\r$/, "") });
    }
    current = null;
  };
  for (const line of head.split("\n")) {
    if (line.startsWith(" ") || line.startsWith("\t")) {
      if (current !== null) current.push(line);
      continue;
    }
    flush();
    if (line.includes(":")) current = [line];
  }
  flush();
  return fields;
}

function field(fields: HeaderField[], name: string): string | null {
  const lower = name.toLowerCase();
  for (const f of fields) if (f.name.toLowerCase() === lower) return f.value;
  return null;
}

/** RFC 2047 encoded words decoded; whitespace between two adjacent encoded words is dropped. */
export function decodeWords(value: string): string {
  const word = /=\?([^?\s]+)\?([BbQq])\?([^?\s]*)\?=/g;
  let out = "";
  let last = 0;
  let pending: { charset: string; bytes: string } | null = null;
  const emit = () => {
    if (pending !== null) out += decodeCharset(toBytes(pending.bytes), pending.charset);
    pending = null;
  };
  for (let m = word.exec(value); m !== null; m = word.exec(value)) {
    const between = value.slice(last, m.index);
    if (pending === null || !/^[ \t]*$/.test(between)) {
      emit();
      out += between;
    }
    const charset = m[1].split("*")[0].toLowerCase();
    const bytes = m[2].toLowerCase() === "b" ? base64ToBinary(m[3]) : qpToBinary(m[3], true);
    const held = pending as { charset: string; bytes: string } | null;
    if (held !== null && held.charset === charset) held.bytes += bytes;
    else {
      emit();
      pending = { charset, bytes };
    }
    last = word.lastIndex;
  }
  emit();
  return out + value.slice(last);
}

/** `type/subtype` lower-cased, and its parameters (names lower-cased, quoted values unescaped). */
function parseContentType(value: string | null): { type: string; params: Record<string, string> } {
  const params: Record<string, string> = {};
  if (value === null) return { type: "", params };
  const pieces = splitTopLevel(value, ";");
  const type = stripComments(pieces.shift() ?? "").trim().toLowerCase();
  for (const piece of pieces) {
    const eq = piece.indexOf("=");
    if (eq < 0) continue;
    const name = piece.slice(0, eq).trim().toLowerCase();
    let v = piece.slice(eq + 1).trim();
    if (v.startsWith('"')) {
      v = v.slice(1, v.endsWith('"') && v.length > 1 ? -1 : undefined).replace(/\\(.)/g, "$1");
    } else v = stripComments(v).trim();
    if (name !== "" && !(name in params)) params[name] = v;
  }
  return { type, params };
}

/** Split on a separator outside quoted strings, comments and angle brackets. */
function splitTopLevel(value: string, sep: string): string[] {
  const out: string[] = [];
  let cur = "";
  let quoted = false;
  let depth = 0;
  let angle = false;
  for (let i = 0; i < value.length; i++) {
    const ch = value[i];
    if (quoted) {
      cur += ch;
      if (ch === "\\" && i + 1 < value.length) cur += value[++i];
      else if (ch === '"') quoted = false;
      continue;
    }
    if (ch === '"' && depth === 0) quoted = true;
    else if (ch === "(") depth++;
    else if (ch === ")" && depth > 0) depth--;
    else if (ch === "<" && depth === 0) angle = true;
    else if (ch === ">" && depth === 0) angle = false;
    else if (ch === sep && depth === 0 && !angle) {
      out.push(cur);
      cur = "";
      continue;
    }
    cur += ch;
  }
  out.push(cur);
  return out;
}

/** Comments removed (nesting and quoted strings respected); each comment leaves one space. */
function stripComments(s: string): string {
  let out = "";
  let depth = 0;
  let quoted = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (depth > 0) {
      if (ch === "\\") i++;
      else if (ch === "(") depth++;
      else if (ch === ")") depth--;
      continue;
    }
    if (quoted) {
      out += ch;
      if (ch === "\\" && i + 1 < s.length) out += s[++i];
      else if (ch === '"') quoted = false;
      continue;
    }
    if (ch === "(") {
      depth = 1;
      out += " ";
      continue;
    }
    if (ch === '"') quoted = true;
    out += ch;
  }
  return out;
}

const ADDRESS = /^[^\s@<>()",;:\\]+@[^\s@<>()",;:\\]+$/;

/** The first mailbox of a From value: its decoded display name and its lower-cased address. */
function firstMailbox(value: string): { name: string | null; address: string | null } {
  const mailbox = stripComments(splitTopLevel(value, ",")[0] ?? "");
  let quoted = false;
  let lt = -1;
  for (let i = 0; i < mailbox.length && lt < 0; i++) {
    if (mailbox[i] === "\\" && quoted) i++;
    else if (mailbox[i] === '"') quoted = !quoted;
    else if (mailbox[i] === "<" && !quoted) lt = i;
  }
  const gt = lt < 0 ? -1 : mailbox.indexOf(">", lt);
  const spec = (lt >= 0 && gt > lt ? mailbox.slice(lt + 1, gt) : mailbox).trim();
  let name = lt >= 0 ? mailbox.slice(0, lt).trim() : "";
  if (name.length > 1 && name.startsWith('"') && name.endsWith('"')) {
    name = name.slice(1, -1).replace(/\\(.)/g, "$1");
  }
  name = decodeWords(name).replace(/\s+/g, " ").trim();
  return { name: name === "" ? null : name, address: ADDRESS.test(spec) ? spec.toLowerCase() : null };
}

const MONTHS = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
/** RFC 5322's obsolete zones, in minutes east of UTC. Any other alphabetic zone reads as -0000 (§4.3). */
const ZONES: Record<string, number> = {
  ut: 0,
  gmt: 0,
  est: -300,
  edt: -240,
  cst: -360,
  cdt: -300,
  mst: -420,
  mdt: -360,
  pst: -480,
  pdt: -420,
};
const DATE_TIME =
  /^(?:[A-Za-z]+\s*,?\s*)?(\d{1,2})\s+([A-Za-z]{3})[A-Za-z]*\.?\s+(\d{2,4})\s+(\d{1,2}):(\d{2})(?::(\d{2}))?\s+([+-]\d{4}|[A-Za-z]{1,5})$/;

/** An RFC 5322 date-time (obsolete forms included) as epoch ms; `null` for anything else, a date
 * that does not exist, or a missing zone. */
export function parseDate(value: string): number | null {
  const m = DATE_TIME.exec(stripComments(value).replace(/\s+/g, " ").trim());
  if (m === null) return null;
  const day = Number(m[1]);
  const month = MONTHS.indexOf(m[2].toLowerCase());
  let year = Number(m[3]);
  if (m[3].length === 2) year += year < 50 ? 2000 : 1900;
  else if (m[3].length === 3) year += 1900;
  const [hour, minute] = [Number(m[4]), Number(m[5])];
  const second = Math.min(Number(m[6] ?? "0"), 59); // a leap second reads as :59
  if (year < 1900 || month < 0 || day < 1 || hour > 23 || minute > 59) return null;
  if (new Date(Date.UTC(year, month, day)).getUTCDate() !== day) return null;
  let offset = 0;
  const zone = m[7];
  if (/^[+-]\d{4}$/.test(zone)) {
    const [hh, mm] = [Number(zone.slice(1, 3)), Number(zone.slice(3))];
    if (mm > 59) return null;
    offset = (zone[0] === "-" ? -1 : 1) * (hh * 60 + mm);
  } else offset = ZONES[zone.toLowerCase()] ?? 0;
  const at = Date.UTC(year, month, day, hour, minute, second) - offset * 60_000;
  return Number.isFinite(at) ? at : null;
}

// ---------------------------------------------------------------- headers

/** The header block's bytes only: a large body is never copied to read the headers. */
function headBinary(input: MailInput): string {
  if (typeof input === "string") return splitEntity(toBinary(input)).head;
  for (let i = 0; i + 1 < input.length; i++) {
    if (input[i] !== 0x0a) continue;
    if (i === 0 || input[i + 1] === 0x0a || (input[i + 1] === 0x0d && input[i + 2] === 0x0a)) {
      return splitEntity(toBinary(input.subarray(0, i + 3))).head;
    }
  }
  return splitEntity(toBinary(input)).head;
}

function headersOf(fields: HeaderField[]): MailHeaders {
  const from = field(fields, "from");
  const mailbox = from === null ? { name: null, address: null } : firstMailbox(from);
  const date = field(fields, "date");
  const dateAt = date === null ? null : parseDate(date);
  const subject = field(fields, "subject");
  const list: Record<string, string> = {};
  for (const f of fields) {
    const lower = f.name.toLowerCase();
    if (lower.startsWith("list-") && !(lower in list)) list[lower] = f.value;
  }
  const precedence = stripComments(field(fields, "precedence") ?? "").trim().toLowerCase();
  const received = field(fields, "received");
  const stamp = received === null ? -1 : received.lastIndexOf(";");
  const receivedAt = received === null || stamp < 0 ? null : parseDate(received.slice(stamp + 1));
  return {
    headers: fields,
    fromName: mailbox.name,
    fromAddress: mailbox.address,
    date,
    dateAt,
    subject: subject === null ? null : decodeWords(subject).trim(),
    messageId: msgId(field(fields, "message-id")),
    list,
    precedence: precedence === "" ? null : precedence,
    receivedAt,
    deliveredAt: receivedAt ?? dateAt,
  };
}

/** The first msg-id's text. One that begins with `[` is not an RFC 5322 id (its left part is a
 * dot-atom), and is refused so no id can ever equal `messageKey`'s Date-From-Subject form. */
function msgId(value: string | null): string | null {
  if (value === null) return null;
  const angled = /<([^<>]*)>/.exec(value);
  const id = (angled ? angled[1] : stripComments(value)).replace(/\s+/g, "");
  return id === "" || id.startsWith("[") ? null : id;
}

/** The headers of a message (spec §4.7 step 1's fields, §4.6 step 7's delivery time). */
export function parseHeaders(input: MailInput): MailHeaders {
  return headersOf(parseFields(headBinary(input)));
}

const collapse = (s: string | null) => (s ?? "").replace(/\s+/g, " ").trim();

/**
 * What a message is keyed on (D20; §4.6 step 7): its Message-ID, else its Date, From and Subject
 * (the Date as an instant, the From as its address, the Subject decoded), so the same message by two
 * routes keys the same. Hash it before it is stored anywhere (`mailUid`; intake's keyed HMAC).
 */
export function messageKey(h: MailHeaders): string {
  if (h.messageId !== null) return h.messageId;
  const date = h.dateAt !== null ? new Date(h.dateAt).toISOString() : collapse(h.date);
  const from = h.fromAddress ?? collapse(field(h.headers, "from"));
  return JSON.stringify([date, from, collapse(h.subject)]);
}

/** D20's uid: `mail:` and the first 32 hex of SHA-256 of `messageKey`. */
export async function mailUid(h: MailHeaders): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(messageKey(h)));
  const hex = Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
  return `mail:${hex.slice(0, 32)}`;
}

// ---------------------------------------------------------------- parts

interface Leaf {
  type: string;
  charset: string | undefined;
  cte: string;
  attachment: boolean;
  /** The part's body bytes, still transfer-encoded. */
  body: string;
}

/** A boundary is 1 to 70 characters on one line (RFC 2046 §5.1.1). */
const validBoundary = (b: string) => b.length >= 1 && b.length <= 70 && !/[\r\n]/.test(b);

/** Every leaf part in document order, within the caps. A `message/rfc822` part is a leaf: its own
 * parts are never visited. A multipart past `MAX_DEPTH`, or without a usable boundary, yields none. */
function leaves(entity: string): Leaf[] {
  const out: Leaf[] = [];
  const budget = { parts: MAX_PARTS };
  const visit = (e: string, depth: number, defaultType: string) => {
    if (budget.parts <= 0) return;
    budget.parts--;
    const { head, body } = splitEntity(e);
    const fields = parseFields(head);
    const ct = parseContentType(field(fields, "content-type"));
    const type = /^[^/\s]+\/[^/\s]+$/.test(ct.type) ? ct.type : defaultType;
    if (type.startsWith("multipart/")) {
      const boundary = ct.params.boundary ?? "";
      if (depth >= MAX_DEPTH || !validBoundary(boundary)) return;
      const inner = type === "multipart/digest" ? "message/rfc822" : "text/plain";
      for (const part of splitMultipart(body, boundary, budget.parts)) visit(part, depth + 1, inner);
      return;
    }
    out.push({
      type,
      charset: ct.params.charset,
      cte: (field(fields, "content-transfer-encoding") ?? "").trim().toLowerCase(),
      attachment: parseContentType(field(fields, "content-disposition")).type === "attachment",
      body,
    });
  };
  visit(entity, 0, "text/plain");
  return out;
}

/** The index of the next delimiter line `--boundary` at or after `from`, or -1. */
function nextDelimiter(body: string, delim: string, from: number): number {
  for (let i = body.indexOf(delim, from); i >= 0; i = body.indexOf(delim, i + 1)) {
    if (i !== 0 && body[i - 1] !== "\n") continue;
    const after = body[i + delim.length];
    if (
      after === undefined || after === "-" || after === "\r" || after === "\n" || after === " " ||
      after === "\t"
    ) {
      return i;
    }
  }
  return -1;
}

/** The body parts between delimiter lines; preamble and epilogue dropped. An unterminated last part
 * runs to the end. At most `limit` parts. */
function splitMultipart(body: string, boundary: string, limit: number): string[] {
  const delim = `--${boundary}`;
  const parts: string[] = [];
  let at = nextDelimiter(body, delim, 0);
  while (at >= 0 && parts.length < limit) {
    if (body.startsWith("--", at + delim.length)) break;
    const eol = body.indexOf("\n", at + delim.length);
    if (eol < 0) break;
    const next = nextDelimiter(body, delim, eol + 1);
    let end = next < 0 ? body.length : next;
    if (next >= 0 && body[end - 1] === "\n") end--;
    if (next >= 0 && body[end - 1] === "\r") end--;
    parts.push(body.slice(eol + 1, Math.max(end, eol + 1)));
    at = next;
  }
  return parts;
}

function decodeLeaf(leaf: Leaf): string {
  return decodeCharset(toBytes(decodeTransfer(leaf.body, leaf.cte)), leaf.charset).replace(/\r\n?/g, "\n");
}

/**
 * §4.7 step 1's text: the first `text/plain` part, else the first `text/html` part reduced to text;
 * `""` when there is neither. A part marked `attachment`, and anything inside a `message/rfc822`
 * part, is never read. The caller cuts and scrubs it (`scrubForPrompt`) before any model sees it.
 */
export function bodyText(input: MailInput): string {
  const readable = leaves(toBinary(input)).filter((l) => !l.attachment);
  const plain = readable.find((l) => l.type === "text/plain");
  if (plain !== undefined) return decodeLeaf(plain).trim();
  const html = readable.find((l) => l.type === "text/html");
  return html === undefined ? "" : htmlToText(decodeLeaf(html));
}

/**
 * §4.6 step 5's batch send: each `message/rfc822` part, in order, as the attached message's own bytes
 * (a transfer encoding, where a client used one, undone). No other part's body is decoded.
 */
export function attachedMessages(input: MailInput): Uint8Array[] {
  return leaves(toBinary(input))
    .filter((l) => l.type === "message/rfc822")
    .map((l) => toBytes(decodeTransfer(l.body, l.cte)));
}

// ---------------------------------------------------------------- HTML

const BLOCK_TAGS = new Set([
  "address",
  "article",
  "blockquote",
  "br",
  "dd",
  "div",
  "dl",
  "dt",
  "footer",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "header",
  "hr",
  "li",
  "ol",
  "p",
  "pre",
  "section",
  "table",
  "tr",
  "ul",
]);
const CELL_TAGS = new Set(["td", "th"]);
/** Elements whose content is never text a reader sees. */
const SKIP_TAGS = new Set(["head", "noscript", "script", "style", "template", "title"]);
const ENTITIES: Record<string, string> = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: " ",
  ndash: "–",
  mdash: "—",
  hellip: "…",
  lsquo: "‘",
  rsquo: "’",
  ldquo: "“",
  rdquo: "”",
  bull: "•",
  middot: "·",
  copy: "©",
  reg: "®",
  trade: "™",
  shy: "",
  zwnj: "",
  zwj: "",
};

function decodeEntity(match: string, body: string): string {
  if (body.startsWith("#")) {
    const hex = body[1] === "x" || body[1] === "X";
    const cp = parseInt(body.slice(hex ? 2 : 1), hex ? 16 : 10);
    const ok = cp > 0 && cp <= 0x10ffff && !(cp >= 0xd800 && cp <= 0xdfff);
    return ok ? String.fromCodePoint(cp) : match;
  }
  return ENTITIES[body.toLowerCase()] ?? match;
}

/**
 * HTML reduced to the text a reader sees, by one linear scan (no backtracking over hostile markup):
 * comments, `script`, `style` and `head` content dropped; block elements end a line, table cells a
 * word; every other tag removed; entities decoded once, after the tags are gone; source whitespace
 * collapsed; empty lines dropped. An unterminated tag or comment ends the text.
 */
export function htmlToText(html: string): string {
  let out = "";
  let i = 0;
  while (i < html.length) {
    const lt = html.indexOf("<", i);
    if (lt < 0) {
      out += html.slice(i).replace(/\s+/g, " ");
      break;
    }
    out += html.slice(i, lt).replace(/\s+/g, " ");
    if (html.startsWith("<!--", lt)) {
      const close = html.indexOf("-->", lt + 4);
      if (close < 0) break;
      i = close + 3;
      continue;
    }
    if (!/[A-Za-z/!?]/.test(html[lt + 1] ?? "")) {
      out += "<";
      i = lt + 1;
      continue;
    }
    const gt = html.indexOf(">", lt);
    if (gt < 0) break;
    const tag = /^<(\/?)([A-Za-z][A-Za-z0-9]*)/.exec(html.slice(lt, Math.min(gt + 1, lt + 64)));
    const name = tag ? tag[2].toLowerCase() : "";
    i = gt + 1;
    if (tag && tag[1] === "" && SKIP_TAGS.has(name)) {
      const end = new RegExp(`</${name}\\s*>`, "gi");
      end.lastIndex = i;
      const m = end.exec(html);
      if (m === null) break;
      i = m.index + m[0].length;
      continue;
    }
    if (BLOCK_TAGS.has(name)) out += "\n";
    else if (CELL_TAGS.has(name)) out += " ";
  }
  return out
    .replace(/&(#[0-9]{1,7}|#[xX][0-9A-Fa-f]{1,6}|[A-Za-z]{2,8});/g, decodeEntity)
    .split("\n")
    .map((line) => line.replace(/[^\S\n]+/g, " ").trim())
    .filter((line) => line !== "")
    .join("\n");
}
