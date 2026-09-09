import { assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const verify = (t: string) =>
  Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null);
const post = () =>
  new Request("http://127.0.0.1:1/billing-portal", {
    method: "POST",
    headers: { authorization: "Bearer good" },
  });
const never: (path: string, form: Record<string, string>) => Promise<Record<string, unknown>> = () =>
  Promise.reject(new Error("Stripe must not be called"));

Deno.test("a GET is 405 and says so in the header", async () => {
  const res = await handle(new Request("http://127.0.0.1:1/billing-portal"), {
    verify,
    getCustomerId: () => Promise.resolve("cus_1"),
    stripe: never,
    returnUrl: "https://knowlu.com/index.html",
  });
  assertEquals(res.status, 405);
  assertEquals(res.headers.get("allow"), "POST");
});

Deno.test("an account that never subscribed is 409, and Stripe is not called", async () => {
  const res = await handle(post(), {
    verify,
    getCustomerId: () => Promise.resolve(null),
    stripe: never,
    returnUrl: "https://knowlu.com/index.html",
  }).catch((e) => e as Response);
  assertEquals(res.status, 409);
  assertEquals(await res.json(), { error: "this account has never subscribed" });
});

Deno.test("a subscriber gets the portal link, and the return URL goes with it", async () => {
  // A list, not a `let x = null` the compiler narrows to `null`: the assignment happens inside a
  // callback, which TypeScript's flow analysis does not follow.
  const sent: Record<string, string>[] = [];
  const res = await handle(post(), {
    verify,
    getCustomerId: () => Promise.resolve("cus_1"),
    stripe: (path, form) => {
      assertEquals(path, "/v1/billing_portal/sessions");
      sent.push(form);
      return Promise.resolve({ url: "https://billing.stripe.com/p/session_1" });
    },
    returnUrl: "https://knowlu.com/index.html",
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { url: "https://billing.stripe.com/p/session_1" });
  // The return URL is the promise the ARL rests on: cancelling lands the student back in Knowlu.
  assertEquals(sent, [{ customer: "cus_1", return_url: "https://knowlu.com/index.html" }]);
});

Deno.test("a portal session with no url is 502, not a 200 carrying undefined", async () => {
  const res = await handle(post(), {
    verify,
    getCustomerId: () => Promise.resolve("cus_1"),
    // Stripe answered, but not with a link. Returning `{url: undefined}` would put the app's
    // *Cancel subscription* button through `open_in_browser(undefined)` and fail with nothing to say.
    stripe: () => Promise.resolve({ id: "bps_1" }),
    returnUrl: "https://knowlu.com/index.html",
  }).catch((e) => e as Response);
  assertEquals(res.status, 502);
  assertEquals(await res.json(), { error: "the payment provider returned no portal link" });
});
