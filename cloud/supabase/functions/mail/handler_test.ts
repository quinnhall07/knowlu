// Email plan T4.1 (spec §4.1, §4.12 step 2, §6.3; D3, D17, P21; tests 11's address half and 16b). The
// handler runs whole over an in-memory store, a fake bucket that records its deletes (no listener: the
// route only deletes by key), an injected MX resolver and the real gates over a fake session and a fake
// `entitlements` row. Every token, address, domain and key here is invented for these tests.
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import { requireUser } from "../_shared/auth.ts";
import { decryptString, encryptString, importAesKey } from "../_shared/crypto.ts";
import { type EntitlementStatus, requireActiveEntitlementWith } from "../_shared/entitlement.ts";
import { hmacKey, tokenHmac } from "../mail-inbound/keys.ts";
import {
  LIVE_CAP,
  type MailDeps,
  mailHandler,
  type MailRow,
  type MailStore,
  MINT_CAP,
  type NewToken,
  ROTATE_GRACE_MS,
} from "./handler.ts";
import { hintFits, type MailProvider, type ResolveMx } from "./provider.ts";

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

interface World {
  store: FakeStore;
  deleted: string[];
  failDelete: boolean;
  lookups: string[];
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
  const w = { store, deleted: [] as string[], failDelete: false, lookups: [] as string[] } as World;
  const resolveMx: ResolveMx = (domain) => {
    w.lookups.push(domain);
    return Promise.resolve(MX[domain] ?? []);
  };
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
    now: () => NOW,
    resolveMx,
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
