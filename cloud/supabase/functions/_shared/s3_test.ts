// Spec test 16, the SigV4 half (email plan T2b): get, put and delete on one bucket, signed with
// SigV4, against a fake bucket on 127.0.0.1:0. The fake checks every request's signature with its own
// implementation written out in this file (never `s3.ts`'s), against the virtual host production
// would send to; the injected fetcher maps the real-shaped `https://<bucket>.s3.<region>.amazonaws.com`
// URL onto the listener, so no request leaves the machine. Every key, body and credential here is
// synthetic; the one published value is AWS's own documented SigV4 example.
import { assert, assertEquals, assertRejects } from "@std/assert";
import { asResponse } from "./http.ts";
import * as s3mod from "./s3.ts";
import { s3Client, type S3Config, S3Error, type S3Fetch, sigV4Authorization } from "./s3.ts";

const REGION = "us-east-2";
const BUCKET = "mail-test-bucket";
const HOST = `${BUCKET}.s3.${REGION}.amazonaws.com`;
const ACCESS = "AKIDTESTONLY0000000";
const SECRET = "test-secret-not-a-real-key-0000000000000";
const CFG: S3Config = { bucket: BUCKET, region: REGION, accessKeyId: ACCESS, secretAccessKey: SECRET };
const NOW = () => new Date("2026-10-02T12:00:00.000Z");
const EMPTY_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

const enc = new TextEncoder();
const hex = (b: ArrayBuffer) => [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
const sha256 = async (b: Uint8Array) => hex(await crypto.subtle.digest("SHA-256", new Uint8Array(b)));
async function hmac(key: Uint8Array, msg: string): Promise<Uint8Array> {
  const k = await crypto.subtle.importKey(
    "raw",
    new Uint8Array(key),
    { name: "HMAC", hash: "SHA-256" },
    false,
    [
      "sign",
    ],
  );
  return new Uint8Array(await crypto.subtle.sign("HMAC", k, enc.encode(msg)));
}

/** The fake bucket's own check of an `Authorization` header, per AWS's SigV4 document. */
async function signatureHolds(req: Request, path: string, body: Uint8Array): Promise<boolean> {
  const auth = req.headers.get("authorization") ?? "";
  const m =
    /^AWS4-HMAC-SHA256 Credential=([^/]+)\/(\d{8})\/([^/]+)\/s3\/aws4_request, ?SignedHeaders=([a-z0-9;-]+), ?Signature=([0-9a-f]{64})$/
      .exec(auth);
  if (!m || m[1] !== ACCESS || m[3] !== REGION) return false;
  const amzDate = req.headers.get("x-amz-date") ?? "";
  const payloadHash = req.headers.get("x-amz-content-sha256") ?? "";
  if (!amzDate.startsWith(m[2]) || payloadHash !== await sha256(body)) return false;
  const names = m[4].split(";");
  for (const need of ["host", "x-amz-content-sha256", "x-amz-date"]) if (!names.includes(need)) return false;
  const value = (n: string) => n === "host" ? HOST : (req.headers.get(n) ?? "").trim();
  const canonical = [
    req.method,
    path,
    new URL(req.url).search.slice(1),
    names.map((n) => `${n}:${value(n)}\n`).join(""),
    m[4],
    payloadHash,
  ].join("\n");
  const scope = `${m[2]}/${REGION}/s3/aws4_request`;
  const toSign = `AWS4-HMAC-SHA256\n${amzDate}\n${scope}\n${await sha256(enc.encode(canonical))}`;
  let k = await hmac(enc.encode(`AWS4${SECRET}`), m[2]);
  for (const part of [REGION, "s3", "aws4_request"]) k = await hmac(k, part);
  return hex((await hmac(k, toSign)).buffer as ArrayBuffer) === m[5];
}

interface Fake {
  store: Map<string, Uint8Array<ArrayBuffer>>;
  counts: Record<string, number>;
  asked: string[];
  failPut?: { status: number; body: string };
}

/** The fake bucket: a map behind a listener on 127.0.0.1:0, and the fetcher that reaches it. */
async function withBucket(fn: (fetcher: S3Fetch, fake: Fake) => Promise<void>): Promise<void> {
  const fake: Fake = { store: new Map(), counts: {}, asked: [] };
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    const path = new URL(req.url).pathname;
    const body = new Uint8Array(await req.arrayBuffer());
    fake.counts[req.method] = (fake.counts[req.method] ?? 0) + 1;
    if (!await signatureHolds(req, path, body)) {
      return new Response("<Error>SignatureDoesNotMatch</Error>", { status: 403 });
    }
    if (req.method === "PUT") {
      if (fake.failPut) return new Response(fake.failPut.body, { status: fake.failPut.status });
      fake.store.set(path, body);
      return new Response(null, { status: 200 });
    }
    if (req.method === "GET") {
      const got = fake.store.get(path);
      return got ? new Response(got) : new Response(`<Error><Key>${path}</Key></Error>`, { status: 404 });
    }
    if (req.method === "DELETE") {
      fake.store.delete(path);
      return new Response(null, { status: 204 });
    }
    return new Response("no", { status: 405 });
  });
  const fetcher: S3Fetch = (raw, init) => {
    fake.asked.push(`${init?.method ?? "GET"} ${raw}`);
    const u = new URL(raw);
    return fetch(`http://127.0.0.1:${server.addr.port}${u.pathname}${u.search}`, init);
  };
  try {
    await fn(fetcher, fake);
  } finally {
    await server.shutdown();
  }
}

Deno.test("sigv4_get_put_and_delete_against_a_local_fake_bucket", async () => {
  await withBucket(async (fetcher, fake) => {
    const s3 = s3Client(CFG, { fetch: fetcher, now: NOW });
    const raw = enc.encode("From: a@example.test\r\nSubject: synthetic\r\n\r\nbody\r\n");
    await s3.put("work/item-1.eml", raw);
    assertEquals([...fake.store.keys()], ["/work/item-1.eml"]);
    assertEquals(await s3.get("work/item-1.eml"), raw);
    await s3.delete("work/item-1.eml");
    assertEquals(fake.store.size, 0);
    // A missing object is an error with its status, never an empty body.
    const err = await assertRejects(() => s3.get("work/item-1.eml"), S3Error);
    assertEquals([err.op, err.status], ["get", 404]);
    // An SES-shaped key (prefix + SES message id) round-trips too.
    await s3.put("inbound/o3ve6pjrsb6ilnq1ndvqgbcv2odp6tba0vcp5o01", raw);
    assertEquals(await s3.get("inbound/o3ve6pjrsb6ilnq1ndvqgbcv2odp6tba0vcp5o01"), raw);
    assertEquals(fake.counts, { PUT: 2, GET: 3, DELETE: 1 });
    // Every request went to the bucket's virtual host over https, and nowhere else.
    for (const line of fake.asked) assert(line.split(" ")[1].startsWith(`https://${HOST}/`), line);
  });
  // The fake really checks: a client holding another secret is refused by it.
  await withBucket(async (fetcher) => {
    const wrong = s3Client({ ...CFG, secretAccessKey: "another-test-secret-00000000000000000000" }, {
      fetch: fetcher,
      now: NOW,
    });
    const err = await assertRejects(() => wrong.put("work/item-2.eml", enc.encode("x")), S3Error);
    assertEquals([err.op, err.status], ["put", 403]);
  });
});

Deno.test("sigv4_matches_awss_published_get_object_example", async () => {
  // AWS's documented SigV4 example for S3 (GET Object with a Range header); the credentials are AWS's
  // published placeholders, not a key.
  const got = await sigV4Authorization({
    method: "GET",
    host: "examplebucket.s3.amazonaws.com",
    path: "/test.txt",
    query: "",
    headers: { range: "bytes=0-9", "x-amz-content-sha256": EMPTY_SHA256, "x-amz-date": "20130524T000000Z" },
    payloadHash: EMPTY_SHA256,
    amzDate: "20130524T000000Z",
    region: "us-east-1",
    accessKeyId: "AKIAIOSFODNN7EXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
  });
  assertEquals(
    got,
    "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request," +
      "SignedHeaders=host;range;x-amz-content-sha256;x-amz-date," +
      "Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41",
  );
});

Deno.test("a_failed_put_surfaces_as_an_error_the_handler_answers_500", async () => {
  const key = "work/marker-key-7f3a.eml";
  await withBucket(async (fetcher, fake) => {
    fake.failPut = {
      status: 503,
      body: `<Error><Key>${key}</Key><Message>MARKER-BODY-91c2</Message></Error>`,
    };
    const s3 = s3Client(CFG, { fetch: fetcher, now: NOW });
    const err = await assertRejects(() => s3.put(key, enc.encode("MARKER-TEXT-55e1")), S3Error);
    assertEquals([err.op, err.status], ["put", 503]);
    const logged: string[] = [];
    const real = console.error;
    console.error = (...a: unknown[]) => logged.push(a.map(String).join(" "));
    let res: Response;
    try {
      res = asResponse(err);
    } finally {
      console.error = real;
    }
    assertEquals(res.status, 500);
    assertEquals(await res.json(), { error: "internal error" });
    // Neither the error nor the line the handler logs names the key, the bucket, S3's answer, the
    // object or a credential.
    for (const text of [err.message, String(err.stack), ...logged]) {
      for (const secret of ["marker-key", "MARKER", BUCKET, SECRET, ACCESS]) {
        assert(!text.includes(secret), `leaked ${secret}`);
      }
    }
  });
  // A transport failure (whose own message names the URL, and so the key) is the same bare error.
  const s3 = s3Client(CFG, {
    fetch: (url) => Promise.reject(new TypeError(`error sending request for url (${url})`)),
    now: NOW,
  });
  const err = await assertRejects(() => s3.put(key, enc.encode("x")), S3Error);
  assertEquals([err.op, err.status], ["put", 0]);
  assert(!err.message.includes("marker-key") && !err.message.includes(BUCKET));
});

Deno.test("the_client_offers_no_list_operation", async () => {
  await withBucket(async (fetcher, fake) => {
    const s3 = s3Client(CFG, { fetch: fetcher, now: NOW });
    assertEquals(Object.keys(s3).sort(), ["delete", "get", "put"]);
    for (const name of Object.keys(s3mod)) assert(!/list/i.test(name), name);
    // No key reaches the bucket's root, a query string or outside `inbound/` and `work/`: a GET on
    // the root or with `?list-type=2` is S3's list call, which this client can never make.
    const hostile = [
      "",
      "/",
      "inbound/",
      "inbound",
      "work//x",
      "work/?list-type=2",
      "work/x?list-type=2",
      "work/x#y",
      "work/../inbound/x",
      "work/./x",
      "/work/x",
      "other/x",
      "work/x y",
      "work/%2e%2e",
      `work/${"a".repeat(600)}`,
    ];
    for (const key of hostile) {
      for (const call of [() => s3.get(key), () => s3.delete(key), () => s3.put(key, enc.encode("x"))]) {
        const err = await assertRejects(call, S3Error);
        assertEquals(err.status, 0);
      }
    }
    assertEquals(fake.asked, []);
  });
  // A bucket or region that is not a plain name is refused when the client is made.
  for (const bad of [{ bucket: "a.b.c" }, { bucket: "x/../y" }, { region: "us-east-2.evil.test" }]) {
    let threw = false;
    try {
      s3Client({ ...CFG, ...bad }, { fetch: () => Promise.reject(new Error("no")), now: NOW });
    } catch (e) {
      threw = true;
      assert(e instanceof Error && !e.message.includes("evil") && !e.message.includes("a.b.c"));
    }
    assert(threw, JSON.stringify(bad));
  }
});
