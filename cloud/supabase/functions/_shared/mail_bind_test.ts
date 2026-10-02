// Email plan T2e (spec §4.6 step 5's first three cases; tests 4, 5 and 6, the recognition halves). Every
// key is made here and served by an injected resolver, so no test queries DNS; the nonce HMAC is a fake.
// Every domain, address, nonce and link is invented for these tests.
import { assert, assertEquals } from "@std/assert";
import { canonicalBody, canonicalHeader, type TxtResolver } from "./dkim.ts";
import {
  type Arrival,
  type BindConfig,
  type BindDeps,
  type BindMailbox,
  classifyArrival,
  type ConfirmRefusal,
  confirmUrlAllowed,
  newNonce,
  nonceTag,
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
};
const resolveTxt: TxtResolver = (name) => Promise.resolve(ZONE[name] ?? []);

const NOW = Date.UTC(2026, 9, 2, 12);
const DEPS: BindDeps = { resolveTxt, now: () => NOW, nonceHmac: (n) => Promise.resolve(`hmac:${n}`) };
const CONFIG: BindConfig = { canaryFrom: "check@knowlu.test" };
const NONCE = "0123456789abcdef0123456789abcdef";
const OTHER_NONCE = "fedcba9876543210fedcba9876543210";
const TENANT = "72f988bf-86f1-41af-91ab-2d7cd011db47";

function mailbox(over: Partial<BindMailbox> = {}): BindMailbox {
  return {
    id: "addr-1",
    provider: "google",
    mailboxHint: "jane@gmail.com",
    confirmUntil: NOW + 10 * 60_000,
    batches: [],
    canaries: [],
    ...over,
  };
}

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
  const left = [...head];
  let data = "";
  for (const name of s.h) {
    for (let i = left.length - 1; i >= 0; i--) {
      if (left[i].slice(0, left[i].indexOf(":")).trim().toLowerCase() !== name) continue;
      data += canonicalHeader(left[i], "relaxed") + "\r\n";
      left.splice(i, 1);
      break;
    }
  }
  data += canonicalHeader(sig, "relaxed");
  const b = b64(new Uint8Array(await crypto.subtle.sign(RSA, s.key.privateKey, bytes(data))));
  return [sig + b, ...head];
}

const assemble = (head: string[], body: string) => head.join("\r\n") + "\r\n\r\n" + body;

function classify(raw: string, box: BindMailbox = mailbox(), config = CONFIG): Promise<Arrival> {
  return classifyArrival(raw, box, config, DEPS);
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
  assertEquals((await classify(await batchSend(), closed)).kind, "other");
  assertEquals((await classify(await batchSend({ subject: "Fwd: three messages" }), box)).kind, "other");
  const seeded = await batchSend({ subject: "Fwd: three messages", above: [`Subject: ${nonceTag(NONCE)}`] });
  assertEquals((await classify(seeded, box)).kind, "other");
});

Deno.test("a_wrong_outer_sender_is_not_a_batch_send", async () => {
  const box = mailbox({ batches: [OPEN] });
  // Another sender, with its own good signature and the right nonce.
  const evil: Signer = { key: EVIL, d: "evil.test", h: ["from", "to", "subject"] };
  assertEquals(
    (await classify(await batchSend({ from: "mallory@evil.test", signer: evil }), box)).kind,
    "other",
  );
  // The mailbox's address, signed by a domain it is not aligned with, or not signed at all.
  assertEquals((await classify(await batchSend({ signer: evil }), box)).kind, "other");
  assertEquals((await classify(await batchSend({ signer: null }), box)).kind, "other");
  // Another Gmail user's signed send, with the mailbox's address in an unsigned From above it.
  const other = await batchSend({ from: "mallory@gmail.com", above: ["From: jane@gmail.com"] });
  assertEquals((await classify(other, box)).kind, "other");
  // No hint yet: nothing can be the mailbox's own send.
  assertEquals(
    (await classify(await batchSend(), mailbox({ mailboxHint: null, batches: [OPEN] }))).kind,
    "other",
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
    assertEquals((await classify(await confirmation({ from, signer }))).kind, "other", from);
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
  assertEquals((await classify(await canary({ nonce: OTHER_NONCE }), SCHOOL)).kind, "other");
});

Deno.test("a_canary_from_a_wrong_sender_domain_is_not_a_canary", async () => {
  const evil: Signer = { ...KNOWLU_SIGNS, key: EVIL, d: "evil.test" };
  for (const from of ["check@knowlu.test.evil.test", "check@evil.test", "other@knowlu.test"]) {
    const signer = from.endsWith("@knowlu.test") ? KNOWLU_SIGNS : evil;
    assertEquals((await classify(await canary({ from, signer }), SCHOOL)).kind, "other", from);
  }
  // Knowlu's sender, signed by another domain or with another key under Knowlu's name.
  assertEquals((await classify(await canary({ signer: evil }), SCHOOL)).kind, "other");
  assertEquals(
    (await classify(await canary({ signer: { ...KNOWLU_SIGNS, key: EVIL } }), SCHOOL)).kind,
    "other",
  );
});
