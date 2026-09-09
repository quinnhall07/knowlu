import { assertEquals } from "@std/assert";
import { parseBearer, requireUser } from "./auth.ts";

const req = (headers: Record<string, string>) => new Request("http://127.0.0.1:1/", { headers });

Deno.test("parseBearer takes the token and nothing around it", () => {
  assertEquals(parseBearer(req({ authorization: "Bearer abc.def.ghi" })), "abc.def.ghi");
  assertEquals(parseBearer(req({ Authorization: "bearer abc" })), "abc");
  assertEquals(parseBearer(req({ authorization: "  Bearer   abc  " })), "abc");
  assertEquals(parseBearer(req({ authorization: "Basic abc" })), null);
  assertEquals(parseBearer(req({ authorization: "Bearer" })), null);
  assertEquals(parseBearer(req({})), null);
});

Deno.test("requireUser throws a 401 Response when there is no token", async () => {
  try {
    await requireUser(req({}), () => Promise.resolve({ id: "u", email: null }));
    throw new Error("requireUser accepted a request with no bearer token");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 401);
    assertEquals(await e.json(), { error: "no bearer token" });
  }
});

Deno.test("requireUser throws a 401 Response when the token does not verify", async () => {
  try {
    await requireUser(req({ authorization: "Bearer stale" }), () => Promise.resolve(null));
    throw new Error("requireUser accepted an unverifiable token");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 401);
  }
});

Deno.test("requireUser hands back exactly what the verifier said", async () => {
  const u = await requireUser(
    req({ authorization: "Bearer good" }),
    (t) => Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null),
  );
  assertEquals(u, { id: "acc-1", email: "a@example.invalid" });
});
