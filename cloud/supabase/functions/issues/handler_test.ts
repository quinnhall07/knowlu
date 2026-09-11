import { assert, assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const post = (body: unknown, auth = "Bearer good") =>
  new Request("http://127.0.0.1:1/issues", {
    method: "POST",
    headers: { authorization: auth },
    body: JSON.stringify(body),
  });

const base = {
  verify: (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null),
  save: () => Promise.resolve("11111111-1111-1111-1111-111111111111"),
};

Deno.test("a report is stored and its id comes back", async () => {
  const res = await handle(
    post({ body: "the run list says amber and I do not know why", payload: { steps: 4 } }),
    base,
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { id: "11111111-1111-1111-1111-111111111111" });
});

Deno.test("whatever the client sent, the stored row is scrubbed again", async () => {
  let stored: Record<string, unknown> | null = null;
  await handle(
    post({
      body: "it broke while fetching https://lms.example.invalid/feed/abc.ics for a.b@x.invalid",
      payload: {
        last_error: "could not open read-chapter-3.md",
        token: "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef",
      },
      app_version: "0.1.0",
    }),
    {
      ...base,
      save: (row) => {
        stored = row as Record<string, unknown>;
        return Promise.resolve("id-1");
      },
    },
  );
  assert(stored);
  // `as`, not `stored!`: TS's control-flow analysis does not see the reassignment inside the `save`
  // closure above, so a non-null assertion here type-checks `stored` as `never` even though `assert`
  // just proved it is not null at runtime.
  const row = stored as Record<string, unknown>;
  assertEquals(row.body, "it broke while fetching <url> for <email>");
  assertEquals((row.payload as Record<string, unknown>).last_error, "could not open <note>");
  assertEquals((row.payload as Record<string, unknown>).token, "<token>");
  assertEquals(row.account_id, "acc-1");
});

Deno.test("an empty body is 400, and an oversized one too", async () => {
  assertEquals(
    (await handle(post({ body: "  ", payload: {} }), base).catch((e) => e as Response)).status,
    400,
  );
  assertEquals(
    (await handle(post({ body: "x".repeat(8193), payload: {} }), base).catch((e) => e as Response)).status,
    400,
  );
});

Deno.test("no bearer token is 401 — a report is never anonymous", async () => {
  const res = await handle(post({ body: "hello", payload: {} }, "Basic nope"), base).catch((e) =>
    e as Response
  );
  assertEquals(res.status, 401);
});

// Fix round 1, item 5: the four metadata columns are token-shaped or refused. Untyped, uncapped
// text in these columns is exactly the kind of stray free text the scrub function exists to keep
// out of the store in the first place — better to refuse a malformed one than to store it.
Deno.test("a metadata field that is not token-shaped is refused, naming the field", async () => {
  const res = await handle(
    post({ body: "hello", payload: {}, os_build: "x".repeat(200) }),
    base,
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  assertEquals(((await res.json()) as { error: string }).error.includes("os_build"), true);
});

// Fix round 1, item 6: `payload` must be a plain object when present — not an array, not a scalar.
Deno.test("a payload that is not an object is refused", async () => {
  const res = await handle(post({ body: "hello", payload: "text" }), base).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

// Fix round 1, item 7: the pins.
Deno.test("GET is 405 with an exact allow header", async () => {
  const res = await handle(
    new Request("http://127.0.0.1:1/issues", { method: "GET" }),
    base,
  );
  assertEquals(res.status, 405);
  assertEquals(res.headers.get("allow"), "POST");
});

Deno.test("a token that does not verify is 401 — the session is not valid", async () => {
  const res = await handle(post({ body: "hello", payload: {} }, "Bearer nope"), base).catch((e) =>
    e as Response
  );
  assertEquals(res.status, 401);
  assertEquals(((await res.json()) as { error: string }).error, "the session is not valid");
});

Deno.test("a client-sent account_id is ignored — the stored row carries the verified user's", async () => {
  let stored: Record<string, unknown> | null = null;
  await handle(
    post({ body: "hello", payload: {}, account_id: "acc-evil" }),
    {
      ...base,
      save: (row) => {
        stored = row as Record<string, unknown>;
        return Promise.resolve("id-1");
      },
    },
  );
  assert(stored);
  const row = stored as Record<string, unknown>;
  assertEquals(row.account_id, "acc-1");
});

Deno.test("valid metadata values pass through and land on the stored row untouched", async () => {
  let stored: Record<string, unknown> | null = null;
  await handle(
    post({
      body: "hello",
      payload: {},
      app_version: "0.1.0",
      engine_build: "6a257c87ba6f08462860101b579145ce21124a2b",
      os_build: "10.0.26200",
      profile_id: "profile_a95daa1d40",
    }),
    {
      ...base,
      save: (row) => {
        stored = row as Record<string, unknown>;
        return Promise.resolve("id-1");
      },
    },
  );
  assert(stored);
  const row = stored as Record<string, unknown>;
  assertEquals(row.app_version, "0.1.0");
  assertEquals(row.engine_build, "6a257c87ba6f08462860101b579145ce21124a2b");
  assertEquals(row.os_build, "10.0.26200");
  assertEquals(row.profile_id, "profile_a95daa1d40");
});
