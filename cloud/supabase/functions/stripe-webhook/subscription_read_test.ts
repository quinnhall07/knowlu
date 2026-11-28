import { assert, assertEquals, assertRejects } from "@std/assert";
import { STRIPE_API_VERSION } from "../_shared/stripe.ts";
import { readFailure, subscriptionReader, SubscriptionReadError } from "./subscription_read.ts";

const KEY = "test-only-not-a-key";

/** A `fetch` that answers `answer` and keeps every request it was given. No socket is opened. */
function fakeFetch(answer: () => Response | Promise<Response>) {
  const asked: { url: string; headers: Headers; method: string }[] = [];
  const impl: typeof fetch = (input, init) => {
    asked.push({
      url: String(input),
      headers: new Headers(init?.headers),
      method: init?.method ?? "GET",
    });
    return Promise.resolve(answer());
  };
  return { impl, asked };
}

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

Deno.test("a_2xx_read_is_the_subscription_at_the_pinned_version_with_the_id_path_encoded", async () => {
  const { impl, asked } = fakeFetch(() =>
    json(200, { id: "sub_test_1", object: "subscription", status: "trialing" })
  );
  const sub = await subscriptionReader(KEY, impl)("sub_test_1/../x?y");
  assertEquals(sub, { id: "sub_test_1", object: "subscription", status: "trialing" });
  assertEquals(asked.length, 1);
  assertEquals(asked[0].method, "GET");
  assertEquals(asked[0].url, "https://api.stripe.com/v1/subscriptions/sub_test_1%2F..%2Fx%3Fy");
  assertEquals(asked[0].headers.get("stripe-version"), STRIPE_API_VERSION);
  assertEquals(asked[0].headers.get("authorization"), `Bearer ${KEY}`);
});

Deno.test("only_stripes_404_means_no_such_subscription", async () => {
  const { impl } = fakeFetch(() =>
    json(404, { error: { code: "resource_missing", message: "No such subscription: 'sub_test_1'" } })
  );
  assertEquals(await subscriptionReader(KEY, impl)("sub_test_1"), null);
});

Deno.test("any_other_answer_is_a_failed_read_named_by_its_status", async () => {
  for (const status of [400, 401, 403, 409, 429, 500, 502, 503]) {
    const { impl } = fakeFetch(() =>
      json(status, { error: { message: "for a@example.invalid on sub_test_1" } })
    );
    const e = await assertRejects(() => subscriptionReader(KEY, impl)("sub_test_1"), SubscriptionReadError);
    assertEquals(e.what, status);
    assertEquals(readFailure(e), String(status));
    for (const never of ["@", "sub_", "a@example"]) assert(!e.message.includes(never), e.message);
  }
});

Deno.test("a_network_error_is_a_failed_read_that_names_no_url", async () => {
  const impl: typeof fetch = () =>
    Promise.reject(
      new TypeError("error sending request for url (https://api.stripe.com/v1/subscriptions/sub_test_1)"),
    );
  const e = await assertRejects(() => subscriptionReader(KEY, impl)("sub_test_1"), SubscriptionReadError);
  assertEquals(e.what, "network");
  assertEquals(readFailure(e), "network");
  assert(!e.message.includes("sub_") && !e.message.includes("https"), e.message);
});

Deno.test("a_2xx_that_is_not_a_json_object_is_a_failed_read", async () => {
  for (
    const body of [
      new Response("<html>not json</html>", { status: 200 }),
      json(200, null),
      json(200, ["sub_test_1"]),
      json(200, "sub_test_1"),
    ]
  ) {
    const { impl } = fakeFetch(() => body);
    const e = await assertRejects(() => subscriptionReader(KEY, impl)("sub_test_1"), SubscriptionReadError);
    assertEquals(e.what, 200);
  }
});

Deno.test("read_failure_names_a_status_or_network_and_nothing_else", () => {
  assertEquals(readFailure(new SubscriptionReadError(429)), "429");
  assertEquals(readFailure(new SubscriptionReadError("network")), "network");
  // Any other throw carries a message that can name a URL or an address: it is never repeated.
  assertEquals(
    readFailure(new Error("mail a@example.invalid at https://api.stripe.com/v1/subscriptions/sub_1")),
    "network",
  );
  assertEquals(readFailure("a string"), "network");
});

Deno.test("a_2xx_empty_object_is_a_failed_read", async () => {
  const { impl } = fakeFetch(() => json(200, {}));
  const e = await assertRejects(() => subscriptionReader(KEY, impl)("sub_test_1"), SubscriptionReadError);
  assertEquals(e.what, 200);
});

Deno.test("a_2xx_object_of_another_kind_is_a_failed_read", async () => {
  for (const object of ["invoice", "checkout.session", "list", "Subscription"]) {
    const { impl } = fakeFetch(() => json(200, { id: "sub_test_1", object, status: "active" }));
    const e = await assertRejects(() => subscriptionReader(KEY, impl)("sub_test_1"), SubscriptionReadError);
    assertEquals(e.what, 200, object);
  }
});
