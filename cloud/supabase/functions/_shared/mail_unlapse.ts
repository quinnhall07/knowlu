/**
 * How a mailbox leaves `lapsed` (email spec §4.6 step 3, amended 2026-10-19), shared by the two entitled
 * readers that can meet one first after a renewal: `mail-pull` and `gmail-read` (review of the lapsed fix,
 * #2: the judge step calls `gmail-read` before `mail-pull`, and a forwarding Gmail mailbox read `lapsed`
 * there would be read by OAuth and then recovered by forwarding, filed twice).
 *
 * Two compare-and-sets on `status=eq.lapsed`, split on whether a canary ever arrived: `quiet` when one did
 * (`last_canary_ok_at` set), `pending` when none did; never `active`, since nothing proves the forward
 * survived the lapse. Neither names `lapsed_since`: only the device's `covered` report clears it. A row intake lapses again between the two is simply lapsed; the next reader moves it.
 * Both are idempotent, so the two readers racing is harmless.
 *
 * Only an entitled caller may call this. It logs nothing; a failed call throws `store unlapse <status>`
 * with the database's answer cancelled unread.
 */
import type { Rest } from "./db.ts";

export async function unlapseAccount(rest: Rest, accountId: string): Promise<void> {
  const lapsed = `mail_addresses?account_id=eq.${encodeURIComponent(accountId)}&status=eq.lapsed`;
  for (const [ok, status] of [["not.is.null", "quiet"], ["is.null", "pending"]]) {
    const res = await rest.fetch(`${rest.url}/rest/v1/${lapsed}&last_canary_ok_at=${ok}`, {
      method: "PATCH",
      body: JSON.stringify({ status }),
      headers: {
        apikey: rest.serviceKey,
        authorization: `Bearer ${rest.serviceKey}`,
        "content-type": "application/json",
        prefer: "return=minimal",
      },
    });
    await res.body?.cancel();
    if (!res.ok) throw new Error(`store unlapse ${res.status}`);
  }
}
