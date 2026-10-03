// Spec test 1 (email plan T2a): SNS signature, topic and certificate host. The certificate host is a
// listener on 127.0.0.1:0 serving a certificate made in this file for a key made in this file; the
// injected fetcher maps the real-shaped `https://sns.<region>.amazonaws.com/...` URL onto it, so the
// allow-list is checked against what production sees and no request leaves the machine.
import { assert, assertEquals } from "@std/assert";
import { type SnsFetch, verifySns } from "./sns.ts";

const TOPIC = "arn:aws:sns:us-east-2:000000000000:mail-test-topic";
const OTHER_TOPIC = "arn:aws:sns:us-east-2:000000000000:someone-elses-topic";
/** Thirty seconds after every fixture's `Timestamp`. */
const NOW = () => Date.UTC(2026, 9, 2, 12, 0, 30);
const CERT_URL = "https://sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem";
const SUBSCRIBE_URL =
  "https://sns.us-east-2.amazonaws.com/?Action=ConfirmSubscription&TopicArn=t&Token=tok-test";
// A synthetic SES notification: every address and id is invented for this test.
const MESSAGE = JSON.stringify({
  notificationType: "Received",
  mail: { destination: ["a1b2c3@in.example.test"], messageId: "ses-test-1" },
  receipt: { action: { type: "S3", bucketName: "b", objectKey: "inbound/ses-test-1" } },
});

// ---- a minimal DER writer, enough for a certificate whose only read part is its key ----
function derLen(n: number): number[] {
  if (n < 0x80) return [n];
  const out: number[] = [];
  for (let v = n; v > 0; v >>= 8) out.unshift(v & 0xff);
  return [0x80 | out.length, ...out];
}
function der(tag: number, ...parts: Uint8Array[]): Uint8Array {
  const body = parts.reduce((n, p) => n + p.length, 0);
  const head = [tag, ...derLen(body)];
  const out = new Uint8Array(head.length + body);
  out.set(head, 0);
  let at = head.length;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}
const bytes = (...b: number[]) => new Uint8Array(b);
function certPem(spki: Uint8Array): string {
  const algId = der(
    0x30,
    der(0x06, bytes(0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b)),
    bytes(5, 0),
  );
  const time = der(0x17, new TextEncoder().encode("260101000000Z"));
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
  const b64 = btoa(String.fromCharCode(...cert)).replace(/(.{64})/g, "$1\n");
  return `-----BEGIN CERTIFICATE-----\n${b64}\n-----END CERTIFICATE-----\n`;
}

async function keyPair(): Promise<CryptoKeyPair> {
  return await crypto.subtle.generateKey(
    { name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: bytes(1, 0, 1), hash: "SHA-256" },
    true,
    ["sign", "verify"],
  ) as CryptoKeyPair;
}
async function spkiOf(keys: CryptoKeyPair): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.exportKey("spki", keys.publicKey));
}

// AWS's string to sign, written out here independently of `sns.ts`.
const SIGNED: Record<string, string[]> = {
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
async function signed(keys: CryptoKeyPair, fields: Record<string, string>): Promise<Record<string, string>> {
  const text = SIGNED[fields.Type]
    .filter((k) => fields[k] !== undefined)
    .map((k) => `${k}\n${fields[k]}\n`)
    .join("");
  const sig = await crypto.subtle.sign("RSASSA-PKCS1-v1_5", keys.privateKey, new TextEncoder().encode(text));
  return { ...fields, Signature: btoa(String.fromCharCode(...new Uint8Array(sig))) };
}
function notification(over: Record<string, string> = {}): Record<string, string> {
  return {
    Type: "Notification",
    MessageId: "00000000-0000-0000-0000-000000000001",
    TopicArn: TOPIC,
    Subject: "Amazon SES Email Receipt Notification",
    Message: MESSAGE,
    Timestamp: "2026-10-02T12:00:00.000Z",
    SignatureVersion: "2",
    SigningCertURL: CERT_URL,
    ...over,
  };
}
function handshake(over: Record<string, string> = {}): Record<string, string> {
  return {
    Type: "SubscriptionConfirmation",
    MessageId: "00000000-0000-0000-0000-000000000002",
    Token: "tok-test",
    TopicArn: TOPIC,
    Message: "You have chosen to subscribe to the topic.",
    SubscribeURL: SUBSCRIBE_URL,
    Timestamp: "2026-10-02T12:00:00.000Z",
    SignatureVersion: "2",
    SigningCertURL: CERT_URL,
    ...over,
  };
}

/** The local certificate host, and a fetcher that maps an AWS URL onto it and records every ask. */
async function withHost(
  pem: string,
  fn: (fetcher: SnsFetch, asked: string[]) => Promise<void>,
): Promise<void> {
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, (req) => {
    const url = new URL(req.url);
    if (url.pathname.endsWith(".pem")) return new Response(pem);
    if (url.searchParams.get("Action") === "ConfirmSubscription") return new Response("<ok/>");
    return new Response("not found", { status: 404 });
  });
  const asked: string[] = [];
  const fetcher: SnsFetch = (raw, init) => {
    asked.push(raw);
    const u = new URL(raw);
    return fetch(`http://127.0.0.1:${server.addr.port}${u.pathname}${u.search}`, init);
  };
  try {
    await fn(fetcher, asked);
  } finally {
    await server.shutdown();
  }
}

function post(body: unknown): Request {
  return new Request("http://127.0.0.1/mail-inbound", {
    method: "POST",
    headers: { "content-type": "text/plain; charset=UTF-8" },
    body: typeof body === "string" ? body : JSON.stringify(body),
  });
}

/** The status a call answers: a thrown `Response` is the refusal; a return is a pass (200). */
async function statusOf(p: Promise<unknown>): Promise<number> {
  try {
    await p;
    return 200;
  } catch (e) {
    if (e instanceof Response) {
      await e.body?.cancel();
      return e.status;
    }
    throw e;
  }
}

Deno.test("a_valid_signature_from_the_local_cert_host_is_accepted", async () => {
  const keys = await keyPair();
  await withHost(certPem(await spkiOf(keys)), async (fetcher, asked) => {
    const got = await verifySns(post(await signed(keys, notification())), { topicArn: TOPIC }, {
      fetch: fetcher,
      now: NOW,
    });
    assertEquals(got, { kind: "notification", message: MESSAGE });
    assertEquals(asked, [CERT_URL]);
    // A notification without a Subject is signed without one, and still verifies.
    const bare = notification();
    delete bare.Subject;
    const got2 = await verifySns(post(await signed(keys, bare)), { topicArn: TOPIC }, {
      fetch: fetcher,
      now: NOW,
    });
    assertEquals(got2, { kind: "notification", message: MESSAGE });
  });
});

Deno.test("a_wrong_topic_is_refused", async () => {
  const keys = await keyPair();
  await withHost(certPem(await spkiOf(keys)), async (fetcher, asked) => {
    const body = await signed(keys, notification({ TopicArn: OTHER_TOPIC }));
    assertEquals(
      await statusOf(verifySns(post(body), { topicArn: TOPIC }, { fetch: fetcher, now: NOW })),
      403,
    );
    // Refused before anything is read: not even the certificate is fetched.
    assertEquals(asked, []);
  });
});

Deno.test("a_bad_signature_is_refused", async () => {
  const keys = await keyPair();
  const stranger = await keyPair();
  await withHost(certPem(await spkiOf(keys)), async (fetcher, asked) => {
    const cfg = { topicArn: TOPIC };
    const deps = { fetch: fetcher, now: NOW };
    // Signed by a key the certificate does not hold.
    assertEquals(await statusOf(verifySns(post(await signed(stranger, notification())), cfg, deps)), 403);
    // Signed by the right key, then one field changed.
    const tampered = await signed(keys, notification());
    tampered.Message = tampered.Message.replace("ses-test-1", "ses-test-2");
    assertEquals(await statusOf(verifySns(post(tampered), cfg, deps)), 403);
    // A signature that is not base64, and no signature at all.
    const junk = { ...(await signed(keys, notification())), Signature: "!!not-base64!!" };
    assertEquals(await statusOf(verifySns(post(junk), cfg, deps)), 403);
    const unsigned = notification();
    assertEquals(await statusOf(verifySns(post(unsigned), cfg, deps)), 403);
    // Only SignatureVersion 2 (SHA-256) is accepted; a version-1 label is refused unread.
    const before = asked.length;
    const v1 = await signed(keys, notification({ SignatureVersion: "1" }));
    assertEquals(await statusOf(verifySns(post(v1), cfg, deps)), 403);
    assertEquals(asked.length, before);
    // Not JSON, not an object, and an unknown type.
    assertEquals(await statusOf(verifySns(post("not json"), cfg, deps)), 403);
    assertEquals(await statusOf(verifySns(post("[1,2]"), cfg, deps)), 403);
    const other = await signed(keys, notification());
    other.Type = "UnsubscribeConfirmation";
    assertEquals(await statusOf(verifySns(post(other), cfg, deps)), 403);
  });
  // A certificate host that serves something other than a certificate is a refusal too.
  await withHost("not a certificate", async (fetcher) => {
    const body = await signed(keys, notification());
    assertEquals(
      await statusOf(verifySns(post(body), { topicArn: TOPIC }, { fetch: fetcher, now: NOW })),
      403,
    );
  });
});

Deno.test("a_cert_url_outside_https_sns_region_amazonaws_com_is_refused", async () => {
  const keys = await keyPair();
  const outside = [
    "http://sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem",
    "https://sns.us-east-1.amazonaws.com/SimpleNotificationService-00test.pem",
    "https://sns.us-east-2.amazonaws.com.attacker.example/SimpleNotificationService-00test.pem",
    "https://attacker.example/sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem",
    "https://xsns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem",
    "https://sns.us-east-2.amazonaws.com:8443/SimpleNotificationService-00test.pem",
    "https://user@sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem",
    "https://sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.txt",
    "https://sns.us-east-2.amazonaws.com/SimpleNotificationService-00test.pem?x=1",
    "not a url",
  ];
  await withHost(certPem(await spkiOf(keys)), async (fetcher, asked) => {
    for (const url of outside) {
      const body = await signed(keys, notification({ SigningCertURL: url }));
      assertEquals(
        await statusOf(verifySns(post(body), { topicArn: TOPIC }, { fetch: fetcher, now: NOW })),
        403,
        url,
      );
    }
    assertEquals(asked, []);
    // The same message from the allowed host passes, so the host is what was refused.
    const ok = await signed(keys, notification());
    assertEquals(await statusOf(verifySns(post(ok), { topicArn: TOPIC }, { fetch: fetcher, now: NOW })), 200);
  });
});

Deno.test("a_subscription_handshake_is_confirmed_only_for_the_configured_topic", async () => {
  const keys = await keyPair();
  const stranger = await keyPair();
  await withHost(certPem(await spkiOf(keys)), async (fetcher, asked) => {
    const cfg = { topicArn: TOPIC };
    const deps = { fetch: fetcher, now: NOW };
    // Another topic: refused, and the SubscribeURL is never visited.
    const wrong = await signed(keys, handshake({ TopicArn: OTHER_TOPIC }));
    assertEquals(await statusOf(verifySns(post(wrong), cfg, deps)), 403);
    // A SubscribeURL off the allowed host: refused unread.
    const offHost = await signed(keys, handshake({ SubscribeURL: "https://attacker.example/?Action=x" }));
    assertEquals(await statusOf(verifySns(post(offHost), cfg, deps)), 403);
    assertEquals(asked, []);
    // A bad signature: the certificate is read, the SubscribeURL is not.
    const forged = await signed(stranger, handshake());
    assertEquals(await statusOf(verifySns(post(forged), cfg, deps)), 403);
    assertEquals(asked, [CERT_URL]);
    // The configured topic, signed: confirmed by visiting the SubscribeURL.
    const right = await signed(keys, handshake());
    assertEquals(await verifySns(post(right), cfg, deps), { kind: "confirmed" });
    assertEquals(asked, [CERT_URL, CERT_URL, SUBSCRIBE_URL]);
  });
  // A confirmation the SNS host refuses is not a refusal of ours: it answers 500 so SNS retries.
  await withHost(certPem(await spkiOf(keys)), async (fetcher) => {
    const failing: SnsFetch = (raw, init) =>
      raw.includes("ConfirmSubscription")
        ? Promise.resolve(new Response("", { status: 503 }))
        : fetcher(raw, init);
    const right = await signed(keys, handshake());
    assert(
      (await statusOf(verifySns(post(right), { topicArn: TOPIC }, { fetch: failing, now: NOW }))) === 500,
    );
  });
});

Deno.test("a_notification_older_than_an_hour_or_with_no_readable_timestamp_is_refused", async () => {
  // Checkpoint B (minor): a captured notification replayed later is refused on its signed Timestamp, before
  // any certificate is fetched, so a replay costs no S3 GET and no 500.
  const keys = await keyPair();
  await withHost(certPem(await spkiOf(keys)), async (fetcher, asked) => {
    const cfg = { topicArn: TOPIC };
    const at = (iso: string) => ({ fetch: fetcher, now: () => Date.parse(iso) });
    const n = await signed(keys, notification());
    assertEquals(await statusOf(verifySns(post(n), cfg, at("2026-10-02T13:00:00.001Z"))), 403);
    assertEquals(await statusOf(verifySns(post(n), cfg, at("2026-10-03T12:00:00.000Z"))), 403);
    const h = await signed(keys, handshake());
    assertEquals(await statusOf(verifySns(post(h), cfg, at("2026-10-02T13:00:00.001Z"))), 403);
    for (const Timestamp of ["", "yesterday", "2026-13-40T99:00:00Z"]) {
      const bad = await signed(keys, notification({ Timestamp }));
      assertEquals(await statusOf(verifySns(post(bad), cfg, at("2026-10-02T12:00:30.000Z"))), 403, Timestamp);
    }
    assertEquals(asked, [], "refused before the certificate is fetched");
    // Inside the hour (SNS's retries of one delivery keep its Timestamp), it verifies.
    assertEquals(await statusOf(verifySns(post(n), cfg, at("2026-10-02T12:59:59.000Z"))), 200);
  });
});
