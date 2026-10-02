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
 * Anything else is `other`; T2f adds the live forward and the unbound case. Every value that binds is
 * read from the fields a passing signature covers, never from a field the signature leaves out
 * (`unsignedExtra`). Results are fixed words, ids and the confirm link: no address, header value or
 * nonce leaves this file, and nothing here logs. The caller (T2g.2) acts on the result.
 */
import { type DkimDeps, type SignatureResult, verifyDkim } from "./dkim.ts";
import { bodyText, decodeWords, type MailInput, parseHeaders } from "./mime.ts";

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
}
export type ConfirmRefusal = "dkim" | "hint_null" | "hint_mismatch" | "requester" | "window" | "link";
export type Arrival =
  | { kind: "canary"; canaryId: string; replay: boolean; forwardHint: string | null }
  | { kind: "gmail_confirm"; link: string }
  | { kind: "gmail_confirm_refused"; reason: ConfirmRefusal }
  | { kind: "batch"; batchId: string; phase: "backfill" | "recover" }
  | { kind: "other" };

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
function forwardHint(input: MailInput, mailbox: BindMailbox): string | null {
  const hint = mailbox.mailboxHint?.trim().toLowerCase();
  if (mailbox.provider !== "m365" || !hint) return null;
  const loop = parseHeaders(input).headers.find((h) =>
    h.name.toLowerCase() === "x-ms-exchange-forwardingloop"
  );
  if (loop === undefined || loop.value.length > 500) return null;
  const [who, tenant, ...rest] = loop.value.split(";").map((s) => s.trim().toLowerCase());
  return rest.length === 0 && who === hint && TENANT.test(tenant ?? "") ? tenant : null;
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

/** Which of §4.6 step 5's first three cases a message is, for the mailbox its recipient token found. */
export async function classifyArrival(
  input: MailInput,
  mailbox: BindMailbox,
  config: BindConfig,
  deps: BindDeps,
): Promise<Arrival> {
  const from = parseHeaders(input).fromAddress;
  if (from === null) return { kind: "other" };
  let verified: SignatureResult[] | null = null;
  const dkim = async () => verified ??= await verifyDkim(input, deps);

  if (from === config.canaryFrom.trim().toLowerCase()) {
    const sig = signedBy(await dkim(), from);
    const canary = sig && await byNonce(subjectOf(sig), mailbox.canaries, deps);
    if (canary) {
      const replay = canary.arrivedAt !== null;
      return { kind: "canary", canaryId: canary.id, replay, forwardHint: forwardHint(input, mailbox) };
    }
  }
  if (from === GMAIL_CONFIRM_SENDER) {
    return gmailConfirm(input, mailbox, await dkim(), (deps.now ?? Date.now)());
  }
  if (from === mailbox.mailboxHint?.trim().toLowerCase()) {
    const sig = signedBy(await dkim(), from);
    const open = mailbox.batches.filter((b) => b.closedAt === null);
    const batch = sig && await byNonce(subjectOf(sig), open, deps);
    if (batch) return { kind: "batch", batchId: batch.id, phase: batch.phase };
  }
  return { kind: "other" };
}
