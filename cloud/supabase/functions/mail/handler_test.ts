// Email plan T4.1 (spec §4.1, §4.12 step 2, §6.3; D3, D17, P21; tests 11's address half and 16b). The
// handler runs whole over an in-memory store, a fake bucket that records its deletes (no listener: the
// route only deletes by key), an injected MX resolver and the real gates over a fake session and a fake
// `entitlements` row. Every token, address, domain and key here is invented for these tests.
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import { requireUser } from "../_shared/auth.ts";
import { decryptString, encryptString, importAesKey } from "../_shared/crypto.ts";
import { type EntitlementStatus, requireActiveEntitlementWith } from "../_shared/entitlement.ts";
import { CANARY_FROM, CANARY_HEADER, CANARY_SUBJECT, canarySender } from "../_shared/canary_send.ts";
import { nonceTag } from "../_shared/mail_bind.ts";
import { MAIL_CANARY_FROM } from "../mail-inbound/handler.ts";
import { hmacKey, nonceHmac, tokenHmac } from "../mail-inbound/keys.ts";
import {
  type BatchRow,
  type HeldRow,
  LIVE_CAP,
  type MailDeps,
  mailHandler,
  type MailRow,
  type MailStore,
  MINT_CAP,
  type NewBatch,
  type NewToken,
  ROTATE_GRACE_MS,
  type StatePatch,
} from "./handler.ts";
import { hintFits, type MailProvider, type ResolveMx } from "./provider.ts";
import {
  BACKFILL_TOO_WIDE,
  CANARY_CAP,
  CONFIRM_WINDOW_MS,
  MAX_OPEN_BATCHES,
  NO_HINT,
  OPEN_BATCH_CAP,
  RECONCILE_BEFORE_WINDOW,
  RECOVER_BEFORE_WINDOW,
  RECOVER_TOO_WIDE,
} from "./routes.ts";

const DOMAIN = "in.knowlu.test";
const NOW = Date.UTC(2026, 9, 2, 12, 0, 0);
const DAY = 86_400_000;
const enc = new TextEncoder();
const ADDRESS_KEY = await hmacKey(enc.encode("test-address-key-0000000000000000"));
const ENC_KEY = await importAesKey(btoa("test-encryption-key-000000000000"));
const TOKEN = /^[a-z2-7]{26}$/;
const iso = (ms: number) => new Date(ms).toISOString();

/** The database as the `mail` function sees it; every call is recorded by name. */
class FakeStore implements MailStore {
  rows: MailRow[] = [];
  accounts: Record<string, { ciphertext: string; iv: string; mints: string[] }> = {};
  work: Record<string, string[]> = {};
  calls: string[] = [];
  /** A concurrent mint landing between this one's insert and its recount. */
  raceInsert: (() => void) | null = null;
  private n = 0;
  addresses(accountId: string) {
    this.calls.push("addresses");
    return Promise.resolve(this.rows.filter((r) => r.accountId === accountId).map((r) => ({ ...r })));
  }
  ensureAccount(accountId: string, fpKey: { ciphertext: string; iv: string }) {
    this.calls.push("ensureAccount");
    this.accounts[accountId] ??= { ...fpKey, mints: [] };
    return Promise.resolve();
  }
  mints(accountId: string) {
    this.calls.push("mints");
    return Promise.resolve([...(this.accounts[accountId]?.mints ?? [])]);
  }
  setMints(accountId: string, seen: string[], next: string[]) {
    this.calls.push("setMints");
    const a = this.accounts[accountId];
    if (a === undefined || a.mints.join() !== seen.join()) return Promise.resolve(false);
    a.mints = [...next];
    return Promise.resolve(true);
  }
  insertAddress(accountId: string, provider: MailRow["provider"], token: NewToken) {
    this.calls.push("insertAddress");
    const id = `0000000${++this.n}-0000-4000-8000-000000000000`;
    this.rows.push(blank(id, accountId, provider, token));
    this.raceInsert?.();
    return Promise.resolve(id);
  }
  setHint(accountId: string, id: string, hint: string) {
    this.calls.push("setHint");
    const r = this.rows.find((x) => x.id === id && x.accountId === accountId);
    if (r === undefined || r.mailboxHint !== null) return Promise.resolve(false);
    r.mailboxHint = hint;
    return Promise.resolve(true);
  }
  rotate(
    accountId: string,
    id: string,
    fromHmac: string,
    next: NewToken & { retiredUntil: string; status: string | null },
  ) {
    this.calls.push("rotate");
    const r = this.rows.find((x) => x.id === id && x.accountId === accountId && x.tokenHmac === fromHmac);
    if (r === undefined) return Promise.resolve(false);
    Object.assign(r, {
      retiredTokenHmac: r.tokenHmac,
      retiredUntil: next.retiredUntil,
      tokenHmac: next.tokenHmac,
      tokenCiphertext: next.tokenCiphertext,
      tokenIv: next.tokenIv,
    });
    if (next.status !== null) r.status = next.status;
    return Promise.resolve(true);
  }
  revoke(accountId: string, id: string) {
    this.calls.push("revoke");
    const r = this.rows.find((x) => x.id === id && x.accountId === accountId);
    if (r !== undefined) r.status = "revoked";
    return Promise.resolve();
  }
  workKeys(accountId: string, id: string) {
    this.calls.push("workKeys");
    const mine = this.rows.some((x) => x.id === id && x.accountId === accountId);
    return Promise.resolve(mine ? [...(this.work[id] ?? [])] : []);
  }
  deleteMailAddress(id: string) {
    this.calls.push("deleteMailAddress");
    this.rows = this.rows.filter((x) => x.id !== id);
    delete this.work[id];
    return Promise.resolve();
  }
  // T4.2's half: canaries, batches, the held set, the state.
  batches: (BatchRow & { nonceHmac: string })[] = [];
  canaries: { id: string; addressId: string; nonceHmac: string; sentAt: string }[] = [];
  seen: (HeldRow & { accountId: string })[] = [];
  heldAsked: string[][] = [];
  private mine(accountId: string, id: string) {
    return this.rows.find((x) => x.id === id && x.accountId === accountId);
  }
  takeCanary(
    accountId: string,
    id: string,
    seen: { count: number; lastSentAt: string | null },
    next: { count: number; lastSentAt: string },
  ) {
    this.calls.push("takeCanary");
    const r = this.mine(accountId, id);
    if (r === undefined || r.canariesToday !== seen.count || r.lastCanarySentAt !== seen.lastSentAt) {
      return Promise.resolve(false);
    }
    r.canariesToday = next.count;
    r.lastCanarySentAt = next.lastSentAt;
    return Promise.resolve(true);
  }
  insertCanary(addressId: string, nonceHmac: string, sentAt: string) {
    this.calls.push("insertCanary");
    const id = crypto.randomUUID();
    this.canaries.push({ id, addressId, nonceHmac, sentAt });
    return Promise.resolve(id);
  }
  deleteCanary(addressId: string, id: string) {
    this.calls.push("deleteCanary");
    this.canaries = this.canaries.filter((c) => !(c.id === id && c.addressId === addressId));
    return Promise.resolve();
  }
  setBackfillSince(accountId: string, id: string, since: string) {
    this.calls.push("setBackfillSince");
    const r = this.mine(accountId, id);
    if (r === undefined || r.backfillSince !== null) return Promise.resolve(false);
    r.backfillSince = since;
    return Promise.resolve(true);
  }
  openBatches(addressId: string) {
    this.calls.push("openBatches");
    return Promise.resolve(
      this.batches.filter((b) => b.addressId === addressId && b.closedAt === null).length,
    );
  }
  insertBatch(b: NewBatch) {
    this.calls.push("insertBatch");
    const id = crypto.randomUUID();
    this.batches.push({ ...b, id, sent: null, received: 0, sentAt: null, closedAt: null });
    return Promise.resolve(id);
  }
  batch(id: string) {
    this.calls.push("batch");
    const b = this.batches.find((x) => x.id === id);
    if (b === undefined) return Promise.resolve(null);
    const { nonceHmac: _, ...row } = b;
    return Promise.resolve(row);
  }
  markSent(addressId: string, id: string, sent: number, sentAt: string) {
    this.calls.push("markSent");
    const b = this.batches.find((x) => x.id === id && x.addressId === addressId);
    if (b === undefined || b.sent !== null || b.closedAt !== null) return Promise.resolve(false);
    Object.assign(b, { sent, sentAt });
    return Promise.resolve(true);
  }
  fpKey(accountId: string) {
    this.calls.push("fpKey");
    const a = this.accounts[accountId];
    return Promise.resolve(a === undefined ? null : { ciphertext: a.ciphertext, iv: a.iv });
  }
  held(accountId: string, from: string, to: string) {
    this.calls.push("held");
    this.heldAsked.push([accountId, from, to]);
    const [lo, hi] = [Date.parse(from), Date.parse(to)];
    return Promise.resolve(
      this.seen
        .filter((s) => s.accountId === accountId && Date.parse(s.minute) >= lo && Date.parse(s.minute) <= hi)
        .map(({ fp, minute }) => ({ fp, minute })),
    );
  }
  setState(accountId: string, id: string, patch: StatePatch) {
    this.calls.push("setState");
    const r = this.mine(accountId, id);
    if (r === undefined || r.status === "revoked") return Promise.resolve(false);
    if (patch.status !== undefined) r.status = patch.status;
    if (patch.reenabledAt !== undefined) r.reenabledAt = patch.reenabledAt;
    if (patch.confirmUntil !== undefined) r.confirmUntil = patch.confirmUntil;
    return Promise.resolve(true);
  }
}

function blank(id: string, accountId: string, provider: MailRow["provider"], token: NewToken): MailRow {
  return {
    id,
    accountId,
    provider,
    status: "pending",
    ...token,
    retiredTokenHmac: null,
    retiredUntil: null,
    mailboxHint: null,
    lapsedSince: null,
    reenabledAt: null,
    backfillSince: null,
    lastReceivedAt: null,
    lastCanaryOkAt: null,
    lastCanarySentAt: null,
    canariesToday: 0,
    confirmUntil: null,
    unbound: 0,
    unauthenticated: 0,
    expired: 0,
  };
}

/** A stored row for `tok`, its token encrypted and HMACed exactly as the handler keeps it. */
async function stored(
  id: string,
  accountId: string,
  tok: string,
  over: Partial<MailRow> = {},
): Promise<MailRow> {
  const box = await encryptString(ENC_KEY, tok);
  const token = {
    tokenHmac: await tokenHmac(ADDRESS_KEY, tok),
    tokenCiphertext: box.ciphertext,
    tokenIv: box.iv,
  };
  return { ...blank(id, accountId, "m365", token), status: "active", ...over };
}

// ---------------------------------------------------------------- the gates, the bucket, the resolver

const ACCT = "acct-1";
const OTHER = "acct-2";
const A1 = "11111111-1111-4111-8111-111111111111";
const A2 = "22222222-2222-4222-8222-222222222222";
const A3 = "33333333-3333-4333-8333-333333333333";
const B1 = "44444444-4444-4444-8444-444444444444"; // the other account's
const TOK1 = "abcdefghijklmnopqrstuvwxyz";
const TOK2 = "bcdefghijklmnopqrstuvwxyz2";
const TOK3 = "cdefghijklmnopqrstuvwxyz23";
const TOKB = "defghijklmnopqrstuvwxyz234";

/** The real gates over a fake session (`session` is account 1, `other` account 2) and a fake row. */
const verify = (t: string) =>
  Promise.resolve(
    t === "session" ? { id: ACCT, email: null } : t === "other" ? { id: OTHER, email: null } : null,
  );
const gate = (status: EntitlementStatus) => (req: Request) =>
  requireActiveEntitlementWith(req, {
    verify,
    lookup: () => Promise.resolve({ plan: "monthly", status, current_period_end: null }),
  });
const signedIn = async (req: Request) => ({ account_id: (await requireUser(req, verify)).id });

/** MX answers by domain; a domain not listed has none. */
const MX: Record<string, string[]> = {
  "school.test": ["school-test.mail.protection.outlook.com"],
  "uni.test": ["aspmx.l.google.com", "alt1.aspmx.l.google.com"],
  "elsewhere.test": ["mx1.elsewhere.test"],
};

/** One request the fake Resend received. */
interface Sent {
  url: string;
  method: string;
  auth: string | null;
  body: { from: string; to: string[]; subject: string; text: string; headers: Record<string, string> };
}
const RESEND_URL = "http://127.0.0.1:9/emails";

interface World {
  store: FakeStore;
  deleted: string[];
  failDelete: boolean;
  lookups: string[];
  /** The fake Resend's inbox, and whether it refuses. */
  resend: Sent[];
  failSend: boolean;
  /** `deps.now()`, movable. */
  clock: number;
  deps: MailDeps;
  call(method: string, path: string, body?: unknown, session?: string): Promise<Response>;
}

async function world(status: EntitlementStatus = "active"): Promise<World> {
  const store = new FakeStore();
  store.rows = [
    await stored(A1, ACCT, TOK1, { provider: "m365", mailboxHint: "jdoe@school.test" }),
    await stored(B1, OTHER, TOKB, { provider: "google", mailboxHint: "joe@gmail.com" }),
  ];
  store.accounts[ACCT] = { ciphertext: "x", iv: "AAAAAAAAAAAAAAAA", mints: [] };
  const w = {
    store,
    deleted: [] as string[],
    failDelete: false,
    lookups: [] as string[],
    resend: [] as Sent[],
    failSend: false,
    clock: NOW,
  } as World;
  const resolveMx: ResolveMx = (domain) => {
    w.lookups.push(domain);
    if (domain === "dnsdown.test") return Promise.reject(new Error("dns"));
    return Promise.resolve(MX[domain] ?? []);
  };
  // A fake Resend: no listener and no egress, the request is read in memory and answered.
  const resendFetch = ((input: string | URL | Request, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    w.resend.push({
      url: String(input),
      method: init?.method ?? "GET",
      auth: headers.get("authorization"),
      body: JSON.parse(String(init?.body)),
    });
    return Promise.resolve(
      w.failSend ? new Response("{}", { status: 500 }) : new Response('{"id":"e-1"}', { status: 200 }),
    );
  }) as typeof fetch;
  w.deps = {
    authenticate: signedIn,
    entitle: gate(status),
    store,
    s3: {
      delete: (key: string) => {
        store.calls.push(`s3 delete ${key}`);
        if (w.failDelete) return Promise.reject(new Error("s3 delete failed (503)"));
        w.deleted.push(key);
        return Promise.resolve();
      },
    },
    tokenHmac: (t) => tokenHmac(ADDRESS_KEY, t),
    encrypt: (s) => encryptString(ENC_KEY, s),
    decrypt: (c, iv) => decryptString(ENC_KEY, c, iv),
    mailDomain: DOMAIN,
    now: () => w.clock,
    resolveMx,
    nonceHmac: (n) => nonceHmac(ADDRESS_KEY, n),
    sendCanary: canarySender({ url: RESEND_URL, key: "test-email-key", fetch: resendFetch }),
  };
  const handler = mailHandler(w.deps);
  w.call = (method, path, body, session = "session") =>
    handler(
      new Request(`http://127.0.0.1/functions/v1/mail${path}`, {
        method,
        headers: { authorization: `Bearer ${session}`, "content-type": "application/json" },
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      }),
    );
  return w;
}

const mints = (w: World) => w.store.accounts[ACCT].mints;
const tokenOf = (address: string) => address.slice(0, address.indexOf("@"));

// ---------------------------------------------------------------- test 11's address half (D3)

Deno.test("a_fourth_live_address_is_refused_by_name", async () => {
  const w = await world();
  w.store.rows.push(await stored(A2, ACCT, TOK2), await stored(A3, ACCT, TOK3, { status: "quiet" }));
  const res = await w.call("POST", "/addresses", { provider: "google" });
  assertEquals(res.status, 409);
  assertEquals(await res.json(), { error: LIVE_CAP });
  // Refused before anything is spent: no mint charged, no row.
  assertEquals(mints(w), []);
  assertEquals(w.store.rows.filter((r) => r.accountId === ACCT).length, 3);
  assert(!w.store.calls.includes("insertAddress"), w.store.calls.join());

  // A revoked row (a Disconnect in progress) is not live: the third mint goes through.
  w.store.rows.find((r) => r.id === A3)!.status = "revoked";
  assertEquals((await w.call("POST", "/addresses", { provider: "google" })).status, 201);

  // Two mints at once: the one whose recount finds four takes its own row back and is refused.
  const v = await world();
  v.store.rows.push(await stored(A2, ACCT, TOK2));
  v.store.raceInsert = () => {
    v.store.raceInsert = null;
    v.store.rows.push(blank("55555555-5555-4555-8555-555555555555", ACCT, "other", {
      tokenHmac: "h",
      tokenCiphertext: "c",
      tokenIv: "AAAAAAAAAAAAAAAA",
    }));
  };
  const raced = await v.call("POST", "/addresses", { provider: "m365" });
  assertEquals(raced.status, 409);
  assertEquals(await raced.json(), { error: LIVE_CAP });
  assertEquals(v.store.rows.filter((r) => r.accountId === ACCT).length, 3);
  assert(v.store.calls.includes("deleteMailAddress"), v.store.calls.join());
});

Deno.test("a_seventh_mint_in_30_days_is_refused_by_name", async () => {
  const w = await world();
  w.store.rows = w.store.rows.filter((r) => r.accountId !== ACCT);
  const six = [29, 20, 10, 5, 2, 1].map((d) => iso(NOW - d * DAY));
  w.store.accounts[ACCT].mints = [...six];
  const res = await w.call("POST", "/addresses", { provider: "m365" });
  assertEquals(res.status, 429);
  assertEquals(await res.json(), { error: MINT_CAP });
  assertEquals(mints(w), six);
  assert(!w.store.calls.includes("insertAddress"), w.store.calls.join());
  // Rotate is a mint too.
  w.store.rows.push(await stored(A1, ACCT, TOK1));
  const rot = await w.call("POST", `/addresses/${A1}/rotate`);
  assertEquals(rot.status, 429);
  assertEquals(await rot.json(), { error: MINT_CAP });
  assertEquals(w.store.rows.find((r) => r.id === A1)!.retiredTokenHmac, null);

  // Once the oldest leaves the window, the seventh goes through and the window is pruned.
  w.store.accounts[ACCT].mints = [iso(NOW - 31 * DAY), ...six.slice(1)];
  assertEquals((await w.call("POST", "/addresses", { provider: "m365" })).status, 201);
  assertEquals(mints(w), [...six.slice(1), iso(NOW)]);
});

Deno.test("a_minted_token_is_26_lowercase_base32_characters_kept_as_hmac_and_ciphertext", async () => {
  const w = await world();
  delete w.store.accounts[ACCT];
  const res = await w.call("POST", "/addresses", { provider: "m365" });
  assertEquals(res.status, 201);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), ["address", "address_id", "provider", "status"]);
  assertEquals(body.provider, "m365");
  assertEquals(body.status, "pending");
  assert(body.address.endsWith(`@${DOMAIN}`), "the address is at MAIL_DOMAIN");
  const tok = tokenOf(body.address);
  assert(TOKEN.test(tok), "26 characters of lower-case base32");
  const row = w.store.rows.find((r) => r.id === body.address_id)!;
  assertEquals(row.accountId, ACCT);
  assertEquals(row.tokenHmac, await tokenHmac(ADDRESS_KEY, tok));
  assertEquals(await decryptString(ENC_KEY, row.tokenCiphertext, row.tokenIv), tok);
  assertEquals(row.tokenIv.length, 16);
  assert(
    !row.tokenCiphertext.includes(tok) && !JSON.stringify(row).includes(tok),
    "the token is never stored bare",
  );
  assertEquals(mints(w), [iso(NOW)]);
  // The account's row was opened with a fingerprint key: 32 random bytes, base64, encrypted.
  const fp = w.store.accounts[ACCT];
  assertEquals(atob(await decryptString(ENC_KEY, fp.ciphertext, fp.iv)).length, 32);
  // Two mints never share a token.
  const again = await (await w.call("POST", "/addresses", { provider: "m365" })).json();
  assertNotEquals(tokenOf(again.address), tok);
  // A provider outside the three is refused before anything is written.
  const bad = await w.call("POST", "/addresses", { provider: "yahoo" });
  assertEquals(bad.status, 400);
  assertEquals(w.store.rows.filter((r) => r.accountId === ACCT).length, 3);
});

Deno.test("rotate_keeps_the_old_token_until_the_new_canary_or_seven_days", async () => {
  // The seven days are written here; the new address's canary ending them early is intake's half
  // (`mail-inbound/handler_test.ts`: `a_canary_through_the_new_token_ends_the_rotate_grace`).
  const w = await world();
  const before = { ...w.store.rows.find((r) => r.id === A1)! };
  const res = await w.call("POST", `/addresses/${A1}/rotate`);
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), ["address", "address_id", "grace_until"]);
  assertEquals(body.address_id, A1);
  assertEquals(ROTATE_GRACE_MS, 7 * DAY);
  assertEquals(body.grace_until, iso(NOW + 7 * DAY));
  const tok = tokenOf(body.address);
  assert(TOKEN.test(tok) && tok !== TOK1, "a new token");
  const row = w.store.rows.find((r) => r.id === A1)!;
  assertEquals(row.tokenHmac, await tokenHmac(ADDRESS_KEY, tok));
  assertEquals(await decryptString(ENC_KEY, row.tokenCiphertext, row.tokenIv), tok);
  // The old token is retired, not dropped: intake still binds it until `retired_until`.
  assertEquals(row.retiredTokenHmac, before.tokenHmac);
  assertEquals(row.retiredUntil, iso(NOW + 7 * DAY));
  // The same mailbox: its hint and state stay (an M365 mailbox needs no confirm), and it is a mint.
  assertEquals(row.mailboxHint, "jdoe@school.test");
  assertEquals(row.status, "active");
  assertEquals(mints(w), [iso(NOW)]);

  // On Gmail the new address needs its own confirm (§4.3 steps 3–5): the mailbox reads `pending`.
  const g = await world();
  g.store.rows.push(await stored(A2, ACCT, TOK2, { provider: "google", mailboxHint: "jane@gmail.com" }));
  assertEquals((await g.call("POST", `/addresses/${A2}/rotate`)).status, 200);
  assertEquals(g.store.rows.find((r) => r.id === A2)!.status, "pending");
  // Another account's mailbox, or none, reads as not found.
  assertEquals((await g.call("POST", `/addresses/${B1}/rotate`)).status, 404);
  assertEquals((await g.call("POST", `/addresses/not-an-id/rotate`)).status, 404);
});

Deno.test("delete_reads_work_keys_deletes_their_objects_then_calls_delete_mail_address", async () => {
  const w = await world();
  w.store.work[A1] = ["work/item-1", "work/item-2"];
  const res = await w.call("DELETE", `/addresses/${A1}`);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { deleted: true });
  assertEquals(w.deleted, ["work/item-1", "work/item-2"]);
  // Revoked first, so intake binds nothing new while the purge runs; objects by key; then the RPC.
  const order = w.store.calls.filter((c) => c !== "addresses");
  assertEquals(order, [
    "revoke",
    "workKeys",
    "s3 delete work/item-1",
    "s3 delete work/item-2",
    "deleteMailAddress",
  ]);
  assertEquals(w.store.rows.map((r) => r.id), [B1]);

  // A failed object delete stops before the RPC: the row stays revoked, and a retry finishes it.
  const v = await world();
  v.store.work[A1] = ["work/item-3"];
  v.failDelete = true;
  const failed = await v.call("DELETE", `/addresses/${A1}`);
  assertEquals(failed.status, 502);
  assert(!v.store.calls.includes("deleteMailAddress"), v.store.calls.join());
  assertEquals(v.store.rows.find((r) => r.id === A1)!.status, "revoked");
  v.failDelete = false;
  assertEquals((await v.call("DELETE", `/addresses/${A1}`)).status, 200);
  assertEquals(v.deleted, ["work/item-3"]);

  // Another account's mailbox is not found and is never touched.
  const u = await world();
  assertEquals((await u.call("DELETE", `/addresses/${B1}`)).status, 404);
  assertEquals(u.store.rows.find((r) => r.id === B1)!.status, "active");
  assert(
    !u.store.calls.includes("revoke") && !u.store.calls.includes("deleteMailAddress"),
    u.store.calls.join(),
  );
});

Deno.test("a_hint_is_set_once_and_must_fit_the_providers_domains", async () => {
  const w = await world();
  w.store.rows.push(
    await stored(A2, ACCT, TOK2, { provider: "google" }),
    await stored(A3, ACCT, TOK3, { provider: "m365" }),
  );
  // Gmail: gmail.com or googlemail.com, with no lookup; a school address does not fit.
  assertEquals((await w.call("POST", `/addresses/${A2}/hint`, { hint: "jdoe@school.test" })).status, 422);
  const set = await w.call("POST", `/addresses/${A2}/hint`, { hint: " Jane.Doe@Gmail.com " });
  assertEquals(set.status, 200);
  assertEquals(await set.json(), { address_id: A2, mailbox_hint: "jane.doe@gmail.com" });
  // Set once: a second hint, even a fitting one (a Google-hosted domain), is refused before any lookup and
  // changes nothing.
  assertEquals((await w.call("POST", `/addresses/${A2}/hint`, { hint: "other@uni.test" })).status, 409);
  assertEquals(w.store.rows.find((r) => r.id === A2)!.mailboxHint, "jane.doe@gmail.com");
  assertEquals(w.lookups, ["school.test"]);

  // M365: only a mismatch the server can prove is refused: a Google-hosted or an MX-less domain, or a
  // Gmail address. (A third-party MX is a filtering gateway's; under PS the curated row wins.)
  for (const hint of ["a@uni.test", "a@nomx.test", "a@gmail.com"]) {
    assertEquals((await w.call("POST", `/addresses/${A3}/hint`, { hint })).status, 422, hint);
  }
  assertEquals(w.store.rows.find((r) => r.id === A3)!.mailboxHint, null);
  // Not an address at all: refused before any lookup.
  const looked = w.lookups.length;
  for (const hint of ["", "no-at-sign", "a@b@school.test", "a@school", `${"x".repeat(250)}@school.test`, 7]) {
    assertEquals((await w.call("POST", `/addresses/${A3}/hint`, { hint })).status, 400, String(hint));
  }
  assertEquals(w.lookups.length, looked);
  // One mailbox per address: the hint another of the account's mailboxes holds is refused.
  assertEquals((await w.call("POST", `/addresses/${A3}/hint`, { hint: "JDoe@school.test" })).status, 409);
  assertEquals((await w.call("POST", `/addresses/${A3}/hint`, { hint: "jsmith@school.test" })).status, 200);
  assertEquals(w.store.rows.find((r) => r.id === A3)!.mailboxHint, "jsmith@school.test");
  // Another account's mailbox reads as not found.
  assertEquals((await w.call("POST", `/addresses/${B1}/hint`, { hint: "joe2@gmail.com" })).status, 404);
});

Deno.test("a_school_hint_behind_a_filtering_gateways_mx_fits_m365_and_google", async () => {
  // Spec §4.10 and PS: the curated row's provider wins, and the server cannot see the row. A school whose
  // MX is a filtering gateway (Proofpoint here) reads `other`; that is no proof against the mailbox's
  // provider, so its students' hints fit an M365 or a Google mailbox.
  const w = await world();
  MX["gateway.test"] = ["mx0a-001.pphosted.com", "mx0b-001.pphosted.com"];
  try {
    w.store.rows.push(
      await stored(A2, ACCT, TOK2, { provider: "google" }),
      await stored(A3, ACCT, TOK3, { provider: "m365" }),
    );
    assertEquals((await w.call("POST", `/addresses/${A3}/hint`, { hint: "jdoe@gateway.test" })).status, 200);
    assertEquals((await w.call("POST", `/addresses/${A2}/hint`, { hint: "jane@gateway.test" })).status, 200);
    // A Gmail address on a mailbox that is not Google's is still refused, with no lookup.
    w.store.rows.find((r) => r.id === A3)!.mailboxHint = null;
    const looked = w.lookups.length;
    assertEquals(
      (await w.call("POST", `/addresses/${A3}/hint`, { hint: "jdoe@googlemail.com" })).status,
      422,
    );
    assertEquals(w.lookups.length, looked);
  } finally {
    delete MX["gateway.test"];
  }
});

Deno.test("hint_fits_refuses_only_a_provable_mismatch", async () => {
  const mx = (domain: string) => Promise.resolve(MX[domain] ?? []);
  const cases: [MailProvider, string, boolean][] = [
    ["m365", "school.test", true],
    ["m365", "elsewhere.test", true],
    ["m365", "uni.test", false],
    ["m365", "nomx.test", false],
    ["m365", "gmail.com", false],
    ["google", "gmail.com", true],
    ["google", "uni.test", true],
    ["google", "elsewhere.test", true],
    ["google", "school.test", false],
    ["other", "elsewhere.test", true],
    ["other", "school.test", false],
    ["other", "uni.test", false],
    ["other", "gmail.com", false],
    ["other", "nomx.test", false],
  ];
  for (const [provider, domain, fits] of cases) {
    assertEquals(await hintFits(provider, domain, mx), fits, `${provider} ${domain}`);
  }
});

// ---------------------------------------------------------------- test 16b: entitlement on the routes

Deno.test("get_and_delete_addresses_succeed_for_a_canceled_account", async () => {
  for (const status of ["canceled", "past_due", "none"] as const) {
    const w = await world(status);
    w.store.work[A1] = ["work/item-9"];
    const list = await w.call("GET", "/addresses");
    assertEquals(list.status, 200, status);
    const { addresses } = await list.json();
    assertEquals(addresses.length, 1, status);
    // The token itself goes back to its own account's Settings (D3); never its HMAC, ciphertext or IV.
    assertEquals(addresses[0].address, `${TOK1}@${DOMAIN}`);
    assertEquals(addresses[0].address_id, A1);
    assertEquals(addresses[0].mailbox_hint, "jdoe@school.test");
    const text = JSON.stringify(addresses);
    const row = w.store.rows.find((r) => r.id === A1)!;
    for (const secret of [row.tokenHmac, row.tokenCiphertext, row.tokenIv, TOKB, "joe@gmail.com"]) {
      assert(!text.includes(secret), `${status}: the list carries a secret or another account's row`);
    }
    const del = await w.call("DELETE", `/addresses/${A1}`);
    assertEquals(del.status, 200, status);
    assertEquals(w.deleted, ["work/item-9"], status);
    assertEquals(w.store.rows.map((r) => r.id), [B1], status);
  }
  // Without a session, even these two answer 401.
  const w = await world();
  assertEquals((await w.call("GET", "/addresses", undefined, "nobody")).status, 401);
  assertEquals((await w.call("DELETE", `/addresses/${A1}`, undefined, "nobody")).status, 401);
  assertEquals(w.store.rows.length, 2);
});

Deno.test("every_other_mail_route_answers_402_without_entitlement", async () => {
  const w = await world("canceled");
  const routes: Array<[string, string, unknown?]> = [
    ["POST", "/addresses", { provider: "m365" }],
    ["POST", `/addresses/${A1}/rotate`],
    ["POST", `/addresses/${A1}/hint`, { hint: "jdoe@school.test" }],
    ["POST", `/addresses/${A1}/state`, { state: "off" }],
    ["POST", "/canary", { address_id: A1 }],
    ["POST", "/batches", { address_id: A1, phase: "backfill" }],
    ["POST", `/batches/${A2}/sent`, { sent: 3 }],
    ["POST", "/reconcile", { address_id: A1 }],
    ["GET", "/provider?domain=school.test"],
    ["GET", `/addresses/${A1}`],
    ["PATCH", "/addresses"],
    ["GET", "/anything-else"],
  ];
  for (const [method, path, body] of routes) {
    const res = await w.call(method, path, body);
    assertEquals(res.status, 402, `${method} ${path}`);
  }
  // Nothing was read or written for any of them.
  assertEquals(w.store.calls, []);
  assertEquals(mints(w), []);
  assertEquals(w.lookups, []);
});

Deno.test("an_entitled_call_to_an_unknown_route_is_404_and_a_wrong_method_405", async () => {
  const w = await world();
  assertEquals((await w.call("GET", "/anything-else")).status, 404);
  assertEquals((await w.call("PUT", "/addresses")).status, 405);
  assertEquals((await w.call("GET", `/addresses/${A1}/rotate`)).status, 405);
  assertEquals(w.store.calls, []);
});

Deno.test("no_mail_log_line_carries_a_token_hint_or_key", async () => {
  const logs: string[] = [];
  const saved = { ...console };
  const capture = (...args: unknown[]) => logs.push(args.map(String).join(" "));
  Object.assign(console, { log: capture, info: capture, warn: capture, error: capture, debug: capture });
  try {
    const w = await world();
    w.store.work[A1] = ["work/item-7"];
    const minted = await (await w.call("POST", "/addresses", { provider: "google" })).json();
    await w.call("POST", `/addresses/${minted.address_id}/hint`, { hint: "jane@gmail.com" });
    await w.call("POST", `/addresses/${A1}/rotate`);
    await w.call("GET", "/addresses");
    w.failDelete = true;
    await w.call("DELETE", `/addresses/${A1}`);
    const secrets = [tokenOf(minted.address), TOK1, "jane@gmail.com", "jdoe@school.test", "work/item-7"];
    for (const line of logs) {
      for (const s of secrets) assert(!line.includes(s), "a log line carries a token, a hint or a key");
    }
  } finally {
    Object.assign(console, saved);
  }
});

// ================================================================ T4.2: canary, batches, reconcile, state,
// provider (spec test 6's route half; §4.2, §4.10, §4.11, §6.3; P9, P21, BW, PS)

const NONCE32 = /^[0-9a-f]{32}$/;
const dayOf = (ms: number) => new Date(ms).toISOString().slice(0, 10);
const TODAY = Date.UTC(2026, 9, 2);

Deno.test("post_canary_refuses_a_hint_outside_the_providers_domains", async () => {
  const w = await world();
  w.store.rows.push(
    // A Gmail mailbox whose hint is a Microsoft-hosted school address, and an M365 one holding a Gmail
    // address: neither fits, so the canary would mail a third party.
    await stored(A2, ACCT, TOK2, { provider: "google", mailboxHint: "jdoe2@school.test" }),
    await stored(A3, ACCT, TOK3, { provider: "m365", mailboxHint: "jane@gmail.com" }),
  );
  for (const id of [A2, A3]) {
    const res = await w.call("POST", "/canary", { address_id: id });
    assertEquals(res.status, 422, id);
    assertEquals(await res.json(), { error: "this address does not fit the mailbox's provider" });
  }
  // A mailbox with no hint yet has nowhere to send it.
  w.store.rows.find((r) => r.id === A3)!.mailboxHint = null;
  const none = await w.call("POST", "/canary", { address_id: A3 });
  assertEquals(none.status, 409);
  assertEquals(await none.json(), { error: NO_HINT });
  // DNS that cannot answer is a retry, never a send.
  w.store.rows.find((r) => r.id === A3)!.mailboxHint = "a@dnsdown.test";
  assertEquals((await w.call("POST", "/canary", { address_id: A3 })).status, 503);
  // Another account's mailbox, a malformed id, or a revoked one: not found.
  for (const id of [B1, "not-an-id", 7]) {
    assertEquals((await w.call("POST", "/canary", { address_id: id })).status, 404, String(id));
  }
  // Nothing was charged, stored or sent for any of them.
  assertEquals(w.resend, []);
  assertEquals(w.store.canaries, []);
  assert(!w.store.calls.includes("takeCanary"), w.store.calls.join());
  // The fitting hint goes through.
  assertEquals((await w.call("POST", "/canary", { address_id: A1 })).status, 201);
  assertEquals(w.resend.map((s) => s.body.to), [["jdoe@school.test"]]);
});

Deno.test("a_fourth_canary_for_one_address_in_a_utc_day_is_refused", async () => {
  const w = await world();
  w.store.rows.push(await stored(A2, ACCT, TOK2, { provider: "google", mailboxHint: "jane@gmail.com" }));
  for (let i = 0; i < 3; i++) {
    w.clock = NOW + i * 3_600_000;
    assertEquals((await w.call("POST", "/canary", { address_id: A1 })).status, 201, `canary ${i + 1}`);
  }
  w.clock = TODAY + DAY - 1; // the last millisecond of the same UTC day
  const fourth = await w.call("POST", "/canary", { address_id: A1 });
  assertEquals(fourth.status, 429);
  assertEquals(await fourth.json(), { error: CANARY_CAP });
  assertEquals(w.resend.length, 3);
  assertEquals(w.store.canaries.length, 3);
  // The cap is per address: the account's other mailbox still has its three.
  assertEquals((await w.call("POST", "/canary", { address_id: A2 })).status, 201);
  // A new UTC day starts a new count.
  w.clock = TODAY + DAY;
  assertEquals((await w.call("POST", "/canary", { address_id: A1 })).status, 201);
  const a1 = w.store.rows.find((r) => r.id === A1)!;
  assertEquals([a1.canariesToday, a1.lastCanarySentAt], [1, iso(TODAY + DAY)]);
  // A send the provider refuses still spends its turn (the endpoint cannot be used to retry at will),
  // and its row is taken back, so the periodic check never waits for a canary that was never sent.
  w.failSend = true;
  const failed = await w.call("POST", "/canary", { address_id: A1 });
  assertEquals(failed.status, 502);
  assertEquals(w.store.rows.find((r) => r.id === A1)!.canariesToday, 2);
  assertEquals(w.store.canaries.filter((c) => c.addressId === A1 && c.sentAt === iso(TODAY + DAY)).length, 1);
});

Deno.test("the_canary_goes_through_resend_with_its_nonce_in_a_header_and_the_subject", async () => {
  const w = await world();
  const res = await w.call("POST", "/canary", { address_id: A1 });
  assertEquals(res.status, 201);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), ["address_id", "canary_id", "sent_at"]);
  assertEquals([body.address_id, body.sent_at], [A1, iso(NOW)]);
  // One request to Resend, as `billing-jobs` sends its mail: a bearer key and a JSON body.
  assertEquals(w.resend.length, 1);
  const [sent] = w.resend;
  assertEquals([sent.url, sent.method, sent.auth], [RESEND_URL, "POST", "Bearer test-email-key"]);
  assertEquals(Object.keys(sent.body).sort(), ["from", "headers", "subject", "text", "to"]);
  assertEquals(sent.body.to, ["jdoe@school.test"]);
  // From the address intake recognises as Knowlu's canary sender.
  assertEquals(CANARY_FROM, MAIL_CANARY_FROM);
  assertEquals(sent.body.from, `Knowlu <${MAIL_CANARY_FROM}>`);
  // The nonce in a header and in the Subject, in the one form intake reads (`nonceTag`).
  const nonce = sent.body.headers[CANARY_HEADER];
  assert(NONCE32.test(nonce), "a 128-bit nonce");
  assertEquals(Object.keys(sent.body.headers), [CANARY_HEADER]);
  assertEquals(CANARY_SUBJECT, "Knowlu delivery check — no need to open this");
  assertEquals(sent.body.subject, `${CANARY_SUBJECT} ${nonceTag(nonce)}`);
  // One sentence of body: what it is and where to turn it off; no address, token or nonce in it.
  assert(sent.body.text.includes("Settings") && !sent.body.text.includes(nonce), sent.body.text);
  assert(!sent.body.text.replace(/\.$/, "").includes(". "), "one sentence");
  assert(!sent.body.text.includes("@") && !sent.body.subject.includes("@"), "no address in the text");
  // Kept only as its HMAC, under the key intake matches canaries by; the row is the answer's.
  assertEquals(w.store.canaries, [
    { id: body.canary_id, addressId: A1, nonceHmac: await nonceHmac(ADDRESS_KEY, nonce), sentAt: iso(NOW) },
  ]);
  assert(!JSON.stringify(body).includes(nonce), "the nonce is never answered to the device");
  // A second canary carries a new nonce.
  await w.call("POST", "/canary", { address_id: A1 });
  assertNotEquals(w.resend[1].body.headers[CANARY_HEADER], nonce);
});

Deno.test("post_batches_opens_a_batch_of_its_phase_and_answers_its_nonce_once", async () => {
  const w = await world();
  const since = dayOf(TODAY - 120 * DAY);
  const res = await w.call("POST", "/batches", { address_id: A1, phase: "backfill", since, expected: 240 });
  assertEquals(res.status, 201);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), ["address_id", "batch_id", "expected", "nonce", "phase", "since"]);
  assertEquals([body.address_id, body.phase, body.since, body.expected], [A1, "backfill", since, 240]);
  assert(NONCE32.test(body.nonce), "a 128-bit nonce");
  // Kept as its HMAC under the key intake matches batches by; the window start is the batch's `since`.
  const [b] = w.store.batches;
  assertEquals(b.id, body.batch_id);
  assertEquals(b.nonceHmac, await nonceHmac(ADDRESS_KEY, body.nonce));
  assertEquals(
    [b.addressId, b.phase, b.since, b.expected, b.sent],
    [A1, "backfill", `${since}T00:00:00.000Z`, 240, null],
  );
  assert(!JSON.stringify(b).includes(body.nonce), "the nonce itself is never stored");
  // Once: no other route answers it again.
  const list = await (await w.call("GET", "/addresses")).text();
  assert(!list.includes(body.nonce) && !list.includes(b.nonceHmac), "the list carries no nonce");
  // A recover batch of the same mailbox, over the last three days: its own nonce.
  const from = iso(NOW - 3 * DAY);
  const rec = await w.call("POST", "/batches", {
    address_id: A1,
    phase: "recover",
    since: from,
    expected: 2,
  });
  assertEquals(rec.status, 201);
  const r = await rec.json();
  assertEquals([r.phase, r.since], ["recover", from]);
  assertNotEquals(r.nonce, body.nonce);
  assertEquals(w.store.batches.map((x) => x.phase), ["backfill", "recover"]);

  // Refused before anything is written: a bad phase, count or date; no hint; another account's mailbox.
  const before = w.store.batches.length;
  const bad: Array<[unknown, number]> = [
    [{ address_id: A1, phase: "live", since: from, expected: 1 }, 400],
    [{ address_id: A1, phase: "recover", since: from, expected: -1 }, 400],
    [{ address_id: A1, phase: "recover", since: from, expected: 1.5 }, 400],
    [{ address_id: A1, phase: "recover", since: "yesterday", expected: 1 }, 400],
    [{ address_id: A1, phase: "recover", since: iso(NOW + DAY), expected: 1 }, 400],
    [{ address_id: A1, phase: "backfill", since: from, expected: 1 }, 400], // a backfill starts on a date
    [{ address_id: A1, phase: "recover", since: from, expected: 1, subject: "x" }, 400],
    [{ address_id: B1, phase: "recover", since: from, expected: 1 }, 404],
  ];
  for (const [req, status] of bad) {
    assertEquals((await w.call("POST", "/batches", req)).status, status, JSON.stringify(req));
  }
  w.store.rows.push(await stored(A2, ACCT, TOK2, { mailboxHint: null }));
  const recover = (id: string) =>
    w.call("POST", "/batches", { address_id: id, phase: "recover", since: from, expected: 1 });
  const unbound = await recover(A2);
  assertEquals(unbound.status, 409);
  assertEquals(await unbound.json(), { error: NO_HINT });
  assertEquals(w.store.batches.length, before);

  // At most ten open batches a mailbox (intake reads twenty), so a loop cannot pile them up.
  for (let i = w.store.batches.length; i < MAX_OPEN_BATCHES; i++) {
    assertEquals((await recover(A1)).status, 201);
  }
  const capped = await recover(A1);
  assertEquals(capped.status, 409);
  assertEquals(await capped.json(), { error: OPEN_BATCH_CAP });
  w.store.batches[0].closedAt = iso(NOW);
  assertEquals((await recover(A1)).status, 201);
});

Deno.test("a_backfill_batch_reaching_more_than_120_days_before_its_opening_is_refused", async () => {
  const w = await world();
  const open = (since: string, phase = "backfill", id = A1) =>
    w.call("POST", "/batches", { address_id: id, phase, since, expected: 10 });
  // 120 days before 2 October 2026 is 4 June; the device's local day may be a day behind UTC's, so the
  // bound is one day wider than the window, never more.
  assertEquals(dayOf(TODAY - 120 * DAY), "2026-06-04");
  for (const since of [dayOf(TODAY - 122 * DAY), "2026-01-01", "1970-01-01"]) {
    const res = await open(since);
    assertEquals(res.status, 422, since);
    assertEquals(await res.json(), { error: BACKFILL_TOO_WIDE });
  }
  // A date that is not one, or one after tomorrow, is malformed.
  for (const since of ["2026-02-31", "2026-6-4", dayOf(TODAY + 2 * DAY)]) {
    assertEquals((await open(since)).status, 400, since);
  }
  assertEquals(w.store.rows.find((r) => r.id === A1)!.backfillSince, null);
  assertEquals(w.store.batches, []);

  // The first backfill batch fixes the mailbox's window start.
  assertEquals((await open("2026-06-04")).status, 201);
  assertEquals(w.store.rows.find((r) => r.id === A1)!.backfillSince, "2026-06-04");
  // Resumed three days later (now 123 days before), it reads the same start; any other start is refused.
  w.clock = NOW + 3 * DAY;
  assertEquals((await open("2026-06-04")).status, 201);
  const moved = await open(dayOf(TODAY + 3 * DAY - 120 * DAY));
  assertEquals(moved.status, 409);
  assertEquals(await moved.json(), { error: "this mailbox's backfill starts 2026-06-04" });
  // A recover that finishes it reuses that start too, and none reaches before it: an instant a day (or
  // a minute) earlier, or the 180 days the service keeps, would forward mail outside the window.
  assertEquals((await open("2026-06-04T00:00:00Z", "recover")).status, 201);
  assertEquals((await open("2026-06-04", "recover")).status, 201);
  for (const since of ["2026-06-03T00:00:00Z", "2026-06-03T23:59:00Z", iso(NOW + 3 * DAY - 179 * DAY)]) {
    const early = await open(since, "recover");
    assertEquals(early.status, 422, since);
    assertEquals(await early.json(), { error: RECOVER_BEFORE_WINDOW });
  }
  assertEquals(w.store.rows.find((r) => r.id === A1)!.backfillSince, "2026-06-04");
  // A recover reaches back at most as far as the service still holds what it saw (180 days), even for a
  // mailbox whose backfill started earlier than that.
  w.store.rows.find((r) => r.id === A1)!.backfillSince = "2026-01-01";
  const old = await open(iso(NOW + 3 * DAY - 181 * DAY), "recover");
  assertEquals(old.status, 422);
  assertEquals(await old.json(), { error: RECOVER_TOO_WIDE });
  assertEquals((await open(iso(NOW + 3 * DAY - 179 * DAY), "recover")).status, 201);
  // Another mailbox, with a device a day behind UTC: 121 days before the service's day is its 120.
  w.clock = NOW;
  w.store.rows.push(await stored(A2, ACCT, TOK2, { mailboxHint: "jdoe2@school.test" }));
  // Before its first backfill, a recover is held to the bound that backfill would have (121 days).
  const unfixed = await open(iso(TODAY - 121 * DAY - 60_000), "recover", A2);
  assertEquals(unfixed.status, 422);
  assertEquals(await unfixed.json(), { error: RECOVER_BEFORE_WINDOW });
  assertEquals((await open(iso(TODAY - 121 * DAY), "recover", A2)).status, 201);
  assertEquals(w.store.rows.find((r) => r.id === A2)!.backfillSince, null);
  assertEquals((await open(dayOf(TODAY - 121 * DAY), "backfill", A2)).status, 201);
  assertEquals(w.store.rows.find((r) => r.id === A2)!.backfillSince, "2026-06-03");
});

Deno.test("sent_closes_a_batch_with_the_devices_count", async () => {
  const w = await world();
  const open = async (since: string, expected: number) =>
    await (await w.call("POST", "/batches", { address_id: A1, phase: "recover", since, expected })).json();
  const opened = await open(iso(NOW - 3 * DAY), 5);
  const path = `/batches/${opened.batch_id}/sent`;
  w.store.batches[0].received = 2;
  w.clock = NOW + 600_000;
  const res = await w.call("POST", path, { sent: 4 });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    batch_id: opened.batch_id,
    expected: 5,
    sent: 4,
    sent_at: iso(NOW + 600_000),
    received: 2,
  });
  const b = w.store.batches[0];
  // The send is closed: the pull holds the batch until `received` reaches `sent` (or the sweep's two hours).
  assertEquals([b.sent, b.sentAt, b.closedAt], [4, iso(NOW + 600_000), null]);
  // The same count again is the same answer; another count is refused and changes nothing.
  w.clock = NOW + 900_000;
  assertEquals((await w.call("POST", path, { sent: 4 })).status, 200);
  assertEquals((await w.call("POST", path, { sent: 5 })).status, 409);
  assertEquals([b.sent, b.sentAt], [4, iso(NOW + 600_000)]);
  // A malformed count, a stray field, or a batch that is not the account's.
  for (const sent of [-1, 2.5, "4", null]) {
    assertEquals((await w.call("POST", path, { sent })).status, 400, String(sent));
  }
  assertEquals((await w.call("POST", path, { sent: 4, nonce: "x" })).status, 400);
  assertEquals((await w.call("POST", path, { sent: 4 }, "other")).status, 404);
  assertEquals((await w.call("POST", `/batches/${crypto.randomUUID()}/sent`, { sent: 4 })).status, 404);
  assertEquals((await w.call("POST", "/batches/not-an-id/sent", { sent: 4 })).status, 404);
  assertEquals((await w.call("GET", path)).status, 405);
  // A batch the sweep already closed (14 days unsent) takes no count.
  const late = await open(iso(NOW - DAY), 1);
  w.store.batches[1].closedAt = iso(NOW + DAY);
  assertEquals((await w.call("POST", `/batches/${late.batch_id}/sent`, { sent: 1 })).status, 409);
  assertEquals(w.store.batches[1].sent, null);
});

/** The account's fingerprint key `k`, kept as the mint keeps it: base64 of 32 bytes, encrypted. */
async function withKey(w: World): Promise<string> {
  const k = btoa(String.fromCharCode(...new Uint8Array(32).map((_, i) => i + 1)));
  const box = await encryptString(ENC_KEY, k);
  w.store.accounts[ACCT] = { ...box, mints: [] };
  return k;
}

Deno.test("reconcile_answers_only_held_fingerprints_in_the_window_and_the_key", async () => {
  const w = await world();
  const k = await withKey(w);
  const since = NOW - 3 * DAY;
  const at = (ms: number) => iso(Math.floor(ms / 60_000) * 60_000);
  w.store.seen = [
    { accountId: ACCT, fp: "aa".repeat(32), minute: at(since + DAY) }, // inside
    { accountId: ACCT, fp: "bb".repeat(32), minute: at(since - 14 * 60_000) }, // 14 minutes before: matchable
    { accountId: ACCT, fp: "cc".repeat(32), minute: at(NOW + 15 * 60_000) }, // 15 minutes after: matchable
    { accountId: ACCT, fp: "dd".repeat(32), minute: at(since - 16 * 60_000) }, // too early to match
    { accountId: ACCT, fp: "ee".repeat(32), minute: at(NOW + 16 * 60_000) }, // too late to match
    { accountId: OTHER, fp: "ff".repeat(32), minute: at(since + DAY) }, // another account's
  ];
  const res = await w.call("POST", "/reconcile", { address_id: A1, since: iso(since), until: iso(NOW) });
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), ["address_id", "held", "key", "since", "until"]);
  assertEquals([body.address_id, body.since, body.until], [A1, iso(since), iso(NOW)]);
  // The key the device computes `fp_hash` with, as intake does (`keys.ts`'s `fingerprintInput`).
  assertEquals(body.key, k);
  // Each held entry is its HMAC and its received minute, and nothing else.
  assertEquals(
    body.held.map((h: { fp: string }) => h.fp).sort(),
    ["aa".repeat(32), "bb".repeat(32), "cc".repeat(32)],
  );
  for (const h of body.held) assertEquals(Object.keys(h).sort(), ["fp", "minute"]);
  // Read by the account (the dedup set is the account's, §4.6 step 7), widened by the 15-minute match.
  assertEquals(w.store.heldAsked, [[ACCT, iso(since - 15 * 60_000), iso(NOW + 15 * 60_000)]]);

  // A window that is not one, or wider than the 180 days the service keeps, is refused before a read.
  const bad: Array<[unknown, number]> = [
    [{ address_id: A1, since: iso(NOW), until: iso(since) }, 400],
    [{ address_id: A1, since: "last week", until: iso(NOW) }, 400],
    [{ address_id: A1, until: iso(NOW) }, 400],
    [{ address_id: A1, since: iso(NOW - 182 * DAY), until: iso(NOW) }, 422],
    // Before its first backfill, a mailbox's window reaches back at most 121 UTC days (the backfill's bound).
    [{ address_id: A1, since: iso(TODAY - 121 * DAY - 60_000), until: iso(NOW) }, 422],
    [{ address_id: B1, since: iso(since), until: iso(NOW) }, 404],
  ];
  for (const [req, status] of bad) {
    assertEquals((await w.call("POST", "/reconcile", req)).status, status, JSON.stringify(req));
  }
  assertEquals(w.store.heldAsked.length, 1);
  // Once the backfill fixed its start, no reconcile reaches before it (spec D7 as amended 2026-10-02).
  w.store.rows.find((r) => r.id === A1)!.backfillSince = "2026-06-04";
  const early = await w.call("POST", "/reconcile", {
    address_id: A1,
    since: "2026-06-03T23:59:00Z",
    until: iso(NOW),
  });
  assertEquals(early.status, 422);
  assertEquals(await early.json(), { error: RECONCILE_BEFORE_WINDOW });
  assertEquals(w.store.heldAsked.length, 1);
  assertEquals(
    (await w.call("POST", "/reconcile", { address_id: A1, since: "2026-06-04", until: iso(NOW) })).status,
    200,
  );
  assertEquals(w.store.heldAsked.length, 2);
});

Deno.test("the_reconcile_request_carries_only_the_window", async () => {
  // §4.11 step 3: the device diffs locally; nothing about a message the service does not hold leaves it.
  // A body carrying anything beyond the mailbox and the window is refused before any read.
  const w = await world();
  await withKey(w);
  const window = { address_id: A1, since: iso(NOW - 3 * DAY), until: iso(NOW) };
  const extras = [
    { fingerprints: ["aa".repeat(32)] },
    { messages: [{ sender: "prof@school.test", subject: "Lab 5", received: iso(NOW) }] },
    { subject: "Lab 5" },
    { sender: "prof@school.test" },
  ];
  for (const extra of extras) {
    const res = await w.call("POST", "/reconcile", { ...window, ...extra });
    assertEquals(res.status, 400, JSON.stringify(extra));
    const text = await res.text();
    assert(!text.includes("prof@school.test") && !text.includes("Lab 5"), "the refusal echoes nothing");
  }
  for (const body of [[window], "window", null]) {
    assertEquals((await w.call("POST", "/reconcile", body)).status, 400, JSON.stringify(body));
  }
  assert(!w.store.calls.includes("held") && !w.store.calls.includes("fpKey"), w.store.calls.join());
  // The window alone goes through.
  assertEquals((await w.call("POST", "/reconcile", window)).status, 200);
});

Deno.test("state_reenabled_stamps_reenabled_at_and_elsewhere_sets_off", async () => {
  const w = await world();
  const row = () => w.store.rows.find((r) => r.id === A1)!;
  const set = (state: unknown, id = A1) => w.call("POST", `/addresses/${id}/state`, { state });
  // Found pointing elsewhere: the mailbox is `off` and nothing else changes.
  const away = await set("elsewhere");
  assertEquals(away.status, 200);
  assertEquals(await away.json(), { address_id: A1, state: "elsewhere", status: "off" });
  assertEquals([row().status, row().reenabledAt], ["off", null]);
  // Turned back on by the mail step: `reenabled_at` is stamped; the canary's arrival makes it `active`.
  w.clock = NOW + 60_000;
  const back = await set("reenabled");
  assertEquals(back.status, 200);
  assertEquals(await back.json(), {
    address_id: A1,
    state: "reenabled",
    status: "off",
    reenabled_at: iso(NOW + 60_000),
  });
  assertEquals([row().status, row().reenabledAt], ["off", iso(NOW + 60_000)]);
  // Found off: `off`.
  row().status = "quiet";
  assertEquals((await set("off")).status, 200);
  assertEquals(row().status, "off");
  // Anything else is refused before a read; another account's or a revoked mailbox is not found.
  const calls = w.store.calls.length;
  for (const state of ["active", "revoked", "lapsed", "", 1, null]) {
    assertEquals((await set(state)).status, 400, String(state));
  }
  assertEquals(w.store.calls.length, calls);
  assertEquals((await set("off", B1)).status, 404);
  assertEquals(w.store.rows.find((r) => r.id === B1)!.status, "active");
  row().status = "revoked";
  assertEquals((await set("reenabled")).status, 404);
  assertEquals(row().reenabledAt, iso(NOW + 60_000));
  assertEquals((await w.call("POST", `/addresses/${A1}/state`, { state: "off", why: "x" })).status, 400);
});

Deno.test("state_confirming_opens_a_fifteen_minute_confirm_window", async () => {
  const w = await world();
  w.store.rows.push(
    await stored(A2, ACCT, TOK2, { provider: "google", status: "pending", mailboxHint: "jane@gmail.com" }),
    await stored(A3, ACCT, TOK3, { provider: "google", status: "pending", mailboxHint: null }),
  );
  assertEquals(CONFIRM_WINDOW_MS, 15 * 60_000);
  const res = await w.call("POST", `/addresses/${A2}/state`, { state: "confirming" });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    address_id: A2,
    state: "confirming",
    status: "pending",
    confirm_until: iso(NOW + 15 * 60_000),
  });
  const a2 = w.store.rows.find((r) => r.id === A2)!;
  // The window opens; the status is intake's to move, on Google's confirmation mail (§4.3 step 4).
  assertEquals([a2.confirmUntil, a2.status], [iso(NOW + 15 * 60_000), "pending"]);
  // Asked again later, it opens again from then.
  w.clock = NOW + 20 * 60_000;
  assertEquals((await w.call("POST", `/addresses/${A2}/state`, { state: "confirming" })).status, 200);
  assertEquals(a2.confirmUntil, iso(NOW + 35 * 60_000));
  // Only a Gmail mailbox confirms an address, and only once its own address is known (the requester check).
  const m365 = await w.call("POST", `/addresses/${A1}/state`, { state: "confirming" });
  assertEquals(m365.status, 422);
  assertEquals(w.store.rows.find((r) => r.id === A1)!.confirmUntil, null);
  const nohint = await w.call("POST", `/addresses/${A3}/state`, { state: "confirming" });
  assertEquals(nohint.status, 409);
  assertEquals(await nohint.json(), { error: NO_HINT });
});

Deno.test("provider_reads_mx_through_an_injected_resolver_as_m365_google_or_other", async () => {
  const w = await world();
  const ask = async (domain: string) => {
    const res = await w.call("GET", `/provider?domain=${encodeURIComponent(domain)}`);
    return [res.status, await res.json()];
  };
  assertEquals(await ask("school.test"), [200, { domain: "school.test", provider: "m365" }]);
  assertEquals(await ask("UNI.test"), [200, { domain: "uni.test", provider: "google" }]);
  assertEquals(await ask("elsewhere.test"), [200, { domain: "elsewhere.test", provider: "other" }]);
  // No MX: nothing can be forwarded from there.
  assertEquals(await ask("nomx.test"), [200, { domain: "nomx.test", provider: null }]);
  assertEquals(w.lookups, ["school.test", "uni.test", "elsewhere.test", "nomx.test"]);
  // Google's own domains are never looked up.
  assertEquals(await ask("gmail.com"), [200, { domain: "gmail.com", provider: "google" }]);
  assertEquals(w.lookups.length, 4);
  // DNS that cannot answer is a retry; a value that is not a domain is refused before any lookup.
  assertEquals((await ask("dnsdown.test"))[0], 503);
  for (const d of ["", "jdoe@school.test", "school", "-bad-.test", "a..test"]) {
    assertEquals((await ask(d))[0], 400, d);
  }
  assertEquals((await w.call("GET", "/provider")).status, 400);
  assertEquals((await w.call("POST", "/provider?domain=school.test")).status, 405);
  assertEquals(w.lookups.length, 5);
  assertEquals(w.store.calls, []);
});

Deno.test("no_t4_2_log_line_carries_a_hint_nonce_key_or_fingerprint", async () => {
  const logs: string[] = [];
  const saved = { ...console };
  const capture = (...args: unknown[]) => logs.push(args.map(String).join(" "));
  Object.assign(console, { log: capture, info: capture, warn: capture, error: capture, debug: capture });
  try {
    const w = await world();
    const k = await withKey(w);
    w.store.seen = [{ accountId: ACCT, fp: "ab".repeat(32), minute: iso(NOW - DAY) }];
    await w.call("POST", "/canary", { address_id: A1 });
    w.failSend = true;
    await w.call("POST", "/canary", { address_id: A1 });
    const batch = await (await w.call("POST", "/batches", {
      address_id: A1,
      phase: "backfill",
      since: dayOf(TODAY - 100 * DAY),
      expected: 3,
    })).json();
    await w.call("POST", `/batches/${batch.batch_id}/sent`, { sent: 3 });
    await w.call("POST", "/reconcile", { address_id: A1, since: iso(NOW - 3 * DAY), until: iso(NOW) });
    await w.call("POST", `/addresses/${A1}/state`, { state: "elsewhere" });
    await w.call("GET", "/provider?domain=dnsdown.test");
    const secrets = [
      "jdoe@school.test",
      TOK1,
      k,
      "ab".repeat(32),
      batch.nonce,
      w.resend[0].body.headers[CANARY_HEADER],
      "dnsdown.test",
    ];
    for (const line of logs) {
      for (const s of secrets) {
        assert(!line.includes(s), "a log line carries a hint, nonce, key or fingerprint");
      }
    }
  } finally {
    Object.assign(console, saved);
  }
});
