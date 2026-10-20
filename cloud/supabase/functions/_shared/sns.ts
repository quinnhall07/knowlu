/**
 * SNS verification for `mail-inbound` (email spec D2 and §4.6 step 1; plan T2a). SNS cannot send a
 * bearer, so this is the function's only authentication: the message's signature, its topic, and the
 * host its signing certificate came from. **Every refusal throws the same bare 403 and reads nothing
 * more**: no field of a refused message is returned, logged or put in an error, and the checks that
 * need no network (shape, type, topic, signature version, both URLs) all run before the certificate
 * is fetched, so a message for another topic or naming another host costs no request at all.
 *
 * Only `SignatureVersion` 2 (RSA with SHA-256) is accepted; version 1 is SHA-1, and the topic is ours
 * to configure (its `SignatureVersion` attribute is set to 2 before the subscription is made).
 *
 * The certificate is trusted because of where it came from: https, port 443, exactly
 * `sns.<region>.amazonaws.com`, where the region is the configured topic's own. Its chain is TLS's
 * job; this file reads only its public key. The fetcher is injected so a test can serve the
 * certificate from 127.0.0.1:0; production passes the platform's `fetch`.
 *
 * Replay: a notification whose signed `Timestamp` is more than `MAX_SNS_AGE_MS` old, or unreadable, is
 * refused before the certificate is fetched (Checkpoint B). Inside the hour a captured notification verifies
 * again, but it names an S3 object the first delivery deleted, and `mail_seen` keys every message (§4.6),
 * so a replay does no new work.
 */
import { fail } from "./http.ts";

export type SnsFetch = (url: string, init?: RequestInit) => Promise<Response>;
export interface SnsConfig {
  /** `MAIL_SNS_TOPIC_ARN`. */
  topicArn: string;
}
export interface SnsDeps {
  /** Epoch milliseconds: the replay bound reads it (`MAX_SNS_AGE_MS`). */
  now: () => number;
  fetch: SnsFetch;
}
/** A verified notification gives back only its `Message` (SES's JSON, unparsed here). */
export type SnsVerified = { kind: "notification"; message: string } | { kind: "confirmed" };

export const MAX_SNS_BODY_CHARS = 512 * 1024; // SNS's own message cap is 256 KiB.
const MAX_CERT_BYTES = 16 * 1024;
const FETCH_TIMEOUT_MS = 10_000;
/** A notification older than this is a replay. SNS keeps one message's `Timestamp` across its retries, and
 * an HTTPS delivery policy's retries end within 3,600 seconds, so the hour holds every genuine retry. */
export const MAX_SNS_AGE_MS = 60 * 60_000;

/** The fields AWS signs, in AWS's order, per type. `Subject` is signed only when present. */
const SIGNED_FIELDS: Record<string, readonly string[]> = {
  Notification: ["Message", "MessageId", "Subject", "Timestamp", "TopicArn", "Type"],
  SubscriptionConfirmation: [
    "Message",
    "MessageId",
    "SubscribeURL",
    "Timestamp",
    "Token",
    "TopicArn",
    "Type",
  ],
};
const OPTIONAL = new Set(["Subject"]);

/** The one refusal. Thrown, never returned; its body names nothing from the message. */
function refused(): Response {
  return fail(403, "forbidden");
}

/** The topic's region, from `arn:aws:sns:<region>:<account>:<name>`; `null` if the ARN is malformed. */
export function topicRegion(topicArn: string): string | null {
  const m = /^arn:aws:sns:([a-z]{2}(?:-[a-z]+)+-\d):\d{12}:[A-Za-z0-9_-]{1,256}$/.exec(topicArn);
  return m ? m[1] : null;
}

/** Is `raw` an https URL on exactly `sns.<region>.amazonaws.com`, port 443, with no user info? */
export function onSnsHost(raw: string, region: string): URL | null {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return null;
  }
  if (url.protocol !== "https:" || url.username !== "" || url.password !== "") return null;
  if (url.port !== "" && url.port !== "443") return null;
  if (url.hostname !== `sns.${region}.amazonaws.com`) return null;
  return url;
}

/** The signing certificate's URL: on the host, a `.pem` path, no query or fragment. */
export function allowedCertUrl(raw: string, region: string): boolean {
  const url = onSnsHost(raw, region);
  return url !== null && url.search === "" && url.hash === "" && url.pathname.endsWith(".pem");
}

/** AWS's string to sign: `Key\nValue\n` for each signed field present, in order. */
export function stringToSign(msg: Record<string, string>): string | null {
  const fields = SIGNED_FIELDS[msg.Type];
  if (!fields) return null;
  let out = "";
  for (const k of fields) {
    const v = msg[k];
    if (v === undefined) {
      if (OPTIONAL.has(k)) continue;
      return null;
    }
    out += `${k}\n${v}\n`;
  }
  return out;
}

// ---- the certificate: PEM to DER, then the SubjectPublicKeyInfo out of the TBSCertificate ----

/** One DER element at `at`: its tag, where its content starts, and where it ends. */
function tlv(der: Uint8Array, at: number): { tag: number; head: number; body: number; end: number } {
  if (at + 2 > der.length) throw new Error("der");
  const tag = der[at];
  let len = der[at + 1];
  let body = at + 2;
  if (len & 0x80) {
    const n = len & 0x7f;
    if (n < 1 || n > 3 || body + n > der.length) throw new Error("der");
    len = 0;
    for (let i = 0; i < n; i++) len = (len << 8) | der[body + i];
    body += n;
  }
  const end = body + len;
  if (end > der.length) throw new Error("der");
  return { tag, head: at, body, end };
}

/** The SubjectPublicKeyInfo of an X.509 certificate in PEM, as DER bytes. Throws on any other shape. */
export function spkiFromCertPem(pem: string): Uint8Array<ArrayBuffer> {
  const m = /-----BEGIN CERTIFICATE-----([A-Za-z0-9+/=\s]+)-----END CERTIFICATE-----/.exec(pem);
  if (!m) throw new Error("pem");
  const der = Uint8Array.from(atob(m[1].replace(/\s+/g, "")), (c) => c.charCodeAt(0));
  const cert = tlv(der, 0);
  if (cert.tag !== 0x30) throw new Error("der");
  const tbs = tlv(der, cert.body);
  if (tbs.tag !== 0x30) throw new Error("der");
  let at = tbs.body;
  let el = tlv(der, at);
  if (el.tag === 0xa0) at = el.end; // [0] version
  // serialNumber, signature, issuer, validity, subject: five elements, then the key.
  const want = [0x02, 0x30, 0x30, 0x30, 0x30];
  for (const tag of want) {
    el = tlv(der, at);
    if (el.tag !== tag) throw new Error("der");
    at = el.end;
  }
  const spki = tlv(der, at);
  if (spki.tag !== 0x30 || spki.end > tbs.end) throw new Error("der");
  return der.slice(spki.head, spki.end);
}

async function fetchCert(url: string, deps: SnsDeps): Promise<CryptoKey> {
  const res = await deps.fetch(url, { redirect: "manual", signal: AbortSignal.timeout(FETCH_TIMEOUT_MS) });
  if (res.status !== 200) {
    await res.body?.cancel();
    throw new Error("cert");
  }
  const buf = new Uint8Array(await res.arrayBuffer());
  if (buf.length > MAX_CERT_BYTES) throw new Error("cert");
  const spki = spkiFromCertPem(new TextDecoder().decode(buf));
  return await crypto.subtle.importKey(
    "spki",
    spki,
    { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
    false,
    ["verify"],
  );
}

function base64Bytes(s: string): Uint8Array<ArrayBuffer> | null {
  if (s === "" || !/^[A-Za-z0-9+/]+={0,2}$/.test(s)) return null;
  try {
    return Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

/** The body as a flat map of strings, or `null` if it is anything else. */
function fields(text: string): Record<string, string> | null {
  let v: unknown;
  try {
    v = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof v !== "object" || v === null || Array.isArray(v)) return null;
  const out: Record<string, string> = {};
  for (const [k, x] of Object.entries(v)) {
    if (typeof x === "string") out[k] = x;
  }
  return out;
}

/**
 * Verify one SNS delivery. Returns the notification's `Message`, or `confirmed` after a handshake for
 * the configured topic has been confirmed by visiting its `SubscribeURL`. **Throws** the bare 403 on
 * every refusal, and a 500 only when a verified handshake's confirmation fails (so SNS retries).
 */
export async function verifySns(req: Request, cfg: SnsConfig, deps: SnsDeps): Promise<SnsVerified> {
  const region = topicRegion(cfg.topicArn);
  if (region === null) throw refused();
  const text = await req.text();
  if (text.length > MAX_SNS_BODY_CHARS) throw refused();
  const msg = fields(text);
  if (msg === null) throw refused();
  if (msg.Type !== "Notification" && msg.Type !== "SubscriptionConfirmation") throw refused();
  if (msg.TopicArn !== cfg.topicArn) throw refused();
  if (msg.SignatureVersion !== "2") throw refused();
  if (msg.SigningCertURL === undefined || !allowedCertUrl(msg.SigningCertURL, region)) throw refused();
  const sentAt = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,3})?Z$/.test(msg.Timestamp ?? "")
    ? Date.parse(msg.Timestamp!)
    : NaN;
  if (!Number.isFinite(sentAt) || deps.now() - sentAt > MAX_SNS_AGE_MS) throw refused();
  const handshake = msg.Type === "SubscriptionConfirmation";
  if (handshake && (msg.SubscribeURL === undefined || onSnsHost(msg.SubscribeURL, region) === null)) {
    throw refused();
  }
  const signed = stringToSign(msg);
  const signature = msg.Signature === undefined ? null : base64Bytes(msg.Signature);
  if (signed === null || signature === null || msg.Message === undefined) throw refused();

  let ok = false;
  try {
    const key = await fetchCert(msg.SigningCertURL, deps);
    ok = await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, signature, new TextEncoder().encode(signed));
  } catch {
    ok = false;
  }
  if (!ok) throw refused();

  if (!handshake) return { kind: "notification", message: msg.Message };
  let confirmed = false;
  try {
    const res = await deps.fetch(msg.SubscribeURL, {
      redirect: "manual",
      signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    });
    confirmed = res.status === 200;
    await res.body?.cancel();
  } catch {
    confirmed = false;
  }
  if (!confirmed) throw fail(500, "subscription confirmation failed");
  return { kind: "confirmed" };
}
