// Email plan T2d (PQ2 as answered 2026-10-02): our own DKIM and ARC verifier. Every key is made in this
// file and served by an injected resolver, so no test queries DNS. The signer here never calls the
// module's canonicalisation: each signed message carries its canonical form written out by hand, so a
// canonicalisation bug in `dkim.ts` fails a test instead of agreeing with itself. Every domain, address
// and subject is invented for these tests.
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import {
  type ArcResult,
  canonicalBody,
  canonicalHeader,
  type DkimDeps,
  interopReport,
  REASONS,
  type TxtResolver,
  verifyArc,
  verifyDkim,
} from "./dkim.ts";

const RSA = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;

async function rsaKey(bits = 2048): Promise<CryptoKeyPair> {
  return await crypto.subtle.generateKey(
    { ...RSA, modulusLength: bits, publicExponent: new Uint8Array([1, 0, 1]) },
    true,
    ["sign", "verify"],
  );
}

const SENDER = await rsaKey(); // sel._domainkey.example.test
const GOOGLE = await rsaKey(); // arc._domainkey.google.com (a test key; the name only mirrors production)
const MICROSOFT = await rsaKey(); // arc._domainkey.microsoft.com
const ATTACKER = await rsaKey(); // never published under any name a test reads

function bytes(binary: string): Uint8Array<ArrayBuffer> {
  return Uint8Array.from(binary, (c) => c.charCodeAt(0) & 0xff);
}
function b64(b: Uint8Array): string {
  let s = "";
  for (const x of b) s += String.fromCharCode(x);
  return btoa(s);
}
async function sha(s: string): Promise<string> {
  return b64(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes(s))));
}
async function sign(key: CryptoKeyPair, data: string): Promise<string> {
  return b64(new Uint8Array(await crypto.subtle.sign(RSA, key.privateKey, bytes(data))));
}
async function keyRecord(key: CryptoKeyPair): Promise<string> {
  return `v=DKIM1; k=rsa; p=${b64(new Uint8Array(await crypto.subtle.exportKey("spki", key.publicKey)))}`;
}

/** A resolver over a fixed zone that records every name it is asked for. */
function zone(records: Record<string, string[][]>): { resolveTxt: TxtResolver; asked: string[] } {
  const asked: string[] = [];
  return {
    asked,
    resolveTxt: (name: string) => {
      asked.push(name);
      return Promise.resolve(records[name] ?? []);
    },
  };
}
async function defaultZone() {
  return zone({
    "sel._domainkey.example.test": [[await keyRecord(SENDER)]],
    // A long record split into DNS character-strings, as real ones are.
    "arc._domainkey.google.com": [splitChunks(await keyRecord(GOOGLE))],
    "arc._domainkey.microsoft.com": [[await keyRecord(MICROSOFT)]],
  });
}
function splitChunks(s: string): string[] {
  const out: string[] = [];
  for (let i = 0; i < s.length; i += 255) out.push(s.slice(i, i + 255));
  return out;
}
const NOW = () => Date.UTC(2026, 9, 2, 12);
async function deps(): Promise<DkimDeps> {
  return { resolveTxt: (await defaultZone()).resolveTxt, now: NOW };
}

// ---------------------------------------------------------------- DKIM messages, canonical forms by hand

/** The relaxed/relaxed case: folding, runs of spaces and tabs, a space before a colon, mixed-case h=. */
async function relaxedMessage(key = SENDER) {
  const body = "Hello  there \t \r\nline two\r\n\r\n\r\n";
  const canonBody = "Hello there\r\nline two\r\n";
  const bh = await sha(canonBody);
  const sigHead = "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.test; s=sel;\r\n" +
    `\th=From : Subject:to; bh=${bh};\r\n\tb=`;
  const canon = "from:Jane Example <jane@example.test>\r\n" +
    "subject:Lab 5 moved to Friday\r\n" +
    "to:someone@in.example.test\r\n" +
    `dkim-signature:v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.test; s=sel; h=From : Subject:to; bh=${bh}; b=`;
  const b = await sign(key, canon);
  const head = [
    "Received: from relay.example.test by mx.example.test; Fri, 2 Oct 2026 10:00:00 +0000",
    sigHead + b.slice(0, 60) + "\r\n\t " + b.slice(60),
    "From: Jane Example <jane@example.test>",
    "TO :  someone@in.example.test",
    "Subject:  Lab 5   moved\r\n\tto Friday  ",
    "X-Forwarded-For: jane@example.test a1b2@in.example.test",
  ];
  return { head, body };
}

/** The simple/simple case: the signed header bytes and the body's inner spaces count as written. */
async function simpleMessage() {
  const body = "Body  line \r\n\r\n\r\n";
  const canonBody = "Body  line \r\n";
  const bh = await sha(canonBody);
  const sigHead = "DKIM-Signature: v=1; a=rsa-sha256; c=simple/simple; d=example.test; s=sel;\r\n" +
    `\th=from:subject; bh=${bh}; b=`;
  const canon = "From: jane@example.test\r\n" + "Subject:  Spaces  kept \r\n" + sigHead;
  const b = await sign(SENDER, canon);
  const head = [sigHead + b, "From: jane@example.test", "Subject:  Spaces  kept "];
  return { head, body };
}

function assemble(m: { head: string[]; body: string }): string {
  return m.head.join("\r\n") + "\r\n\r\n" + m.body;
}

// ---------------------------------------------------------------- DKIM

Deno.test("verifies_relaxed_and_simple_rsa_sha256_signatures_made_with_an_in_test_key", async () => {
  // RFC 6376 §3.4.6's own example, both ways.
  assertEquals(canonicalHeader("A: X", "relaxed"), "a:X");
  assertEquals(canonicalHeader("B : Y\t\r\n\tZ  ", "relaxed"), "b:Y Z");
  assertEquals(canonicalHeader("B : Y\t\r\n\tZ  ", "simple"), "B : Y\t\r\n\tZ  ");
  assertEquals(canonicalBody(" C \r\nD \t E\r\n\r\n\r\n", "relaxed"), " C\r\nD E\r\n");
  assertEquals(canonicalBody(" C \r\nD \t E\r\n\r\n\r\n", "simple"), " C \r\nD \t E\r\n");
  // The empty body: simple is one CRLF, relaxed is nothing (RFC 6376 errata 3192).
  assertEquals(canonicalBody("", "simple"), "\r\n");
  assertEquals(canonicalBody("\r\n\r\n", "relaxed"), "");
  assertEquals(canonicalBody("no break at the end", "relaxed"), "no break at the end\r\n");

  const d = await deps();
  for (const m of [await relaxedMessage(), await simpleMessage()]) {
    const text = assemble(m);
    for (const input of [text, bytes(text), text.replaceAll("\r\n", "\n")]) {
      const [r, ...rest] = await verifyDkim(input, d);
      assertEquals(rest, []);
      assertEquals(r.result, "pass", `reason ${r.reason}`);
      assertEquals(r.reason, null);
      assertEquals(r.header, "dkim-signature");
      assertEquals(r.domain, "example.test");
      assertEquals(r.selector, "sel");
    }
  }
  const [relaxed] = await verifyDkim(assemble(await relaxedMessage()), d);
  assertEquals(relaxed.signedHeaders, ["from", "subject", "to"]);
  assertEquals(relaxed.covered.map((f) => f.name), ["from", "subject", "to"]);
  assertEquals(relaxed.covered[0].value, "Jane Example <jane@example.test>");
});

Deno.test("a_changed_signed_header_fails", async () => {
  const d = await deps();
  const m = await relaxedMessage();
  const subject = m.head.findIndex((h) => h.startsWith("Subject:"));
  const changed = { ...m, head: m.head.with(subject, "Subject: Lab 5 moved to Monday") };
  const [r] = await verifyDkim(assemble(changed), d);
  assertEquals(r.result, "fail");
  assertEquals(r.reason, "signature");
  assertEquals(r.covered, [], "a failed signature vouches for no field");

  // Relaxed forgives whitespace only: a refold of the same words still passes.
  const refolded = { ...m, head: m.head.with(subject, "Subject: Lab 5\r\n moved   to Friday") };
  assertEquals((await verifyDkim(assemble(refolded), d))[0].result, "pass");

  const body = { ...m, body: "Hello there, changed\r\n" };
  const [rb] = await verifyDkim(assemble(body), d);
  assertEquals([rb.result, rb.reason], ["fail", "body_hash"]);

  // A different key signed it: the published key does not verify it.
  const [ra] = await verifyDkim(assemble(await relaxedMessage(ATTACKER)), d);
  assertEquals([ra.result, ra.reason], ["fail", "signature"]);

  // Simple canonicalisation counts every byte of a signed header.
  const s = await simpleMessage();
  const spaced = { ...s, head: s.head.with(2, "Subject: Spaces  kept ") };
  assertEquals((await verifyDkim(assemble(spaced), d))[0].result, "fail");
});

Deno.test("a_header_absent_from_h_is_not_covered", async () => {
  const d = await deps();
  const m = await relaxedMessage();
  // X-Forwarded-For is not in h=: rewriting it leaves the signature passing and vouching for nothing new.
  const xff = m.head.findIndex((h) => h.startsWith("X-Forwarded-For:"));
  const forged = { ...m, head: m.head.with(xff, "X-Forwarded-For: someone-else@example.test") };
  const [r] = await verifyDkim(assemble(forged), d);
  assertEquals(r.result, "pass");
  assert(!r.signedHeaders.includes("x-forwarded-for"));
  assert(!r.covered.some((f) => f.name === "x-forwarded-for"));

  // A second From added above the signed one passes too (signed fields are taken from the bottom up),
  // and what the signature covers is the original From, never the added one.
  const added = { ...m, head: ["From: Attacker <attacker@example.test>", ...m.head] };
  const [ra] = await verifyDkim(assemble(added), d);
  assertEquals(ra.result, "pass");
  assertEquals(ra.covered.filter((f) => f.name === "from").map((f) => f.value), [
    "Jane Example <jane@example.test>",
  ]);
});

// ---------------------------------------------------------------- ARC messages

interface Msg {
  head: string[];
  body: string;
  /** The canonical (relaxed) lines of every ARC set so far, in instance order, each ending CRLF. */
  lower: string;
}
interface Hop {
  i: number;
  d: string;
  key: CryptoKeyPair;
  cv: string;
  aar: string;
  h: string;
  /** Fields the hop adds above the message before it signs. */
  adds?: string[];
  /** The canonical form of the fields `h` selects, written by hand, each ending CRLF. */
  covered: string;
}
const BODY = "Lab 5 is due Friday.\r\n"; // already canonical under relaxed
/** Relaxed form of a field written `Name: value` with single spaces and no folding. */
function rc(line: string): string {
  const c = line.indexOf(":");
  return line.slice(0, c).toLowerCase() + ":" + line.slice(c + 2);
}
function original(): Msg {
  return {
    head: ["From: Prof Example <prof@example.test>", "Subject: Lab 5", "Message-ID: <m1@example.test>"],
    body: BODY,
    lower: "",
  };
}
async function seal(m: Msg, hop: Hop): Promise<Msg> {
  const bh = await sha(BODY);
  const amsHead =
    `ARC-Message-Signature: i=${hop.i}; a=rsa-sha256; c=relaxed/relaxed; d=${hop.d}; s=arc; h=${hop.h}; bh=${bh}; b=`;
  const ams = amsHead + await sign(hop.key, hop.covered + rc(amsHead));
  const aar = `ARC-Authentication-Results: i=${hop.i}; ${hop.aar}`;
  const asHead = `ARC-Seal: i=${hop.i}; a=rsa-sha256; t=1790000000; cv=${hop.cv}; d=${hop.d}; s=arc; b=`;
  const set = rc(aar) + "\r\n" + rc(ams) + "\r\n";
  const as = asHead + await sign(hop.key, m.lower + set + rc(asHead));
  return {
    head: [as, ams, aar, ...(hop.adds ?? []), ...m.head],
    body: m.body,
    lower: m.lower + set + rc(as) + "\r\n",
  };
}
const XFF = "X-Forwarded-For: jane@example.test a1b2@in.example.test";
const MS_HOP: Hop = {
  i: 1,
  d: "microsoft.com",
  key: MICROSOFT,
  cv: "none",
  aar: "mx.microsoft.com; dkim=pass header.d=example.test",
  h: "from:subject",
  covered: "from:Prof Example <prof@example.test>\r\nsubject:Lab 5\r\n",
};
const GOOGLE_HOP: Hop = {
  i: 2,
  d: "google.com",
  key: GOOGLE,
  cv: "pass",
  aar: "mx.google.com; arc=pass (i=1); dmarc=pass header.from=example.test",
  h: "x-forwarded-for:from:subject",
  adds: [XFF],
  covered: "x-forwarded-for:jane@example.test a1b2@in.example.test\r\n" +
    "from:Prof Example <prof@example.test>\r\nsubject:Lab 5\r\n",
};
async function twoHops(top: Partial<Hop> = {}): Promise<Msg> {
  return await seal(await seal(original(), MS_HOP), { ...GOOGLE_HOP, ...top });
}
function text(m: { head: string[]; body: string }): string {
  return m.head.join("\r\n") + "\r\n\r\n" + m.body;
}
function failed(r: ArcResult): { part: string; reason: string; instance: number } {
  assert(r.result === "fail" || r.result === "temperror", `expected a failure, got ${r.result}`);
  assert(!("sealDomain" in r) && !("covered" in r), "a failure reports no domain and covers nothing");
  return { part: r.part, reason: r.reason, instance: r.instance };
}

// ---------------------------------------------------------------- ARC

Deno.test("the_topmost_arc_instances_seal_and_its_ams_verify_and_report_the_sealing_domain", async () => {
  const d = await deps();
  const r = await verifyArc(text(await twoHops()), d);
  assert(r.result === "pass", JSON.stringify(r));
  assertEquals(r.instance, 2);
  assertEquals(r.cv, "pass");
  assertEquals(r.sealDomain, "google.com");
  assertEquals(r.amsDomain, "google.com");
  assertEquals(r.signedHeaders, ["x-forwarded-for", "from", "subject"]);
  assertEquals(r.covered[0], { name: "x-forwarded-for", value: "jane@example.test a1b2@in.example.test" });
  assertEquals(r.authResults, "mx.google.com; arc=pass (i=1); dmarc=pass header.from=example.test");

  const one = await verifyArc(text(await seal(original(), MS_HOP)), d);
  assert(
    one.result === "pass" && one.sealDomain === "microsoft.com" && one.instance === 1 && one.cv === "none",
  );
  assertEquals(await verifyArc(text(original()), d), { result: "none", instance: 0 });

  // The AMS covers the binding header: a rewritten X-Forwarded-For fails the AMS.
  const m = await twoHops();
  const forged = {
    ...m,
    head: m.head.map((h) => h === XFF ? "X-Forwarded-For: x@example.test a1b2@in.example.test" : h),
  };
  assertEquals(failed(await verifyArc(text(forged), d)), { part: "ams", reason: "signature", instance: 2 });
  assertEquals(failed(await verifyArc(text({ ...m, body: "Lab 5 is due Monday.\r\n" }), d)), {
    part: "ams",
    reason: "body_hash",
    instance: 2,
  });
  // The seal covers every lower set: a rewritten i=1 result fails the topmost seal.
  const lowered = {
    ...m,
    head: m.head.map((h) => h.replace("dkim=pass header.d=example.test", "dkim=pass")),
  };
  assertEquals(failed(await verifyArc(text(lowered), d)), { part: "seal", reason: "signature", instance: 2 });
  // A seal whose cv does not fit its instance is refused.
  assertEquals(failed(await verifyArc(text(await twoHops({ cv: "none" })), d)), {
    part: "structure",
    reason: "cv",
    instance: 2,
  });
});

Deno.test("a_lower_arc_instance_is_never_read_as_the_topmost", async () => {
  const d = await deps();
  // The topmost seal does not verify (another key, google.com's name): no fallback to the good i=1.
  assertEquals(failed(await verifyArc(text(await twoHops({ key: ATTACKER })), d)), {
    part: "seal",
    reason: "signature",
    instance: 2,
  });
  // A stranger's set above the provider's: it is the topmost, it fails, and i=2 is never read instead.
  const m = await twoHops();
  const stranger = await seal(m, {
    i: 3,
    d: "attacker.test",
    key: ATTACKER,
    cv: "pass",
    aar: "attacker.test; dmarc=pass",
    h: "from",
    covered: "from:Prof Example <prof@example.test>\r\n",
  });
  assertEquals(failed(await verifyArc(text(stranger), d)), { part: "seal", reason: "no_key", instance: 3 });
  // Instances are read from i=, never from position: the i=1 set moved to the top changes nothing.
  const moved = { ...m, head: [...m.head.slice(4, 7), ...m.head.slice(0, 4), ...m.head.slice(7)] };
  assert(moved.head[0].startsWith("ARC-Seal: i=1;"));
  const r = await verifyArc(text(moved), d);
  assert(r.result === "pass" && r.instance === 2 && r.sealDomain === "google.com", JSON.stringify(r));
  // A lone higher seal, a duplicated instance, or a gap is a broken chain, never a shorter one.
  const lone = {
    ...m,
    head: ["ARC-Seal: i=3; a=rsa-sha256; cv=pass; d=google.com; s=arc; b=AAAA", ...m.head],
  };
  assertEquals(failed(await verifyArc(text(lone), d)), {
    part: "structure",
    reason: "structure",
    instance: 3,
  });
  const dup = { ...m, head: [m.head[1], ...m.head] };
  assertEquals(failed(await verifyArc(text(dup), d)), {
    part: "structure",
    reason: "structure",
    instance: 2,
  });
  const gap = await seal(original(), { ...GOOGLE_HOP, cv: "pass" }); // i=2 with no i=1 beneath it
  assertEquals(failed(await verifyArc(text(gap), d)), {
    part: "structure",
    reason: "structure",
    instance: 2,
  });
});

Deno.test("the_key_resolver_is_injected_and_no_test_queries_dns", async () => {
  const realResolve = Deno.resolveDns;
  let dnsCalls = 0;
  // deno-lint-ignore no-explicit-any
  (Deno as any).resolveDns = () => {
    dnsCalls++;
    return Promise.reject(new Error("no DNS in tests"));
  };
  try {
    const z = await defaultZone();
    const d = { resolveTxt: z.resolveTxt, now: NOW };
    assertEquals((await verifyDkim(text(await relaxedMessage()), d))[0].result, "pass");
    assertEquals((await verifyArc(text(await twoHops()), d)).result, "pass");
    // Each key is asked for once per message, though google.com's signs both the seal and the AMS.
    assertEquals(z.asked, ["sel._domainkey.example.test", "arc._domainkey.google.com"]);

    const msg = text(await relaxedMessage());
    const outcome = async (resolveTxt: TxtResolver) => {
      const [r] = await verifyDkim(msg, { resolveTxt, now: NOW });
      return [r.result, r.reason];
    };
    assertEquals(await outcome(() => Promise.reject(new Error("timeout"))), ["temperror", "dns"]);
    assertEquals(await outcome(() => Promise.resolve([])), ["permerror", "no_key"]);
    assertEquals(await outcome(() => Promise.resolve([["v=DKIM1; k=rsa; p="]])), [
      "permerror",
      "key_revoked",
    ]);
    assertEquals(await outcome(() => Promise.resolve([["v=DKIM1; k=ed25519; p=AAAA"]])), [
      "permerror",
      "key_type",
    ]);
    assertEquals(await outcome(() => Promise.resolve([["v=DKIM1; k=rsa; p=bm90IGEga2V5"]])), [
      "permerror",
      "key_syntax",
    ]);
    const weak = await keyRecord(await rsaKey(512));
    assertEquals(await outcome(() => Promise.resolve([[weak]])), ["permerror", "key_weak"]);
    assertEquals(dnsCalls, 0);
  } finally {
    // deno-lint-ignore no-explicit-any
    (Deno as any).resolveDns = realResolve;
  }
  // The platform resolver is named in one place only, the production factory.
  const src = await Deno.readTextFile(new URL("./dkim.ts", import.meta.url));
  assertEquals(src.split("Deno.resolveDns").length - 1, 1);
});

// ---------------------------------------------------------------- what the module never does

Deno.test("hostile_input_never_throws_and_every_reason_is_a_fixed_word", async () => {
  const d = await deps();
  const now = NOW();
  const expired = (await relaxedMessage()).head; // x= in the past, on a signature that is otherwise fine
  const inputs = [
    "",
    "\r\n\r\n",
    "no header block at all",
    "DKIM-Signature: garbage\r\n\r\nbody",
    "DKIM-Signature: v=1; a=rsa-sha1; d=example.test; s=sel; h=from; bh=AA==; b=AA==\r\nFrom: a\r\n\r\n",
    "DKIM-Signature: v=1; a=rsa-sha256; d=example.test; s=sel; h=subject; bh=AA==; b=AA==\r\n\r\n",
    "DKIM-Signature: v=1; a=rsa-sha256; d=../x; s=sel; h=from; bh=AA==; b=AA==\r\n\r\n",
    "DKIM-Signature: v=1; v=1; a=rsa-sha256\r\n\r\n",
    "ARC-Seal: i=x; cv=none\r\n\r\n",
    "ARC-Authentication-Results: no instance\r\n\r\n",
    // Long whitespace runs through both canonicalisations: a linear walk, never a backtracking one.
    ...["relaxed/relaxed", "simple/simple"].map((c) =>
      `DKIM-Signature: v=1; a=rsa-sha256; c=${c}; d=example.test; s=sel; h=from:subject; bh=AAAA; b=AAAA\r\n` +
      "From: a\r\nSubject: " + " ".repeat(200_000) + "x\r\n\r\n" + " \t".repeat(200_000) + "x" +
      "\r\n".repeat(100_000) + "x" + "\r\n".repeat(100_000)
    ),
    expired.join("\r\n").replace("d=example.test;", `d=example.test; x=${Math.floor(now / 1000) - 60};`),
  ];
  const words = new Set(REASONS);
  for (const input of inputs) {
    for (const r of await verifyDkim(input, d)) {
      assertNotEquals(r.result, "pass");
      assert(r.reason !== null && words.has(r.reason), `reason ${r.reason}`);
    }
    const a = await verifyArc(input, d);
    if (a.result === "fail" || a.result === "temperror") assert(words.has(a.reason), `reason ${a.reason}`);
  }
  const [x] = await verifyDkim(inputs[inputs.length - 1], d);
  assertEquals([x.result, x.reason], ["fail", "expired"]);
  const [weak] = await verifyDkim(inputs[4], d);
  assertEquals([weak.result, weak.reason], ["permerror", "algorithm"]);
  const [nofrom] = await verifyDkim(inputs[5], d);
  assertEquals([nofrom.result, nofrom.reason], ["permerror", "from_unsigned"]);
});

Deno.test("the_interop_report_names_results_and_provider_domains_and_no_content", async () => {
  const d = await deps();
  const m = await twoHops();
  const signed = await relaxedMessage();
  const lines = await interopReport(text({ head: [...m.head, ...signed.head.slice(1, 2)], body: m.body }), d);
  const all = lines.join("\n");
  assert(all.includes("arc i=2 pass seal=google.com ams=google.com cv=pass"), all);
  assert(all.includes("x-forwarded-for=yes"), all);
  assert(all.includes("x-ms-exchange-forwardingloop=no"), all);
  assert(/dkim #1 fail body_hash d=other/.test(all), all);
  for (const secret of ["@", "example.test", "jane", "Prof", "Lab 5", "a1b2"]) {
    assert(!all.includes(secret), `the report carries "${secret}"`);
  }
});
