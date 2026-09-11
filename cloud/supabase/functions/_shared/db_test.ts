import { assertEquals } from "@std/assert";
import { Rest, restSelectAll } from "./db.ts";

/** `config.toml` sets `max_rows = 1000`: PostgREST silently caps any single `select` there, so the
 * one paging helper in this codebase is the one place that matters. */
Deno.test("restSelectAll pages past PostgREST's row cap until a short page ends it", async () => {
  const requests: { url: string; offset: string | null }[] = [];
  const pageSizes = [1000, 1000, 3];
  let call = 0;

  const rest: Rest = {
    url: "https://example.invalid",
    serviceKey: "test-service-key",
    fetch: (input: string | URL | Request) => {
      const url = new URL(String(input));
      requests.push({ url: url.pathname, offset: url.searchParams.get("offset") });
      const n = pageSizes[call];
      call++;
      const rows = Array.from({ length: n }, (_, i) => ({ id: `row-${requests.length}-${i}` }));
      return Promise.resolve(new Response(JSON.stringify(rows), { status: 200 }));
    },
  };

  const rows = await restSelectAll<{ id: string }>(rest, "billing_subscribers", "select=*&order=account_id");

  assertEquals(requests.map((r) => r.offset), ["0", "1000", "2000"]);
  assertEquals(rows.length, 2003);
});

Deno.test("a single short page makes exactly one request", async () => {
  const offsets: (string | null)[] = [];
  const rest: Rest = {
    url: "https://example.invalid",
    serviceKey: "test-service-key",
    fetch: (input: string | URL | Request) => {
      const url = new URL(String(input));
      offsets.push(url.searchParams.get("offset"));
      return Promise.resolve(new Response(JSON.stringify([{ id: "only" }]), { status: 200 }));
    },
  };

  const rows = await restSelectAll<{ id: string }>(rest, "billing_subscribers", "select=*&order=account_id");

  assertEquals(offsets, ["0"]);
  assertEquals(rows, [{ id: "only" }]);
});
