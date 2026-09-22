import {
  authDeleteUser,
  authGetUser,
  restDelete,
  restFromEnv,
  restPatch,
  restSelect,
  restSelectAll,
  restUpsert,
} from "../_shared/db.ts";
import { encryptString, importAesKey, sha256Hex } from "../_shared/crypto.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { serviceDb } from "../_shared/judge_db.ts";
import { handle } from "./handler.ts";
import { liveGoogleDeleteDeps, revokeGoogleGrantOnDelete } from "./google_delete.ts";

const eq = (id: string) => `account_id=eq.${encodeURIComponent(id)}`;

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = Deno.env.get("STRIPE_SECRET_KEY");
    if (!stripeKey) throw fail(500, "the function is not configured");
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      requireEntitled: requireActiveEntitlement,
      getAccount: async (id) => {
        const rows = await restSelect<{ email: string; stripe_customer_id: string | null }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(id)}&select=email,stripe_customer_id&limit=1`,
        );
        return rows[0] ?? null;
      },
      getSubscriptionId: async (id) => {
        const rows = await restSelect<{ stripe_subscription_id: string | null }>(
          rest,
          "entitlements",
          `${eq(id)}&select=stripe_subscription_id&limit=1`,
        );
        return rows[0]?.stripe_subscription_id ?? null;
      },
      stripe: stripePostFrom(stripeKey, globalThis.fetch),
      purge: async (id) => {
        // F-5: the account's Google grant, FIRST — `google_accounts` cascades with the `accounts`
        // row below, but a cascade only ever removes the ROW. The refresh token Google is still
        // holding stays valid there forever unless it is revoked here; `revokeGoogleGrantOnDelete`
        // never blocks the deletion below on a failed revoke (the account's right to delete wins).
        await revokeGoogleGrantOnDelete(id, liveGoogleDeleteDeps(serviceDb()));
        // The consent log survives, with its account_id and its ip nulled: California's ARL wants
        // the record for at least three years, and `subject_hash` is what keeps it meaningful
        // without identifying. **`ip` goes with the account id** (R-C1-56, C2): an IP address is
        // personal data, it is no part of "what did the seller show that day", and the privacy
        // policy tells the reader that what survives a deletion is the hash, the price, the terms
        // version and the date — which is only true if this PATCH nulls both.
        await restPatch(rest, "consents", eq(id), { account_id: null, ip: null });
        for (
          const table of [
            "sources",
            "telemetry_events",
            "corrections",
            "issues",
            "billing_reminders",
            // C3': the account's copy of the vault, in plain text (the amendment's ruling 2). The
            // foreign key already cascades from `accounts`, but this list is what the privacy
            // policy's deletion paragraph is written from, so a table that holds the student's data
            // is named here whether or not the cascade would also reach it. `sync_usage` is the
            // account's byte counter: it holds no content, but it holds a fact about the student
            // and the same argument applies.
            "sync_records",
            "sync_notes",
            "sync_usage",
            "entitlements",
          ]
        ) {
          await restDelete(rest, table, eq(id));
        }
        await restDelete(rest, "accounts", `id=eq.${encodeURIComponent(id)}`);
      },
      deleteAuthUser: (id) => authDeleteUser(rest, id),
      tombstone: (hash, at) =>
        restUpsert(rest, "deleted_accounts", [{ email_hash: hash, deleted_at: at }], "email_hash"),
      exportAll: async (id) => {
        // Single-row reads stay `restSelect`: there is at most one `accounts`/`entitlements` row per
        // account. Everything else can run past PostgREST's own `max_rows` (1000), and an access
        // right that silently truncates at row 1000 is a bug, not a smaller export — so those five
        // page with `restSelectAll`, each ordered by its own primary key so paging is stable.
        const one = async <T>(table: string, q: string) => await restSelect<T>(rest, table, q);
        const all = async <T>(table: string, q: string) => await restSelectAll<T>(rest, table, q);
        return {
          account: (await one("accounts", `id=eq.${encodeURIComponent(id)}&select=*`))[0] ?? null,
          entitlement: (await one("entitlements", `${eq(id)}&select=*`))[0] ?? null,
          consents: await all("consents", `${eq(id)}&select=*&order=id`),
          // Kinds and dates, not the URL — and the comment now says what the code does. The access
          // right does cover the capability URL, but an export is a file that ends up in a downloads
          // folder, and a feed link is a password: the student can always re-copy it from their own
          // LMS, which is where it came from. `GET /account/sources` returns the same two columns.
          // `sources`' primary key is `(account_id, kind)`, so it orders by `kind`, not `id`.
          sources: await all("sources", `${eq(id)}&select=kind,added_at&order=kind`),
          telemetry_events: await all("telemetry_events", `${eq(id)}&select=*&order=id`),
          corrections: await all("corrections", `${eq(id)}&select=*&order=id`),
          issues: await all("issues", `${eq(id)}&select=*&order=id`),
        };
      },
      hashEmail: sha256Hex,
      getSources: async (id) => await restSelect(rest, "sources", `${eq(id)}&select=kind,added_at`),
      hasConsent: async (id) => {
        const rows = await restSelect<{ id: number }>(
          rest,
          "consents",
          `account_id=eq.${encodeURIComponent(id)}&kind=eq.tos&select=id&limit=1`,
        );
        return rows.length > 0;
      },
      recordAccountConsent: async (c) => {
        // `age_attested_at=is.null` in the filter, not just in the handler's guard: two sign-ins
        // racing each other must not restamp an attestation this account already made.
        await restPatch(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(c.account_id)}&age_attested_at=is.null`,
          {
            tos_version: c.tos_version,
            tos_accepted_at: c.at,
            privacy_version: c.privacy_version,
            age_attested_at: c.at,
          },
        );
        // `sha256Hex` lower-cases, exactly as the C1 trigger's `lower(new.email)` did, so a row
        // written here and a row written before this migration hash the same address the same way.
        const hash = await sha256Hex(c.email);
        // A plain INSERT, not an upsert: `consents.id` is a `generated always as identity` column
        // this payload never carries, so `restUpsert` (no `on_conflict`) can never find a row to
        // conflict on. Two sign-ins racing past `hasConsent` therefore write six rows, not three —
        // a check-then-act with nothing behind it. Left as is, on purpose (review finding 5): there
        // is no unique index on `(account_id, kind)` because `billing-checkout` writes its own
        // `auto_renew` row per checkout, and the app calls this route once per sign-in and retries
        // once on failure, so a duplicate identical row from a concurrent double-call is harmless —
        // it changes no entitlement decision and `hasConsent`'s `limit=1` reads past it either way.
        await restUpsert(rest, "consents", [
          { account_id: c.account_id, subject_hash: hash, kind: "tos", version: c.tos_version },
          { account_id: c.account_id, subject_hash: hash, kind: "privacy", version: c.privacy_version },
          // `'1'` — the literal the C1 trigger stamped an `age_18` row with. The attestation has no
          // document and no date to version, and the log has to read the same on both sides of this
          // migration (review M6).
          { account_id: c.account_id, subject_hash: hash, kind: "age_18", version: "1" },
        ]);
      },
      putSource: async (id, kind, url) => {
        const keyB64 = Deno.env.get("SOURCES_ENC_KEY");
        if (!keyB64) throw fail(500, "the function is not configured");
        const box = await encryptString(await importAesKey(keyB64), url);
        await restUpsert(rest, "sources", [{
          account_id: id,
          kind,
          url_ciphertext: box.ciphertext,
          url_iv: box.iv,
          added_at: new Date().toISOString(),
        }], "account_id,kind");
      },
      now: () => new Date(),
    });
  } catch (e) {
    return asResponse(e);
  }
});
