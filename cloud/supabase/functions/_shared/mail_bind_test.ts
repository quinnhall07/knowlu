// Email plan T2e (spec §4.6 step 5's first three cases; tests 4, 5 and 6, the recognition halves) and T2f
// (step 5's live case and "anything else", step 6; tests 2's forgeries and 3). Every key is made here and
// served by an injected resolver, so no test queries DNS; the nonce HMAC is a fake. Every domain, address,
// nonce, envelope and link is invented for these tests: no real capture is used.
import { assert, assertEquals } from "@std/assert";
import { canonicalBody, canonicalHeader, type TxtResolver } from "./dkim.ts";
import {
  type Arrival,
  type Authenticity,
  type BindConfig,
  type BindDeps,
  type BindEnvelope,
  type BindMailbox,
  classifyArrival,
  type ConfirmRefusal,
  confirmUrlAllowed,
  newNonce,
  nonceTag,
  originalAuthenticity,
  unboundOrigin,
} from "./mail_bind.ts";

const RSA = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;
const rsaKey = () =>
  crypto.subtle.generateKey(
    { ...RSA, modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]) },
    true,
    ["sign", "verify"],
  );
const KNOWLU = await rsaKey(); // s1._domainkey.knowlu.test
const GOOGLE = await rsaKey(); // s1._domainkey.google.com (a test key; the name only mirrors production)
const GMAIL = await rsaKey(); // s1._domainkey.gmail.com
const EVIL = await rsaKey(); // s1._domainkey.evil.test
const MICROSOFT = await rsaKey(); // s1._domainkey.microsoft.com (a test key, as GOOGLE)

const bytes = (bin: string) => Uint8Array.from(bin, (c) => c.charCodeAt(0) & 0xff);
const b64 = (b: Uint8Array) => btoa(String.fromCharCode(...b));
const sha = async (s: string) => b64(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes(s))));
async function keyRecord(k: CryptoKeyPair): Promise<string[][]> {
  return [[`v=DKIM1; k=rsa; p=${b64(new Uint8Array(await crypto.subtle.exportKey("spki", k.publicKey)))}`]];
}
const ZONE: Record<string, string[][]> = {
  "s1._domainkey.knowlu.test": await keyRecord(KNOWLU),
  "s1._domainkey.google.com": await keyRecord(GOOGLE),
  "s1._domainkey.gmail.com": await keyRecord(GMAIL),
  "s1._domainkey.evil.test": await keyRecord(EVIL),
  "s1._domainkey.microsoft.com": await keyRecord(MICROSOFT),
};
const resolveTxt: TxtResolver = (name) => Promise.resolve(ZONE[name] ?? []);

/** MX records by domain (PQ9 and PQ12: a Workspace mailbox binds when its domain's MX is Google's or a
 * gateway's, never Microsoft's, none, or unanswered). No test
 * queries DNS; every lookup is recorded so a test can say none was made. */
const MX: Record<string, string[]> = {
  "school.test": ["aspmx.l.google.com", "alt1.aspmx.l.google.com"],
  "other-school.test": ["smtp.google.com"],
  "gateway.test": ["mx1.filter.example"],
  "m365.test": ["m365-test.mail.protection.outlook.com"],
};
const mxAsked: string[] = [];
const resolveMx = (domain: string) => {
  mxAsked.push(domain);
  if (domain === "dnsfail.test") return Promise.reject(new Error("dns"));
  return Promise.resolve(MX[domain] ?? []);
};
const NOW = Date.UTC(2026, 9, 2, 12);
const DEPS: BindDeps = {
  resolveTxt,
  resolveMx,
  now: () => NOW,
  nonceHmac: (n) => Promise.resolve(`hmac:${n}`),
};
const CONFIG: BindConfig = { canaryFrom: "check@knowlu.test" };
const NONCE = "0123456789abcdef0123456789abcdef";
const OTHER_NONCE = "fedcba9876543210fedcba9876543210";
const TENANT = "72f988bf-86f1-41af-91ab-2d7cd011db47";
const TENANT2 = "0d5f1a7c-1111-4abc-9def-0123456789ab";

function mailbox(over: Partial<BindMailbox> = {}): BindMailbox {
  return {
    id: "addr-1",
    provider: "google",
    mailboxHint: "jane@gmail.com",
    confirmUntil: NOW + 10 * 60_000,
    forwardHint: null,
    batches: [],
    canaries: [],
    ...over,
  };
}

/** The Knowlu address SES delivered to, and the envelope a direct send to it has. */
const KNOWLU_ADDR = "tok@in.knowlu.test";
const DIRECT: BindEnvelope = { mailFrom: "bounce@sender.test", spfPass: true, recipient: KNOWLU_ADDR };

interface Signer {
  key: CryptoKeyPair;
  d: string;
  h: string[];
}

/** A relaxed/relaxed DKIM-Signature over `head` (fields picked bottom-up, as a signer picks them) and
 * `body`, put on top. dkim.ts's own tests check its canonicalisation by hand; here it only signs. */
async function signed(head: string[], body: string, s: Signer): Promise<string[]> {
  const bh = await sha(canonicalBody(body, "relaxed"));
  const sig = `DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=${s.d}; s=s1; h=${s.h.join(":")};` +
    ` bh=${bh}; b=`;
  const data = picked(head, s.h) + canonicalHeader(sig, "relaxed");
  return [sig + await signData(s.key, data), ...head];
}

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

const signData = async (key: CryptoKeyPair, data: string) =>
  b64(new Uint8Array(await crypto.subtle.sign(RSA, key.privateKey, bytes(data))));

const assemble = (head: string[], body: string) => head.join("\r\n") + "\r\n\r\n" + body;

function classify(
  raw: string,
  box: BindMailbox = mailbox(),
  config = CONFIG,
  envelope: BindEnvelope = DIRECT,
): Promise<Arrival> {
  return classifyArrival(raw, box, config, envelope, DEPS);
}

// ---------------------------------------------------------------- the batch send (test 4)

const GMAIL_SIGNS: Signer = { key: GMAIL, d: "gmail.com", h: ["from", "to", "subject"] };
const BATCH_BODY =
  "--b1\r\nContent-Type: text/plain\r\n\r\nsee attached\r\n--b1\r\nContent-Type: message/rfc822\r\n" +
  "\r\nFrom: prof@school.test\r\nSubject: Lab 5\r\n\r\nLab 5 moved.\r\n--b1--\r\n";

async function batchSend(
  o: { from?: string; subject?: string; signer?: Signer | null; above?: string[] } = {},
) {
  const head = [
    `From: Jane <${o.from ?? "jane@gmail.com"}>`,
    "To: tok@in.knowlu.test",
    `Subject: ${o.subject ?? `Fwd: three messages ${nonceTag(NONCE)}`}`,
    'Content-Type: multipart/mixed; boundary="b1"',
  ];
  const out = o.signer === null ? head : await signed(head, BATCH_BODY, o.signer ?? GMAIL_SIGNS);
  return assemble([...(o.above ?? []), ...out], BATCH_BODY);
}

const OPEN = { id: "b-1", phase: "backfill" as const, nonceHmac: `hmac:${NONCE}`, closedAt: null };

Deno.test("a_send_from_the_mailbox_with_aligned_outer_dkim_and_an_open_nonce_is_a_batch_send", async () => {
  const box = mailbox({ batches: [OPEN] });
  assertEquals(await classify(await batchSend(), box), { kind: "batch", batchId: "b-1", phase: "backfill" });
  // The hint is compared case-folded.
  const upper = mailbox({ mailboxHint: "Jane@Gmail.com", batches: [OPEN] });
  assertEquals((await classify(await batchSend(), upper)).kind, "batch");
  // A recover batch reports its own phase.
  const recover = { id: "b-2", phase: "recover" as const, nonceHmac: `hmac:${OTHER_NONCE}`, closedAt: null };
  const both = mailbox({ batches: [OPEN, recover] });
  const second = await batchSend({ subject: `Fwd: recover ${nonceTag(OTHER_NONCE)}` });
  assertEquals(await classify(second, both), { kind: "batch", batchId: "b-2", phase: "recover" });

  // Not a batch send: a closed batch's nonce, no nonce, and a nonce only in an unsigned Subject above
  // the signed one.
  const closed = mailbox({ batches: [{ ...OPEN, closedAt: NOW - 1 }] });
  assertEquals((await classify(await batchSend(), closed)).kind, "unbound");
  assertEquals((await classify(await batchSend({ subject: "Fwd: three messages" }), box)).kind, "unbound");
  const seeded = await batchSend({ subject: "Fwd: three messages", above: [`Subject: ${nonceTag(NONCE)}`] });
  assertEquals((await classify(seeded, box)).kind, "unbound");
});

Deno.test("a_wrong_outer_sender_is_not_a_batch_send", async () => {
  const box = mailbox({ batches: [OPEN] });
  // Another sender, with its own good signature and the right nonce.
  const evil: Signer = { key: EVIL, d: "evil.test", h: ["from", "to", "subject"] };
  assertEquals(
    (await classify(await batchSend({ from: "mallory@evil.test", signer: evil }), box)).kind,
    "unbound",
  );
  // The mailbox's address, signed by a domain it is not aligned with, or not signed at all.
  assertEquals((await classify(await batchSend({ signer: evil }), box)).kind, "unbound");
  assertEquals((await classify(await batchSend({ signer: null }), box)).kind, "unbound");
  // Another Gmail user's signed send, with the mailbox's address in an unsigned From above it.
  const other = await batchSend({ from: "mallory@gmail.com", above: ["From: jane@gmail.com"] });
  assertEquals((await classify(other, box)).kind, "unbound");
  // No hint yet: nothing can be the mailbox's own send.
  assertEquals(
    (await classify(await batchSend(), mailbox({ mailboxHint: null, batches: [OPEN] }))).kind,
    "unbound",
  );
});

// ---------------------------------------------------------------- Gmail's confirmation (test 5)

const GOOGLE_SIGNS: Signer = { key: GOOGLE, d: "google.com", h: ["from", "to", "subject", "content-type"] };
const LINK = "https://mail-settings.google.com/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x";

async function confirmation(
  o: { requester?: string; from?: string; signer?: Signer | null; link?: string; above?: string[] } = {},
) {
  const who = o.requester ?? "jane@gmail.com";
  const head = [
    `From: Gmail Team <${o.from ?? "forwarding-noreply@google.com"}>`,
    "To: tok@in.knowlu.test",
    `Subject: (#123456789) Gmail Forwarding Confirmation - Receive Mail from ${who}`,
    'Content-Type: text/plain; charset="UTF-8"',
  ];
  const body = `${who} has requested to automatically forward mail to your email address\r\n` +
    "tok@in.knowlu.test.\r\nConfirmation code: 123456789\r\n\r\n" +
    `To allow ${who} to automatically forward mail to your address, please click the link below\r\n` +
    `to confirm the request:\r\n\r\n${o.link ?? LINK}\r\n\r\n` +
    "If you click the link and it appears to be broken, please copy and paste it into a new window.\r\n";
  const out = o.signer === null ? head : await signed(head, body, o.signer ?? GOOGLE_SIGNS);
  return assemble([...(o.above ?? []), ...out], body);
}

const refused = (reason: ConfirmRefusal): Arrival => ({ kind: "gmail_confirm_refused", reason });

Deno.test("the_mailboxs_own_gmail_confirmation_is_recognised", async () => {
  assertEquals(await classify(await confirmation()), { kind: "gmail_confirm", link: LINK });
  // Case-folded on both sides, never more than that.
  const upper = mailbox({ mailboxHint: "Jane@Gmail.com" });
  assertEquals(await classify(await confirmation({ requester: "JANE@gmail.com" }), upper), {
    kind: "gmail_confirm",
    link: LINK,
  });
  const other = "https://mail.google.com/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x";
  assertEquals(await classify(await confirmation({ link: other })), { kind: "gmail_confirm", link: other });
});

Deno.test("notjane_is_not_jane", async () => {
  for (
    const requester of ["notjane@gmail.com", "jane@gmail.com.evil.test", "jane.x@gmail.com", "jane@gmail.co"]
  ) {
    assertEquals(await classify(await confirmation({ requester })), refused("hint_mismatch"), requester);
  }
});

Deno.test("a_null_hint_is_refused", async () => {
  assertEquals(await classify(await confirmation(), mailbox({ mailboxHint: null })), refused("hint_null"));
});

Deno.test("a_closed_confirm_window_is_refused", async () => {
  for (const confirmUntil of [null, NOW - 1, NOW]) {
    assertEquals(await classify(await confirmation(), mailbox({ confirmUntil })), refused("window"));
  }
});

Deno.test("a_sender_other_than_googles_exact_sender_or_without_aligned_dkim_is_refused", async () => {
  const evil: Signer = { ...GOOGLE_SIGNS, key: EVIL, d: "evil.test" };
  // Not Google's exact sender: never a confirmation at all.
  for (
    const from of ["forwarding-noreply@google.com.evil.test", "noreply@google.com", "forwarding@evil.test"]
  ) {
    const signer = from.endsWith("@google.com") ? GOOGLE_SIGNS : evil;
    assertEquals((await classify(await confirmation({ from, signer }))).kind, "unbound", from);
  }
  // Google's exact sender without Google's aligned signature: refused.
  assertEquals(await classify(await confirmation({ signer: evil })), refused("dkim"));
  assertEquals(await classify(await confirmation({ signer: null })), refused("dkim"));
  const forged = { ...GOOGLE_SIGNS, key: EVIL }; // claims d=google.com, signed with another key
  assertEquals(await classify(await confirmation({ signer: forged })), refused("dkim"));
  // A second From above the signed one is not covered by the signature.
  const doubled = await confirmation({ above: ["From: forwarding-noreply@google.com"] });
  assertEquals(await classify(doubled), refused("dkim"));
});

Deno.test("a_link_off_the_allow_list_is_refused", async () => {
  const off = [
    "https://mail-settings.google.com.evil.test/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x",
    "http://mail-settings.google.com/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x",
    "https://mail-settings.google.com:8443/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x",
    "https://mail-settings.google.com@evil.test/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x",
    "https://mail-settings.google.com/mail/uf-%5BANGjdJ-test%5D-AbCdEf123_x",
    "https://mail-settings.google.com/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x?continue=https://evil.test",
    "https://mail-settings.google.com/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x#frag",
    "https://evil.test/mail/vf-%5BANGjdJ-test%5D-AbCdEf123_x",
  ];
  assert(confirmUrlAllowed(LINK));
  for (const link of off) {
    assert(!confirmUrlAllowed(link), link);
    assertEquals(await classify(await confirmation({ link })), refused("link"), link);
  }
  // Two different allowed links: which one Google meant is not ours to guess.
  const two = `${LINK}\r\n\r\n${LINK}2`;
  assertEquals(await classify(await confirmation({ link: two })), refused("link"));
});

// ---------------------------------------------------------------- the canary (test 6)

const KNOWLU_SIGNS: Signer = { key: KNOWLU, d: "knowlu.test", h: ["from", "to", "subject"] };
const LOOP = `X-MS-Exchange-ForwardingLoop: jdoe@school.test;${TENANT}`;

/** A canary as it reaches intake: Knowlu's signed message, with the forwarding hop's fields on top. */
async function canary(o: { from?: string; signer?: Signer; nonce?: string; above?: string[] } = {}) {
  const head = [
    `From: Knowlu <${o.from ?? "check@knowlu.test"}>`,
    "To: jdoe@school.test",
    `Subject: Knowlu delivery check - no need to open this ${nonceTag(o.nonce ?? NONCE)}`,
    "Content-Type: text/plain",
  ];
  const body = "Knowlu sent this to check that your mail reaches it. Turn it off in Settings, Email.\r\n";
  return assemble([...(o.above ?? [LOOP]), ...await signed(head, body, o.signer ?? KNOWLU_SIGNS)], body);
}

const SCHOOL = mailbox({
  provider: "m365",
  mailboxHint: "jdoe@school.test",
  confirmUntil: null,
  canaries: [{ id: "c-1", nonceHmac: `hmac:${NONCE}`, arrivedAt: null }],
});

Deno.test("a_canary_with_its_nonce_and_knowlus_dkim_is_recognised_with_its_forward_hint", async () => {
  // The nonce's form, shared with the sender (T4.2) and the device's batch sends.
  assertEquals(nonceTag(NONCE), `[knowlu ${NONCE}]`);
  const fresh = newNonce();
  assert(/^[0-9a-f]{32}$/.test(fresh));
  assert(fresh !== newNonce());

  const want: Extract<Arrival, { kind: "canary" }> = {
    kind: "canary",
    canaryId: "c-1",
    replay: false,
    forwardHint: TENANT,
  };
  assertEquals(await classify(await canary(), SCHOOL), want);
  // The tenant id is recorded lower-cased; the mailbox is matched case-folded.
  const upper = await canary({
    above: [`X-MS-Exchange-ForwardingLoop: JDoe@School.test;${TENANT.toUpperCase()}`],
  });
  assertEquals(await classify(upper, SCHOOL), want);
  // Only the topmost ForwardingLoop is read: the hop that handed the canary on.
  const two = await canary({
    above: [LOOP, "X-MS-Exchange-ForwardingLoop: x@other.test;" + TENANT.replace("7", "8")],
  });
  assertEquals(await classify(two, SCHOOL), want);
  // An arrived canary is still the canary: a replay, which T2g.2 leaves as it is.
  const arrived = {
    ...SCHOOL,
    canaries: [{ id: "c-1", nonceHmac: `hmac:${NONCE}`, arrivedAt: NOW - 60_000 }],
  };
  assertEquals(await classify(await canary(), arrived), { ...want, replay: true });
  // No forward hint: a loop naming another mailbox, a malformed tenant, none at all, a Gmail mailbox.
  for (
    const above of [[`X-MS-Exchange-ForwardingLoop: other@school.test;${TENANT}`], [
      "X-MS-Exchange-ForwardingLoop: jdoe@school.test;not-a-tenant",
    ], []]
  ) {
    assertEquals(await classify(await canary({ above }), SCHOOL), { ...want, forwardHint: null });
  }
  const gmail = mailbox({ canaries: SCHOOL.canaries });
  assertEquals(await classify(await canary(), gmail), { ...want, forwardHint: null });
  // A nonce that is not this mailbox's canary is not a canary.
  assertEquals((await classify(await canary({ nonce: OTHER_NONCE }), SCHOOL)).kind, "unbound");
});

Deno.test("a_canary_from_a_wrong_sender_domain_is_not_a_canary", async () => {
  const evil: Signer = { ...KNOWLU_SIGNS, key: EVIL, d: "evil.test" };
  for (const from of ["check@knowlu.test.evil.test", "check@evil.test", "other@knowlu.test"]) {
    const signer = from.endsWith("@knowlu.test") ? KNOWLU_SIGNS : evil;
    assertEquals((await classify(await canary({ from, signer }), SCHOOL)).kind, "unbound", from);
  }
  // Knowlu's sender, signed by another domain or with another key under Knowlu's name.
  assertEquals((await classify(await canary({ signer: evil }), SCHOOL)).kind, "unbound");
  assertEquals(
    (await classify(await canary({ signer: { ...KNOWLU_SIGNS, key: EVIL } }), SCHOOL)).kind,
    "unbound",
  );
});

// ---------------------------------------------------------------- live forwards (T2f; tests 2 and 3)

/** One forwarding hop: the fields it adds on top, then its ARC set (AMS over `h` picked bottom-up and the
 * body; the seal over every set so far), as Gmail and Microsoft add theirs. */
interface Hop {
  key: CryptoKeyPair;
  d: string;
  h: string[];
  aar: string;
  adds?: string[];
}
interface Fwd {
  head: string[];
  body: string;
  /** The relaxed lines of every ARC set so far, in instance order. */
  sets: string;
  n: number;
}
async function hop(m: Fwd, o: Hop): Promise<Fwd> {
  const i = m.n + 1;
  const head = [...(o.adds ?? []), ...m.head];
  const bh = await sha(canonicalBody(m.body, "relaxed"));
  const amsHead = `ARC-Message-Signature: i=${i}; a=rsa-sha256; c=relaxed/relaxed; d=${o.d}; s=s1;` +
    ` h=${o.h.join(":")}; bh=${bh}; b=`;
  const ams = amsHead + await signData(o.key, picked(head, o.h) + canonicalHeader(amsHead, "relaxed"));
  const aar = `ARC-Authentication-Results: i=${i}; ${o.aar}`;
  const asHead = `ARC-Seal: i=${i}; a=rsa-sha256; t=1790000000; cv=${i === 1 ? "none" : "pass"}; d=${o.d};` +
    " s=s1; b=";
  const set = canonicalHeader(aar, "relaxed") + "\r\n" + canonicalHeader(ams, "relaxed") + "\r\n";
  const as = asHead + await signData(o.key, m.sets + set + canonicalHeader(asHead, "relaxed"));
  return {
    head: [as, ams, aar, ...head],
    body: m.body,
    sets: m.sets + set + canonicalHeader(as, "relaxed") + "\r\n",
    n: i,
  };
}

const LIVE_BODY = "Lab 5 is now due Friday at noon.\r\n";
/** The original as the professor sent it; `below` is what a sender writes into it. */
const original = (from = "Prof Example <prof@school.test>", below: string[] = []): Fwd => ({
  head: [
    ...below,
    `From: ${from}`,
    "To: jane@gmail.com",
    "Subject: Lab 5 moved",
    "Date: Fri, 02 Oct 2026 09:00:00 -0500",
    "Message-ID: <m1@school.test>",
  ],
  body: LIVE_BODY,
  sets: "",
  n: 0,
});

const XFF_JANE = `X-Forwarded-For: jane@gmail.com ${KNOWLU_ADDR}`;
const XFT = `X-Forwarded-To: ${KNOWLU_ADDR}`;
const GOOGLE_AAR = "mx.google.com; dkim=pass header.i=@school.test header.s=s1 header.b=AbCd;" +
  " spf=pass (google.com: domain of prof@school.test designates 192.0.2.1 as permitted sender)" +
  " smtp.mailfrom=prof@school.test; dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=school.test";
/** Google's hop as S0 saw it: X-Forwarded-* added on top, its AMS covering neither. */
const GOOGLE_HOP: Hop = {
  key: GOOGLE,
  d: "google.com",
  h: ["subject", "to", "from"],
  aar: GOOGLE_AAR,
  adds: [XFF_JANE, XFT],
};
async function gmailForward(o: Partial<Hop> & { from?: string; below?: string[]; above?: string[] } = {}) {
  const m = await hop(original(o.from, o.below), { ...GOOGLE_HOP, ...o });
  return assemble([...(o.above ?? []), ...m.head], m.body);
}
const GMAIL_BOX = mailbox({ confirmUntil: null });
/** Gmail's forwarding envelope: the mailbox's local part, `+caf_=`, our address with `@` as `=`. */
const CAF: BindEnvelope = {
  mailFrom: "jane+caf_=tok=in.knowlu.test@gmail.com",
  spfPass: true,
  recipient: KNOWLU_ADDR,
};
const LIVE_ALIGNED: Arrival = { kind: "live", authenticity: "aligned" };
const UNBOUND: Arrival = { kind: "unbound" };

const MS_AAR = "mx.microsoft.com 1; spf=pass (sender ip is 192.0.2.7) smtp.rcpttodomain=school.test" +
  " smtp.mailfrom=friend.test; dmarc=pass (p=none sp=none pct=100) action=none header.from=friend.test;" +
  " dkim=pass (signature was verified) header.d=friend.test; arc=pass (0 oda=1 ltdi=1" +
  " spf=[1,1,smtp.mailfrom=friend.test] dkim=[1,1,header.d=friend.test] dmarc=[1,1,header.from=friend.test])";
/** Microsoft's hop as S0 saw it: ForwardingLoop added on top, its AMS not covering it. */
const MS_HOP: Hop = {
  key: MICROSOFT,
  d: "microsoft.com",
  h: ["from", "date", "subject", "message-id"],
  aar: MS_AAR,
  adds: [LOOP],
};
async function m365Forward(o: Partial<Hop> & { below?: string[] } = {}) {
  const m = await hop(original("Pat <pat@friend.test>", o.below), { ...MS_HOP, ...o });
  return assemble(m.head, m.body);
}
const MS_BOX: BindMailbox = { ...SCHOOL, forwardHint: TENANT };
/** Microsoft's SRS envelope on the mailbox's own domain. */
const SRS: BindEnvelope = {
  mailFrom: "jdoe+SRS=Ab1C=2Q=friend.test=pat@school.test",
  spfPass: true,
  recipient: KNOWLU_ADDR,
};
const live = (raw: string, box: BindMailbox, envelope: BindEnvelope) => classify(raw, box, CONFIG, envelope);

Deno.test("a_gmail_forward_binds_by_googles_arc_seal_and_its_caf_return_path", async () => {
  const fwd = await gmailForward();
  assertEquals(await live(fwd, GMAIL_BOX, CAF), LIVE_ALIGNED);
  // The envelope is compared case-folded.
  const upper = { ...CAF, mailFrom: "Jane+CAF_=Tok=In.Knowlu.Test@Gmail.com" };
  assertEquals(await live(fwd, GMAIL_BOX, upper), LIVE_ALIGNED);
  // A gmail.com mailbox is Google's own: no MX lookup (PQ9, amended 2026-10-02).
  assertEquals(mxAsked.includes("gmail.com"), false);

  // Check (ii): SES's SPF pass on exactly this mailbox's forwarding envelope to exactly this address.
  for (
    const env of [
      { ...CAF, spfPass: false },
      { ...CAF, mailFrom: null },
      { ...CAF, mailFrom: "jane@gmail.com" },
      { ...CAF, mailFrom: "jane+caf_=tok=in.knowlu.test@gmail.test" },
      { ...CAF, mailFrom: "jane+caf_=other=in.knowlu.test@gmail.com" },
      { ...CAF, mailFrom: "xjane+caf_=tok=in.knowlu.test@gmail.com" },
      { ...CAF, mailFrom: "jane+caf_=tok=in.knowlu.test.evil.test@gmail.com" },
      { ...CAF, recipient: "tok2@in.knowlu.test" },
    ]
  ) {
    assertEquals(await live(fwd, GMAIL_BOX, env), UNBOUND, String(env.mailFrom));
  }
  // Check (i): Google's own topmost seal, verifying.
  assertEquals(await live(await gmailForward({ key: EVIL, d: "evil.test" }), GMAIL_BOX, CAF), UNBOUND);
  assertEquals(await live(await gmailForward({ key: EVIL }), GMAIL_BOX, CAF), UNBOUND);
  assertEquals(await live(fwd.replace("noon", "dusk"), GMAIL_BOX, CAF), UNBOUND); // the AMS fails
  const bare = assemble(original().head, LIVE_BODY);
  assertEquals(await live(bare, GMAIL_BOX, CAF), UNBOUND);
  // A Microsoft seal does not bind a Gmail mailbox; nothing binds without a hint or for `other`.
  assertEquals(
    await live(await gmailForward({ key: MICROSOFT, d: "microsoft.com" }), GMAIL_BOX, CAF),
    UNBOUND,
  );
  assertEquals(await live(fwd, mailbox({ mailboxHint: null }), CAF), UNBOUND);
  assertEquals(await live(fwd, mailbox({ provider: "other" }), CAF), UNBOUND);
});

Deno.test("an_m365_forward_binds_by_microsofts_arc_forwardingloop_and_srs_return_path", async () => {
  const fwd = await m365Forward();
  assertEquals(await live(fwd, MS_BOX, SRS), LIVE_ALIGNED);
  const upper = { ...SRS, mailFrom: "JDoe+srs=Ab1C=2Q=friend.test=pat@School.Test" };
  assertEquals(await live(fwd, MS_BOX, upper), LIVE_ALIGNED);
  // Were Microsoft's AMS to cover the ForwardingLoop, the covered instance is the one read.
  assertEquals(
    await live(await m365Forward({ h: [...MS_HOP.h, "x-ms-exchange-forwardingloop"] }), MS_BOX, SRS),
    LIVE_ALIGNED,
  );

  // Check (ii): SES's SPF pass on the mailbox's own SRS envelope.
  for (
    const env of [
      { ...SRS, spfPass: false },
      { ...SRS, mailFrom: null },
      { ...SRS, mailFrom: "jdoe@school.test" },
      { ...SRS, mailFrom: "pat@friend.test" },
      { ...SRS, mailFrom: "other+SRS=Ab1C=2Q=friend.test=pat@school.test" },
      { ...SRS, mailFrom: "jdoe+SRS=Ab1C=2Q=friend.test=pat@school.test.evil.test" },
      { ...SRS, mailFrom: "jdoe+SRS=Ab1C=2Q=friend.test=pat@evil.test" },
    ]
  ) {
    assertEquals(await live(fwd, MS_BOX, env), UNBOUND, String(env.mailFrom));
  }
  // Check (i): Microsoft's verifying seal, and the topmost ForwardingLoop naming the mailbox and its tenant.
  assertEquals(await live(await m365Forward({ key: GOOGLE, d: "google.com" }), MS_BOX, SRS), UNBOUND);
  assertEquals(await live(await m365Forward({ key: EVIL }), MS_BOX, SRS), UNBOUND);
  assertEquals(await live(await m365Forward({ adds: [] }), MS_BOX, SRS), UNBOUND);
  const otherBox = await m365Forward({ adds: [`X-MS-Exchange-ForwardingLoop: other@school.test;${TENANT}`] });
  assertEquals(await live(otherBox, MS_BOX, SRS), UNBOUND);
  // No tenant recorded yet (no canary has arrived): nothing binds.
  assertEquals(await live(fwd, { ...MS_BOX, forwardHint: null }, SRS), UNBOUND);
  assertEquals(await live(fwd, { ...MS_BOX, provider: "google" }, SRS), UNBOUND);
});

Deno.test("an_intra_school_m365_forward_without_an_srs_envelope_is_unbound", async () => {
  // Checkpoint B's open item (plan T2f): whether EOP SRS-rewrites a forward whose sender is on the school's
  // own domain is unrecorded. Unrewritten, the envelope is the colleague's own address with SES's SPF pass,
  // under Microsoft's verifying seal and a ForwardingLoop naming the mailbox and tenant: nothing a sender on
  // any tenant could not also write, so it binds nothing (risk 10). It is unbound, never live.
  const aar = "mx.microsoft.com 1; spf=pass (sender ip is 192.0.2.7) smtp.rcpttodomain=school.test" +
    " smtp.mailfrom=school.test; dmarc=pass action=none header.from=school.test;" +
    " dkim=pass (signature was verified) header.d=school.test";
  const m = await hop(original("Prof Example <prof@school.test>"), { ...MS_HOP, aar });
  const fwd = assemble(m.head, m.body);
  for (const mailFrom of ["prof@school.test", "PROF@School.Test", "jdoe@school.test"]) {
    assertEquals(await live(fwd, MS_BOX, { ...SRS, mailFrom }), UNBOUND, mailFrom);
  }
  // Rewritten by SRS, the same intra-school forward binds, and is aligned on the school's own record.
  const srs = { ...SRS, mailFrom: "jdoe+SRS=Ab1C=2Q=school.test=prof@school.test" };
  assertEquals(await live(fwd, MS_BOX, srs), LIVE_ALIGNED);
});

// PQ9 (Quinn, 2026-10-02): a Google Workspace mailbox on its own domain binds on Google's verifying seal and
// SES's SPF pass on its `+caf_=` envelope at that same domain. PQ12 (Quinn, 2026-10-03, answer (a)): the
// domain's MX reads Google or another host (a filtering gateway in front of Google), as `hintFits` reads it.
const WS_BOX = mailbox({ mailboxHint: "jdoe@school.test", confirmUntil: null });
const WS_CAF: BindEnvelope = { ...CAF, mailFrom: "jdoe+caf_=tok=in.knowlu.test@school.test" };

Deno.test("a_workspace_forward_binds_by_googles_seal_its_own_domain_caf_return_path_and_a_google_or_gateway_mx", async () => {
  const fwd = await gmailForward();
  assertEquals(await live(fwd, WS_BOX, WS_CAF), LIVE_ALIGNED);
  const upper = { ...WS_CAF, mailFrom: "JDoe+CAF_=Tok=In.Knowlu.Test@School.Test" };
  assertEquals(await live(fwd, WS_BOX, upper), LIVE_ALIGNED);
  // Google's newer single MX host reads as Google too.
  const other = mailbox({ mailboxHint: "pat@other-school.test", confirmUntil: null });
  const otherEnv = { ...CAF, mailFrom: "pat+caf_=tok=in.knowlu.test@other-school.test" };
  assertEquals(await live(fwd, other, otherEnv), LIVE_ALIGNED);
  // A Proofpoint, Mimecast or campus-relay gateway in front of Google binds too (PQ12): its MX reads `other`.
  const gateway = mailbox({ mailboxHint: "jdoe@gateway.test", confirmUntil: null });
  const gatewayEnv = { ...CAF, mailFrom: "jdoe+caf_=tok=in.knowlu.test@gateway.test" };
  mxAsked.length = 0;
  assertEquals(await live(fwd, gateway, gatewayEnv), LIVE_ALIGNED);
  assertEquals(mxAsked, ["gateway.test"]);

  // Check (ii) is still exactly this mailbox's envelope at its own domain, SPF-passed, to this address.
  for (
    const env of [
      { ...WS_CAF, spfPass: false },
      { ...WS_CAF, mailFrom: "jdoe+caf_=tok=in.knowlu.test@gmail.com" },
      { ...WS_CAF, mailFrom: "jdoe+caf_=tok=in.knowlu.test@other-school.test" },
      { ...WS_CAF, mailFrom: "xjdoe+caf_=tok=in.knowlu.test@school.test" },
      { ...WS_CAF, mailFrom: "jdoe@school.test" },
      { ...WS_CAF, recipient: "tok2@in.knowlu.test" },
    ]
  ) {
    assertEquals(await live(fwd, WS_BOX, env), UNBOUND, String(env.mailFrom));
  }
  // Check (i) is still Google's verifying seal.
  assertEquals(await live(await gmailForward({ key: EVIL, d: "evil.test" }), WS_BOX, WS_CAF), UNBOUND);
  assertEquals(
    await live(await gmailForward({ key: MICROSOFT, d: "microsoft.com" }), WS_BOX, WS_CAF),
    UNBOUND,
  );
  // A message no provider sealed never reaches DNS.
  mxAsked.length = 0;
  assertEquals(await live(assemble(original().head, LIVE_BODY), WS_BOX, WS_CAF), UNBOUND);
  assertEquals(mxAsked, []);
  // An M365 mailbox on the same domain never binds by Gmail's envelope.
  assertEquals(await live(fwd, { ...WS_BOX, provider: "m365", forwardHint: TENANT }, WS_CAF), UNBOUND);
});

Deno.test("a_workspace_caf_envelope_on_a_domain_whose_mx_is_microsoft_missing_or_unanswered_is_unbound", async () => {
  const fwd = await gmailForward();
  // Microsoft's MX, no MX at all, and a DNS failure: only these refuse (PQ12, answer (a)). A gateway's MX
  // binds, in the test above.
  for (const domain of ["m365.test", "nomx.test", "dnsfail.test"]) {
    const box = mailbox({ mailboxHint: `jdoe@${domain}`, confirmUntil: null });
    const env = { ...CAF, mailFrom: `jdoe+caf_=tok=in.knowlu.test@${domain}` };
    mxAsked.length = 0;
    assertEquals(await live(fwd, box, env), UNBOUND, domain);
    assertEquals(mxAsked, [domain], domain);
  }
});

Deno.test("a_googlemail_hint_expects_a_gmail_com_caf_return_path", async () => {
  const fwd = await gmailForward();
  const box = mailbox({ mailboxHint: "jane@googlemail.com", confirmUntil: null });
  mxAsked.length = 0;
  assertEquals(await live(fwd, box, CAF), LIVE_ALIGNED);
  const onGooglemail = { ...CAF, mailFrom: "jane+caf_=tok=in.knowlu.test@googlemail.com" };
  assertEquals(await live(fwd, box, onGooglemail), UNBOUND);
  assertEquals(await live(fwd, box, { ...CAF, spfPass: false }), UNBOUND);
  assertEquals(await live(fwd, box, { ...CAF, mailFrom: "joan+caf_=tok=in.knowlu.test@gmail.com" }), UNBOUND);
  // Google's own two domains are never looked up.
  assertEquals(mxAsked, []);
});

// Quinn's unbound-display ruling (2026-10-03): an unbound message whose original From domain matches the
// hint's domain, or whose topmost seal is the mailbox's own provider's and verifies, is `own` (Settings'
// "skipped" reason); anything else is `foreign` (the "leaked" reason, the only one that suggests Rotate).
Deno.test("an_unbound_message_is_own_when_from_the_hints_domain_or_sealed_by_the_mailboxs_provider_else_foreign", async () => {
  const origin = (raw: string, box: BindMailbox) => unboundOrigin(raw, box, DEPS);
  const unsealed = (from: string) => assemble(original(from).head, LIVE_BODY);
  mxAsked.length = 0;
  // The provider's own verifying seal: Google's on a Gmail mailbox, Microsoft's on an M365 one.
  assertEquals(await origin(await gmailForward(), GMAIL_BOX), "own");
  assertEquals(await origin(await m365Forward(), MS_BOX), "own");
  // Another provider's seal, or a seal that is not a provider's, says nothing.
  assertEquals(await origin(await m365Forward({ key: GOOGLE, d: "google.com" }), MS_BOX), "foreign");
  assertEquals(await origin(await gmailForward({ key: EVIL, d: "evil.test" }), GMAIL_BOX), "foreign");
  assertEquals(await origin(await gmailForward(), { ...GMAIL_BOX, provider: "other" }), "foreign");
  // A school mailbox: the hint's domain, a domain under it, or one it sits under.
  assertEquals(await origin(unsealed("Prof <prof@school.test>"), WS_BOX), "own");
  assertEquals(await origin(unsealed("prof@dept.school.test"), WS_BOX), "own");
  const crimson = mailbox({ mailboxHint: "jdoe@crimson.school.test", confirmUntil: null });
  assertEquals(await origin(unsealed("prof@school.test"), crimson), "own");
  assertEquals(await origin(unsealed("prof@school.test"), { ...WS_BOX, provider: "other" }), "own");
  for (const from of ["prof@notschool.test", "prof@school.test.evil.test", "prof@test"]) {
    assertEquals(await origin(unsealed(from), WS_BOX), "foreign", from);
  }
  // A Gmail mailbox's own domain is gmail.com, read as the ruling words it.
  assertEquals(await origin(unsealed("pal@gmail.com"), GMAIL_BOX), "own");
  assertEquals(await origin(unsealed("prof@school.test"), GMAIL_BOX), "foreign");
  // No hint: only the seal can say `own`. Two From fields, or no message at all, are `foreign`.
  const bare = mailbox({ mailboxHint: null, confirmUntil: null });
  assertEquals(await origin(unsealed("prof@school.test"), bare), "foreign");
  assertEquals(await origin(await gmailForward(), bare), "own");
  const twice = assemble(["From: prof@school.test", ...original("prof@school.test").head], LIVE_BODY);
  assertEquals(await origin(twice, WS_BOX), "foreign");
  assertEquals(await origin("\u0000not mail", WS_BOX), "foreign");
  // It reads no MX: the answer names a reason on the Settings row and binds nothing.
  assertEquals(mxAsked, []);
});

Deno.test("a_direct_send_with_a_forged_x_forwarded_for_is_unbound", async () => {
  // A stranger who learnt the address writes Gmail's forwarding fields, and even a DKIM signature of their own.
  const head = [XFF_JANE, XFT, ...original("Prof Example <prof@school.test>").head];
  const evil: Signer = { key: EVIL, d: "evil.test", h: ["from", "subject", "x-forwarded-for"] };
  for (const raw of [assemble(head, LIVE_BODY), assemble(await signed(head, LIVE_BODY, evil), LIVE_BODY)]) {
    assertEquals(await live(raw, GMAIL_BOX, { ...CAF, mailFrom: "mallory@evil.test" }), UNBOUND);
    // Writing Gmail's envelope from the stranger's own server fails gmail.com's SPF at SES.
    assertEquals(await live(raw, GMAIL_BOX, { ...CAF, spfPass: false }), UNBOUND);
    // Even with an envelope SES passed: no provider seal, no binding.
    assertEquals(await live(raw, GMAIL_BOX, CAF), UNBOUND);
  }
  // The same for Microsoft's field.
  const ms = assemble([LOOP, ...original("Pat <pat@friend.test>").head], LIVE_BODY);
  assertEquals(await live(ms, MS_BOX, SRS), UNBOUND);
});

Deno.test("a_forward_sealed_by_google_from_an_attackers_own_gmail_is_unbound", async () => {
  // Mallory points her own Gmail at Jane's address. Google seals it honestly, and here its AMS even covers
  // an X-Forwarded-For naming Jane: Gmail's fields never bind, and the envelope is Mallory's.
  const fwd = await gmailForward({ adds: [XFF_JANE, XFT], h: ["x-forwarded-for", "x-forwarded-to", "from"] });
  const mallory = { ...CAF, mailFrom: "mallory+caf_=tok=in.knowlu.test@gmail.com" };
  assertEquals(await live(fwd, GMAIL_BOX, mallory), UNBOUND);
  // Mallory's own M365 tenant: Microsoft names her mailbox, her tenant and her domain.
  const ms = await m365Forward({ adds: [`X-MS-Exchange-ForwardingLoop: mallory@evil.test;${TENANT2}`] });
  const msEnv = { ...SRS, mailFrom: "mallory+SRS=Ab1C=2Q=friend.test=pat@evil.test" };
  assertEquals(await live(ms, MS_BOX, msEnv), UNBOUND);
  // Each check alone stops it: her envelope with Jane's loop, and Jane's envelope with her loop.
  const janeLoop = await m365Forward();
  assertEquals(await live(janeLoop, MS_BOX, msEnv), UNBOUND);
  assertEquals(await live(ms, MS_BOX, SRS), UNBOUND);
});

Deno.test("a_sender_seeded_x_forwarded_for_below_googles_own_is_unbound", async () => {
  // Google adds its own X-Forwarded-For above the one the sender wrote, and its AMS signs the bottom one.
  const xff = ["x-forwarded-for", "subject", "to", "from"];
  const control = await gmailForward({ h: xff });
  assertEquals(await live(control, GMAIL_BOX, CAF), LIVE_ALIGNED);
  const seeded = await gmailForward({
    h: xff,
    adds: [`X-Forwarded-For: mallory@gmail.com ${KNOWLU_ADDR}`, XFT],
    below: [XFF_JANE],
  });
  // Check (ii) is met here; check (i) alone refuses, because the signed instance has an unsigned twin.
  assertEquals(await live(seeded, GMAIL_BOX, CAF), UNBOUND);
  // And check (ii) alone refuses the same message with Mallory's envelope.
  const mallory = { ...CAF, mailFrom: "mallory+caf_=tok=in.knowlu.test@gmail.com" };
  assertEquals(await live(seeded, GMAIL_BOX, mallory), UNBOUND);
  // Microsoft's field seeded below its own, were its AMS to cover it.
  const ms = await m365Forward({ h: [...MS_HOP.h, "x-ms-exchange-forwardingloop"], below: [LOOP] });
  assertEquals(await live(ms, MS_BOX, SRS), UNBOUND);
});

Deno.test("an_m365_forward_naming_another_tenant_than_forward_hint_is_unbound", async () => {
  const other = await m365Forward({ adds: [`X-MS-Exchange-ForwardingLoop: jdoe@school.test;${TENANT2}`] });
  assertEquals(await live(other, MS_BOX, SRS), UNBOUND);
  // The recorded tenant matches case-folded; a malformed or extended loop field is not a tenant.
  const upper = await m365Forward({
    adds: [`X-MS-Exchange-ForwardingLoop: JDoe@School.test;${TENANT.toUpperCase()}`],
  });
  assertEquals(await live(upper, MS_BOX, SRS), LIVE_ALIGNED);
  for (
    const loop of [
      `jdoe@school.test;${TENANT};x`,
      "jdoe@school.test;not-a-tenant",
      `jdoe@school.test ${TENANT}`,
    ]
  ) {
    const raw = await m365Forward({ adds: [`X-MS-Exchange-ForwardingLoop: ${loop}`] });
    assertEquals(await live(raw, MS_BOX, SRS), UNBOUND, loop);
  }
  // Only the topmost loop is read: the hop that handed the message to SES.
  const twice = await m365Forward({
    adds: [`X-MS-Exchange-ForwardingLoop: jdoe@school.test;${TENANT2}`, LOOP],
  });
  assertEquals(await live(twice, MS_BOX, SRS), UNBOUND);
});

// ---------------------------------------------------------------- authenticity (test 3)

const LIVE_UNALIGNED: Arrival = { kind: "live", authenticity: "unaligned" };
const gmailLive = async (o: Parameters<typeof gmailForward>[0]) =>
  live(await gmailForward(o), GMAIL_BOX, CAF);

Deno.test("a_dmarc_failing_original_with_an_aligned_provider_hop_result_is_aligned", async () => {
  // SES's own result on the forwarded copy fails DMARC (the forward broke SPF); only Google's hop counts.
  const ses = "Authentication-Results: amazonses.com; spf=fail smtp.mailfrom=gmail.com; dmarc=fail" +
    " header.from=school.test";
  assertEquals(await gmailLive({ above: [ses] }), LIVE_ALIGNED);
  // An aligned DKIM or SPF pass alone, on the From domain or a parent of it, is aligned too.
  for (
    const aar of [
      "mx.google.com; dkim=pass header.i=@school.test header.s=s1",
      "mx.google.com; spf=pass smtp.mailfrom=school.test",
      "mx.google.com;dmarc=pass header.from=SCHOOL.TEST",
      // A domain under the From aligns only through the provider's own DMARC pass (its public-suffix list).
      "mx.google.com; dkim=pass header.d=mail.school.test; dmarc=pass header.from=school.test",
    ]
  ) {
    assertEquals(await gmailLive({ aar }), LIVE_ALIGNED, aar);
  }
  const parent = "mx.google.com; dkim=pass header.d=school.test; spf=fail smtp.mailfrom=x@evil.test";
  assertEquals(await gmailLive({ aar: parent, from: "Dept <dept@cs.school.test>" }), LIVE_ALIGNED);
  // Microsoft's form (an `n` version after the authserv-id; `;` without a space) reads the same.
  assertEquals(await live(await m365Forward(), MS_BOX, SRS), LIVE_ALIGNED);
  const msDkim = "mx.microsoft.com 1; spf=fail smtp.mailfrom=other.test;dkim=pass (signature was verified)" +
    " header.d=friend.test;dmarc=fail action=none header.from=friend.test";
  assertEquals(await live(await m365Forward({ aar: msDkim }), MS_BOX, SRS), LIVE_ALIGNED);
});

Deno.test("a_pass_only_in_a_sender_written_authentication_results_is_unaligned", async () => {
  const failing = "mx.google.com; dkim=none; spf=softfail (google.com: domain of transitioning x@evil.test" +
    " does not designate 192.0.2.9 as permitted sender) smtp.mailfrom=x@evil.test; dmarc=fail (p=NONE)" +
    " header.from=school.test";
  const written = [
    "Authentication-Results: mx.google.com; dmarc=pass header.from=school.test",
    "Authentication-Results: school.test; dkim=pass header.d=school.test; spf=pass smtp.mailfrom=school.test",
  ];
  assertEquals(await gmailLive({ aar: failing, below: written }), LIVE_UNALIGNED);
  // A pass the hop's record states for another domain, inside a comment, or for an unaligned domain.
  for (
    const aar of [
      "mx.google.com; dmarc=pass header.from=evil.test",
      "mx.google.com; dkim=pass header.d=school.test.evil.test",
      "mx.google.com; dkim=pass header.d=test",
      // A child of the From domain, with no DMARC pass from the provider, is not ours to align.
      "mx.google.com; dkim=pass header.d=mail.school.test; dmarc=fail header.from=school.test",
      "mx.google.com; spf=pass smtp.mailfrom=bounce@lists.school.test",
      "mx.google.com; spf=pass smtp.mailfrom=x@notschool.test",
      "mx.google.com; arc=pass (i=1 dmarc=pass header.from=school.test dkim=pass header.d=school.test)",
      "mx.google.com; dmarc=passed header.from=school.test",
      "mx.google.com; dmarc=pass",
      "mx.google.com; none",
      "dmarc=pass header.from=school.test",
    ]
  ) {
    assertEquals(await gmailLive({ aar }), LIVE_UNALIGNED, aar);
  }
  // Two From fields: which one the record speaks for is not ours to guess.
  assertEquals(await gmailLive({ below: ["From: prof@school.test"] }), LIVE_UNALIGNED);
});

const STAMP_G = "Authentication-Results: mx.google.com; dkim=pass header.i=@school.test header.s=s1;" +
  " spf=pass smtp.mailfrom=prof@school.test; dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=school.test";
const STAMP_M = "Authentication-Results: spf=pass (sender IP is 192.0.2.7) smtp.mailfrom=school.test;" +
  " dkim=pass (signature was verified) header.d=school.test;dmarc=pass action=none" +
  " header.from=school.test;compauth=pass reason=100";
const RECEIVED = "Received: from mail.school.test by mx.provider.test; Fri, 02 Oct 2026 07:00:01 -0700";
/** An original as the student's provider stored it: its delivery fields on top of what the sender wrote. */
const stored = (top: string[], o: { from?: string; below?: string[] } = {}) =>
  assemble([...top, RECEIVED, ...original(o.from, o.below).head], LIVE_BODY);
const sealedBy = async (h: Partial<Hop>, top: string[] = []) => {
  const m = await hop(original(), { ...GOOGLE_HOP, adds: [], ...h });
  return assemble([...top, ...m.head], m.body);
};
const auth = (raw: string | Uint8Array, provider: BindMailbox["provider"]): Promise<Authenticity> =>
  originalAuthenticity(raw, provider, DEPS);

Deno.test("a_backfilled_original_reads_only_the_providers_delivery_stamp_or_arc", async () => {
  // Gmail's stamp (`mx.google.com`), the topmost Authentication-Results, as text or as attached bytes.
  assertEquals(await auth(stored([STAMP_G]), "google"), "aligned");
  assertEquals(await auth(new TextEncoder().encode(stored([STAMP_G])), "google"), "aligned");
  const written = "Authentication-Results: mx.google.com; dmarc=pass header.from=school.test";
  const failing = "Authentication-Results: mx.google.com; dkim=none; dmarc=fail header.from=school.test";
  for (
    const raw of [
      stored([failing], { below: [written] }), // the sender's pass, below the provider's fail
      stored(["Authentication-Results: mx.evil.test; dmarc=pass header.from=school.test"], {
        below: [written],
      }),
      stored([STAMP_M]), // Microsoft's form, on a Gmail mailbox
      stored([STAMP_G], { from: "Pat <pat@friend.test>" }), // a pass for another domain than the From
      stored([STAMP_G], { below: ["From: prof@school.test"] }), // two From fields
    ]
  ) {
    assertEquals(await auth(raw, "google"), "unaligned");
  }
  // Microsoft's stamp, written without an authserv-id.
  assertEquals(await auth(stored([STAMP_M]), "m365"), "aligned");
  // Only the topmost counts: a sender's copy of Microsoft's form below another stamp is never read.
  for (const top of [STAMP_G, "Authentication-Results: mx.evil.test; spf=pass smtp.mailfrom=school.test"]) {
    assertEquals(await auth(stored([top], { below: [STAMP_M] }), "m365"), "unaligned", top);
  }
  // Or the provider's own verifying ARC set at delivery, when it is the topmost.
  assertEquals(await auth(await sealedBy({}), "google"), "aligned");
  assertEquals(await auth(await sealedBy({ key: MICROSOFT, d: "microsoft.com" }), "m365"), "aligned");
  assertEquals(await auth(await sealedBy({ key: MICROSOFT, d: "microsoft.com" }), "google"), "unaligned");
  assertEquals(await auth(await sealedBy({ key: EVIL, d: "evil.test" }), "google"), "unaligned");
  assertEquals(await auth(await sealedBy({ key: EVIL }), "google"), "unaligned"); // google.com, wrong key
  // A seal that no longer verifies (a rotated key, a re-wrapped copy) leaves the stamp to speak.
  const broken = (await sealedBy({})).replace("noon", "dusk");
  assertEquals(await auth(broken, "google"), "unaligned");
  assertEquals(await auth(broken.replace("ARC-Seal:", `${STAMP_G}\r\nARC-Seal:`), "google"), "aligned");
  // A provider Knowlu reads no stamp for has none.
  assertEquals(await auth(stored([STAMP_G]), "other"), "unaligned");
});

Deno.test("no_authentication_result_ever_produces_a_non_2xx_answer", async () => {
  // Nothing a message carries makes binding or authenticity throw: each is a word the handler records.
  const hostile = [
    "",
    "\r\n\r\n",
    "Authentication-Results:\r\n\r\n",
    `Authentication-Results: mx.google.com; ${
      "dmarc=pass header.from=".repeat(20_000)
    }\r\nFrom: a@b.test\r\n\r\n`,
    `Authentication-Results: ${"(".repeat(50_000)}dmarc=pass header.from=b.test\r\nFrom: a@b.test\r\n\r\n`,
    `Authentication-Results: mx.google.com;${";".repeat(100_000)}\r\nFrom: a@b.test\r\n\r\n`,
    "Authentication-Results: mx.google.com; dmarc=pass header.from=\u0000ÿ\r\nFrom: a@ÿ\r\n\r\n",
    `ARC-Authentication-Results: i=1; ${"x".repeat(100_000)}\r\nARC-Seal: i=1; cv=none\r\n\r\n`,
    await gmailForward({ aar: `mx.google.com; ${"dkim=pass header.d=(school.test; ".repeat(2_000)}` }),
  ];
  const throwing: BindDeps = { ...DEPS, resolveTxt: () => Promise.reject(new Error("dns down")) };
  const kinds = new Set(["unbound", "live"]);
  for (const raw of [...hostile, await gmailForward(), stored([STAMP_G])]) {
    for (const input of [raw, Uint8Array.from(raw, (c) => c.charCodeAt(0) & 0xff)]) {
      for (const deps of [DEPS, throwing]) {
        const a = await classifyArrival(input, GMAIL_BOX, CONFIG, CAF, deps);
        assert(kinds.has(a.kind), a.kind);
        for (const p of ["google", "m365", "other"] as const) {
          const w = await originalAuthenticity(input, p, deps);
          assert(w === "aligned" || w === "unaligned", w);
        }
      }
    }
  }
  // An unreachable key server is never a pass: the live forward is unbound, not failed.
  assertEquals(await classifyArrival(await gmailForward(), GMAIL_BOX, CONFIG, CAF, throwing), UNBOUND);
});

Deno.test("a_from_on_a_bare_label_or_public_suffix_never_aligns_with_a_signer_under_it", async () => {
  // Without a public-suffix list, a child of the From domain may be a stranger's registration under it.
  const cases = [
    { from: "Registrar <registrar@edu>", d: "evil.edu" },
    { from: "Registrar <x@co.uk>", d: "evil.co.uk" },
  ];
  for (const { from, d } of cases) {
    const stamp = `Authentication-Results: mx.google.com; dkim=pass header.d=${d} header.s=s1;` +
      ` spf=pass smtp.mailfrom=bounce@${d}`;
    assertEquals(await auth(stored([stamp], { from }), "google"), "unaligned", d);
    assertEquals(
      await auth(stored([stamp.replace("mx.google.com; ", "")], { from }), "m365"),
      "unaligned",
      d,
    );
    const aar = `mx.google.com; dkim=pass header.d=${d} header.s=s1; spf=pass smtp.mailfrom=bounce@${d}`;
    assertEquals(await gmailLive({ from, aar }), LIVE_UNALIGNED, d);
  }
  // A bare-label From never aligns, whatever the record says.
  const own = "mx.google.com; dkim=pass header.d=edu; dmarc=pass header.from=edu";
  assertEquals(await gmailLive({ from: "Registrar <registrar@edu>", aar: own }), LIVE_UNALIGNED);
});
