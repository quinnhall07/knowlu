// Email plan T4.6 (spec test 14's purge and export half; §4.13, D17): account deletion purges every
// mailbox the way Disconnect does, names all seven forwarding tables, and the export carries the
// mailboxes and queued claims and never a token, a key or a hash.
import { assert, assertEquals } from "@std/assert";
import {
  CLAIM_EXPORT_COLUMNS,
  exportMail,
  MAIL_PURGE_TABLES,
  MAIL_TABLES,
  MAILBOX_EXPORT_COLUMNS,
  type MailPurgeDeps,
  purgeMailboxes,
  purgeMailTables,
} from "./mail.ts";

function recorder(over: Partial<MailPurgeDeps> = {}): { calls: string[]; deps: MailPurgeDeps } {
  const calls: string[] = [];
  const keys: Record<string, string[]> = { "box-1": ["work/a", "work/b"], "box-2": [], "box-3": ["work/c"] };
  const deps: MailPurgeDeps = {
    addressIds: (account) => {
      calls.push(`addressIds ${account}`);
      return Promise.resolve(["box-1", "box-2", "box-3"]);
    },
    revoke: (account, id) => {
      calls.push(`revoke ${account} ${id}`);
      return Promise.resolve();
    },
    workKeys: (account, id) => {
      calls.push(`workKeys ${account} ${id}`);
      return Promise.resolve(keys[id]);
    },
    deleteObject: (key) => {
      calls.push(`s3 delete ${key}`);
      return Promise.resolve();
    },
    deleteMailAddress: (id) => {
      calls.push(`delete_mail_address ${id}`);
      return Promise.resolve();
    },
    ...over,
  };
  return { calls, deps };
}

Deno.test("deletion_purges_each_mailbox_by_delete_mail_address_after_deleting_its_objects_by_key", async () => {
  const { calls, deps } = recorder();
  const purged = await purgeMailboxes(deps, "acc-1");
  assertEquals(purged, ["box-1", "box-2", "box-3"]);
  // Each mailbox: revoked (intake binds nothing new), its keys read from `mail_work`, those objects
  // deleted by key (the IAM user cannot list the bucket), and only then `delete_mail_address`.
  assertEquals(calls, [
    "addressIds acc-1",
    "revoke acc-1 box-1",
    "workKeys acc-1 box-1",
    "s3 delete work/a",
    "s3 delete work/b",
    "delete_mail_address box-1",
    "revoke acc-1 box-2",
    "workKeys acc-1 box-2",
    "delete_mail_address box-2",
    "revoke acc-1 box-3",
    "workKeys acc-1 box-3",
    "s3 delete work/c",
    "delete_mail_address box-3",
  ]);
});

Deno.test("a failed object delete never blocks the account's deletion, and logs no key", async () => {
  const logged: string[] = [];
  const original = console.error;
  console.error = (...a: unknown[]) => logged.push(a.map(String).join(" "));
  try {
    const { calls, deps } = recorder({
      deleteObject: (key) => Promise.reject(new Error(`s3 delete failed for ${key}`)),
    });
    await purgeMailboxes(deps, "acc-1");
    // Every mailbox still reaches `delete_mail_address`: the bucket's one-day rule takes a straggler.
    assertEquals(calls.filter((c) => c.startsWith("delete_mail_address")).length, 3);
  } finally {
    console.error = original;
  }
  assert(logged.length > 0, "the failure is named in the log");
  for (const line of logged) {
    assert(!line.includes("work/"), `a log line carried an object key: ${line}`);
    assert(!line.includes("acc-1") && !line.includes("box-"), `a log line carried an id: ${line}`);
  }
});

Deno.test("an account with no mailbox purges nothing and reads no keys", async () => {
  const { calls, deps } = recorder({ addressIds: () => Promise.resolve([]) });
  assertEquals(await purgeMailboxes(deps, "acc-1"), []);
  assertEquals(calls, []);
});

Deno.test("the_purge_list_names_all_seven_mail_tables", async () => {
  assertEquals([...MAIL_TABLES].sort(), [
    "mail_accounts",
    "mail_addresses",
    "mail_batches",
    "mail_canaries",
    "mail_claims",
    "mail_seen",
    "mail_work",
  ]);
  const names = MAIL_PURGE_TABLES.map((t) => t.table);
  assertEquals([...names].sort(), [...MAIL_TABLES].sort(), "the purge list names each mail table once");

  const deleted: string[] = [];
  await purgeMailTables(
    (table, query) => {
      deleted.push(`${table}?${query}`);
      return Promise.resolve();
    },
    "acc-1",
    ["box-1", "box-2"],
  );
  for (const table of MAIL_TABLES) {
    assert(deleted.some((d) => d.startsWith(`${table}?`)), `${table} was never deleted from`);
  }
  // `mail_batches` and `mail_canaries` carry no account id: they are deleted by the account's mailboxes.
  assert(deleted.includes("mail_batches?address_id=in.(box-1,box-2)"));
  assert(deleted.includes("mail_canaries?address_id=in.(box-1,box-2)"));
  assert(deleted.includes("mail_claims?account_id=eq.acc-1"));
  assert(deleted.includes("mail_accounts?account_id=eq.acc-1"));
  // Children before the mailboxes, the mailboxes before the account's mail row.
  const at = (t: string) => deleted.findIndex((d) => d.startsWith(`${t}?`));
  for (const child of ["mail_claims", "mail_work", "mail_seen", "mail_batches", "mail_canaries"]) {
    assert(at(child) < at("mail_addresses"), `${child} is deleted after mail_addresses`);
  }
  assert(at("mail_addresses") < at("mail_accounts"));

  // With no mailbox, the two address-scoped deletes are skipped rather than sent with an empty list.
  const bare: string[] = [];
  await purgeMailTables(
    (table, query) => {
      bare.push(`${table}?${query}`);
      return Promise.resolve();
    },
    "acc-1",
    [],
  );
  assert(!bare.some((d) => d.includes("address_id=in.()")));
  assert(bare.includes("mail_addresses?account_id=eq.acc-1"));
});

Deno.test("index.ts purges the mailboxes after the Google revoke and deletes the mail tables from mail.ts", async () => {
  // `index.ts` is never loaded by `deno test`, so its wiring is pinned by its text, as
  // `handler_test.ts` pins the consents PATCH (and `migrations_sync_test.ts` the older tables' names).
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const google = text.indexOf("await revokeGoogleGrantOnDelete(");
  const mail = text.indexOf("await purgeMailboxes(");
  const consents = text.search(/restPatch\(rest,\s*"consents"/);
  const older = text.indexOf('"entitlements",', consents);
  const tables = text.indexOf("await purgeMailTables(");
  const accounts = text.search(/restDelete\(rest,\s*"accounts"/);
  assert(google >= 0 && mail > google, "the mailboxes are purged after the Google grant is revoked");
  assert(consents > mail && older > consents, "the older tables are deleted after the consents PATCH");
  assert(tables > older && accounts > tables, "the mail tables go before the accounts row");
  // The mail tables are named once, in `mail.ts`: a second literal here would be a second list.
  const purge = text.slice(text.indexOf("purge:"), text.indexOf("deleteAuthUser:"));
  for (const table of MAIL_TABLES) {
    assert(!purge.includes(`"${table}"`), `the purge names ${table} outside mail.ts's list`);
  }
  assert(text.includes("...(await exportMail("), "exportAll carries the mail rows");
});

/** Every column the forwarding tables hold that is a token, a key, a hash or an object key. */
const SECRET_COLUMNS = [
  "token_hmac",
  "token_ciphertext",
  "token_iv",
  "retired_token_hmac",
  "fp_key_ciphertext",
  "fp_key_iv",
  "msg_hash",
  "fp_hash",
  "nonce_hmac",
  "s3_key",
  // Hashes of SES object keys, one per counted batch send (email follow-ups T5).
  "counted_receipts",
];

Deno.test("the_export_carries_mailboxes_and_queued_claims_and_never_a_token_key_or_hash", async () => {
  const asked: string[] = [];
  // A store that answers more than it was asked for, as a careless wire would (`select=*`): the export
  // projects onto its own columns, so a secret cannot ride out even then.
  const mailbox = {
    id: "box-1",
    account_id: "acc-1",
    provider: "m365",
    mailbox_hint: "student@school.example.invalid",
    status: "active",
    lapsed_since: null,
    last_lapsed_at: "2026-09-12T07:00:00Z",
    reenabled_at: "2026-09-20T10:00:00Z",
    confirm_until: null,
    backfill_since: "2026-06-01",
    last_received_at: "2026-09-30T08:00:00Z",
    last_canary_sent_at: "2026-09-29T08:00:00Z",
    last_canary_ok_at: "2026-09-29T08:01:00Z",
    canaries_today: 1,
    unbound: 2,
    unauthenticated: 3,
    expired: 4,
    unbound_own_since_live: 1,
    unbound_foreign_since_live: 0,
    token_hmac: "SECRET-HMAC",
    token_ciphertext: "SECRET-CIPHERTEXT",
    token_iv: "SECRET-IV-123456",
    retired_token_hmac: "SECRET-RETIRED",
    retired_until: "2026-10-05T00:00:00Z",
  };
  const claim = {
    id: 7,
    account_id: "acc-1",
    address_id: "box-1",
    batch_id: null,
    uid: "mail:abc",
    message_date: "2026-09-30T07:00:00Z",
    ordering_date: "2026-09-30T07:00:00Z",
    sender_name: "Prof",
    sender_addr: "prof@school.example.invalid",
    phase: "live",
    authenticity: "aligned",
    claims: [{ kind: "due" }],
    judgment_id: null,
    queued_at: "2026-09-30T08:00:00Z",
    delivered_at: null,
    msg_hash: "SECRET-MSG-HASH",
    fp_hash: "SECRET-FP-HASH",
  };
  const out = await exportMail({
    mailboxes: (account, columns) => {
      asked.push(`mailboxes ${account} ${columns.join(",")}`);
      return Promise.resolve([mailbox]);
    },
    claims: (account, columns) => {
      asked.push(`claims ${account} ${columns.join(",")}`);
      return Promise.resolve([claim]);
    },
  }, "acc-1");

  assertEquals(Object.keys(out).sort(), ["mail_addresses", "mail_claims"]);
  const [box] = out.mail_addresses;
  for (
    const k of [
      "provider",
      "mailbox_hint",
      "status",
      // The latest lapse is the student's own data like `lapsed_since` (review of the lapsed fix, #1).
      "lapsed_since",
      "last_lapsed_at",
      "backfill_since",
      "last_received_at",
      "unbound",
      "expired",
      "unbound_own_since_live",
      "unbound_foreign_since_live",
    ]
  ) assert(Object.hasOwn(box, k), `the mailbox export lacks ${k}`);
  assertEquals(box.mailbox_hint, "student@school.example.invalid");
  const [row] = out.mail_claims;
  for (const k of ["uid", "ordering_date", "sender_addr", "claims", "queued_at", "delivered_at"]) {
    assert(Object.hasOwn(row, k), `the claim export lacks ${k}`);
  }

  // Neither what was asked for nor what went out names a secret column or carries a secret value.
  for (const cols of [MAILBOX_EXPORT_COLUMNS, CLAIM_EXPORT_COLUMNS]) {
    for (const s of SECRET_COLUMNS) {
      assert(!(cols as readonly string[]).includes(s), `an export column list names ${s}`);
    }
  }
  assertEquals(asked, [
    `mailboxes acc-1 ${MAILBOX_EXPORT_COLUMNS.join(",")}`,
    `claims acc-1 ${CLAIM_EXPORT_COLUMNS.join(",")}`,
  ]);
  const text = JSON.stringify(out);
  for (const s of SECRET_COLUMNS) assert(!text.includes(s), `the export carries ${s}`);
  assert(!text.includes("SECRET"), "the export carries a secret value");
  // The address itself is the token: no column that could rebuild it leaves.
  assert(!text.includes("retired_until"));
});

Deno.test("the_secret_column_list_names_the_counted_receipts_the_follow_ups_migration_adds", async () => {
  // Email follow-ups T5: `mail_batches.counted_receipts` holds hashes of SES object keys, so it is a secret
  // column and no export list may carry it. The name is the migration's own, found by the file's suffix.
  const dir = new URL("../../migrations/", import.meta.url);
  const files = [...Deno.readDirSync(dir)].map((e) => e.name).filter((n) =>
    /^\d{14}_mail_follow_ups\.sql$/.test(n)
  );
  assertEquals(files.length, 1);
  const sql = await Deno.readTextFile(new URL(files[0], dir));
  assert(/alter table public\.mail_batches add column if not exists counted_receipts\b/.test(sql));
  assert(SECRET_COLUMNS.includes("counted_receipts"), "the secret-column list lacks counted_receipts");
  for (const cols of [MAILBOX_EXPORT_COLUMNS, CLAIM_EXPORT_COLUMNS]) {
    assert(!(cols as readonly string[]).includes("counted_receipts"));
  }
});
