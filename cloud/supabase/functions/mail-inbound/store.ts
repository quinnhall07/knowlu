/**
 * `mail-inbound`'s store over PostgREST, apart from `index.ts` so a test can drive it over an injected
 * `rest.fetch` without serving anything, as `mail-pull`'s is (lapsed-mailbox fix, 2026-10-19: the
 * `mail_addresses` PATCH filters were copied only by the handler test's fake).
 *
 * The store logs nothing: a failed call throws `store <what> <status>`, the database's answer is
 * cancelled unread (a constraint error can echo a hash), and the handler logs only that a store call
 * failed.
 */
import { decryptString } from "../_shared/crypto.ts";
import type { Rest } from "../_shared/db.ts";
import { isActive, lookupFrom } from "../_shared/entitlement.ts";
import type { AddressRow, InboundStore } from "./handler.ts";
import { hmacKeyFromBase64 } from "./keys.ts";

/** One PostgREST call; a failure throws a fixed message and its body is never read. */
async function call(
  rest: Rest,
  what: string,
  path: string,
  init: RequestInit = {},
  prefer = init.method === "PATCH" ? "return=minimal" : undefined,
): Promise<Response> {
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
const rows = async <T>(res: Response) => await res.json() as T[];
const id = (s: string) => encodeURIComponent(s);
const HEX = /^[0-9a-f]{64}$/;

interface AddressDb {
  id: string;
  account_id: string;
  provider: AddressRow["provider"];
  status: string;
  token_hmac: string;
  retired_token_hmac: string | null;
  retired_until: string | null;
  lapsed_since: string | null;
  mailbox_hint: string | null;
  confirm_until: string | null;
  forward_hint: string | null;
}
const ADDRESS_COLUMNS = "id,account_id,provider,status,token_hmac,retired_token_hmac,retired_until," +
  "lapsed_since,mailbox_hint,confirm_until,forward_hint";

/** The store over `rest`; `encKey` (`MAIL_ENC_KEY`) opens each account's fingerprint key, and `now` stamps
 * `lapsed_since`. */
export function storeOver(rest: Rest, encKey: CryptoKey, now: () => number = Date.now): InboundStore {
  const lookup = lookupFrom(rest);
  const patch = async (what: string, query: string, body: unknown) =>
    await (await call(rest, what, `mail_addresses?${query}`, { method: "PATCH", body: JSON.stringify(body) }))
      .body?.cancel();
  const rpc = (fn: string, args: unknown) =>
    call(rest, fn, `rpc/${fn}`, { method: "POST", body: JSON.stringify(args) });
  /** A PATCH that answers how many rows it changed (their ids only). */
  const patched = async (what: string, table: string, query: string, body: unknown) => {
    const init = { method: "PATCH", body: JSON.stringify(body) };
    const res = await call(rest, what, `${table}?${query}&select=id`, init, "return=representation");
    return (await rows<{ id: string }>(res)).length;
  };

  return {
    async addressByTokenHmac(h) {
      if (!HEX.test(h)) return null;
      const q = `or=(token_hmac.eq.${h},retired_token_hmac.eq.${h})&select=${ADDRESS_COLUMNS}&limit=1`;
      const [r] = await rows<AddressDb>(await call(rest, "address", `mail_addresses?${q}`));
      if (r === undefined) return null;
      return {
        id: r.id,
        accountId: r.account_id,
        provider: r.provider,
        status: r.status,
        tokenHmac: r.token_hmac,
        retiredTokenHmac: r.retired_token_hmac,
        retiredUntil: r.retired_until,
        lapsedSince: r.lapsed_since,
        mailboxHint: r.mailbox_hint,
        confirmUntil: r.confirm_until,
        forwardHint: r.forward_hint,
      };
    },
    entitled: async (account) => isActive(await lookup(account)),
    async markLapsed(address) {
      // Status and stamp in one compare-and-set each way, never apart, so no row reads `lapsed` without
      // a stamp and an earlier stamp is kept (its gap may still be uncovered). The stamped write goes
      // first: a `covered` can clear the stamp only while the row is not lapsed, so if it clears it
      // between the two, the second write finds the row unstamped and stamps it now. An already lapsed
      // or revoked mailbox matches neither, nor does an `off` one: the student's choice outlives a lapse
      // (its mail is deleted unread meanwhile all the same), where `lapsed` would come back `quiet`.
      // Each move into `lapsed` also sets `last_lapsed_at`, so a `covered` the device sent about the gap
      // before this one no longer matches (review of the lapsed fix, #1).
      const at = new Date(now()).toISOString();
      const live = `id=eq.${id(address)}&status=not.in.(lapsed,revoked,off)`;
      const relapse = { status: "lapsed", last_lapsed_at: at };
      if (await patched("lapsed", "mail_addresses", `${live}&lapsed_since=not.is.null`, relapse) > 0) return;
      await patched("lapsed", "mail_addresses", `${live}&lapsed_since=is.null`, {
        status: "lapsed",
        lapsed_since: at,
        last_lapsed_at: at,
      });
    },
    async bindRows(address) {
      const b = await rows<{ id: string; phase: "backfill" | "recover"; nonce_hmac: string }>(
        await call(
          rest,
          "batches",
          `mail_batches?address_id=eq.${id(address)}&closed_at=is.null&select=id,phase,nonce_hmac&limit=20`,
        ),
      );
      const c = await rows<{ id: string; nonce_hmac: string; arrived_at: string | null }>(
        await call(
          rest,
          "canaries",
          `mail_canaries?address_id=eq.${
            id(address)
          }&select=id,nonce_hmac,arrived_at&order=sent_at.desc&limit=20`,
        ),
      );
      return {
        batches: b.map((x) => ({ id: x.id, phase: x.phase, nonceHmac: x.nonce_hmac, closedAt: null })),
        canaries: c.map((x) => ({
          id: x.id,
          nonceHmac: x.nonce_hmac,
          arrivedAt: x.arrived_at === null ? null : Date.parse(x.arrived_at),
        })),
      };
    },
    async fpKey(account) {
      const q = `account_id=eq.${id(account)}&select=fp_key_ciphertext,fp_key_iv&limit=1`;
      const [r] = await rows<{ fp_key_ciphertext: string; fp_key_iv: string }>(
        await call(rest, "fp key", `mail_accounts?${q}`),
      );
      if (r === undefined) throw new Error("store fp key missing");
      return await hmacKeyFromBase64(await decryptString(encKey, r.fp_key_ciphertext, r.fp_key_iv));
    },
    async seen(account, hash) {
      const q = `account_id=eq.${id(account)}&msg_hash=eq.${id(hash)}&select=msg_hash&limit=1`;
      return (await rows(await call(rest, "seen", `mail_seen?${q}`))).length > 0;
    },
    handOff: async (w, s) =>
      await (await rpc("mail_hand_off", {
        p_account: w.accountId,
        p_address: w.addressId,
        p_batch: w.batchId,
        p_s3_key: w.s3Key,
        p_phase: w.phase,
        p_authenticity: w.authenticity,
        p_msg_hash: s.msgHash,
        p_fp_hash: s.fpHash,
        p_received_minute: s.receivedMinute,
      })).json() === true,
    count: async (address, counter) => {
      // `returns void`: the answer carries no body worth reading.
      await (await rpc("count_mail_arrival", { p_address: address, p_counter: counter })).body?.cancel();
    },
    received: async (address, iso) => {
      await patch("received", `id=eq.${id(address)}`, { last_received_at: iso });
    },
    async canaryArrived(canary, address, hint, at, current) {
      // The mark is the guard: only one delivery turns `arrived_at` from null. If the mailbox's own
      // writes then fail, the mark is taken back out so SNS's retry is processed as new.
      const mine = `id=eq.${id(canary)}&address_id=eq.${id(address)}`;
      const unarrived = `${mine}&arrived_at=is.null`;
      if (await patched("canary", "mail_canaries", unarrived, { arrived_at: at }) === 0) return false;
      try {
        await patch("canary ok", `id=eq.${id(address)}`, {
          last_canary_ok_at: at,
          ...(hint === null ? {} : { forward_hint: hint }),
        });
        // Through a retired token the status stays: the forward is not re-pointed yet (D3). A `lapsed`
        // mailbox is lifted too: intake reaches a canary only for an entitled account, and the canary
        // proves the forward survived; `lapsed_since` stays for the device's `covered` (2026-10-19).
        if (current) {
          await patch(
            "active",
            `id=eq.${id(address)}&status=in.(pending,confirmed,quiet,blocked,off,lapsed)`,
            {
              status: "active",
            },
          );
        }
      } catch (e) {
        await call(rest, "canary undo", `mail_canaries?${mine}`, {
          method: "PATCH",
          body: JSON.stringify({ arrived_at: null }),
        }).then((r) => r.body?.cancel()).catch(() => {});
        throw e;
      }
      return true;
    },
    async endRotateGrace(address) {
      await patch("rotate grace", `id=eq.${id(address)}&retired_token_hmac=not.is.null`, {
        retired_token_hmac: null,
        retired_until: null,
      });
    },
    async confirmed(address) {
      await patch("confirmed", `id=eq.${id(address)}&status=eq.pending`, { status: "confirmed" });
      await patch("confirm window", `id=eq.${id(address)}`, { confirm_until: null });
    },
    async batchReceived(batch, address, n) {
      // PostgREST has no increment: read, then write only if nobody else wrote between (a concurrent
      // send of the same batch), a few times over.
      const mine = `id=eq.${id(batch)}&address_id=eq.${id(address)}`;
      for (let tries = 0; tries < 8; tries++) {
        const [b] = await rows<{ received: number }>(
          await call(rest, "batch", `mail_batches?${mine}&select=received&limit=1`),
        );
        if (b === undefined) return; // pruned or disconnected: nothing left to count toward
        const unchanged = `${mine}&received=eq.${b.received}`;
        if (await patched("batch received", "mail_batches", unchanged, { received: b.received + n }) > 0) {
          return;
        }
      }
      throw new Error("store batch received contended");
    },
  };
}
