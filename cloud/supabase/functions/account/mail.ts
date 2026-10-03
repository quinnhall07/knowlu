/**
 * Email forwarding's half of `DELETE /account` and `GET /account/export` (email spec §4.13, D17; plan
 * T4.6), and the purge list every deletion deletes through.
 *
 * - `purgeMailboxes` runs Disconnect's purge (§4.12 step 2) for each of the account's mailboxes: revoke,
 *   read its `work/` keys from `mail_work`, delete those objects by key (the IAM user cannot list the
 *   bucket), then `delete_mail_address`. Unlike Disconnect, **a failed object delete never stops the
 *   account's deletion**: the account's right to delete wins, as it does for the Google grant
 *   (`google_delete.ts`), and the bucket's one-day rule removes a straggler (D2). The failure is logged
 *   by its class only, never a key, an id or a message.
 * - `MAIL_PURGE_TABLES` names all seven forwarding tables for the purge, whether or not the cascade from
 *   `accounts` would reach them: the privacy page's deletion paragraph is written from the purge's list.
 * - `exportMail` hands back the mailboxes (provider, `mailbox_hint`, status, dates, counters) and the
 *   claims the service holds for the account, projected onto fixed column lists: never a token, its
 *   HMAC, a key, a hash or an object key.
 */

/** The seven forwarding tables (§4.13), each named in the purge list below. */
export const MAIL_TABLES = [
  "mail_claims",
  "mail_work",
  "mail_seen",
  "mail_canaries",
  "mail_batches",
  "mail_addresses",
  "mail_accounts",
] as const;

/** How a table's rows are found: by the account, or (two tables with no account id) by its mailboxes. */
export interface PurgeTable {
  table: string;
  by: "account_id" | "address_id";
}

/** Email forwarding's part of the purge list, children before the mailboxes and the mailboxes before the
 * account's row. `delete_mail_address` has already emptied all but `mail_accounts` per mailbox; this is the
 * list's promise that every table is named. `index.ts` keeps the older tables' list beside it. */
export const MAIL_PURGE_TABLES: readonly PurgeTable[] = [
  { table: "mail_claims", by: "account_id" },
  { table: "mail_work", by: "account_id" },
  { table: "mail_seen", by: "account_id" },
  { table: "mail_canaries", by: "address_id" },
  { table: "mail_batches", by: "address_id" },
  { table: "mail_addresses", by: "account_id" },
  { table: "mail_accounts", by: "account_id" },
];

export type DeleteRows = (table: string, query: string) => Promise<void>;

/** Deletes every table in `MAIL_PURGE_TABLES` for the account; `addressIds` are its mailboxes, read before
 * `purgeMailboxes` removed them. An address-scoped table is skipped when there are none. */
export async function purgeMailTables(
  del: DeleteRows,
  accountId: string,
  addressIds: string[],
): Promise<void> {
  const account = `account_id=eq.${encodeURIComponent(accountId)}`;
  const boxes = `address_id=in.(${addressIds.map(encodeURIComponent).join(",")})`;
  for (const { table, by } of MAIL_PURGE_TABLES) {
    if (by === "address_id") {
      if (addressIds.length > 0) await del(table, boxes);
    } else {
      await del(table, account);
    }
  }
}

export interface MailPurgeDeps {
  /** Every `mail_addresses` id of the account, whatever its status. */
  addressIds(accountId: string): Promise<string[]>;
  /** `status = 'revoked'`, so intake binds nothing new while the purge runs. */
  revoke(accountId: string, id: string): Promise<void>;
  /** The mailbox's `mail_work` S3 keys, all of them. */
  workKeys(accountId: string, id: string): Promise<string[]>;
  deleteObject(key: string): Promise<void>;
  /** The `delete_mail_address` RPC (D17). */
  deleteMailAddress(id: string): Promise<void>;
}

const errorClass = (e: unknown) => (e instanceof Error ? e.constructor.name : "unknown");

/** D17's purge, for every mailbox of the account; answers the ids it purged. */
export async function purgeMailboxes(deps: MailPurgeDeps, accountId: string): Promise<string[]> {
  const ids = await deps.addressIds(accountId);
  for (const id of ids) {
    await deps.revoke(accountId, id);
    const keys = await deps.workKeys(accountId, id);
    let failed = 0;
    for (let at = 0; at < keys.length; at += 10) {
      const results = await Promise.allSettled(keys.slice(at, at + 10).map((k) => deps.deleteObject(k)));
      for (const r of results) {
        if (r.status === "rejected") {
          if (failed === 0) console.error(`account delete: mail object delete ${errorClass(r.reason)}`);
          failed += 1;
        }
      }
    }
    if (failed > 0) console.error(`account delete: ${failed} mail object(s) left to the bucket's expiry`);
    await deps.deleteMailAddress(id);
  }
  return ids;
}

/** The mailbox columns the export carries (D17): provider, hint, status, dates and counters. */
export const MAILBOX_EXPORT_COLUMNS = [
  "id",
  "provider",
  "mailbox_hint",
  "status",
  "lapsed_since",
  "reenabled_at",
  "confirm_until",
  "backfill_since",
  "last_received_at",
  "last_canary_sent_at",
  "last_canary_ok_at",
  "canaries_today",
  "unbound",
  "unauthenticated",
  "expired",
  "unbound_own_since_live",
  "unbound_foreign_since_live",
] as const;

/** The claim columns the export carries: everything a queued claim holds but its account id. */
export const CLAIM_EXPORT_COLUMNS = [
  "id",
  "address_id",
  "batch_id",
  "uid",
  "message_date",
  "ordering_date",
  "sender_name",
  "sender_addr",
  "phase",
  "authenticity",
  "claims",
  "judgment_id",
  "queued_at",
  "delivered_at",
] as const;

type Row = Record<string, unknown>;

export interface MailExportStore {
  /** The account's `mail_addresses` rows, reading `columns` only. */
  mailboxes(accountId: string, columns: readonly string[]): Promise<Row[]>;
  /** The account's `mail_claims` rows, reading `columns` only. */
  claims(accountId: string, columns: readonly string[]): Promise<Row[]>;
}

const project = (rows: Row[], columns: readonly string[]): Row[] =>
  rows.map((r) => Object.fromEntries(columns.map((c) => [c, r[c] ?? null])));

export async function exportMail(
  store: MailExportStore,
  accountId: string,
): Promise<{ mail_addresses: Row[]; mail_claims: Row[] }> {
  return {
    mail_addresses: project(await store.mailboxes(accountId, MAILBOX_EXPORT_COLUMNS), MAILBOX_EXPORT_COLUMNS),
    mail_claims: project(await store.claims(accountId, CLAIM_EXPORT_COLUMNS), CLAIM_EXPORT_COLUMNS),
  };
}
