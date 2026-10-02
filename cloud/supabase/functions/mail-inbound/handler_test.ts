// Email plan T2g.1 (spec §4.6 steps 1–4 and 7–8, D8; tests 1, 2, 7, 15 and 16, the intake halves). The
// handler runs whole: SNS verification against a certificate host and S3 against a fake bucket, both one
// listener on 127.0.0.1:0 reached through injected fetchers that keep the real-shaped AWS URLs; DKIM keys
// served by an injected resolver; the database is an in-memory store. Every token, address, key, domain
// and message here is invented for these tests: no real capture is used.
import { assert, assertEquals } from "@std/assert";
import { canonicalBody, canonicalHeader, type TxtResolver } from "../_shared/dkim.ts";
import { s3Client } from "../_shared/s3.ts";
import { type AddressRow, handle, type InboundConfig, type InboundDeps, type InboundStore } from "./handler.ts";
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
const ZONE: Record<string, string[][]> = {
  "s1._domainkey.google.com": await keyRecord(GOOGLE),
  "s1._domainkey.microsoft.com": await keyRecord(MICROSOFT),
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
  bindRows(_addressId: string) {
    this.calls.push("bindRows");
    return Promise.resolve({ batches: [], canaries: [] });
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
  count(addressId: string, counter: "unbound" | "unauthenticated") {
    this.calls.push("count");
    this.counters[`${addressId}:${counter}`] = (this.counters[`${addressId}:${counter}`] ?? 0) + 1;
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

interface World {
  store: FakeStore;
  /** The fake bucket, by key. */
  bucket: Map<string, Uint8Array>;
  gets: number;
  failPut: boolean;
  logs: string[];
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
  const w = { store, bucket, gets: 0, failPut: false, logs: [] as string[] } as World;
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    const path = new URL(req.url).pathname;
    if (path.endsWith(".pem")) return new Response(PEM);
    const key = decodeURIComponent(path.slice(1));
    const body = new Uint8Array(await req.arrayBuffer());
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
    bind: { resolveTxt, now: () => NOW, nonceHmac: (x) => nonceHmac(ADDRESS_KEY, x) },
    tokenHmac: (tok) => tokenHmac(ADDRESS_KEY, tok),
    now: () => NOW,
    workKey: () => `work/item-${++n}`,
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
