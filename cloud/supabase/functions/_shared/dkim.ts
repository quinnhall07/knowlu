/**
 * DKIM and ARC verification for `mail-inbound` (email plan T2d; PQ2 as answered 2026-10-02; spec §4.6
 * steps 5 and 6). Our own small verifier, scoped to what binding needs and nothing more:
 *
 * - `verifyDkim`: every `DKIM-Signature` (RFC 6376), rsa-sha256 only, `simple` and `relaxed`
 *   canonicalisation, `l=` honoured, `x=` checked against the injected clock.
 * - `verifyArc` (RFC 8617): the **topmost** ARC instance only (the highest `i=`, sealed by the provider
 *   that handed the message to SES). Its `ARC-Seal` is verified over every set from 1 to N, and its
 *   `ARC-Message-Signature` over the headers its `h=` lists and the body. A lower instance is never read
 *   in its place: a topmost set that fails is a failure, and a chain with a gap, a duplicate or a
 *   missing part is a failure. The seal's `cv=` is reported, not judged, beyond fitting its instance.
 *
 * A pass reports the signing domain and the **exact field instances** the signature covers (picked from
 * the bottom up, as a signer picks them), so binding (T2e, T2f) reads the covered value and never another
 * instance of the same name. A failure reports a fixed word from `REASONS` and nothing else: no field
 * of the message reaches a result's reason, an error or a log, and this file logs nothing.
 *
 * The key resolver is injected (`DkimDeps.resolveTxt`); production passes `dnsTxtResolver()`, the one
 * place that reaches DNS. Each key is asked for once per message and at most `MAX_LOOKUPS` per message.
 * Every input is hostile: nothing here throws on a malformed message, and every walk is linear.
 */
import type { MailInput } from "./mime.ts";

/** TXT records for a name, each as its DNS character-strings (the platform resolver's shape). */
export type TxtResolver = (name: string) => Promise<string[][]>;
export interface DkimDeps {
  resolveTxt: TxtResolver;
  /** Epoch milliseconds; defaults to `Date.now`. Only `x=` reads it. */
  now?: () => number;
}
export type Canon = "simple" | "relaxed";

/** Every reason a result can carry. Fixed words, so a reason can never carry a message's bytes. */
export const REASONS = [
  "syntax",
  "algorithm",
  "from_unsigned",
  "body_length",
  "expired",
  "body_hash",
  "signature",
  "dns",
  "too_many",
  "no_key",
  "key_revoked",
  "key_type",
  "key_hash",
  "key_syntax",
  "key_weak",
  "structure",
  "cv",
] as const;
export type Reason = typeof REASONS[number];

/** A field a passing signature covers: its lower-cased name and its value, unfolded and trimmed. */
export interface CoveredField {
  name: string;
  value: string;
}
export interface SignatureResult {
  header: "dkim-signature" | "arc-message-signature";
  result: "pass" | "fail" | "permerror" | "temperror";
  reason: Reason | null;
  /** `d=`, lower-cased; `null` when the signature could not be read. */
  domain: string | null;
  selector: string | null;
  /** `h=`, lower-cased, in order. */
  signedHeaders: string[];
  /** The field instances hashed, in `h=` order; empty unless the result is `pass`. */
  covered: CoveredField[];
}
/** The topmost ARC instance. Only `pass` carries a domain, the covered fields or the AAR. */
export type ArcResult =
  | { result: "none"; instance: 0 }
  | { result: "fail" | "temperror"; part: "structure" | "seal" | "ams"; reason: Reason; instance: number }
  | {
    result: "pass";
    instance: number;
    cv: "none" | "pass" | "fail";
    sealDomain: string;
    amsDomain: string;
    signedHeaders: string[];
    covered: CoveredField[];
    /** The topmost `ARC-Authentication-Results`, after its `i=N;`, unfolded and trimmed. */
    authResults: string;
  };

export const MAX_SIGNATURES = 10;
export const MAX_LOOKUPS = 16;
export const MAX_ARC_INSTANCES = 50;
const MIN_KEY_BITS = 1024; // RFC 8301
const DNS_TIMEOUT_MS = 5_000;
const RSA = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;

// ---------------------------------------------------------------- bytes and fields

/** Bytes as a binary string (one char per byte); a JS string is taken as text and encoded UTF-8 first.
 * Bare LF line ends become CRLF, as every verifier must assume the wire form. */
function toBinary(input: MailInput): string {
  const bytes = typeof input === "string" ? new TextEncoder().encode(input) : input;
  let out = "";
  for (let i = 0; i < bytes.length; i += 0x8000) {
    out += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return out.replace(/\r?\n/g, "\r\n");
}

function toBytes(binary: string): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i) & 0xff;
  return out;
}

interface Field {
  /** Lower-cased, trailing WSP before the colon removed. */
  lower: string;
  /** The exact bytes, folds included, without the final CRLF. */
  raw: string;
}
interface Message {
  fields: Field[];
  body: string;
}

const FIELD_NAME = /^[\x21-\x39\x3b-\x7e]+$/;

/** The header fields in order, and the body after the first empty line. A line that is neither a
 * field nor a fold is skipped, with the folds that follow it. */
function parse(input: MailInput): Message {
  const bin = toBinary(input);
  let head: string;
  let body: string;
  if (bin.startsWith("\r\n")) {
    head = "";
    body = bin.slice(2);
  } else {
    const at = bin.indexOf("\r\n\r\n");
    head = at < 0 ? bin : bin.slice(0, at);
    body = at < 0 ? "" : bin.slice(at + 4);
  }
  const fields: Field[] = [];
  let lines: string[] | null = null;
  const flush = () => {
    if (lines === null) return;
    const raw = lines.join("\r\n");
    const colon = raw.indexOf(":");
    const name = trimWsp(raw.slice(0, colon));
    if (colon > 0 && FIELD_NAME.test(name)) fields.push({ lower: name.toLowerCase(), raw });
    lines = null;
  };
  for (const line of head === "" ? [] : head.split("\r\n")) {
    if (line.startsWith(" ") || line.startsWith("\t")) {
      if (lines !== null) lines.push(line);
      continue;
    }
    flush();
    lines = line.includes(":") ? [line] : null;
  }
  flush();
  return { fields, body };
}

const isWsp = (c: string) => c === " " || c === "\t";
const isFws = (c: string) => c === " " || c === "\t" || c === "\r" || c === "\n";

/** `s` without leading and trailing characters `strip` accepts. A loop, because a regex anchored at
 * `$` backtracks over a long run on hostile input. */
function trimBy(s: string, strip: (c: string) => boolean): string {
  let a = 0;
  let b = s.length;
  while (a < b && strip(s[a])) a++;
  while (b > a && strip(s[b - 1])) b--;
  return s.slice(a, b);
}
const trimWsp = (s: string) => trimBy(s, isWsp);
const trimFws = (s: string) => trimBy(s, isFws);

/** Raw 8-bit header text (RFC 6532) is UTF-8; pure ASCII passes through untouched. */
function headerText(binary: string): string {
  return /[\x80-\xff]/.test(binary) ? new TextDecoder().decode(toBytes(binary)) : binary;
}

/** A field's value after its colon: unfolded, each WSP run one space, trimmed. */
function unfolded(raw: string): string {
  return trimWsp(raw.slice(raw.indexOf(":") + 1).replaceAll("\r\n", "").replace(/[ \t]+/g, " "));
}

// ---------------------------------------------------------------- canonicalisation (RFC 6376 §3.4)

/** One field in canonical form, without its CRLF. Relaxed: the name lower-cased, the value unfolded
 * with each WSP run one space, no WSP around the colon or at the ends. */
export function canonicalHeader(raw: string, mode: Canon): string {
  if (mode === "simple") return raw;
  return trimWsp(raw.slice(0, raw.indexOf(":"))).toLowerCase() + ":" + unfolded(raw);
}

/** A CRLF body in canonical form. Simple: trailing empty lines dropped, an empty body one CRLF.
 * Relaxed: each WSP run one space, none at a line's end, trailing empty lines dropped, an empty body
 * empty (RFC 6376 errata 3192). A last line without its CRLF gains one either way. */
export function canonicalBody(body: string, mode: Canon): string {
  if (mode === "simple") {
    let end = body.length;
    while (end >= 2 && body.charCodeAt(end - 2) === 13 && body.charCodeAt(end - 1) === 10) end -= 2;
    return end === 0 ? "\r\n" : body.slice(0, end) + "\r\n";
  }
  const lines = body.split("\r\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].replace(/[ \t]+/g, " ");
    lines[i] = line.endsWith(" ") ? line.slice(0, -1) : line;
  }
  let n = lines.length;
  while (n > 0 && lines[n - 1] === "") n--;
  return n === 0 ? "" : lines.slice(0, n).join("\r\n") + "\r\n";
}

// ---------------------------------------------------------------- tags, names, base64

type Tags = Map<string, string>;

/** A tag-list (RFC 6376 §3.2); `null` on a malformed or repeated tag. Values are trimmed of FWS. */
function tagList(value: string): Tags | null {
  const out: Tags = new Map();
  for (const part of value.split(";")) {
    const t = trimFws(part);
    if (t === "") continue;
    const eq = t.indexOf("=");
    if (eq < 1) return null;
    const name = trimFws(t.slice(0, eq));
    if (!/^[A-Za-z][A-Za-z0-9_]*$/.test(name) || out.has(name)) return null;
    out.set(name, trimFws(t.slice(eq + 1)));
  }
  return out;
}

const fieldTags = (f: Field) => tagList(f.raw.slice(f.raw.indexOf(":") + 1));

/** Strict base64 once FWS is removed; `null` for anything else, the empty string included. */
function base64(s: string): Uint8Array<ArrayBuffer> | null {
  const t = s.replace(/[ \t\r\n]+/g, "");
  if (t === "" || t.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(t)) return null;
  try {
    return Uint8Array.from(atob(t), (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

/** A DNS name (a domain or a selector), lower-cased; `null` unless every label is a plain label. */
function dnsName(s: string): string | null {
  const v = s.toLowerCase();
  if (v === "" || v.length > 253) return null;
  for (const label of v.split(".")) {
    if (!/^[a-z0-9_](?:[a-z0-9_-]{0,61}[a-z0-9_])?$/.test(label)) return null;
  }
  return v;
}

/** A signature field with its `b=` value removed (RFC 6376 §3.7): the base64 and the folding inside
 * it go, the trailing FWS and every other byte stay. */
function withoutB(raw: string): string {
  const colon = raw.indexOf(":");
  const parts = raw.slice(colon + 1).split(";");
  for (let k = 0; k < parts.length; k++) {
    const p = parts[k];
    const eq = p.indexOf("=");
    if (eq < 0 || trimFws(p.slice(0, eq)) !== "b") continue;
    let last = p.length;
    while (last > eq + 1 && isFws(p[last - 1])) last--;
    parts[k] = p.slice(0, eq + 1) + p.slice(last);
    break;
  }
  return raw.slice(0, colon + 1) + parts.join(";");
}

/** The fields `names` sign, each name taking the bottom-most instance not yet taken (RFC 6376 §5.4.2).
 * A name with no instance left contributes nothing. */
function select(fields: Field[], names: string[]): Field[] {
  const byName = new Map<string, Field[]>();
  for (const f of fields) {
    const list = byName.get(f.lower);
    if (list) list.push(f);
    else byName.set(f.lower, [f]);
  }
  const out: Field[] = [];
  for (const n of names) {
    const f = byName.get(n)?.pop();
    if (f) out.push(f);
  }
  return out;
}

// ---------------------------------------------------------------- keys

type KeyOutcome = { key: CryptoKey } | { result: "permerror" | "temperror"; reason: Reason };

/** One verification's state: each key fetched once, each canonical body and body hash made once. */
interface Ctx {
  msg: Message;
  deps: DkimDeps;
  keys: Map<string, Promise<KeyOutcome>>;
  bodies: Map<Canon, string>;
  hashes: Map<string, Promise<Uint8Array>>;
}
function context(msg: Message, deps: DkimDeps): Ctx {
  return { msg, deps, keys: new Map(), bodies: new Map(), hashes: new Map() };
}

function derLength(n: number): number[] {
  if (n < 0x80) return [n];
  const out: number[] = [];
  for (let v = n; v > 0; v = Math.floor(v / 256)) out.unshift(v & 0xff);
  return [0x80 | out.length, ...out];
}

/** A bare PKCS#1 RSAPublicKey wrapped as a SubjectPublicKeyInfo (some publishers put the former in p=). */
function pkcs1ToSpki(pkcs1: Uint8Array): Uint8Array<ArrayBuffer> {
  const alg = [0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00];
  const bits = [0x03, ...derLength(pkcs1.length + 1), 0x00];
  return Uint8Array.from([
    0x30,
    ...derLength(alg.length + bits.length + pkcs1.length),
    ...alg,
    ...bits,
    ...pkcs1,
  ]);
}

async function importRsa(der: Uint8Array<ArrayBuffer>): Promise<CryptoKey | null> {
  for (const spki of [der, pkcs1ToSpki(der)]) {
    try {
      return await crypto.subtle.importKey("spki", spki, RSA, false, ["verify"]);
    } catch {
      // not this form; try the next
    }
  }
  return null;
}

async function fetchKey(deps: DkimDeps, name: string): Promise<KeyOutcome> {
  let records: unknown;
  try {
    records = await deps.resolveTxt(name);
  } catch {
    return { result: "temperror", reason: "dns" };
  }
  let tags: Tags | null = null;
  for (const r of Array.isArray(records) ? records : []) {
    const t = Array.isArray(r) ? tagList(r.join("")) : null;
    if (t === null || !t.has("p") || (t.has("v") && t.get("v") !== "DKIM1")) continue;
    tags = t;
    break;
  }
  if (tags === null) return { result: "permerror", reason: "no_key" };
  if ((tags.get("k") ?? "rsa").toLowerCase() !== "rsa") return { result: "permerror", reason: "key_type" };
  const hashes = tags.get("h");
  if (hashes !== undefined && !hashes.split(":").some((h) => trimFws(h).toLowerCase() === "sha256")) {
    return { result: "permerror", reason: "key_hash" };
  }
  const p = (tags.get("p") ?? "").replace(/[ \t\r\n]+/g, "");
  if (p === "") return { result: "permerror", reason: "key_revoked" };
  const der = base64(p);
  const key = der === null ? null : await importRsa(der);
  if (key === null) return { result: "permerror", reason: "key_syntax" };
  if ((key.algorithm as RsaHashedKeyAlgorithm).modulusLength < MIN_KEY_BITS) {
    return { result: "permerror", reason: "key_weak" };
  }
  return { key };
}

function lookupKey(ctx: Ctx, selector: string, domain: string): Promise<KeyOutcome> {
  const name = `${selector}._domainkey.${domain}`;
  let found = ctx.keys.get(name);
  if (found === undefined) {
    if (ctx.keys.size >= MAX_LOOKUPS) return Promise.resolve({ result: "temperror", reason: "too_many" });
    found = fetchKey(ctx.deps, name);
    ctx.keys.set(name, found);
  }
  return found;
}

// ---------------------------------------------------------------- one signature (DKIM or AMS)

function parseCanon(c: string | undefined): { header: Canon; body: Canon } | null {
  const [h, b, ...rest] = (c ?? "simple/simple").toLowerCase().split("/");
  const ok = (x: string) => x === "simple" || x === "relaxed";
  if (rest.length > 0 || !ok(h) || (b !== undefined && !ok(b))) return null;
  return { header: h as Canon, body: (b ?? "simple") as Canon };
}

function bodyCanon(ctx: Ctx, mode: Canon): string {
  let v = ctx.bodies.get(mode);
  if (v === undefined) {
    v = canonicalBody(ctx.msg.body, mode);
    ctx.bodies.set(mode, v);
  }
  return v;
}

function bodyHash(ctx: Ctx, mode: Canon, length: number | null): Promise<Uint8Array> {
  const k = `${mode}:${length ?? ""}`;
  let h = ctx.hashes.get(k);
  if (h === undefined) {
    const body = bodyCanon(ctx, mode);
    h = crypto.subtle.digest("SHA-256", toBytes(length === null ? body : body.slice(0, length)))
      .then((buf) => new Uint8Array(buf));
    ctx.hashes.set(k, h);
  }
  return h;
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}

/** Verify one `DKIM-Signature` or `ARC-Message-Signature` field against the message it sits in. */
async function verifySignature(
  ctx: Ctx,
  sig: Field,
  header: SignatureResult["header"],
): Promise<SignatureResult> {
  const out: SignatureResult = {
    header,
    result: "permerror",
    reason: "syntax",
    domain: null,
    selector: null,
    signedHeaders: [],
    covered: [],
  };
  const done = (result: SignatureResult["result"], reason: Reason) => ({ ...out, result, reason });
  const dkim = header === "dkim-signature";
  const t = fieldTags(sig);
  if (t === null || (dkim && t.get("v") !== "1")) return out;
  const d = dnsName(t.get("d") ?? "");
  const s = dnsName(t.get("s") ?? "");
  const h = t.get("h");
  if (d === null || s === null || h === undefined) return out;
  const names = h.split(":").map((n) => trimFws(n).toLowerCase());
  if (names.some((n) => !FIELD_NAME.test(n))) return out;
  out.domain = d;
  out.selector = s;
  out.signedHeaders = names;
  if ((t.get("a") ?? "").toLowerCase() !== "rsa-sha256") return done("permerror", "algorithm");
  const c = parseCanon(t.get("c"));
  const bh = base64(t.get("bh") ?? "");
  const b = base64(t.get("b") ?? "");
  if (c === null || bh === null || b === null) return out;
  if (dkim && !names.includes("from")) return done("permerror", "from_unsigned");
  const auid = dkim ? t.get("i") : undefined; // on an AMS, i= is the instance
  if (auid !== undefined) {
    const at = auid.lastIndexOf("@");
    const dom = auid.slice(at + 1).toLowerCase();
    if (at < 0 || (dom !== d && !dom.endsWith("." + d))) return out;
  }
  let length: number | null = null;
  const l = t.get("l");
  if (l !== undefined) {
    if (!/^\d{1,15}$/.test(l)) return out;
    length = Number(l);
  }
  const x = t.get("x");
  if (x !== undefined) {
    if (!/^\d{1,12}$/.test(x)) return out;
    if (Number(x) * 1000 < (ctx.deps.now ?? Date.now)()) return done("fail", "expired");
  }
  if (length !== null && length > bodyCanon(ctx, c.body).length) return done("permerror", "body_length");
  if (!sameBytes(await bodyHash(ctx, c.body, length), bh)) return done("fail", "body_hash");
  const k = await lookupKey(ctx, s, d);
  if (!("key" in k)) return done(k.result, k.reason);
  const selected = select(ctx.msg.fields, names);
  let data = "";
  for (const f of selected) data += canonicalHeader(f.raw, c.header) + "\r\n";
  data += canonicalHeader(withoutB(sig.raw), c.header);
  let ok = false;
  try {
    ok = await crypto.subtle.verify(RSA, k.key, b, toBytes(data));
  } catch {
    ok = false;
  }
  if (!ok) return done("fail", "signature");
  const covered = selected.map((f) => ({ name: f.lower, value: headerText(unfolded(f.raw)) }));
  return { ...out, result: "pass", reason: null, covered };
}

/** Every `DKIM-Signature`, top to bottom. Past `MAX_SIGNATURES`, a signature is `too_many`, unread. */
export async function verifyDkim(input: MailInput, deps: DkimDeps): Promise<SignatureResult[]> {
  const ctx = context(parse(input), deps);
  const out: SignatureResult[] = [];
  for (const f of ctx.msg.fields) {
    if (f.lower !== "dkim-signature") continue;
    out.push(
      out.length < MAX_SIGNATURES ? await verifySignature(ctx, f, "dkim-signature") : {
        header: "dkim-signature",
        result: "permerror",
        reason: "too_many",
        domain: null,
        selector: null,
        signedHeaders: [],
        covered: [],
      },
    );
  }
  return out;
}

// ---------------------------------------------------------------- ARC (RFC 8617), the topmost instance

type ArcPart = "aar" | "ams" | "as";
const ARC_PART: Record<string, ArcPart> = {
  "arc-authentication-results": "aar",
  "arc-message-signature": "ams",
  "arc-seal": "as",
};

/** A set's `i=`, 1 to `MAX_ARC_INSTANCES`; `null` when it cannot be read. An AAR's `i=` is the first
 * element of its value, before the authserv-id. */
function arcInstance(f: Field, part: ArcPart): number | null {
  const v = part === "aar"
    ? tagList(f.raw.slice(f.raw.indexOf(":") + 1).split(";")[0])?.get("i")
    : fieldTags(f)?.get("i");
  if (v === undefined || !/^\d{1,2}$/.test(v)) return null;
  const i = Number(v);
  return i >= 1 && i <= MAX_ARC_INSTANCES ? i : null;
}

/** The topmost ARC instance's seal and AMS. See the file's comment for what is and is not read. */
export async function verifyArc(input: MailInput, deps: DkimDeps): Promise<ArcResult> {
  const msg = parse(input);
  const sets = new Map<number, Partial<Record<ArcPart, Field>>>();
  let top = 0;
  let any = false;
  let broken = false;
  for (const f of msg.fields) {
    const part = ARC_PART[f.lower];
    if (part === undefined) continue;
    any = true;
    const i = arcInstance(f, part);
    if (i === null) {
      broken = true;
      continue;
    }
    top = Math.max(top, i);
    const set = sets.get(i) ?? {};
    if (set[part] !== undefined) broken = true;
    set[part] = f;
    sets.set(i, set);
  }
  if (!any) return { result: "none", instance: 0 };
  const fail = (part: "structure" | "seal" | "ams", reason: Reason, temp = false): ArcResult => ({
    result: temp ? "temperror" : "fail",
    part,
    reason,
    instance: top,
  });
  if (broken || top === 0) return fail("structure", "structure");
  const chain: Record<ArcPart, Field>[] = [];
  for (let i = 1; i <= top; i++) {
    const s = sets.get(i);
    if (s?.aar === undefined || s.ams === undefined || s.as === undefined) {
      return fail("structure", "structure");
    }
    chain.push({ aar: s.aar, ams: s.ams, as: s.as });
  }
  const topSet = chain[top - 1];
  const st = fieldTags(topSet.as);
  if (st === null) return fail("seal", "syntax");
  const cv = (st.get("cv") ?? "").toLowerCase();
  if (top === 1 ? cv !== "none" : cv !== "pass" && cv !== "fail") return fail("structure", "cv");
  if ((st.get("a") ?? "").toLowerCase() !== "rsa-sha256") return fail("seal", "algorithm");
  const d = dnsName(st.get("d") ?? "");
  const s = dnsName(st.get("s") ?? "");
  const b = base64(st.get("b") ?? "");
  if (d === null || s === null || b === null) return fail("seal", "syntax");

  const ctx = context(msg, deps);
  const k = await lookupKey(ctx, s, d);
  if (!("key" in k)) return fail("seal", k.reason, k.result === "temperror");
  // The seal signs every set from 1 to N (AAR, AMS, AS), relaxed, its own b= emptied and last.
  let data = "";
  for (const [n, set] of chain.entries()) {
    data += canonicalHeader(set.aar.raw, "relaxed") + "\r\n" + canonicalHeader(set.ams.raw, "relaxed") +
      "\r\n";
    data += n < top - 1
      ? canonicalHeader(set.as.raw, "relaxed") + "\r\n"
      : canonicalHeader(withoutB(set.as.raw), "relaxed");
  }
  let ok = false;
  try {
    ok = await crypto.subtle.verify(RSA, k.key, b, toBytes(data));
  } catch {
    ok = false;
  }
  if (!ok) return fail("seal", "signature");

  const ams = await verifySignature(ctx, topSet.ams, "arc-message-signature");
  if (ams.result !== "pass" || ams.domain === null) {
    return fail("ams", ams.reason ?? "syntax", ams.result === "temperror");
  }
  const aar = headerText(unfolded(topSet.aar.raw));
  return {
    result: "pass",
    instance: top,
    cv: cv as "none" | "pass" | "fail",
    sealDomain: d,
    amsDomain: ams.domain,
    signedHeaders: ams.signedHeaders,
    covered: ams.covered,
    authResults: trimWsp(aar.slice(aar.indexOf(";") + 1)),
  };
}

// ---------------------------------------------------------------- the interop check, and production DNS

const PROVIDER_DOMAINS = new Set(["google.com", "gmail.com", "microsoft.com"]);
/** The binding headers the interop check asks about (spec §4.6 step 5; plan T2f's S0 facts). */
const BINDING_HEADERS = ["x-forwarded-for", "x-forwarded-to", "x-ms-exchange-forwardingloop", "from", "to"];
const named = (d: string | null) => d === null ? "none" : PROVIDER_DOMAINS.has(d) ? d : "other";

/**
 * One message's verdicts as lines for PQ2's interop check (`cloud/supabase/scripts/dkim_interop.ts`):
 * each signature's result and reason, the topmost ARC instance's, and whether its AMS covers each
 * binding header. Domains are named only when they are a provider's; every other word is fixed, so a
 * line carries no address, subject, body or header value.
 */
export async function interopReport(input: MailInput, deps: DkimDeps): Promise<string[]> {
  const lines: string[] = [];
  for (const [n, r] of (await verifyDkim(input, deps)).entries()) {
    lines.push(`dkim #${n + 1} ${r.result}${r.reason === null ? "" : " " + r.reason} d=${named(r.domain)}`);
  }
  const a = await verifyArc(input, deps);
  if (a.result === "none") {
    lines.push("arc none");
  } else if (a.result === "pass") {
    const covers = BINDING_HEADERS.map((h) => `${h}=${a.signedHeaders.includes(h) ? "yes" : "no"}`);
    lines.push(
      `arc i=${a.instance} pass seal=${named(a.sealDomain)} ams=${named(a.amsDomain)} cv=${a.cv} ` +
        `covers ${covers.join(" ")}`,
    );
  } else {
    lines.push(`arc i=${a.instance} ${a.result} ${a.part} ${a.reason}`);
  }
  return lines;
}

/** Production's resolver: TXT over the platform's DNS, a no-such-name as no records, any other
 * failure as a bare error (a `temperror`). The one place in this file that reaches the network. */
export function dnsTxtResolver(timeoutMs = DNS_TIMEOUT_MS): TxtResolver {
  return async (name: string) => {
    try {
      return await Deno.resolveDns(name, "TXT", { signal: AbortSignal.timeout(timeoutMs) });
    } catch (e) {
      if (e instanceof Deno.errors.NotFound) return [];
      throw new Error("dns");
    }
  };
}
