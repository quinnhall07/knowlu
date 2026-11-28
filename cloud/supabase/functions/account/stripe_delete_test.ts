import { assert, assertEquals } from "@std/assert";
import { STRIPE_API_VERSION } from "../_shared/stripe.ts";
import {
  cancelPostFrom,
  customerSubscriptionsFrom,
  redactStripeText,
  subscriptionStatusFrom,
} from "./stripe_delete.ts";

// Not a credential of any account: `sk_test_` plus words, never sent anywhere by these tests. Every
// `fetch` below is a fake that answers in-process; nothing leaves the machine.
const KEY = "sk_test_not_a_real_key";
const SUB = "sub_1InventedForTests";
const CUS = "cus_1InventedForTests";
const EMAIL = "someone@example.invalid";

type Seen = { url: string; method: string; headers: Headers; body: string | null };

/** A fake `fetch` that records each call and answers with `status` and `body` (a string is sent raw). */
function fakeFetch(status: number, body: unknown, seen: Seen[] = []): typeof fetch {
  return (input, init) => {
    seen.push({
      url: String(input),
      method: init?.method ?? "GET",
      headers: new Headers(init?.headers),
      body: init?.body === undefined || init?.body === null ? null : String(init.body),
    });
    const text = typeof body === "string" ? body : JSON.stringify(body);
    return Promise.resolve(new Response(text, { status, headers: { "content-type": "application/json" } }));
  };
}

const offline: typeof fetch = () =>
  Promise.reject(
    new TypeError(`error sending request for url (https://api.stripe.com/v1/subscriptions/${SUB})`),
  );

/** Stripe's own shape of a refusal, carrying the id and an email as Stripe's messages can. */
function stripeError(code: string, message: string, type = "invalid_request_error") {
  return {
    error: { code, type, param: "id", message, request_log_url: "https://dashboard.example.invalid/x" },
  };
}

const MISSING = stripeError("resource_missing", `No such subscription: '${SUB}'`);

/** Every `console` line written while `run` runs; the console is restored afterwards. */
async function captureLogs(run: () => Promise<unknown>): Promise<string[]> {
  const lines: string[] = [];
  const saved = { log: console.log, warn: console.warn, error: console.error, info: console.info };
  const keep = (...a: unknown[]) => lines.push(a.map(String).join(" "));
  console.log =
    console.warn =
    console.error =
    console.info =
      keep;
  try {
    await run();
  } finally {
    Object.assign(console, saved);
  }
  return lines;
}

/** Runs `run`, expecting it to throw the delete's 502; returns the response and the log lines. */
async function expect502(run: () => Promise<unknown>, label: string): Promise<string[]> {
  let thrown: unknown = null;
  const lines = await captureLogs(async () => {
    try {
      await run();
    } catch (e) {
      thrown = e;
    }
  });
  assert(thrown instanceof Response, `${label}: expected a thrown Response, got ${String(thrown)}`);
  assertEquals(thrown.status, 502, label);
  assertEquals(await thrown.json(), { error: "the payment provider refused the request" }, label);
  return lines;
}

function assertClean(lines: string[], label: string) {
  for (const line of lines) {
    assert(!line.includes(SUB), `${label}: a log line carries the subscription id: ${line}`);
    assert(!line.includes(CUS), `${label}: a log line carries the customer id: ${line}`);
    assert(!line.includes(EMAIL), `${label}: a log line carries an email: ${line}`);
    assert(!line.includes(KEY), `${label}: a log line carries the key: ${line}`);
  }
}

Deno.test("the read is one versioned GET of the subscription, and answers its status", async () => {
  const seen: Seen[] = [];
  const status = await subscriptionStatusFrom(KEY, fakeFetch(200, { id: SUB, status: "canceled" }, seen))(
    SUB,
  );
  assertEquals(status, "canceled");
  assertEquals(seen.length, 1);
  assertEquals(seen[0].method, "GET");
  assertEquals(seen[0].url, `https://api.stripe.com/v1/subscriptions/${SUB}`);
  assertEquals(seen[0].headers.get("stripe-version"), STRIPE_API_VERSION);
  assertEquals(seen[0].headers.get("authorization"), `Bearer ${KEY}`);
});

Deno.test("a subscription Stripe does not know (404 resource_missing) reads as null, and the body is not logged", async () => {
  let status: string | null | undefined;
  const lines = await captureLogs(async () => {
    status = await subscriptionStatusFrom(KEY, fakeFetch(404, MISSING))(SUB);
  });
  assertEquals(status, null);
  assertClean(lines, "missing");
});

Deno.test("a 404 that says the object exists in the other mode is a wrong key, never a missing subscription", async () => {
  // A test-mode key reading a live subscription (or the reverse) answers 404 resource_missing too.
  // Reading that as "nothing to cancel" would delete an account whose subscription still charges.
  for (const mode of ["live", "test"]) {
    const body = stripeError(
      "resource_missing",
      `No such subscription: '${SUB}'; a similar object exists in ${mode} mode, but a ${
        mode === "live" ? "test" : "live"
      } mode key was used to make this request.`,
    );
    const lines = await expect502(() => subscriptionStatusFrom(KEY, fakeFetch(404, body))(SUB), mode);
    assertClean(lines, mode);
  }
});

Deno.test("every other failure of the read is the delete's 502, logged without an id, an email or the key", async () => {
  const cases: Array<[string, typeof fetch]> = [
    ["a 404 that is not resource_missing", fakeFetch(404, stripeError("url_invalid", `Unrecognized ${SUB}`))],
    ["a 404 with no Stripe body", fakeFetch(404, "not json")],
    ["a 400", fakeFetch(400, stripeError("parameter_invalid", `${SUB} of ${CUS} for ${EMAIL}`))],
    [
      "a 401",
      fakeFetch(401, { error: { type: "invalid_request_error", message: `Invalid API Key: ${KEY}` } }),
    ],
    ["a 429", fakeFetch(429, stripeError("rate_limit", "Too many requests", "rate_limit_error"))],
    ["a 500", fakeFetch(500, stripeError("", `boom on ${SUB}`, "api_error"))],
    ["a 503 with an HTML body", fakeFetch(503, "<html>unavailable</html>")],
    ["a network error", offline],
    ["a 200 that is not JSON", fakeFetch(200, "<html>proxy</html>")],
    ["a 200 with no status", fakeFetch(200, { id: SUB })],
    ["a 200 with a status that is not a string", fakeFetch(200, { id: SUB, status: 7 })],
    // Fix round 1 (review finding 1): only a received 404 `resource_missing` is "unknown to Stripe".
    // A 2xx whose JSON is `null` came back from `stripeGetFrom` as the same `null`, and was purged past.
    ["a 200 whose body is null", fakeFetch(200, "null")],
    ["a 200 whose body is an array", fakeFetch(200, "[]")],
    ["a 200 whose body is a string", fakeFetch(200, '"canceled"')],
    ["a 200 whose body is a number", fakeFetch(200, "7")],
    ["a 200 whose body is true", fakeFetch(200, "true")],
    ["a 200 whose body is an array holding a status", fakeFetch(200, '[{"status":"canceled"}]')],
  ];
  for (const [label, f] of cases) {
    const lines = await expect502(() => subscriptionStatusFrom(KEY, f)(SUB), label);
    assert(lines.length >= 1, `${label}: the failure is logged`);
    assertClean(lines, label);
  }
});

Deno.test("a refused read's log line keeps what a reader needs: the status and Stripe's code", async () => {
  const lines = await expect502(
    () => subscriptionStatusFrom(KEY, fakeFetch(400, stripeError("parameter_invalid", `bad ${SUB}`)))(SUB),
    "400",
  );
  assert(lines.some((l) => l.includes("400") && l.includes("parameter_invalid")), JSON.stringify(lines));
});

Deno.test("the update is one versioned POST of cancel_at_period_end=true, and answers Stripe's body", async () => {
  const seen: Seen[] = [];
  const out = await cancelPostFrom(KEY, fakeFetch(200, { id: SUB, cancel_at_period_end: true }, seen))(
    `/v1/subscriptions/${SUB}`,
    { cancel_at_period_end: "true" },
  );
  assertEquals(out, { id: SUB, cancel_at_period_end: true });
  assertEquals(seen.length, 1);
  assertEquals(seen[0].method, "POST");
  assertEquals(seen[0].url, `https://api.stripe.com/v1/subscriptions/${SUB}`);
  assertEquals(seen[0].body, "cancel_at_period_end=true");
  assertEquals(seen[0].headers.get("stripe-version"), STRIPE_API_VERSION);
});

Deno.test("every failure of the update is the delete's 502, 404 included, logged without the id in its path", async () => {
  const cases: Array<[string, typeof fetch]> = [
    ["a 404 resource_missing", fakeFetch(404, MISSING)],
    ["a 400", fakeFetch(400, stripeError("", `A canceled subscription ${SUB} can only update metadata.`))],
    [
      "a 401",
      fakeFetch(401, { error: { type: "invalid_request_error", message: `Invalid API Key: ${KEY}` } }),
    ],
    ["a 500", fakeFetch(500, stripeError("", `boom for ${EMAIL}`, "api_error"))],
    ["a network error", offline],
    ["a 200 that is not JSON", fakeFetch(200, "<html>proxy</html>")],
  ];
  for (const [label, f] of cases) {
    const lines = await expect502(
      () => cancelPostFrom(KEY, f)(`/v1/subscriptions/${SUB}`, { cancel_at_period_end: "true" }),
      label,
    );
    assert(lines.length >= 1, `${label}: the failure is logged`);
    assertClean(lines, label);
  }
});

Deno.test("redactStripeText strips subscription and customer ids and emails, and keeps Stripe's codes", () => {
  const text = JSON.stringify(
    stripeError(
      "resource_missing",
      `No such subscription: '${SUB}' of ${CUS} (${EMAIL}); sub_sched_1Abc too`,
    ),
  );
  const out = redactStripeText(text);
  for (const gone of [SUB, CUS, EMAIL, "sub_sched_1Abc"]) assert(!out.includes(gone), `${gone} in ${out}`);
  assert(out.includes("resource_missing"), out);
  assert(out.includes("invalid_request_error"), out);
  assert(redactStripeText("x".repeat(5000)).length <= 600, "a long body is cut");
});

// T1d fix round 1 (review finding 2): the customer's subscriptions, when the stored id is ended,
// unknown or absent. The default list (ended ones left out), 100 at most, and nothing but the exact
// shape is trusted: a list we cannot read whole could hide the subscription that still charges.
const listOf = (data: unknown[], hasMore = false) => ({ object: "list", data, has_more: hasMore });
const SUB2 = "sub_2InventedForTests";

Deno.test("the list is one versioned GET of the customer's subscriptions, 100 at most, and answers each one", async () => {
  const seen: Seen[] = [];
  const out = await customerSubscriptionsFrom(
    KEY,
    fakeFetch(
      200,
      listOf([
        { id: SUB, object: "subscription", status: "active", cancel_at_period_end: false, customer: CUS },
        { id: SUB2, status: "trialing", cancel_at_period_end: true },
        { id: "sub_3InventedForTests", status: "past_due" },
      ]),
      seen,
    ),
  )(CUS);
  assertEquals(out, [
    { id: SUB, status: "active", cancel_at_period_end: false },
    { id: SUB2, status: "trialing", cancel_at_period_end: true },
    // An absent flag is read as not cancelling, so the subscription is set to cancel.
    { id: "sub_3InventedForTests", status: "past_due", cancel_at_period_end: false },
  ]);
  assertEquals(seen.length, 1);
  assertEquals(seen[0].method, "GET");
  assertEquals(seen[0].url, `https://api.stripe.com/v1/subscriptions?customer=${CUS}&limit=100`);
  assertEquals(seen[0].headers.get("stripe-version"), STRIPE_API_VERSION);
  assertEquals(seen[0].headers.get("authorization"), `Bearer ${KEY}`);
});

Deno.test("an empty list is an empty list", async () => {
  assertEquals(await customerSubscriptionsFrom(KEY, fakeFetch(200, listOf([])))(CUS), []);
});

Deno.test("a customer Stripe does not know (404 resource_missing) lists as null; the other mode's 404 is the 502", async () => {
  let out: unknown;
  // Fix round 2: the customer rule names `param: customer`, which is what Stripe sends for this filter.
  const missing = {
    error: {
      type: "invalid_request_error",
      code: "resource_missing",
      param: "customer",
      message: `No such customer: '${CUS}'`,
    },
  };
  const lines = await captureLogs(async () => {
    out = await customerSubscriptionsFrom(KEY, fakeFetch(404, missing))(CUS);
  });
  assertEquals(out, null);
  assertClean(lines, "missing customer");
  const other = stripeError(
    "resource_missing",
    `No such customer: '${CUS}'; a similar object exists in live mode, but a test mode key was used.`,
  );
  assertClean(
    await expect502(() => customerSubscriptionsFrom(KEY, fakeFetch(404, other))(CUS), "other"),
    "other",
  );
});

Deno.test("a list that is not the shape expected, or says it has more, is the 502", async () => {
  const cases: Array<[string, unknown]> = [
    ["null", "null"],
    ["an array", "[]"],
    ["a string", '"list"'],
    ["no data", { object: "list", has_more: false }],
    ["data that is not an array", { object: "list", data: {}, has_more: false }],
    ["has_more true", listOf([{ id: SUB, status: "active" }], true)],
    ["no has_more", { object: "list", data: [] }],
    ["has_more that is not a boolean", { object: "list", data: [], has_more: "false" }],
    ["an item that is null", listOf([null])],
    ["an item that is an array", listOf([[SUB, "active"]])],
    ["an item with no id", listOf([{ status: "active" }])],
    ["an item whose id is not a string", listOf([{ id: 7, status: "active" }])],
    ["an item whose id is not a subscription's", listOf([{ id: "..", status: "active" }])],
    ["an item whose id carries a path", listOf([{ id: `${SUB}/x`, status: "active" }])],
    ["an item with no status", listOf([{ id: SUB }])],
    ["an item whose status is not a string", listOf([{ id: SUB, status: null }])],
  ];
  for (const [label, body] of cases) {
    const lines = await expect502(() => customerSubscriptionsFrom(KEY, fakeFetch(200, body))(CUS), label);
    assertClean(lines, label);
  }
});

Deno.test("every failure of the list is the 502, logged without an id, an email or the key", async () => {
  const cases: Array<[string, typeof fetch]> = [
    ["a 400", fakeFetch(400, stripeError("parameter_invalid", `bad ${CUS} for ${EMAIL}`))],
    [
      "a 401",
      fakeFetch(401, { error: { type: "invalid_request_error", message: `Invalid API Key: ${KEY}` } }),
    ],
    ["a 404 that is not resource_missing", fakeFetch(404, stripeError("url_invalid", `Unrecognized ${CUS}`))],
    ["a 500", fakeFetch(500, stripeError("", `boom on ${CUS}`, "api_error"))],
    ["a network error", offline],
    ["a 200 that is not JSON", fakeFetch(200, "<html>proxy</html>")],
  ];
  for (const [label, f] of cases) {
    const lines = await expect502(() => customerSubscriptionsFrom(KEY, f)(CUS), label);
    assert(lines.length >= 1, `${label}: the failure is logged`);
    assertClean(lines, label);
  }
});

Deno.test("redactStripeText also strips the platform's account id, request ids and other object ids", () => {
  // Fix round 1, optional item 3: no Stripe object id of any kind survives into a log line, and
  // Stripe's codes (lower-case words joined by `_`) do.
  const text = JSON.stringify({
    error: {
      code: "card_declined",
      decline_code: "insufficient_funds",
      type: "card_error",
      message: "Invoice in_1AbCdEf23 on acct_1XyZ9abcDEF for pm_1QwErTy4 failed.",
      request_log_url: "https://dashboard.example.invalid/acct_1XyZ9abcDEF/logs/req_Ab12Cd34Ef56Gh?t=1",
    },
  });
  const out = redactStripeText(text);
  for (const gone of ["in_1AbCdEf23", "acct_1XyZ9abcDEF", "pm_1QwErTy4", "req_Ab12Cd34Ef56Gh"]) {
    assert(!out.includes(gone), `${gone} in ${out}`);
  }
  for (const kept of ["card_declined", "insufficient_funds", "card_error"]) {
    assert(out.includes(kept), `${kept} was redacted: ${out}`);
  }
  for (
    const code of [
      "resource_missing",
      "invalid_request_error",
      "authentication_required",
      "parameter_invalid",
    ]
  ) {
    assertEquals(redactStripeText(code), code);
  }
});

// T1d fix round 2, item 1 (measured on Stripe's test mode): a list filtered by a customer Stripe does
// not know answers HTTP 400 `resource_missing`, `param: customer`, "No such customer: '<id>'". For the
// LIST only, a 400 or a 404 of that kind means nothing to cancel; the read stays 404-only.
function customerMissing(status: number, error: Record<string, unknown>): typeof fetch {
  return fakeFetch(status, { error: { type: "invalid_request_error", ...error } });
}
const NO_SUCH_CUSTOMER = `No such customer: '${CUS}'`;

Deno.test("the list reads a 400 or 404 resource_missing on the customer as an unknown customer, and logs nothing", async () => {
  const cases: Array<[string, typeof fetch]> = [
    [
      "400 param customer",
      customerMissing(400, { code: "resource_missing", param: "customer", message: NO_SUCH_CUSTOMER }),
    ],
    [
      "404 param customer",
      customerMissing(404, { code: "resource_missing", param: "customer", message: NO_SUCH_CUSTOMER }),
    ],
    [
      "400 no param, the customer message",
      customerMissing(400, { code: "resource_missing", message: NO_SUCH_CUSTOMER }),
    ],
    [
      "400 param null, the customer message",
      customerMissing(400, { code: "resource_missing", param: null, message: NO_SUCH_CUSTOMER }),
    ],
  ];
  for (const [label, f] of cases) {
    let out: unknown = "unset";
    const lines = await captureLogs(async () => {
      out = await customerSubscriptionsFrom(KEY, f)(CUS);
    });
    assertEquals(out, null, label);
    assertEquals(lines, [], `${label}: the missing answer's body is never logged`);
  }
});

Deno.test("any other 400 on the list is the 502: another param, another code, another message, or the other mode", async () => {
  const otherMode =
    `${NO_SUCH_CUSTOMER}; a similar object exists in live mode, but a test mode key was used.`;
  const cases: Array<[string, typeof fetch]> = [
    [
      "param price",
      customerMissing(400, { code: "resource_missing", param: "price", message: "No such price" }),
    ],
    ["param id", customerMissing(400, { code: "resource_missing", param: "id", message: NO_SUCH_CUSTOMER })],
    [
      "no param, another message",
      customerMissing(400, { code: "resource_missing", message: `No such subscription: '${SUB}'` }),
    ],
    [
      "no param, the message not at the start",
      customerMissing(400, { code: "resource_missing", message: `Oops. ${NO_SUCH_CUSTOMER}` }),
    ],
    ["no param, no message", customerMissing(400, { code: "resource_missing" })],
    [
      "param customer, another code",
      customerMissing(400, { code: "parameter_invalid", param: "customer", message: NO_SUCH_CUSTOMER }),
    ],
    ["param customer, no code", customerMissing(400, { param: "customer", message: NO_SUCH_CUSTOMER })],
    [
      "param customer, the other mode",
      customerMissing(400, { code: "resource_missing", param: "customer", message: otherMode }),
    ],
    ["404 no param, the other mode", customerMissing(404, { code: "resource_missing", message: otherMode })],
    [
      "a 409 of the customer kind",
      customerMissing(409, { code: "resource_missing", param: "customer", message: NO_SUCH_CUSTOMER }),
    ],
  ];
  for (const [label, f] of cases) {
    const lines = await expect502(() => customerSubscriptionsFrom(KEY, f)(CUS), label);
    assertClean(lines, label);
  }
});

Deno.test("the single-subscription read stays 404-only: a 400 resource_missing is the 502", async () => {
  for (const param of ["id", "customer", undefined]) {
    const error = { code: "resource_missing", param, message: `No such subscription: '${SUB}'` };
    const lines = await expect502(
      () => subscriptionStatusFrom(KEY, customerMissing(400, error))(SUB),
      String(param),
    );
    assertClean(lines, String(param));
  }
  const customer = { code: "resource_missing", param: "customer", message: NO_SUCH_CUSTOMER };
  assertClean(
    await expect502(() => subscriptionStatusFrom(KEY, customerMissing(400, customer))(SUB), "400"),
    "400",
  );
});

Deno.test("an update's 400 resource_missing on the customer is still the 502", async () => {
  const error = { code: "resource_missing", param: "customer", message: NO_SUCH_CUSTOMER };
  const lines = await expect502(
    () =>
      cancelPostFrom(KEY, customerMissing(400, error))(`/v1/subscriptions/${SUB}`, {
        cancel_at_period_end: "true",
      }),
    "update",
  );
  assertClean(lines, "update");
});

Deno.test("redactStripeText removes an id that carries _secret_ whole", () => {
  // Fix round 2, item 3: a client secret has no word boundary inside it, so the general id rule alone
  // would leave it standing.
  // The samples are invented and built from parts, so a secret scanner does not read them as real.
  const mid = ["", "secret", ""].join("_");
  for (const sample of [`pi_3AbCdEf12${mid}dEfGh34`, `seti_1XyZ${mid}Q9w`, `cs_test_a1B2c3${mid}zZ9`]) {
    const out = redactStripeText(`Bad client secret ${sample} on the request.`);
    assert(!out.includes("_secret_"), `${sample}: ${out}`);
    for (const part of sample.split("_")) {
      if (part.length >= 3 && /[0-9A-Z]/.test(part)) {
        assert(!out.includes(part), `${part} of ${sample}: ${out}`);
      }
    }
    assert(out.startsWith("Bad client secret <") && out.endsWith("> on the request."), out);
  }
});
