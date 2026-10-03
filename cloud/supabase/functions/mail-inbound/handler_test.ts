// Email plan T2g.1 (spec §4.6 steps 1–4 and 7–8, D8; tests 1, 2, 7, 15 and 16, the intake halves). The
// handler runs whole: SNS verification against a certificate host and S3 against a fake bucket, both one
// listener on 127.0.0.1:0 reached through injected fetchers that keep the real-shaped AWS URLs; DKIM keys
// served by an injected resolver; the database is an in-memory store. Every token, address, key, domain
// and message here is invented for these tests: no real capture is used.
import { assert, assertEquals } from "@std/assert";
import { canonicalBody, canonicalHeader, type TxtResolver } from "../_shared/dkim.ts";
import { type BindBatch, type BindCanary, nonceTag } from "../_shared/mail_bind.ts";
import { s3Client } from "../_shared/s3.ts";
import {
  type AddressRow,
  type Counter,
  handle,
  type InboundConfig,
  type InboundDeps,
  type InboundStore,
} from "./handler.ts";
import { hmacKey, nonceHmac, tokenHmac } from "./keys.ts";

const TOPIC = "arn:aws:sns:us-east-2:000000000000:mail-test-topic";
const CERT_URL = "https://sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem";
const BUCKET = "mail-test-bucket";
const DOMAIN = "in.knowlu.test";
const CFG: InboundConfig = { topicArn: TOPIC, mailDomain: DOMAIN, bucket: BUCKET, canaryFrom: "check@knowlu.test" };
const NOW = Date.UTC(2026, 9, 2, 12, 0, 30);
const MARKER = "zq-marker-7731";

const enc = new TextEncoder();
const bin = (s: string) => Uint8Array.from(s, (c) => c.charCodeAt(0) & 0xff);
const b64 = (b: Uint8Array) => btoa(String.fromCharCode(...b));
const RSA = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;
const rsaKey = () =>
  crypto.subtle.generateKey(
    { ...RSA, modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]) },
    true,
    ["sign", "verify"],
  );
const signData = async (key: CryptoKeyPair, data: string) =>
  b64(new Uint8Array(await crypto.subtle.sign(RSA, key.privateKey, bin(data))));

// ---------------------------------------------------------------- SNS: a certificate made here

function derLen(n: number): number[] {
  if (n < 0x80) return [n];
  const out: number[] = [];
  for (let v = n; v > 0; v >>= 8) out.unshift(v & 0xff);
  return [0x80 | out.length, ...out];
}
function der(tag: number, ...parts: Uint8Array[]): Uint8Array {
  const body = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array([tag, ...derLen(body), ...new Uint8Array(body)]);
  let at = out.length - body;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}
const bytes = (...b: number[]) => new Uint8Array(b);
function certPem(spki: Uint8Array): string {
  const algId = der(0x30, der(0x06, bytes(0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b)), bytes(5, 0));
  const time = der(0x17, enc.encode("260101000000Z"));
  const tbs = der(
    0x30,
    der(0xa0, der(0x02, bytes(2))),
    der(0x02, bytes(1)),
    algId,
    der(0x30),
    der(0x30, time, time),
    der(0x30),
    spki,
  );
  const cert = der(0x30, tbs, algId, der(0x03, bytes(0, 1, 2, 3)));
  return `-----BEGIN CERTIFICATE-----\n${b64(cert).replace(/(.{64})/g, "$1\n")}\n-----END CERTIFICATE-----\n`;
}
const SNS_KEY = await rsaKey();
const PEM = certPem(new Uint8Array(await crypto.subtle.exportKey("spki", SNS_KEY.publicKey)));
const SIGNED = ["Message", "MessageId", "Subject", "Timestamp", "TopicArn", "Type"];

interface Delivery {
  /** The SES object key under `inbound/`. */
  key: string;
  recipients: string[];
  /** `mail.source`, the envelope sender. */
  source?: string | null;
  spf?: string;
  topic?: string;
  /** Sign with another key: a forged notification. */
  forge?: CryptoKeyPair;
}

/** One SNS delivery of SES's S3-action receipt: headers and recipients, never the body. The headers SES
 * copies into the notification carry the marker too, so a log of the notification would show it. */
async function delivery(d: Delivery): Promise<Request> {
  const message = JSON.stringify({
    notificationType: "Received",
    mail: {
      timestamp: "2026-10-02T12:00:00.000Z",
      source: d.source ?? "bounce@sender.test",
      messageId: d.key.slice("inbound/".length),
      destination: d.recipients,
      headers: [{ name: "Subject", value: `Lab 5 moved ${MARKER}` }],
      commonHeaders: { subject: `Lab 5 moved ${MARKER}` },
    },
    receipt: {
      timestamp: "2026-10-02T12:00:00.000Z",
      recipients: d.recipients,
      spfVerdict: { status: d.spf ?? "PASS" },
      dkimVerdict: { status: "PASS" },
      action: { type: "S3", topicArn: TOPIC, bucketName: BUCKET, objectKey: d.key },
    },
  });
  const fields: Record<string, string> = {
    Type: "Notification",
    MessageId: crypto.randomUUID(),
    TopicArn: d.topic ?? TOPIC,
    Subject: "Amazon SES Email Receipt Notification",
    Message: message,
    Timestamp: "2026-10-02T12:00:00.000Z",
    SignatureVersion: "2",
    SigningCertURL: CERT_URL,
  };
  const text = SIGNED.filter((k) => fields[k] !== undefined).map((k) => `${k}\n${fields[k]}\n`).join("");
  const sig = await signData(d.forge ?? SNS_KEY, text);
  return new Request("http://127.0.0.1/mail-inbound", {
    method: "POST",
    headers: { "content-type": "text/plain; charset=UTF-8" },
    body: JSON.stringify({ ...fields, Signature: sig }),
  });
}

// ---------------------------------------------------------------- the mail: forwards made here

const GOOGLE = await rsaKey(); // s1._domainkey.google.com (a test key; the name only mirrors production)
const MICROSOFT = await rsaKey(); // s1._domainkey.microsoft.com (a test key, as GOOGLE)
const keyRecord = async (k: CryptoKeyPair) => [[
  `v=DKIM1; k=rsa; p=${b64(new Uint8Array(await crypto.subtle.exportKey("spki", k.publicKey)))}`,
]];
const KNOWLU = await rsaKey(); // s1._domainkey.knowlu.test: the canary sender's domain
const GMAIL = await rsaKey(); // s1._domainkey.gmail.com: the student's own sends (a test key)
const ZONE: Record<string, string[][]> = {
  "s1._domainkey.google.com": await keyRecord(GOOGLE),
  "s1._domainkey.microsoft.com": await keyRecord(MICROSOFT),
  "s1._domainkey.knowlu.test": await keyRecord(KNOWLU),
  "s1._domainkey.gmail.com": await keyRecord(GMAIL),
};
const resolveTxt: TxtResolver = (name) => Promise.resolve(ZONE[name] ?? []);

/** The relaxed lines of the fields `names` selects, bottom-up, each ending CRLF. */
function picked(head: string[], names: string[]): string {
  const left = [...head];
  let data = "";
  for (const name of names) {
    for (let i = left.length - 1; i >= 0; i--) {
      if (left[i].slice(0, left[i].indexOf(":")).trim().toLowerCase() !== name) continue;
      data += canonicalHeader(left[i], "relaxed") + "\r\n";
      left.splice(i, 1);
      break;
    }
  }
  return data;
}

interface Hop {
  key: CryptoKeyPair;
  d: string;
  h: string[];
  aar: string;
  adds: string[];
}
/** One forwarding hop's fields on top, then its ARC set (i=1), as Gmail and Microsoft add theirs. */
async function forwarded(head: string[], body: string, o: Hop): Promise<string> {
  const all = [...o.adds, ...head];
  const bh = b64(new Uint8Array(await crypto.subtle.digest("SHA-256", bin(canonicalBody(body, "relaxed")))));
  const amsHead = `ARC-Message-Signature: i=1; a=rsa-sha256; c=relaxed/relaxed; d=${o.d}; s=s1;` +
    ` h=${o.h.join(":")}; bh=${bh}; b=`;
  const ams = amsHead + await signData(o.key, picked(all, o.h) + canonicalHeader(amsHead, "relaxed"));
  const aar = `ARC-Authentication-Results: i=1; ${o.aar}`;
  const asHead = `ARC-Seal: i=1; a=rsa-sha256; t=1790000000; cv=none; d=${o.d}; s=s1; b=`;
  const set = canonicalHeader(aar, "relaxed") + "\r\n" + canonicalHeader(ams, "relaxed") + "\r\n";
  const seal = asHead + await signData(o.key, set + canonicalHeader(asHead, "relaxed"));
  return [seal, ams, aar, ...all].join("\r\n") + "\r\n\r\n" + body;
}

const BODY = `Lab 5 is now due Friday at noon. ${MARKER}\r\n`;
/** The original as the professor sent it, the marker in its display name, Subject, Message-ID and body. */
function original(o: { messageId?: boolean; from?: string } = {}): string[] {
  return [
    `From: ${o.from ?? `Prof ${MARKER} <prof@school.test>`}`,
    "To: jane@gmail.com",
    `Subject: Lab 5 moved ${MARKER}`,
    "Date: Fri, 02 Oct 2026 09:00:00 -0500",
    ...(o.messageId === false ? [] : [`Message-ID: <${MARKER}.m1@school.test>`]),
  ];
}
const PASS_AAR = "mx.google.com; dkim=pass header.i=@school.test header.s=s1 header.b=AbCd;" +
  " spf=pass smtp.mailfrom=prof@school.test; dmarc=pass (p=REJECT) header.from=school.test";
const FAIL_AAR = "mx.google.com; dkim=none; spf=softfail smtp.mailfrom=bounce@evil.test;" +
  " dmarc=fail (p=NONE) header.from=school.test";

/** A Gmail forward as S0 saw it: X-Forwarded-* on top, Google's ARC set not covering them. */
function gmailForward(o: { aar?: string; messageId?: boolean; to?: string } = {}): Promise<string> {
  const to = o.to ?? `${TOK1}@${DOMAIN}`;
  return forwarded(original(o), BODY, {
    key: GOOGLE,
    d: "google.com",
    h: ["subject", "to", "from"],
    aar: o.aar ?? PASS_AAR,
    adds: [`X-Forwarded-For: jane@gmail.com ${to}`, `X-Forwarded-To: ${to}`],
  });
}
const TENANT = "72f988bf-86f1-41af-91ab-2d7cd011db47";
const TENANT2 = "0d5f1a7c-1111-4abc-9def-0123456789ab";
/** An M365 forward whose ForwardingLoop names `tenant`. */
function m365Forward(tenant: string): Promise<string> {
  return forwarded(original(), BODY, {
    key: MICROSOFT,
    d: "microsoft.com",
    h: ["from", "date", "subject", "message-id"],
    aar: "mx.microsoft.com 1; dmarc=pass action=none header.from=school.test; dkim=pass header.d=school.test",
    adds: [`X-MS-Exchange-ForwardingLoop: jdoe@school.test;${tenant}`],
  });
}
/** A direct send that writes a forwarding line naming the student: no provider hop at all. */
const directForgery = () =>
  [`X-Forwarded-For: jane@gmail.com ${TOK1}@${DOMAIN}`, ...original()].join("\r\n") + "\r\n\r\n" + BODY;

// The tokens (26 characters of lower-case base32) and the envelopes their forwards carry.
const TOK1 = "abcdefghijklmnopqrstuvwxyz"; // jane@gmail.com, account 1
const TOK2 = "bcdefghijklmnopqrstuvwxyz2"; // joe@gmail.com, account 2
const RETIRED = "cdefghijklmnopqrstuvwxyz23"; // jane's token before a Rotate
const REVOKED = "defghijklmnopqrstuvwxyz234";
const TOK_M365 = "efghijklmnopqrstuvwxyz2345"; // jdoe@school.test, account 1
const TOK_LAPSED = "fghijklmnopqrstuvwxyz23456"; // sam@gmail.com, account 3
const UNKNOWN = "ghijklmnopqrstuvwxyz234567";
const at = (tok: string) => `${tok}@${DOMAIN}`;
const caf = (local: string, tok: string) => `${local}+caf_=${tok}=${DOMAIN}@gmail.com`;

// ---------------------------------------------------------------- the store, the bucket, the logs

const ADDRESS_KEY = await hmacKey(enc.encode("test-address-key-0000000000000000"));
const FP_RAW: Record<string, Uint8Array> = {
  "acct-1": enc.encode("fingerprint-key-of-account-one-01"),
  "acct-2": enc.encode("fingerprint-key-of-account-two-02"),
  "acct-3": enc.encode("fingerprint-key-of-account-three3"),
};

type Seen = Parameters<InboundStore["handOff"]>[1];
type Work = Parameters<InboundStore["handOff"]>[0];

/** The database as intake sees it; every call is recorded by name. */
class FakeStore implements InboundStore {
  addresses: AddressRow[] = [];
  entitledAccounts = new Set(["acct-1", "acct-2"]);
  seenRows: Seen[] = [];
  workRows: Work[] = [];
  counters: Record<string, number> = {};
  receivedAt: Record<string, string> = {};
  failHandOff = false;
  calls: string[] = [];
  /** Each mailbox's open batches and canaries, as `bindRows` answers them. */
  batches: Record<string, BindBatch[]> = {};
  canaries: Record<string, BindCanary[]> = {};
  /** `mail_batches.received`, by batch id. */
  batchReceivedCounts: Record<string, number> = {};
  /** `mail_addresses.last_canary_ok_at`, by address id. */
  canaryOk: Record<string, string> = {};
  canaryArrived(
    canaryId: string,
    addressId: string,
    forwardHint: string | null,
    iso: string,
    current: boolean,
  ) {
    this.calls.push("canaryArrived");
    const c = (this.canaries[addressId] ?? []).find((x) => x.id === canaryId);
    if (c === undefined || c.arrivedAt !== null) return Promise.resolve(false);
    c.arrivedAt = Date.parse(iso);
    const a = this.addresses.find((x) => x.id === addressId)!;
    if (forwardHint !== null) a.forwardHint = forwardHint;
    this.canaryOk[addressId] = iso;
    const promotes = ["pending", "confirmed", "quiet", "blocked", "off"];
    if (current && promotes.includes(a.status)) a.status = "active";
    return Promise.resolve(true);
  }
  endRotateGrace(addressId: string) {
    this.calls.push("endRotateGrace");
    const a = this.addresses.find((x) => x.id === addressId)!;
    a.retiredTokenHmac = null;
    a.retiredUntil = null;
    return Promise.resolve();
  }
  confirmed(addressId: string) {
    this.calls.push("confirmed");
    const a = this.addresses.find((x) => x.id === addressId)!;
    if (a.status === "pending") a.status = "confirmed";
    a.confirmUntil = null;
    return Promise.resolve();
  }
  batchReceived(batchId: string, _addressId: string, n: number) {
    this.calls.push("batchReceived");
    this.batchReceivedCounts[batchId] = (this.batchReceivedCounts[batchId] ?? 0) + n;
    return Promise.resolve();
  }
  async addressByTokenHmac(hmac: string) {
    this.calls.push("addressByTokenHmac");
    return await Promise.resolve(
      this.addresses.find((a) => a.tokenHmac === hmac || a.retiredTokenHmac === hmac) ?? null,
    );
  }
  entitled(accountId: string) {
    this.calls.push("entitled");
    return Promise.resolve(this.entitledAccounts.has(accountId));
  }
  markLapsed(addressId: string) {
    this.calls.push("markLapsed");
    const a = this.addresses.find((x) => x.id === addressId)!;
    a.status = "lapsed";
    a.lapsedSince ??= new Date(NOW).toISOString();
    return Promise.resolve();
  }
  bindRows(addressId: string) {
    this.calls.push("bindRows");
    return Promise.resolve({
      batches: (this.batches[addressId] ?? []).map((b) => ({ ...b })),
      canaries: (this.canaries[addressId] ?? []).map((c) => ({ ...c })),
    });
  }
  async fpKey(accountId: string) {
    this.calls.push("fpKey");
    return await hmacKey(FP_RAW[accountId]);
  }
  seen(accountId: string, msgHash: string) {
    this.calls.push("seen");
    return Promise.resolve(this.seenRows.some((s) => s.accountId === accountId && s.msgHash === msgHash));
  }
  handOff(work: Work, seen: Seen) {
    this.calls.push("handOff");
    if (this.failHandOff) return Promise.reject(new Error("store"));
    if (this.seenRows.some((s) => s.accountId === seen.accountId && s.msgHash === seen.msgHash)) {
      return Promise.resolve(false);
    }
    this.seenRows.push(seen);
    this.workRows.push(work);
    return Promise.resolve(true);
  }
  /** `unbound_own_since_live` and `unbound_foreign_since_live`, by address id, as `count_mail_arrival`
   * runs them; `counters` keeps the cumulative `unbound` and `unauthenticated` columns. */
  sinceLive: Record<string, { own: number; foreign: number }> = {};
  count(addressId: string, counter: Counter) {
    this.calls.push("count");
    const run = this.sinceLive[addressId] ??= { own: 0, foreign: 0 };
    if (counter === "live" || counter === "unauthenticated") Object.assign(run, { own: 0, foreign: 0 });
    if (counter === "unbound_own") run.own++;
    if (counter === "unbound_foreign") run.foreign++;
    if (counter === "live") return Promise.resolve();
    const column = counter.startsWith("unbound") ? "unbound" : counter;
    this.counters[`${addressId}:${column}`] = (this.counters[`${addressId}:${column}`] ?? 0) + 1;
    return Promise.resolve();
  }
  received(addressId: string, iso: string) {
    this.calls.push("received");
    this.receivedAt[addressId] = iso;
    return Promise.resolve();
  }
}

function row(id: string, accountId: string, tok: string, over: Partial<AddressRow> = {}): Promise<AddressRow> {
  return tokenHmac(ADDRESS_KEY, tok).then((tokenHmac) => ({
    id,
    accountId,
    provider: "google" as const,
    status: "active",
    tokenHmac,
    retiredTokenHmac: null,
    retiredUntil: null,
    lapsedSince: null,
    mailboxHint: "jane@gmail.com",
    confirmUntil: null,
    forwardHint: null,
    ...over,
  }));
}

// ---------------------------------------------------------------- Google's confirmation page, faked

/** Google's `vf-` link as the confirmation mail carries it (an invented code). */
const CONFIRM_LINK = "https://mail-settings.google.com/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x";
interface ConfirmRequest {
  method: string;
  path: string;
  cookie: string | null;
  contentType: string | null;
  body: string;
}

/** The page's one form: a hidden field (its value carries an entity) and a nameless submit button. */
const confirmHtml = (action: string) =>
  `<!doctype html><html><head><title>Confirm</title></head><body><p>Confirm forwarding?</p>` +
  `<form method="POST" action="${action}"><input type="hidden" name="vf" value="ANGjdJ&amp;x7">` +
  `<input type="submit" value="Confirm"></form></body></html>`;

/** The fake answers by `w.confirm.mode`: the page (setting a cookie a browser would send back), a
 * redirect, or a page whose form posts off the allow-list; a POST answers Google's success page, or a
 * 303 to it under `post_redirect` (a Post/Redirect/Get answer). */
function confirmPage(w: World, req: Request, path: string, body: string): Response {
  w.confirm.requests.push({
    method: req.method,
    path,
    cookie: req.headers.get("cookie"),
    contentType: req.headers.get("content-type"),
    body,
  });
  if (req.method === "POST") {
    if (w.confirm.mode === "post_redirect") {
      const location = `${new URL(CONFIRM_LINK).pathname}-done`;
      return new Response(null, { status: 303, headers: { location } });
    }
    return new Response("<p>Confirmation Success!</p>", { status: 200 });
  }
  if (w.confirm.mode === "redirect") {
    // Relative, so a fetch that followed it would come back to this listener, never leave the machine.
    return new Response(null, { status: 302, headers: { location: `${new URL(CONFIRM_LINK).pathname}-2` } });
  }
  const action = w.confirm.mode === "offlist" ? "https://evil.test/mail/vf-collect" : CONFIRM_LINK;
  return new Response(confirmHtml(action), {
    headers: { "content-type": "text/html; charset=utf-8", "set-cookie": "GMAIL_RTT=1; Secure; HttpOnly" },
  });
}

interface KickRequest {
  url: string;
  method: string;
  token: string | null;
}
const KICK_URL = "http://127.0.0.1:9/functions/v1/mail-process";
const KICK_TOKEN = "test-jobs-token-0000";

/** D8's kick, never on the wire: `ok` answers 200, `fail` 500, and `slow` nothing until it is aborted. */
function kickFetch(w: World) {
  return (url: string, init?: RequestInit): Promise<Response> => {
    const headers = new Headers(init?.headers);
    w.kick.sent.push({ url, method: init?.method ?? "GET", token: headers.get("x-knowlu-job-token") });
    if (w.kick.mode === "ok") return Promise.resolve(new Response("{}", { status: 200 }));
    if (w.kick.mode === "fail") return Promise.resolve(new Response("{}", { status: 500 }));
    return new Promise((_, reject) => {
      init?.signal?.addEventListener("abort", () => reject(new Error("aborted")));
    });
  };
}

interface World {
  store: FakeStore;
  /** The fake bucket, by key. */
  bucket: Map<string, Uint8Array>;
  gets: number;
  failPut: boolean;
  logs: string[];
  /** Google's confirmation page, faked on the same listener: how it answers, and what reached it. */
  confirm: { mode: "ok" | "redirect" | "post_redirect" | "offlist"; requests: ConfirmRequest[] };
  /** D8's kick, faked: how `mail-process` answers, and each POST intake sent it. */
  kick: { mode: "ok" | "fail" | "slow"; sent: KickRequest[] };
  deps: InboundDeps;
  /** Put `raw` at `key` as SES would, then deliver the notification; the handler's answer. */
  arrive(raw: string, d: Delivery): Promise<Response>;
}

/** One world per test: the listener on 127.0.0.1:0 serves the certificate and the bucket. */
async function withWorld(fn: (w: World) => Promise<void>): Promise<void> {
  const store = new FakeStore();
  store.addresses = [
    await row("addr-1", "acct-1", TOK1, {
      retiredTokenHmac: await tokenHmac(ADDRESS_KEY, RETIRED),
      retiredUntil: new Date(NOW + 86_400_000).toISOString(),
    }),
    await row("addr-2", "acct-2", TOK2, { mailboxHint: "joe@gmail.com" }),
    await row("addr-3", "acct-1", REVOKED, { status: "revoked" }),
    await row("addr-4", "acct-1", TOK_M365, {
      provider: "m365",
      mailboxHint: "jdoe@school.test",
      forwardHint: TENANT,
    }),
    await row("addr-5", "acct-3", TOK_LAPSED, { mailboxHint: "sam@gmail.com" }),
  ];
  const bucket = new Map<string, Uint8Array>();
  const confirm = { mode: "ok" as World["confirm"]["mode"], requests: [] as ConfirmRequest[] };
  const kick = { mode: "ok" as World["kick"]["mode"], sent: [] as KickRequest[] };
  const w = { store, bucket, gets: 0, failPut: false, logs: [] as string[], confirm, kick } as World;
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    const path = new URL(req.url).pathname;
    if (path.endsWith(".pem")) return new Response(PEM);
    const key = decodeURIComponent(path.slice(1));
    const body = new Uint8Array(await req.arrayBuffer());
    if (path.startsWith("/mail/vf-")) return confirmPage(w, req, path, new TextDecoder().decode(body));
    if (req.method === "GET") {
      w.gets++;
      const got = bucket.get(key);
      return got ? new Response(new Uint8Array(got)) : new Response("<Error/>", { status: 404 });
    }
    if (req.method === "PUT") {
      if (w.failPut) return new Response("<Error/>", { status: 503 });
      bucket.set(key, body);
      return new Response(null, { status: 200 });
    }
    if (req.method === "DELETE") {
      bucket.delete(key);
      return new Response(null, { status: 204 });
    }
    return new Response("<Error/>", { status: 405 });
  });
  const local = (raw: string, init?: RequestInit) => {
    const u = new URL(raw);
    return fetch(`http://127.0.0.1:${server.addr.port}${u.pathname}${u.search}`, init);
  };
  const s3 = s3Client(
    { bucket: BUCKET, region: "us-east-2", accessKeyId: "AKIDTESTONLY0000000", secretAccessKey: "test-only" },
    { fetch: local, now: () => new Date(NOW) },
  );
  let n = 0;
  w.deps = {
    sns: { fetch: local },
    s3,
    store,
    bind: {
      resolveTxt,
      resolveMx: () => Promise.resolve([]),
      now: () => NOW,
      nonceHmac: (x) => nonceHmac(ADDRESS_KEY, x),
    },
    tokenHmac: (tok) => tokenHmac(ADDRESS_KEY, tok),
    now: () => NOW,
    workKey: () => `work/item-${++n}`,
    confirmFetch: local,
    kick: { url: KICK_URL, token: KICK_TOKEN, fetch: kickFetch(w), timeoutMs: 50 },
  };
  w.arrive = async (raw, d) => {
    bucket.set(d.key, bin(raw));
    return await handle(await delivery(d), CFG, w.deps);
  };
  const saved = { ...console };
  const capture = (...args: unknown[]) => w.logs.push(args.map(String).join(" "));
  Object.assign(console, { log: capture, info: capture, warn: capture, error: capture, debug: capture });
  try {
    await fn(w);
  } finally {
    Object.assign(console, saved);
    await server.shutdown();
  }
}

const keysUnder = (w: World, prefix: string) => [...w.bucket.keys()].filter((k) => k.startsWith(prefix)).sort();
const SRS = "jdoe+SRS=Ab1C=2Q=school.test=prof@school.test";

// ---------------------------------------------------------------- tests 1 and 2: verify, find, entitle

Deno.test("a_bad_notification_answers_403_and_reads_nothing", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    const forged = await w.arrive(raw, { key: "inbound/f1", recipients: [at(TOK1)], forge: await rsaKey() });
    assertEquals(forged.status, 403);
    const topic = await w.arrive(raw, {
      key: "inbound/f2",
      recipients: [at(TOK1)],
      topic: "arn:aws:sns:us-east-2:000000000000:someone-elses-topic",
    });
    assertEquals(topic.status, 403);
    const get = await handle(new Request("http://127.0.0.1/mail-inbound"), CFG, w.deps);
    assertEquals(get.status, 405);
    // Nothing was read: no lookup, no GET, and the objects are where SES put them.
    assertEquals(w.store.calls, []);
    assertEquals(w.gets, 0);
    assertEquals(keysUnder(w, "inbound/"), ["inbound/f1", "inbound/f2"]);
  });
});

Deno.test("an_unknown_or_revoked_token_is_deleted_with_zero_s3_gets", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    for (const [key, to] of [["inbound/u1", at(UNKNOWN)], ["inbound/u2", at(REVOKED)], ["inbound/u3", "x@else.test"]]) {
      const res = await w.arrive(raw, { key, recipients: [to], source: caf("jane", TOK1) });
      assertEquals(res.status, 200, to);
    }
    assertEquals(w.gets, 0);
    assertEquals(keysUnder(w, "inbound/"), []);
    // Counted globally, never per account; nothing else is touched.
    assertEquals(w.store.counters, {});
    assert(w.store.calls.every((c) => c === "addressByTokenHmac"), w.store.calls.join());
    assertEquals(w.store.addresses.find((a) => a.id === "addr-3")!.status, "revoked");
  });
});

Deno.test("a_lapsed_accounts_message_is_deleted_unread_with_no_seen_row_and_lapsed_since_set", async () => {
  await withWorld(async (w) => {
    const res = await w.arrive(await gmailForward({ to: at(TOK_LAPSED) }), {
      key: "inbound/l1",
      recipients: [at(TOK_LAPSED)],
      source: caf("sam", TOK_LAPSED),
    });
    assertEquals(res.status, 200);
    assertEquals(w.gets, 0);
    assertEquals(keysUnder(w, "inbound/"), []);
    assertEquals(w.store.seenRows, []);
    assertEquals(w.store.workRows, []);
    const box = w.store.addresses.find((a) => a.id === "addr-5")!;
    assertEquals(box.status, "lapsed");
    assertEquals(box.lapsedSince, new Date(NOW).toISOString());
    assert(!w.store.calls.includes("fpKey") && !w.store.calls.includes("seen"), w.store.calls.join());
  });
});

Deno.test("a_retired_token_inside_its_rotate_grace_still_binds", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward({ to: at(RETIRED) });
    const res = await w.arrive(raw, { key: "inbound/r1", recipients: [at(RETIRED)], source: caf("jane", RETIRED) });
    assertEquals(res.status, 200);
    assertEquals(w.store.workRows.map((x) => [x.addressId, x.phase, x.authenticity]), [["addr-1", "live", "aligned"]]);
    // Past its grace the retired token finds nothing: deleted unread.
    w.store.addresses[0].retiredUntil = new Date(NOW - 1).toISOString();
    const gets = w.gets;
    const late = await w.arrive(raw, { key: "inbound/r2", recipients: [at(RETIRED)], source: caf("jane", RETIRED) });
    assertEquals(late.status, 200);
    assertEquals(w.gets, gets);
    assertEquals(w.store.workRows.length, 1);
    assertEquals(keysUnder(w, "inbound/"), []);
  });
});

Deno.test("no_forgery_moves_last_received_at_or_charges_a_cap", async () => {
  await withWorld(async (w) => {
    const forgeries: Array<[string, Delivery]> = [
      // A direct send carrying a forged X-Forwarded-For naming the student and the address.
      [directForgery(), { key: "inbound/g1", recipients: [at(TOK1)], source: "bounce@sender.test" }],
      // Google-sealed, from an attacker's own Gmail: its +caf_= return path names the attacker.
      [await gmailForward(), { key: "inbound/g2", recipients: [at(TOK1)], source: caf("mallory", TOK1) }],
      // An M365 forward whose ForwardingLoop names another tenant than the mailbox's forward_hint.
      [await m365Forward(TENANT2), { key: "inbound/g3", recipients: [at(TOK_M365)], source: SRS }],
    ];
    for (const [raw, d] of forgeries) assertEquals((await w.arrive(raw, d)).status, 200, d.key);
    assertEquals(w.store.receivedAt, {});
    assertEquals(w.store.workRows, []);
    assertEquals(w.store.seenRows, []);
    assertEquals(w.store.counters, { "addr-1:unbound": 2, "addr-4:unbound": 1 });
    // No work item, so `mail-process` never sees one and no cap or allowance is charged for it.
    for (const c of ["fpKey", "seen", "handOff", "received"]) assert(!w.store.calls.includes(c), c);
    assertEquals(keysUnder(w, "inbound/"), []);
    assertEquals(keysUnder(w, "work/"), []);
    // The controls: the same mailboxes' genuine forwards bind and move last_received_at.
    await w.arrive(await m365Forward(TENANT), { key: "inbound/c1", recipients: [at(TOK_M365)], source: SRS });
    // (Without a Message-ID, so account 1 does not read it as the M365 copy of the same message.)
    const own = await gmailForward({ messageId: false });
    await w.arrive(own, { key: "inbound/c2", recipients: [at(TOK1)], source: caf("jane", TOK1) });
    assertEquals(Object.keys(w.store.receivedAt).sort(), ["addr-1", "addr-4"]);
    // The arrival is SES's receipt time, as the notification states it.
    assertEquals(w.store.receivedAt["addr-1"], "2026-10-02T12:00:00.000Z");
  });
});

Deno.test("an_intra_school_forward_microsoft_did_not_srs_rewrite_is_counted_unbound_never_live", async () => {
  // Checkpoint B's open item (plan T2f): S0 did not record whether EOP SRS-rewrites a forward whose original
  // sender is on the school's own domain. If it does not, the envelope is the professor's own address: check
  // (ii) cannot name the mailbox, so the message is unbound (deleted, counted on the mailbox, the count the
  // Settings row shows), never a silent live item, and it never moves `last_received_at`.
  await withWorld(async (w) => {
    const raw = await m365Forward(TENANT);
    const d = { key: "inbound/i1", recipients: [at(TOK_M365)], source: "prof@school.test" };
    assertEquals((await w.arrive(raw, d)).status, 200);
    assertEquals([w.store.workRows, w.store.seenRows, w.store.receivedAt], [[], [], {}]);
    assertEquals(w.store.counters, { "addr-4:unbound": 1 });
    assertEquals([keysUnder(w, "inbound/"), keysUnder(w, "work/")], [[], []]);
    // The same message with the SRS envelope binds: it is the envelope alone that refuses it.
    await w.arrive(raw, { ...d, key: "inbound/i2", source: SRS });
    const bound = w.store.workRows.map((x) => [x.addressId, x.phase, x.authenticity]);
    assertEquals(bound, [["addr-4", "live", "aligned"]]);
  });
});

Deno.test("unbound_live_forwards_count_own_or_foreign_since_the_last_bound_live_forward", async () => {
  // Quinn's unbound-display ruling (2026-10-03): each unbound arrival runs one of two counts on its mailbox
  // since the last bound live forward. `own`: its From is on the hint's domain, or the mailbox's provider
  // sealed it (the "skipped" reason); `foreign`: anything else (the "leaked" reason, which offers Rotate).
  await withWorld(async (w) => {
    const intra = await m365Forward(TENANT); // From prof@school.test, Microsoft-sealed, no SRS envelope
    const noSrs = { recipients: [at(TOK_M365)], source: "prof@school.test" };
    await w.arrive(intra, { ...noSrs, key: "inbound/u1" });
    await w.arrive(intra, { ...noSrs, key: "inbound/u2" });
    // To a Gmail mailbox: a direct send from a school address, unsealed, is from outside.
    await w.arrive(directForgery(), {
      key: "inbound/u3",
      recipients: [at(TOK1)],
      source: "bounce@sender.test",
    });
    // Google-sealed, from another Gmail's forwarding: the provider's seal, so `own`.
    await w.arrive(await gmailForward(), {
      key: "inbound/u4",
      recipients: [at(TOK1)],
      source: caf("mallory", TOK1),
    });
    assertEquals(w.store.sinceLive, { "addr-4": { own: 2, foreign: 0 }, "addr-1": { own: 1, foreign: 1 } });
    // The cumulative column counts every one of them, as before.
    assertEquals(w.store.counters, { "addr-4:unbound": 2, "addr-1:unbound": 2 });
    // A bound live forward sets the mailbox's counts back to 0; the other mailbox's stay.
    await w.arrive(intra, { recipients: [at(TOK_M365)], source: SRS, key: "inbound/u5" });
    assertEquals(w.store.sinceLive["addr-4"], { own: 0, foreign: 0 });
    assertEquals(w.store.sinceLive["addr-1"], { own: 1, foreign: 1 });
    // A forward that binds and then fails step 6 is still a bound live forward: deleted, counted
    // unauthenticated, and the counts restart.
    const unaligned = await gmailForward({ aar: FAIL_AAR, messageId: false });
    await w.arrive(unaligned, { key: "inbound/u6", recipients: [at(TOK1)], source: caf("jane", TOK1) });
    assertEquals(w.store.sinceLive["addr-1"], { own: 0, foreign: 0 });
    assertEquals(w.store.counters["addr-1:unauthenticated"], 1);
    // The log lines stay fixed phrases: neither the origin nor any address is in them.
    assert(w.logs.every((l) => !l.includes("@") && !/\bown\b|\bforeign\b/.test(l)), w.logs.join("\n"));
  });
});

// ---------------------------------------------------------------- test 7: dedup and the hand-off

const JANE = (key: string): Delivery => ({ key, recipients: [at(TOK1)], source: caf("jane", TOK1) });

Deno.test("one_message_id_twice_makes_one_work_item", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    assertEquals((await w.arrive(raw, JANE("inbound/d1"))).status, 200);
    assertEquals((await w.arrive(raw, JANE("inbound/d2"))).status, 200);
    assertEquals(w.store.workRows.length, 1);
    assertEquals(w.store.seenRows.length, 1);
    assertEquals(keysUnder(w, "work/"), ["work/item-1"]);
    assertEquals(w.store.workRows[0].s3Key, "work/item-1");
    // The work object is the original, byte for byte.
    assertEquals(new TextDecoder().decode(w.bucket.get("work/item-1")), raw);
    // A message with no Message-ID dedups on its Date, From and Subject.
    const bare = await gmailForward({ messageId: false });
    await w.arrive(bare, JANE("inbound/d3"));
    await w.arrive(bare, JANE("inbound/d4"));
    assertEquals(w.store.workRows.length, 2);
    assertEquals(keysUnder(w, "inbound/"), []);
    assertEquals(keysUnder(w, "work/"), ["work/item-1", "work/item-2"]);
  });
});

async function hmacHex(raw: Uint8Array, text: string): Promise<string> {
  const k = await crypto.subtle.importKey("raw", new Uint8Array(raw), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
  ]);
  const mac = new Uint8Array(await crypto.subtle.sign("HMAC", k, enc.encode(text)));
  return Array.from(mac, (b) => b.toString(16).padStart(2, "0")).join("");
}

Deno.test("seen_hashes_are_hmacs_under_the_accounts_key", async () => {
  await withWorld(async (w) => {
    // One message forwarded by two students' Gmail, to two accounts' addresses.
    const raw = await gmailForward();
    await w.arrive(raw, JANE("inbound/h1"));
    await w.arrive(raw, { key: "inbound/h2", recipients: [at(TOK2)], source: caf("joe", TOK2) });
    assertEquals(w.store.seenRows.map((s) => s.accountId), ["acct-1", "acct-2"]);
    const [one, two] = w.store.seenRows;
    assert(one.msgHash !== two.msgHash, "the same Message-ID under two accounts hashes differently");
    assert(one.fpHash !== two.fpHash);
    for (const s of w.store.seenRows) {
      const k = FP_RAW[s.accountId];
      assertEquals(s.msgHash, await hmacHex(k, JSON.stringify(["msg", `${MARKER}.m1@school.test`])));
      assertEquals(s.fpHash, await hmacHex(k, JSON.stringify(["fp", "prof@school.test", `lab 5 moved ${MARKER}`])));
      assertEquals(s.receivedMinute, "2026-10-02T12:00:00.000Z");
      assertEquals(s.phase, "live");
      assert(!JSON.stringify(s).includes(MARKER));
    }
    assertEquals(w.store.workRows.map((x) => [x.accountId, x.addressId]), [["acct-1", "addr-1"], ["acct-2", "addr-2"]]);
  });
});

Deno.test("a_failed_put_answers_500_and_writes_neither_work_nor_seen", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    w.failPut = true;
    assertEquals((await w.arrive(raw, JANE("inbound/p1"))).status, 500);
    assertEquals(w.store.workRows, []);
    assertEquals(w.store.seenRows, []);
    assertEquals(keysUnder(w, "work/"), []);
    // The original stays where SES put it, so SNS's retry is processed as new.
    assertEquals(keysUnder(w, "inbound/"), ["inbound/p1"]);
    assertEquals(w.store.receivedAt, {});
    // A failed row write answers 500 too, and takes its work object back out.
    w.failPut = false;
    w.store.failHandOff = true;
    assertEquals((await w.arrive(raw, JANE("inbound/p1"))).status, 500);
    assertEquals(keysUnder(w, "work/"), []);
    assertEquals(keysUnder(w, "inbound/"), ["inbound/p1"]);
    // The retry.
    w.store.failHandOff = false;
    assertEquals((await w.arrive(raw, JANE("inbound/p1"))).status, 200);
    assertEquals(w.store.workRows.length, 1);
    assertEquals(keysUnder(w, "inbound/"), []);
  });
});

// ---------------------------------------------------------------- tests 15 and 16: logs and objects

Deno.test("no_intake_log_line_carries_the_messages_marker", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    await w.arrive(raw, { key: "inbound/m0", recipients: [at(TOK1)], forge: await rsaKey() });
    await w.arrive(raw, { key: "inbound/m1", recipients: [at(UNKNOWN)] });
    await w.arrive(raw, { key: "inbound/m2", recipients: [at(REVOKED)] });
    await w.arrive(raw, { key: "inbound/m3", recipients: [at(TOK_LAPSED)], source: caf("sam", TOK_LAPSED) });
    await w.arrive(directForgery(), { key: "inbound/m4", recipients: [at(TOK1)] });
    await w.arrive(await gmailForward({ aar: FAIL_AAR, messageId: false }), JANE("inbound/m5"));
    w.failPut = true;
    await w.arrive(raw, JANE("inbound/m6"));
    w.failPut = false;
    await w.arrive(raw, JANE("inbound/m6"));
    await w.arrive(raw, JANE("inbound/m7"));
    assert(w.logs.length >= 6, `the paths log their outcome (${w.logs.length} lines)`);
    for (const line of w.logs) {
      assert(!line.includes(MARKER), "a log line carries the message's marker");
      assert(!line.includes("@"), "a log line carries an address");
      for (const tok of [TOK1, UNKNOWN, REVOKED, TOK_LAPSED]) assert(!line.includes(tok), "a log line carries a token");
      assert(!line.includes("inbound/") && !line.includes("work/"), "a log line carries an object key");
    }
  });
});

Deno.test("no_object_is_left_after_unbound_revoked_unauthenticated_and_duplicate", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    await w.arrive(directForgery(), { key: "inbound/o1", recipients: [at(TOK1)] }); // unbound
    await w.arrive(raw, { key: "inbound/o2", recipients: [at(REVOKED)] }); // revoked
    await w.arrive(await gmailForward({ aar: FAIL_AAR, messageId: false }), JANE("inbound/o3")); // unauthenticated
    await w.arrive(raw, JANE("inbound/o4")); // queued
    await w.arrive(raw, JANE("inbound/o5")); // duplicate
    assertEquals(keysUnder(w, "inbound/"), []);
    // Only the queued original remains, under work/, for `mail-process` to read and delete.
    assertEquals(keysUnder(w, "work/"), ["work/item-1"]);
    assertEquals(w.store.counters, { "addr-1:unbound": 1, "addr-1:unauthenticated": 1 });
    assertEquals(w.store.workRows.length, 1);
  });
});

// ---------------------------------------------------------------- T2g.2: what each arrival does

/** A relaxed/relaxed DKIM-Signature over `head` (fields picked bottom-up) and `body`, put on top. */
async function dkimSigned(head: string[], body: string, key: CryptoKeyPair, d: string, h: string[]) {
  const bh = b64(new Uint8Array(await crypto.subtle.digest("SHA-256", bin(canonicalBody(body, "relaxed")))));
  const sig = `DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=${d}; s=s1; h=${h.join(":")}; bh=${bh}; b=`;
  return [sig + await signData(key, picked(head, h) + canonicalHeader(sig, "relaxed")), ...head];
}
const assemble = (head: string[], body: string) => head.join("\r\n") + "\r\n\r\n" + body;

const NONCE = "0123456789abcdef0123456789abcdef";
const NONCE2 = "fedcba9876543210fedcba9876543210";
const BACKFILL: BindBatch = { id: "b-1", phase: "backfill", nonceHmac: "", closedAt: null };
const RECOVER: BindBatch = { id: "b-2", phase: "recover", nonceHmac: "", closedAt: null };

/** An original as Gmail delivered it: its own Received line and delivery stamp, the marker inside. */
const ORIGINAL_A = [
  "Received: by 2002:a05:6000:1::1 with SMTP id x1; Thu, 1 Oct 2026 07:00:01 -0700",
  "Authentication-Results: mx.google.com; dkim=pass header.i=@school.test header.s=s1 header.b=AbCd;" +
  " spf=pass smtp.mailfrom=prof@school.test; dmarc=pass (p=REJECT) header.from=school.test",
  `From: Prof ${MARKER} <prof@school.test>`,
  "To: jane@gmail.com",
  `Subject: Quiz 2 moved ${MARKER}`,
  "Date: Thu, 01 Oct 2026 08:59:00 -0500",
  `Message-ID: <${MARKER}.a@school.test>`,
].join("\r\n") + `\r\n\r\nQuiz 2 is now on Monday. ${MARKER}\r\n`;
/** An original with no aligned pass in its delivery stamp, and no Received line (its Date keys it). */
const ORIGINAL_B = [
  `Authentication-Results: mx.google.com; ${FAIL_AAR.slice("mx.google.com; ".length)}`,
  `From: Prof ${MARKER} <prof@school.test>`,
  "To: jane@gmail.com",
  `Subject: Office hours ${MARKER}`,
  "Date: Wed, 30 Sep 2026 09:15:40 -0500",
  `Message-ID: <${MARKER}.b@school.test>`,
].join("\r\n") + `\r\n\r\nOffice hours move to Room 12. ${MARKER}\r\n`;
const PDF_MARK = "zq-pdf-4410";

/** The student's own batch send: From the hint, signed by gmail.com, the batch's nonce in the Subject,
 * a covering note, then each part (`eml` an attached original, `pdf` a file). */
async function batchSend(nonce: string, parts: Array<{ eml: string } | { pdf: string }>): Promise<string> {
  const segs = [`--b1\r\nContent-Type: text/plain\r\n\r\nForwarding these. ${MARKER}-outer`];
  for (const p of parts) {
    segs.push(
      "eml" in p
        ? `--b1\r\nContent-Type: message/rfc822\r\nContent-Disposition: attachment; filename="m.eml"\r\n\r\n${p.eml}`
        : `--b1\r\nContent-Type: application/pdf; name="syllabus.pdf"\r\nContent-Transfer-Encoding: base64\r\n` +
          `Content-Disposition: attachment; filename="syllabus.pdf"\r\n\r\n${btoa(p.pdf)}`,
    );
  }
  const body = segs.join("\r\n") + "\r\n--b1--\r\n";
  const head = [
    "From: Jane <jane@gmail.com>",
    `To: ${at(TOK1)}`,
    `Subject: Fwd: ${parts.length} messages ${nonceTag(nonce)}`,
    'Content-Type: multipart/mixed; boundary="b1"',
  ];
  return assemble(await dkimSigned(head, body, GMAIL, "gmail.com", ["from", "to", "subject"]), body);
}
/** Jane's batch sends travel straight from Gmail: the envelope is her own address. */
const SEND = (key: string): Delivery => ({ key, recipients: [at(TOK1)], source: "jane@gmail.com" });
const text = (b: Uint8Array | undefined) => new TextDecoder().decode(b);

async function openBatches(w: World) {
  w.store.batches["addr-1"] = [
    { ...BACKFILL, nonceHmac: await nonceHmac(ADDRESS_KEY, NONCE) },
    { ...RECOVER, nonceHmac: await nonceHmac(ADDRESS_KEY, NONCE2) },
  ];
}

Deno.test("each_rfc822_attachment_becomes_one_work_item_counted_toward_its_batch", async () => {
  await withWorld(async (w) => {
    await openBatches(w);
    const send = await batchSend(NONCE, [{ eml: ORIGINAL_A }, { eml: ORIGINAL_B }]);
    assertEquals((await w.arrive(send, SEND("inbound/s1"))).status, 200);
    assertEquals(
      w.store.workRows.map((x) => [x.addressId, x.batchId, x.phase, x.authenticity]),
      [["addr-1", "b-1", "backfill", "aligned"], ["addr-1", "b-1", "backfill", "unaligned"]],
    );
    assertEquals(w.store.batchReceivedCounts, { "b-1": 2 });
    // Each work object is the attached original, byte for byte; the outer send is never stored.
    assertEquals(keysUnder(w, "work/"), ["work/item-1", "work/item-2"]);
    assertEquals(text(w.bucket.get("work/item-1")), ORIGINAL_A);
    assertEquals(text(w.bucket.get("work/item-2")), ORIGINAL_B);
    // Each original keys on its own delivery time: the topmost Received, else its Date.
    assertEquals(
      w.store.seenRows.map((s) => [s.phase, s.receivedMinute]),
      [["backfill", "2026-10-01T14:00:00.000Z"], ["backfill", "2026-09-30T14:15:00.000Z"]],
    );
    // A batch send is not bound live mail, and nothing is counted against the address.
    assertEquals(w.store.receivedAt, {});
    assertEquals(w.store.counters, {});
    assertEquals(keysUnder(w, "inbound/"), []);
    // Sent again (a resend of the same originals): each counts toward the batch, neither is queued twice.
    await w.arrive(send, SEND("inbound/s2"));
    assertEquals(w.store.workRows.length, 2);
    assertEquals(w.store.batchReceivedCounts, { "b-1": 4 });
    assertEquals(keysUnder(w, "work/"), ["work/item-1", "work/item-2"]);
  });
});

Deno.test("a_pdf_attachment_is_ignored", async () => {
  await withWorld(async (w) => {
    await openBatches(w);
    const send = await batchSend(NONCE, [{ pdf: `%PDF-1.4 ${PDF_MARK}` }, { eml: ORIGINAL_A }]);
    assertEquals((await w.arrive(send, SEND("inbound/p1"))).status, 200);
    assertEquals(w.store.workRows.map((x) => [x.batchId, x.authenticity]), [["b-1", "aligned"]]);
    assertEquals(w.store.batchReceivedCounts, { "b-1": 1 });
    assertEquals(keysUnder(w, "work/"), ["work/item-1"]);
    for (const k of keysUnder(w, "work/")) {
      const stored = text(w.bucket.get(k));
      assert(!stored.includes(btoa(`%PDF-1.4 ${PDF_MARK}`)) && !stored.includes(PDF_MARK), "a PDF was stored");
      assert(!stored.includes(`${MARKER}-outer`), "the outer text was stored");
    }
    // A send with only a PDF queues nothing and counts nothing.
    await w.arrive(await batchSend(NONCE, [{ pdf: PDF_MARK }]), SEND("inbound/p2"));
    assertEquals(w.store.workRows.length, 1);
    assertEquals(w.store.batchReceivedCounts, { "b-1": 1 });
    assertEquals(keysUnder(w, "inbound/"), []);
  });
});

Deno.test("an_unaligned_backfilled_original_is_kept_unaligned", async () => {
  await withWorld(async (w) => {
    await openBatches(w);
    const send = await batchSend(NONCE2, [{ eml: ORIGINAL_B }]);
    assertEquals((await w.arrive(send, SEND("inbound/n1"))).status, 200);
    // Kept, as the recover batch's, and marked unaligned: never deleted as unauthenticated.
    assertEquals(w.store.workRows.map((x) => [x.batchId, x.phase, x.authenticity]), [["b-2", "recover", "unaligned"]]);
    assertEquals(text(w.bucket.get(w.store.workRows[0].s3Key)), ORIGINAL_B);
    assertEquals(w.store.counters, {});
    assertEquals(w.store.batchReceivedCounts, { "b-2": 1 });
    assertEquals(w.store.receivedAt, {});
  });
});

/** Google's forwarding confirmation for `jane@gmail.com`, signed by google.com, carrying `CONFIRM_LINK`. */
async function confirmation(): Promise<string> {
  const head = [
    "From: Gmail Team <forwarding-noreply@google.com>",
    `To: ${at(TOK1)}`,
    "Subject: (#123456789) Gmail Forwarding Confirmation - Receive Mail from jane@gmail.com",
    'Content-Type: text/plain; charset="UTF-8"',
  ];
  const body = `jane@gmail.com has requested to automatically forward mail to your email address\r\n` +
    `${at(TOK1)}.\r\nConfirmation code: 123456789\r\n\r\nTo allow jane@gmail.com to automatically forward ` +
    `mail to your address, please click the link below\r\nto confirm the request:\r\n\r\n${CONFIRM_LINK}\r\n`;
  return assemble(await dkimSigned(head, body, GOOGLE, "google.com", ["from", "to", "subject"]), body);
}
const GOOGLE_SENDS = (key: string): Delivery => ({ key, recipients: [at(TOK1)], source: "bounce@google.com" });
/** Jane is mid-setup: her mailbox is pending, its confirm window open for ten more minutes. */
function confirming(w: World) {
  const jane = w.store.addresses[0];
  jane.status = "pending";
  jane.confirmUntil = new Date(NOW + 600_000).toISOString();
  return jane;
}

Deno.test("the_confirmation_is_completed_by_a_cookieless_get_and_post_to_a_local_fake", async () => {
  await withWorld(async (w) => {
    const jane = confirming(w);
    assertEquals((await w.arrive(await confirmation(), GOOGLE_SENDS("inbound/v1"))).status, 200);
    const path = new URL(CONFIRM_LINK).pathname;
    assertEquals(w.confirm.requests.map((r) => [r.method, r.path]), [["GET", path], ["POST", path]]);
    // Cookie-less both ways: the page set a cookie, and the POST did not send it back.
    assertEquals(w.confirm.requests.map((r) => r.cookie), [null, null]);
    const post = w.confirm.requests[1];
    assertEquals(post.contentType, "application/x-www-form-urlencoded");
    assertEquals(post.body, "vf=ANGjdJ%26x7");
    // The mailbox reads `confirmed`, and its window is closed: a second confirmation is refused.
    assertEquals(jane.status, "confirmed");
    assertEquals(jane.confirmUntil, null);
    assertEquals(w.store.counters, {});
    assertEquals(w.store.workRows, []);
    assertEquals(keysUnder(w, "inbound/"), []);
    await w.arrive(await confirmation(), GOOGLE_SENDS("inbound/v2"));
    assertEquals(w.confirm.requests.length, 2);
    assertEquals(w.store.counters, { "addr-1:unbound": 1 });
  });
});

Deno.test("a_confirmation_redirect_is_refused", async () => {
  await withWorld(async (w) => {
    const jane = confirming(w);
    w.confirm.mode = "redirect";
    assertEquals((await w.arrive(await confirmation(), GOOGLE_SENDS("inbound/v3"))).status, 200);
    // The redirect is not followed and nothing is posted.
    assertEquals(w.confirm.requests.map((r) => r.method), ["GET"]);
    assertEquals(jane.status, "pending");
    assert(jane.confirmUntil !== null);
    assertEquals(w.store.counters, { "addr-1:unbound": 1 });
    assertEquals(keysUnder(w, "inbound/"), []);
    const named = w.logs.some((l) => l.includes("gmail confirmation refused (redirect)"));
    assert(named, "the page redirect is named");
  });
});

Deno.test("a_redirect_after_the_post_is_refused_and_named", async () => {
  await withWorld(async (w) => {
    const jane = confirming(w);
    w.confirm.mode = "post_redirect";
    assertEquals((await w.arrive(await confirmation(), GOOGLE_SENDS("inbound/v3b"))).status, 200);
    // The 303 is not followed: one GET, one POST, nothing after.
    assertEquals(w.confirm.requests.map((r) => r.method), ["GET", "POST"]);
    // Refused until the live proof (step 4) shows what Google answers; named apart from a page redirect.
    assertEquals(jane.status, "pending");
    assertEquals(w.store.counters, { "addr-1:unbound": 1 });
    const named = w.logs.some((l) => l.includes("gmail confirmation refused (post_redirect)"));
    assert(named, "the POST's 3xx is named");
    assert(!w.logs.some((l) => l.includes("refused (redirect)")), "a POST's 3xx reads as a page redirect");
  });
});

Deno.test("a_form_action_off_the_allow_list_is_refused", async () => {
  await withWorld(async (w) => {
    const jane = confirming(w);
    w.confirm.mode = "offlist";
    assertEquals((await w.arrive(await confirmation(), GOOGLE_SENDS("inbound/v4"))).status, 200);
    assertEquals(w.confirm.requests.map((r) => r.method), ["GET"]);
    assertEquals(jane.status, "pending");
    assertEquals(w.store.counters, { "addr-1:unbound": 1 });
    for (const line of w.logs) assert(!line.includes("evil") && !line.includes("vf-"), "a log line carries the link");
  });
});

/** Knowlu's canary as the school's forwarding hands it on: its ForwardingLoop naming `tenant` on top. */
async function canaryMail(nonce: string, tenant: string): Promise<string> {
  const head = [
    "From: Knowlu <check@knowlu.test>",
    "To: jdoe@school.test",
    `Subject: Knowlu delivery check - no need to open this ${nonceTag(nonce)}`,
    "Content-Type: text/plain",
  ];
  const body = "Knowlu sent this to check that your mail reaches it. Turn it off in Settings, Email.\r\n";
  const signed = await dkimSigned(head, body, KNOWLU, "knowlu.test", ["from", "to", "subject"]);
  return assemble([`X-MS-Exchange-ForwardingLoop: jdoe@school.test;${tenant}`, ...signed], body);
}
const SCHOOL_FWD = (key: string): Delivery => ({ key, recipients: [at(TOK_M365)], source: SRS });

Deno.test("a_canary_marks_arrived_and_records_forward_hint", async () => {
  await withWorld(async (w) => {
    const school = w.store.addresses[3];
    school.status = "pending";
    school.forwardHint = null;
    w.store.canaries["addr-4"] = [{ id: "c-1", nonceHmac: await nonceHmac(ADDRESS_KEY, NONCE), arrivedAt: null }];
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT), SCHOOL_FWD("inbound/c1"))).status, 200);
    assertEquals(w.store.canaries["addr-4"][0].arrivedAt, Date.parse("2026-10-02T12:00:00.000Z"));
    assertEquals(school.forwardHint, TENANT);
    assertEquals(w.store.canaryOk, { "addr-4": "2026-10-02T12:00:00.000Z" });
    assertEquals(school.status, "active");
    // A canary is not mail: no work, no seen row, no last_received_at, nothing counted.
    assertEquals(w.store.workRows, []);
    assertEquals(w.store.seenRows, []);
    assertEquals(w.store.receivedAt, {});
    assertEquals(w.store.counters, {});
    assertEquals(keysUnder(w, "inbound/"), []);
    assertEquals(keysUnder(w, "work/"), []);
  });
});

Deno.test("a_canary_through_the_new_token_ends_the_rotate_grace", async () => {
  // T4.1's Rotate (D3): the old token binds until the new address's canary arrives, at most 7 days.
  await withWorld(async (w) => {
    const school = w.store.addresses[3];
    const OLD = "hijklmnopqrstuvwxyz2345672";
    school.retiredTokenHmac = await tokenHmac(ADDRESS_KEY, OLD);
    school.retiredUntil = new Date(NOW + 6 * 86_400_000).toISOString();
    const hmac = await nonceHmac(ADDRESS_KEY, NONCE);
    // Through the retired token (forwarding not yet re-pointed): the canary counts, the grace holds.
    w.store.canaries["addr-4"] = [{ id: "c-1", nonceHmac: hmac, arrivedAt: null }];
    const old = { key: "inbound/g1", recipients: [at(OLD)], source: SRS };
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT), old)).status, 200);
    assertEquals(w.store.canaries["addr-4"][0].arrivedAt, Date.parse("2026-10-02T12:00:00.000Z"));
    assertEquals(school.retiredTokenHmac, await tokenHmac(ADDRESS_KEY, OLD));
    assert(!w.store.calls.includes("endRotateGrace"), w.store.calls.join());
    // A replay through the new token ends nothing either: only a canary that arrives does.
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT), SCHOOL_FWD("inbound/g2"))).status, 200);
    assert(!w.store.calls.includes("endRotateGrace"), w.store.calls.join());
    // The next canary, through the new token: the old token stops binding at once.
    w.store.canaries["addr-4"] = [{ id: "c-2", nonceHmac: hmac, arrivedAt: null }];
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT), SCHOOL_FWD("inbound/g3"))).status, 200);
    assertEquals([school.retiredTokenHmac, school.retiredUntil], [null, null]);
    const gets = w.gets;
    await w.arrive(await m365Forward(TENANT), { key: "inbound/g4", recipients: [at(OLD)], source: SRS });
    assertEquals(w.gets, gets);
    assertEquals(w.store.workRows, []);
  });
});

Deno.test("a_canary_through_a_retired_token_leaves_a_rotated_gmail_mailbox_pending", async () => {
  // D3 and §4.3 steps 3–5: after a Rotate, Gmail's new address needs its own confirm, so the mailbox reads
  // `pending`. A canary that comes back through the old forward proves only the old address: it is
  // recorded, but the mailbox stays pending (its heads-up stays open) and the grace runs on.
  await withWorld(async (w) => {
    const jane = w.store.addresses[0];
    jane.status = "pending";
    const hmac = await nonceHmac(ADDRESS_KEY, NONCE);
    w.store.canaries["addr-1"] = [{ id: "c-1", nonceHmac: hmac, arrivedAt: null }];
    const old = { key: "inbound/r1", recipients: [at(RETIRED)], source: caf("jane", RETIRED) };
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT), old)).status, 200);
    assertEquals(w.store.canaries["addr-1"][0].arrivedAt, Date.parse("2026-10-02T12:00:00.000Z"));
    assertEquals(w.store.canaryOk, { "addr-1": "2026-10-02T12:00:00.000Z" });
    assertEquals(jane.status, "pending");
    assertEquals(jane.retiredTokenHmac, await tokenHmac(ADDRESS_KEY, RETIRED));
    // The next canary, through the new address: the mailbox reads active and the grace ends.
    w.store.canaries["addr-1"] = [{ id: "c-2", nonceHmac: hmac, arrivedAt: null }];
    const now = { key: "inbound/r2", recipients: [at(TOK1)], source: caf("jane", TOK1) };
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT), now)).status, 200);
    assertEquals(jane.status, "active");
    assertEquals([jane.retiredTokenHmac, jane.retiredUntil], [null, null]);
  });
});

Deno.test("a_replayed_canary_changes_nothing", async () => {
  await withWorld(async (w) => {
    const school = w.store.addresses[3];
    const hmac = await nonceHmac(ADDRESS_KEY, NONCE);
    w.store.canaries["addr-4"] = [{ id: "c-1", nonceHmac: hmac, arrivedAt: null }];
    await w.arrive(await canaryMail(NONCE, TENANT), SCHOOL_FWD("inbound/c2"));
    const after = { ...school };
    const ok = { ...w.store.canaryOk };
    // The same canary again, this time with another tenant in its loop: nothing moves.
    school.status = "quiet";
    const calls = w.store.calls.length;
    assertEquals((await w.arrive(await canaryMail(NONCE, TENANT2), SCHOOL_FWD("inbound/c3"))).status, 200);
    assertEquals({ ...school, status: after.status }, after);
    assertEquals(school.status, "quiet");
    assertEquals(w.store.canaryOk, ok);
    assert(!w.store.calls.slice(calls).includes("canaryArrived"), "a replay wrote the canary");
    assertEquals(w.store.counters, {});
    assertEquals(keysUnder(w, "inbound/"), []);
    // A canary already arrived before this delivery (a concurrent delivery won): the store refuses it.
    w.store.canaries["addr-4"] = [{ id: "c-9", nonceHmac: hmac, arrivedAt: null }];
    const won = w.store.canaryArrived.bind(w.store);
    w.store.canaryArrived = (id, address, hint, iso, current) => {
      w.store.canaries["addr-4"][0].arrivedAt = NOW - 1;
      return won(id, address, hint, iso, current);
    };
    await w.arrive(await canaryMail(NONCE, TENANT2), SCHOOL_FWD("inbound/c4"));
    assertEquals(school.forwardHint, TENANT);
    assertEquals(school.status, "quiet");
    assertEquals(w.store.canaryOk, ok);
  });
});

// ---------------------------------------------------------------- T3d: D8's kick

Deno.test("a_hand_off_kicks_mail_process_once_with_the_job_token", async () => {
  await withWorld(async (w) => {
    const raw = await gmailForward();
    assertEquals((await w.arrive(raw, JANE("inbound/k1"))).status, 200);
    assertEquals(w.kick.sent, [{ url: KICK_URL, method: "POST", token: KICK_TOKEN }]);
    // A duplicate makes no new work: no kick.
    await w.arrive(raw, JANE("inbound/k2"));
    assertEquals(w.kick.sent.length, 1);
    // A batch send of two new originals is one delivery: one kick.
    await openBatches(w);
    await w.arrive(await batchSend(NONCE, [{ eml: ORIGINAL_A }, { eml: ORIGINAL_B }]), SEND("inbound/k3"));
    assertEquals(w.store.workRows.length, 3);
    assertEquals(w.kick.sent.length, 2);
    // Arrivals that make no work send none: unbound, unknown, a canary, the same batch again.
    await w.arrive(directForgery(), { key: "inbound/k4", recipients: [at(TOK1)] });
    await w.arrive(raw, { key: "inbound/k5", recipients: [at(UNKNOWN)] });
    const canary = { id: "c-1", nonceHmac: await nonceHmac(ADDRESS_KEY, NONCE), arrivedAt: null };
    w.store.canaries["addr-4"] = [canary];
    await w.arrive(await canaryMail(NONCE, TENANT), SCHOOL_FWD("inbound/k6"));
    await w.arrive(await batchSend(NONCE, [{ eml: ORIGINAL_A }]), SEND("inbound/k7"));
    assertEquals(w.kick.sent.length, 2);
    for (const line of w.logs) assert(!line.includes(KICK_TOKEN), "a log line carries the job token");
  });
});

Deno.test("a_failed_or_slow_kick_still_answers_200_and_keeps_the_work", async () => {
  await withWorld(async (w) => {
    w.kick.mode = "fail";
    assertEquals((await w.arrive(await gmailForward(), JANE("inbound/s1"))).status, 200);
    w.kick.mode = "slow";
    const started = performance.now();
    const slow = await w.arrive(await gmailForward({ messageId: false }), JANE("inbound/s2"));
    assertEquals(slow.status, 200);
    assert(performance.now() - started < 2_000, "the kick is bounded in time");
    assertEquals(w.kick.sent.length, 2);
    // The work stays for the cron: both rows and both objects.
    assertEquals(w.store.workRows.length, 2);
    assertEquals(keysUnder(w, "work/"), ["work/item-1", "work/item-2"]);
    assertEquals(keysUnder(w, "inbound/"), []);
    // Each failure is one fixed line, never the URL or the token.
    const kicks = w.logs.filter((l) => l.includes("kick"));
    assertEquals(kicks, [
      "mail-inbound: kick failed (500), left for the cron",
      "mail-inbound: kick failed (no answer), left for the cron",
    ]);
  });
});
