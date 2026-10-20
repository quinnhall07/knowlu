// The lapsed-mailbox fix (2026-10-19): intake's own `mail_addresses` writes, driven over an injected
// `fetch` that answers itself, so the PATCH filters are the ones the wire sends and nothing leaves the
// process. Every id here is invented.
import { assert, assertEquals } from "@std/assert";
import { storeOver } from "./store.ts";

const ADDR = "11111111-1111-4111-8111-111111111111";
const CANARY = "22222222-2222-4222-8222-222222222222";
const NOW = Date.UTC(2026, 9, 19, 12, 0, 0);
const BASE = "http://127.0.0.1:9/rest/v1/";

interface Seen {
  method: string;
  path: string;
  body?: unknown;
  prefer: string | null;
}

/** A store whose every request is recorded and answered by `answer` (a PATCH changes no row unless it
 * says otherwise). */
async function recording(answer: (path: string) => unknown = () => []) {
  const seen: Seen[] = [];
  const fetch = (input: string | URL | Request, init?: RequestInit) => {
    const path = String(input).slice(BASE.length);
    const prefer = new Headers(init?.headers).get("prefer");
    seen.push({
      method: init?.method ?? "GET",
      path,
      prefer,
      ...(init?.body === undefined ? {} : { body: JSON.parse(String(init.body)) }),
    });
    return Promise.resolve(
      prefer === "return=minimal"
        ? new Response(null, { status: 204 })
        : new Response(JSON.stringify(answer(path)), { status: 200 }),
    );
  };
  const encKey = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["decrypt"]);
  const store = storeOver(
    { url: "http://127.0.0.1:9", serviceKey: "test-not-a-key", fetch: fetch as typeof globalThis.fetch },
    encKey,
    () => NOW,
  );
  return { seen, store };
}

const live = `mail_addresses?id=eq.${ADDR}&status=not.in.(lapsed,revoked,off)`;

Deno.test("mark_lapsed_keeps_an_earlier_stamp_and_sets_status_in_the_same_write", async () => {
  // A mailbox unlapsed whose gap the device has not yet covered still holds its stamp: it lapses again
  // with that stamp kept, in one compare-and-set, and nothing else is written. This lapse is recorded
  // apart, in `last_lapsed_at`, so a `covered` in flight for the earlier gap no longer matches (review of
  // the lapsed fix, #1).
  const { seen, store } = await recording((p) =>
    p.startsWith(`${live}&lapsed_since=not.is.null`) ? [{ id: ADDR }] : []
  );
  await store.markLapsed(ADDR);
  assertEquals(seen, [{
    method: "PATCH",
    path: `${live}&lapsed_since=not.is.null&select=id`,
    prefer: "return=representation",
    body: { status: "lapsed", last_lapsed_at: new Date(NOW).toISOString() },
  }]);
});

Deno.test("mark_lapsed_stamps_a_first_lapse_with_its_status_in_one_write", async () => {
  // No stamp: the second compare-and-set sets both together, so a row never reads `lapsed` unstamped.
  const { seen, store } = await recording((p) =>
    p.startsWith(`${live}&lapsed_since=is.null`) ? [{ id: ADDR }] : []
  );
  await store.markLapsed(ADDR);
  assertEquals(seen.map((s) => [s.path, s.body]), [
    [`${live}&lapsed_since=not.is.null&select=id`, {
      status: "lapsed",
      last_lapsed_at: new Date(NOW).toISOString(),
    }],
    [`${live}&lapsed_since=is.null&select=id`, {
      status: "lapsed",
      lapsed_since: new Date(NOW).toISOString(),
      last_lapsed_at: new Date(NOW).toISOString(),
    }],
  ]);
});

Deno.test("mark_lapsed_changes_nothing_on_a_lapsed_revoked_or_off_mailbox", async () => {
  // Both writes are filtered on `status=not.in.(lapsed,revoked,off)`: an already lapsed mailbox keeps its
  // stamp, a revoked one is never brought back, and an `off` one keeps the student's choice (review of
  // the lapsed fix, #3): its mail is deleted unread while the account is unentitled anyway, and a pull
  // after renewal would otherwise read it `quiet`.
  const { seen, store } = await recording();
  await store.markLapsed(ADDR);
  assertEquals(seen.length, 2);
  for (const s of seen) {
    assertEquals(s.method, "PATCH");
    assertEquals(s.path.startsWith(`${live}&`), true, s.path);
  }
});

Deno.test("a_canary_through_the_current_token_lifts_a_lapsed_mailbox_to_active", async () => {
  // Intake reaches a canary only for an entitled account, so a lapsed mailbox's canary proves the forward.
  const at = new Date(NOW).toISOString();
  const { seen, store } = await recording((p) => p.startsWith("mail_canaries?") ? [{ id: CANARY }] : []);
  assertEquals(await store.canaryArrived(CANARY, ADDR, null, at, true), true);
  const active = seen.find((s) => (s.body as { status?: string } | undefined)?.status === "active");
  assertEquals(
    active?.path,
    `mail_addresses?id=eq.${ADDR}&status=in.(pending,confirmed,quiet,blocked,off,lapsed)`,
  );
  // `lapsed_since` is not intake's to clear: the device reports the gap covered.
  assertEquals(
    seen.some((s) => s.body !== undefined && Object.hasOwn(s.body as object, "lapsed_since")),
    false,
  );
});

Deno.test("ending_the_grace_by_canary_clears_rotated_from_with_the_retired_pair", async () => {
  // Email follow-ups T4 (Q4, 2026-10-20): the canary through the new token ends the Rotate grace, and the
  // status before the Rotate goes with it, in the same PATCH. Tidiness only: D22 ignores `rotated_from`
  // once `retired_until` is gone.
  const { seen, store } = await recording();
  await store.endRotateGrace(ADDR);
  assertEquals(seen, [{
    method: "PATCH",
    path: `mail_addresses?id=eq.${ADDR}&retired_token_hmac=not.is.null`,
    prefer: "return=minimal",
    body: { retired_token_hmac: null, retired_until: null, rotated_from: null },
  }]);
});

Deno.test("batch_received_is_one_rpc_carrying_a_64_hex_receipt_and_never_the_object_key", async () => {
  // Email follow-ups T5: one count per SES receipt. The store makes one `count_batch_receipt` call, with
  // nothing read before it, and answers what the RPC answered: false for a receipt already counted or a
  // batch gone. The receipt is the hash intake computed; no object key reaches the wire.
  const BATCH = "33333333-3333-4333-8333-333333333333";
  const RECEIPT = "0123456789abcdef".repeat(4);
  for (const answer of [true, false]) {
    // The RPC answers `answer`; a table read (none is expected) would find the batch at zero.
    const { seen, store } = await recording((p) =>
      p.startsWith("rpc/") ? answer : [{ id: BATCH, received: 0 }]
    );
    assertEquals(await store.batchReceived(BATCH, ADDR, 3, RECEIPT), answer);
    assertEquals(seen, [{
      method: "POST",
      path: "rpc/count_batch_receipt",
      prefer: null,
      body: { p_batch: BATCH, p_address: ADDR, p_receipt: RECEIPT, p_n: 3 },
    }]);
    assert(/^[0-9a-f]{64}$/.test(RECEIPT));
    assert(!JSON.stringify(seen).includes("inbound/"), "an object key reached the wire");
  }
});
