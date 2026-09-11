import { assertEquals } from "@std/assert";
import { asResponse, fail, json, methodNotAllowed, readJson, subPath } from "./http.ts";

Deno.test("json() sets the status, the body and one content type", async () => {
  const r = json(200, { a: 1 });
  assertEquals(r.status, 200);
  assertEquals(r.headers.get("content-type"), "application/json; charset=utf-8");
  assertEquals(await r.json(), { a: 1 });
});

Deno.test("fail() is a body with an error key, never a bare string", async () => {
  const r = fail(402, "this account has no active subscription");
  assertEquals(r.status, 402);
  assertEquals(await r.json(), { error: "this account has no active subscription" });
});

Deno.test("methodNotAllowed() names what is allowed, in the body and in the header", async () => {
  const r = methodNotAllowed(["GET", "PUT"]);
  assertEquals(r.status, 405);
  assertEquals(r.headers.get("allow"), "GET, PUT");
  assertEquals(await r.json(), { error: "method not allowed; use GET, PUT" });
});

Deno.test("subPath() strips the /functions/v1/<name> prefix so one function can route", () => {
  assertEquals(subPath("https://x.supabase.co/functions/v1/account", "account"), "/");
  assertEquals(subPath("https://x.supabase.co/functions/v1/account/export", "account"), "/export");
  assertEquals(subPath("https://x.supabase.co/functions/v1/account/sources?a=1", "account"), "/sources");
  // A request that does not carry the prefix at all (a direct invoke in a test) still routes.
  assertEquals(subPath("http://127.0.0.1:9999/sources", "account"), "/sources");
});

Deno.test("readJson() refuses a body over the limit rather than parsing it", async () => {
  const big = new Request("http://127.0.0.1:1/", {
    method: "POST",
    body: JSON.stringify({ s: "x".repeat(50) }),
  });
  try {
    await readJson(big, 16);
    throw new Error("readJson accepted a body over the limit");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 413);
  }
});

Deno.test("readJson() refuses a body that is not JSON with 400, not 500", async () => {
  const bad = new Request("http://127.0.0.1:1/", { method: "POST", body: "{oh no" });
  try {
    await readJson(bad);
    throw new Error("readJson accepted a body that is not JSON");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 400);
  }
});

Deno.test("asResponse() passes a thrown Response through and hides anything else", async () => {
  const thrown = fail(401, "no bearer token");
  assertEquals(asResponse(thrown), thrown);
  const hidden = asResponse(new Error("connection string: postgres://user:pw@host/db"));
  assertEquals(hidden.status, 500);
  assertEquals(await hidden.json(), { error: "internal error" });
});
