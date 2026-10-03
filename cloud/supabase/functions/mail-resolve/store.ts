/**
 * `mail-resolve`'s store over PostgREST and its charges over the judgment tables (email plan T4.5), apart
 * from `index.ts` so a test can drive them over an injected `fetch` or `Db` without serving anything.
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is cancelled
 * unread, and `asResponse` answers a bare 500.
 */
import type { Rest } from "../_shared/db.ts";
import { MAIL_DAILY_CAP, MONTHLY_CEILING_USD } from "../_shared/judge_caps.ts";
import type { Db } from "../_shared/judge_db.ts";
import type { ClaimRowRef, ResolveCharges, ResolveStore } from "./handler.ts";

/** One PostgREST call; a failure throws a fixed message and its body is never read. */
async function call(rest: Rest, what: string, path: string, init: RequestInit = {}, prefer?: string) {
  const res = await rest.fetch(`${rest.url}/rest/v1/${path}`, {
    ...init,
    headers: {
      apikey: rest.serviceKey,
      authorization: `Bearer ${rest.serviceKey}`,
      "content-type": "application/json",
      ...(prefer === undefined ? {} : { prefer }),
    },
  });
  if (!res.ok) {
    await res.body?.cancel();
    throw new Error(`store ${what} ${res.status}`);
  }
  return res;
}
const id = (s: string) => encodeURIComponent(s);

export function storeOver(rest: Rest): ResolveStore {
  const account = (accountId: string) => `account_id=eq.${id(accountId)}`;
  return {
    async rows(accountId, uids) {
      if (uids.length === 0) return [];
      // The uids are D20's shape (checked by the handler), so the list needs no quoting.
      const q = `${account(accountId)}&uid=in.(${uids.map(id).join(",")})&select=id,uid,claims&order=id.asc`;
      return await (await call(rest, "rows", `mail_claims?${q}`)).json() as ClaimRowRef[];
    },
    async answer(accountId, rowId, open, claims) {
      // The compare-and-set: the write lands only while each claim it fills still has no resolution
      // (`->>` reads a JSON null and a missing key alike as SQL null). Answers are only ever added, so a
      // claim still unanswered means the row is as it was read.
      const unanswered = open.map((i) => `claims->${Math.trunc(i)}->>resolution=is.null`).join("&");
      const q = `id=eq.${Math.trunc(rowId)}&${account(accountId)}&${unanswered}&select=id`;
      const res = await call(
        rest,
        "answer",
        `mail_claims?${q}`,
        { method: "PATCH", body: JSON.stringify({ claims }) },
        "return=representation",
      );
      return (await res.json() as unknown[]).length === 1;
    },
  };
}

/** The pipeline's guards for `mail_resolve`. Each that cannot reach the database refuses, as `capStore`
 * does: an unreachable cap is no licence to call a metered model. */
export function resolveCharges(db: Db): ResolveCharges {
  const yes = async (call: Promise<unknown>) => {
    try {
      return await call === true;
    } catch {
      return false;
    }
  };
  return {
    withinBudget: (account) =>
      yes(db.rpc("enforce_budget", { p_account: account, p_ceiling: MONTHLY_CEILING_USD })),
    charge: (account) =>
      yes(
        db.rpc("charge_call", {
          p_account: account,
          p_kind: "mail_resolve",
          p_cap: MAIL_DAILY_CAP.mail_resolve,
        }),
      ),
    async recordTokens(account, inTokens, outTokens) {
      try {
        await db.rpc("record_tokens", {
          p_account: account,
          p_kind: "mail_resolve",
          p_in: inTokens,
          p_out: outTokens,
        });
      } catch {
        // As `capStore.recordTokens`: the next `enforce_budget` still reads every call that did record.
      }
    },
  };
}
