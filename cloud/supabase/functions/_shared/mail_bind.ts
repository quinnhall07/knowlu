/**
 * Binding an arrival to its mailbox, part 1 (email plan T2e; spec §4.6 step 5's first three cases).
 * `classifyArrival` reads one message held in memory and says which it is, in the spec's order:
 *
 * - **Canary:** from Knowlu's canary sender (`BindConfig.canaryFrom`), with a passing DKIM signature
 *   aligned with that sender's domain, whose covered Subject carries the nonce of one of this mailbox's
 *   canaries. It carries the forwarding evidence the mailbox's hop added: for M365, the tenant id in the
 *   topmost `X-MS-Exchange-ForwardingLoop`, read only when that field names the mailbox's hint.
 * - **Gmail forwarding confirmation (E3):** From exactly `GMAIL_CONFIRM_SENDER`. It is completed only
 *   with Google's aligned DKIM, a requester (the one address in the covered Subject) equal, case-folded,
 *   to the mailbox's hint, the confirm window open, and exactly one body link on the allow-list
 *   (`confirmUrlAllowed`). Anything less is `gmail_confirm_refused` with a fixed reason.
 * - **Batch send:** From the mailbox's hint, with a passing DKIM signature aligned with the hint's domain,
 *   whose covered Subject carries the nonce of one of this mailbox's open batches.
 *
 * Part 2 (email plan T2f; step 5's live case and "anything else", and step 6):
 *
 * - **Live forward:** bound only by what the provider's own forwarding hop added. Check (i): the topmost
 *   ARC set is sealed and its AMS signed by the provider (`google.com`, `microsoft.com`) and verifies, no
 *   forwarding header has an unsigned twin of a signed instance, and for M365 the ForwardingLoop names
 *   the mailbox and its `forward_hint` tenant. Check (ii): SES's SPF verdict passed on an envelope sender
 *   that is the provider's forwarding return path for this mailbox (Gmail's
 *   `<local>+caf_=<our address, @ as =>@gmail.com`, Microsoft's SRS `<local>+SRS=…@<domain>`). PQ2's
 *   interop check (2026-10-02) found that no provider's AMS covers X-Forwarded-For, X-Forwarded-To or
 *   the ForwardingLoop, so X-Forwarded-* never bind, and the envelope carries the binding with the seal.
 *   Quinn's PQ9 amendment (2026-10-02) accepts that: on Gmail check (i) is the seal alone and check (ii)
 *   names the mailbox. A Workspace mailbox (a `google` mailbox on its own domain) binds the same way on a
 *   `+caf_=` envelope at its own domain, only when that domain's MX is Google's (`BindDeps.resolveMx`); a
 *   `googlemail.com` mailbox's envelope is at `gmail.com`. The residual risk (a leaked address plus a
 *   sender able to set that MAIL FROM on Google's relay) is accepted; Rotate mitigates it.
 * - **Unbound:** anything else.
 * - **Authenticity** (`aligned` or `unaligned`): for a live forward, only the verified hop's
 *   ARC-Authentication-Results; for a backfilled or recovered original (`originalAuthenticity`), the
 *   provider's own verifying topmost ARC set, else its delivery stamp, the topmost Authentication-Results.
 *
 * Every value that binds is read from the fields a passing signature covers, or from the envelope SES
 * reported, never from a field a signature leaves out (`unsignedExtra`). Results are fixed words, ids and
 * the confirm link: no address, header value or nonce leaves this file, and nothing here logs or throws.
 * The caller (T2g.2) acts on the result.
 */
import { type ArcResult, type DkimDeps, type SignatureResult, verifyArc, verifyDkim } from "./dkim.ts";
import { bodyText, decodeWords, type MailHeaders, type MailInput, parseHeaders } from "./mime.ts";
import { GMAIL_DOMAINS, providerOf, type ResolveMx } from "../mail/provider.ts";

export interface BindBatch {
  id: string;
  phase: "backfill" | "recover";
  nonceHmac: string;
  /** Epoch ms; `null` while the batch is open. */
  closedAt: number | null;
}
export interface BindCanary {
  id: string;
  nonceHmac: string;
  arrivedAt: number | null;
}
/** The mailbox a recipient token found (§4.6 step 2), with its batches and canaries. */
export interface BindMailbox {
  id: string;
  provider: "m365" | "google" | "other";
  mailboxHint: string | null;
  /** Epoch ms; Gmail's confirm window ends here (P21). */
  confirmUntil: number | null;
  /** The M365 tenant id a canary recorded (`mail_addresses.forward_hint`); live M365 mail binds only to it. */
  forwardHint: string | null;
  batches: BindBatch[];
  canaries: BindCanary[];
}
export interface BindConfig {
  /** Knowlu's canary sender, e.g. a fixed address on Knowlu's verified sending domain. */
  canaryFrom: string;
}
export interface BindDeps extends DkimDeps {
  /** A nonce as `mail_batches.nonce_hmac` and `mail_canaries.nonce_hmac` hold it. */
  nonceHmac: (nonce: string) => Promise<string>;
  /** MX exchanges for a Workspace mailbox's domain (PQ9); production passes `provider.ts`'s `mxResolver()`. */
  resolveMx: ResolveMx;
}
/** The SMTP envelope as the verified SES notification states it (T2g.1 reads it, never the message). */
export interface BindEnvelope {
  /** `mail.source`: the envelope sender (MAIL FROM); `null` for a null sender. */
  mailFrom: string | null;
  /** `receipt.spfVerdict.status` is `PASS`: SES's SPF check passed for `mailFrom`'s domain. */
  spfPass: boolean;
  /** The address in `receipt.recipients` whose token found this mailbox (a retired one in its grace too). */
  recipient: string;
}
/** §4.6 step 6's two words; the only thing about authentication a work item records. */
export type Authenticity = "aligned" | "unaligned";
export type ConfirmRefusal = "dkim" | "hint_null" | "hint_mismatch" | "requester" | "window" | "link";
export type Arrival =
  | { kind: "canary"; canaryId: string; replay: boolean; forwardHint: string | null }
  | { kind: "gmail_confirm"; link: string }
  | { kind: "gmail_confirm_refused"; reason: ConfirmRefusal }
  | { kind: "batch"; batchId: string; phase: "backfill" | "recover" }
  | { kind: "live"; authenticity: Authenticity }
  | { kind: "unbound" };

export const GMAIL_CONFIRM_SENDER = "forwarding-noreply@google.com";
/** Hosts Google's `vf-` confirmation link is served from; the form's action (T2g.2) uses the same list. */
export const CONFIRM_HOSTS: readonly string[] = ["mail-settings.google.com", "mail.google.com"];
const CONFIRM_PATH = /^\/mail\/vf-[A-Za-z0-9%._~\-\[\]]+$/;
const NONCE = /\[knowlu ([0-9a-f]{32})\]/g;
const MAX_NONCES = 4;
const TENANT = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const ADDRESS = /^[^\s@<>()",;:\\[\]]+@[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$/;

// ---------------------------------------------------------------- the nonce's one form

/** A new batch or canary nonce: 128 random bits, 32 lower-case hex digits. */
export function newNonce(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** The nonce as a Subject carries it: the canary's (T4.2) and each batch send's (T10). */
export function nonceTag(nonce: string): string {
  return `[knowlu ${nonce}]`;
}

/** Whether a link is Google's forwarding-confirmation page: https on an allowed host, the default port,
 * no user info, query or fragment, and a `/mail/vf-` path. The same test gates the page's form action. */
export function confirmUrlAllowed(raw: string): boolean {
  if (raw.length > 2048 || /[?#\s]/.test(raw)) return false;
  let u: URL;
  try {
    u = new URL(raw);
  } catch {
    return false;
  }
  return u.protocol === "https:" && u.username === "" && u.password === "" && u.port === "" &&
    CONFIRM_HOSTS.includes(u.hostname) && CONFIRM_PATH.test(u.pathname);
}

// ---------------------------------------------------------------- reading what a signature covers

/** The one address of a From value: its one angle-addr, else the bare value; lower-cased. `null` for a
 * list, a group or anything that is not one plain address. */
function addressOf(value: string): string | null {
  if (value.length > 1000) return null;
  const v = value.replace(/"(?:[^"\\]|\\.)*"/g, "").replace(/\([^()]*\)/g, "");
  const angles = v.match(/<[^<>]*>/g) ?? [];
  if (angles.length > 1 || (angles.length === 0 && /[<>]/.test(v))) return null;
  const addr = (angles.length === 1 ? angles[0].slice(1, -1) : v).trim().toLowerCase();
  return addr.length <= 254 && ADDRESS.test(addr) ? addr : null;
}

/** DKIM's relaxed alignment without a public-suffix list: `d=` is the domain or a parent of it, and is
 * never a bare label (no key can be published there). */
const aligned = (d: string, domain: string) => d.includes(".") && (domain === d || domain.endsWith("." + d));

/** A field the signature covers, unless an unsigned instance of the same name sits above it. */
function covered(sig: SignatureResult, name: string): string | null {
  if (sig.unsignedExtra.includes(name)) return null;
  return sig.covered.find((f) => f.name === name)?.value ?? null;
}

/** The first passing signature aligned with `address`'s domain whose covered From is `address`. */
function signedBy(results: SignatureResult[], address: string): SignatureResult | null {
  const domain = address.slice(address.lastIndexOf("@") + 1);
  return results.find((r) => {
    if (r.result !== "pass" || r.domain === null || !aligned(r.domain, domain)) return false;
    const from = covered(r, "from");
    return from !== null && addressOf(from) === address;
  }) ?? null;
}

const subjectOf = (sig: SignatureResult) => {
  const s = covered(sig, "subject");
  return s === null || s.length > 2000 ? null : decodeWords(s);
};

/** The row whose nonce the covered Subject carries; at most `MAX_NONCES` distinct nonces are tried. */
async function byNonce<T extends { nonceHmac: string }>(
  subject: string | null,
  rows: T[],
  deps: BindDeps,
): Promise<T | null> {
  if (subject === null || rows.length === 0) return null;
  const tried = new Set<string>();
  for (const m of subject.matchAll(NONCE)) {
    if (tried.has(m[1])) continue;
    if (tried.size >= MAX_NONCES) break;
    tried.add(m[1]);
    const hmac = await deps.nonceHmac(m[1]);
    const row = rows.find((r) => r.nonceHmac === hmac);
    if (row !== undefined) return row;
  }
  return null;
}

// ---------------------------------------------------------------- the three cases

/** The M365 tenant id the topmost `X-MS-Exchange-ForwardingLoop` names, when that field names this
 * mailbox; `null` for any other provider or anything else. */
function forwardHint(head: MailHeaders, mailbox: BindMailbox): string | null {
  const hint = hintOf(mailbox);
  if (mailbox.provider !== "m365" || hint === null) return null;
  const loop = loopOf(topField(head, LOOP_FIELD));
  return loop !== null && loop.who === hint ? loop.tenant : null;
}

const LOOP_FIELD = "x-ms-exchange-forwardingloop";
const hintOf = (mailbox: BindMailbox) => mailbox.mailboxHint?.trim().toLowerCase() || null;
/** The topmost field of a (lower-case) name. */
const topField = (head: MailHeaders, name: string) =>
  head.headers.find((h) => h.name.trim().toLowerCase() === name)?.value ?? null;

/** A ForwardingLoop value, `<mailbox>;<tenant GUID>` and nothing more, lower-cased. */
function loopOf(value: string | null): { who: string; tenant: string } | null {
  if (value === null || value.length > 500) return null;
  const [who, tenant, ...rest] = value.split(";").map((s) => s.trim().toLowerCase());
  return rest.length === 0 && TENANT.test(tenant ?? "") ? { who, tenant } : null;
}

const refuse = (reason: ConfirmRefusal): Arrival => ({ kind: "gmail_confirm_refused", reason });
const EMAIL_IN_TEXT = /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+/g;
const LINK_IN_TEXT = /https?:\/\/[^\s<>"']+/gi;

/** E3: Google's confirmation for this mailbox's own requester, inside its window, with one allowed link. */
function gmailConfirm(
  input: MailInput,
  mailbox: BindMailbox,
  results: SignatureResult[],
  now: number,
): Arrival {
  const sig = signedBy(results, GMAIL_CONFIRM_SENDER);
  if (sig === null) return refuse("dkim");
  const hint = mailbox.mailboxHint?.trim().toLowerCase();
  if (!hint) return refuse("hint_null");
  const requesters = new Set((subjectOf(sig) ?? "").match(EMAIL_IN_TEXT)?.map((a) => a.toLowerCase()) ?? []);
  if (requesters.size !== 1) return refuse("requester");
  if (!requesters.has(hint)) return refuse("hint_mismatch");
  if (mailbox.confirmUntil === null || now >= mailbox.confirmUntil) return refuse("window");
  const links = new Set(
    (bodyText(input).match(LINK_IN_TEXT) ?? []).map((l) => l.replace(/[.,;:!)]+$/, "")).filter(
      confirmUrlAllowed,
    ),
  );
  if (links.size !== 1) return refuse("link");
  return { kind: "gmail_confirm", link: [...links][0] };
}

// ---------------------------------------------------------------- part 2: the live forward

/** The domain each provider's forwarding hop seals its ARC set and signs its AMS with (S0, 2026-10-02). */
const PROVIDER_ARC: Partial<Record<BindMailbox["provider"], string>> = {
  google: "google.com",
  m365: "microsoft.com",
};
/** Fields that would name a mailbox. They never bind (no provider's AMS covers them), but a signed
 * instance with an unsigned twin means a sender seeded one, and nothing in that message binds. */
const FORWARD_FIELDS = ["x-forwarded-for", "x-forwarded-to", LOOP_FIELD];
type ArcPass = Extract<ArcResult, { result: "pass" }>;

const split = (address: string): [string, string] | null => {
  const at = address.lastIndexOf("@");
  return at > 0 ? [address.slice(0, at), address.slice(at + 1)] : null;
};

/** The topmost ARC set when the mailbox's provider sealed and signed it and it verifies; else `null`. */
async function providerArc(input: MailInput, provider: BindMailbox["provider"], deps: DkimDeps) {
  const domain = PROVIDER_ARC[provider];
  if (domain === undefined) return null;
  let arc: ArcResult;
  try {
    arc = await verifyArc(input, deps);
  } catch {
    return null;
  }
  return arc.result === "pass" && arc.sealDomain === domain && arc.amsDomain === domain ? arc : null;
}

/** Check (ii): SES passed SPF on the provider's forwarding return path for this mailbox and address.
 * Gmail's `+caf_=` envelope is at `gmail.com` for a `gmail.com` or `googlemail.com` mailbox, and at the
 * mailbox's own domain for a Workspace mailbox (PQ9, amended 2026-10-02; `liveForward` then also requires
 * that domain's MX to be Google's). Microsoft's SRS envelope is at the mailbox's own domain. */
function envelopeBinds(mailbox: BindMailbox, hint: string, envelope: BindEnvelope): boolean {
  const from = envelope.mailFrom?.trim().toLowerCase() ?? "";
  const own = split(hint);
  const env = from.length <= 512 && ADDRESS.test(from) ? split(from) : null;
  if (!envelope.spfPass || own === null || env === null) return false;
  if (mailbox.provider === "google") {
    if (env[1] !== (GMAIL_DOMAINS.includes(own[1]) ? "gmail.com" : own[1])) return false;
    const to = envelope.recipient.trim().toLowerCase();
    return ADDRESS.test(to) && env[0] === `${own[0]}+caf_=${to.replace("@", "=")}`;
  }
  const srs = `${own[0]}+srs=`;
  return mailbox.provider === "m365" && env[1] === own[1] && env[0].length > srs.length &&
    env[0].startsWith(srs);
}

/** PQ9's Workspace condition: a `google` mailbox off Google's own two domains binds only when its domain's
 * MX reads Google (`provider.ts`'s one table). No MX, another host, or DNS failing to answer is `false`;
 * called only once the envelope and Google's seal have passed, so a forgery never reaches DNS. */
async function googleHosted(hint: string, mailbox: BindMailbox, deps: BindDeps): Promise<boolean> {
  const domain = hint.slice(hint.lastIndexOf("@") + 1);
  if (mailbox.provider !== "google" || GMAIL_DOMAINS.includes(domain)) return true;
  try {
    return await providerOf(domain, deps.resolveMx) === "google";
  } catch {
    return false;
  }
}

/** Check (i) beyond the seal: no seeded forwarding field, and for M365 the loop naming mailbox and tenant
 * (the covered instance were the AMS to cover it; the topmost, which Microsoft's hop adds, otherwise). */
function hopNames(arc: ArcPass, head: MailHeaders, mailbox: BindMailbox, hint: string): boolean {
  if (arc.unsignedExtra.some((n) => FORWARD_FIELDS.includes(n))) return false;
  if (mailbox.provider !== "m365") return true;
  const loop = loopOf(
    arc.signedHeaders.includes(LOOP_FIELD)
      ? arc.covered.find((f) => f.name === LOOP_FIELD)?.value ?? null
      : topField(head, LOOP_FIELD),
  );
  return loop !== null && mailbox.forwardHint !== null && loop.who === hint &&
    loop.tenant === mailbox.forwardHint.trim().toLowerCase();
}

/** §4.6 step 5's live case: both checks, then step 6 on the verified hop's own record. */
async function liveForward(
  input: MailInput,
  head: MailHeaders,
  mailbox: BindMailbox,
  envelope: BindEnvelope,
  deps: BindDeps,
): Promise<Arrival> {
  const hint = hintOf(mailbox);
  if (hint === null || !envelopeBinds(mailbox, hint, envelope)) return { kind: "unbound" };
  const arc = await providerArc(input, mailbox.provider, deps);
  if (arc === null || !hopNames(arc, head, mailbox, hint)) return { kind: "unbound" };
  if (!(await googleHosted(hint, mailbox, deps))) return { kind: "unbound" };
  return { kind: "live", authenticity: verdict(arc.authResults, soleFromDomain(head), "arc") };
}

// ---------------------------------------------------------------- part 2: authenticity (step 6)

/** The authserv-id each provider's delivery stamp carries; `null` where it writes none (Microsoft's EOP
 * writes `Authentication-Results: spf=…; dkim=…; dmarc=…` with no authserv-id). */
const DELIVERY_STAMP: Partial<Record<BindMailbox["provider"], string | null>> = {
  google: "mx.google.com",
  m365: null,
};
const MAX_RESULTS = 16_384;
const DOMAIN = /^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+$/;
/** DMARC's relaxed alignment from the safe side, without a public-suffix list: the From domain or a parent
 * of it (never a bare label; no stranger can sign as a parent). A domain under the From aligns only through
 * the provider's own `dmarc=pass header.from=`, which it computed with a public-suffix list. */
const relaxed = (d: string, from: string) => DOMAIN.test(d) && aligned(d, from);
const domainOf = (s: string | undefined) => s?.slice(s.lastIndexOf("@") + 1);

/** The From domain when the message has exactly one From field with one address on a dotted domain;
 * else `null`. */
function soleFromDomain(head: MailHeaders): string | null {
  const froms = head.headers.filter((h) => h.name.trim().toLowerCase() === "from").length;
  const domain = froms === 1 && head.fromAddress !== null ? domainOf(head.fromAddress) : undefined;
  return domain !== undefined && DOMAIN.test(domain) ? domain : null;
}

/** An Authentication-Results value's elements (RFC 8601), comments and quoted strings removed; `null`
 * when it is too long or a comment does not close. */
function elements(value: string): string[] | null {
  if (value.length > MAX_RESULTS) return null;
  let v = value.replace(/"(?:[^"\\]|\\.)*"/g, " ");
  for (let i = 0; i < 8 && v.includes("("); i++) v = v.replace(/\([^()]*\)/g, " ");
  return /[()"]/.test(v) ? null : v.split(";").map((s) => s.trim());
}

/** Whether these result elements show `dmarc=pass` for the From domain, or an aligned dkim or spf pass. */
function shows(results: string[], domain: string): boolean {
  for (const element of results) {
    const [method, ...props] = element.split(/\s+/);
    const m = /^([a-z0-9-]+)(?:\/[0-9]+)?=([a-z]+)$/.exec(method.toLowerCase());
    if (m === null || m[2] !== "pass") continue;
    const p = new Map<string, string>();
    for (const t of props) {
      const eq = t.indexOf("=");
      const key = t.slice(0, eq).toLowerCase();
      if (eq > 0 && !p.has(key)) p.set(key, t.slice(eq + 1).toLowerCase());
    }
    if (m[1] === "dmarc" && p.get("header.from") === domain) return true;
    const d = m[1] === "dkim"
      ? p.get("header.d") ?? domainOf(p.get("header.i"))
      : m[1] === "spf"
      ? domainOf(p.get("smtp.mailfrom"))
      : undefined;
    if (d !== undefined && relaxed(d, domain)) return true;
  }
  return false;
}

/** Step 6 on the one record that counts: a verified ARC set's ARC-Authentication-Results (`arc`, its first
 * element the authserv-id), or a provider's delivery stamp, read only when its authserv-id is the
 * provider's own (or, for EOP, absent). */
function verdict(
  value: string,
  domain: string | null,
  record: "arc" | BindMailbox["provider"],
): Authenticity {
  const el = elements(value);
  if (el === null || domain === null) return "unaligned";
  const id = el[0].split(/\s+/)[0].toLowerCase();
  let results: string[];
  if (record === "arc") {
    results = el.slice(1);
  } else {
    const stamp = DELIVERY_STAMP[record];
    if (stamp === undefined || (stamp === null ? !id.includes("=") : id !== stamp)) return "unaligned";
    results = stamp === null ? el : el.slice(1);
  }
  return shows(results, domain) ? "aligned" : "unaligned";
}

/**
 * §4.6 step 6 for a backfilled or recovered original (one `message/rfc822` attachment of a batch send),
 * delivered to a mailbox of `provider`: the provider's own topmost ARC set when it verifies, else the
 * provider's delivery stamp, the topmost `Authentication-Results`. Nothing else in the message is read,
 * and the outer send's DKIM says nothing here. Never throws.
 */
export async function originalAuthenticity(
  original: MailInput,
  provider: BindMailbox["provider"],
  deps: DkimDeps,
): Promise<Authenticity> {
  try {
    const head = parseHeaders(original);
    const domain = soleFromDomain(head);
    if (domain === null || DELIVERY_STAMP[provider] === undefined) return "unaligned";
    const arc = await providerArc(original, provider, deps);
    if (arc !== null) return verdict(arc.authResults, domain, "arc");
    const stamp = topField(head, "authentication-results");
    return stamp === null ? "unaligned" : verdict(stamp, domain, provider);
  } catch {
    return "unaligned";
  }
}

/** Which case of §4.6 step 5 a message is, for the mailbox its recipient token found, with step 6's word
 * for a live forward. `envelope` is what the verified SES notification states. */
export async function classifyArrival(
  input: MailInput,
  mailbox: BindMailbox,
  config: BindConfig,
  envelope: BindEnvelope,
  deps: BindDeps,
): Promise<Arrival> {
  const head = parseHeaders(input);
  const from = head.fromAddress;
  let verified: SignatureResult[] | null = null;
  const dkim = async () => verified ??= await verifyDkim(input, deps);

  if (from !== null && from === config.canaryFrom.trim().toLowerCase()) {
    const sig = signedBy(await dkim(), from);
    const canary = sig && await byNonce(subjectOf(sig), mailbox.canaries, deps);
    if (canary) {
      const replay = canary.arrivedAt !== null;
      return { kind: "canary", canaryId: canary.id, replay, forwardHint: forwardHint(head, mailbox) };
    }
  }
  if (from === GMAIL_CONFIRM_SENDER) {
    return gmailConfirm(input, mailbox, await dkim(), (deps.now ?? Date.now)());
  }
  if (from !== null && from === hintOf(mailbox)) {
    const sig = signedBy(await dkim(), from);
    const open = mailbox.batches.filter((b) => b.closedAt === null);
    const batch = sig && await byNonce(subjectOf(sig), open, deps);
    if (batch) return { kind: "batch", batchId: batch.id, phase: batch.phase };
  }
  return await liveForward(input, head, mailbox, envelope, deps);
}
