/**
 * The wire for `mail` (email plan T4.1): the environment, the store over PostgREST, S3 (delete only, here)
 * and DNS, then `handler.ts`. `verify_jwt = false` (config.toml): the handler authenticates every request
 * itself, a session for the list and the delete and a session plus entitlement for the rest (§6.3).
 * Secrets, named only (spec §5.1): `MAIL_DOMAIN`, `MAIL_ADDRESS_KEY`, `MAIL_ENC_KEY`, `MAIL_BUCKET`,
 * `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`; and for the canary (T4.2, plan P9) the
 * project's existing `EMAIL_API_URL` and `EMAIL_API_KEY`, the Resend endpoint and key `billing-jobs` uses.
 *
 * The store over PostgREST is `store.ts`'s, which logs nothing.
 */
import { requireUser } from "../_shared/auth.ts";
import { decryptString, encryptString, importAesKey } from "../_shared/crypto.ts";
import { authGetUser, restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { canarySender } from "../_shared/canary_send.ts";
import { s3Client } from "../_shared/s3.ts";
import { hmacKeyFromBase64, nonceHmac, tokenHmac } from "../mail-inbound/keys.ts";
import { type MailDeps, mailHandler } from "./handler.ts";
import { mxResolver } from "./provider.ts";
import { storeOver } from "./store.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

async function wire(): Promise<MailDeps> {
  const rest = restFromEnv();
  const addressKey = await hmacKeyFromBase64(env("MAIL_ADDRESS_KEY"));
  const encKey = await importAesKey(env("MAIL_ENC_KEY"));
  const s3 = s3Client({
    bucket: env("MAIL_BUCKET"),
    region: env("AWS_REGION"),
    accessKeyId: env("AWS_ACCESS_KEY_ID"),
    secretAccessKey: env("AWS_SECRET_ACCESS_KEY"),
  }, { fetch: globalThis.fetch });
  return {
    authenticate: async (req) => ({ account_id: (await requireUser(req, (t) => authGetUser(rest, t))).id }),
    entitle: requireActiveEntitlement,
    store: storeOver(rest),
    s3,
    tokenHmac: (t) => tokenHmac(addressKey, t),
    encrypt: (s) => encryptString(encKey, s),
    decrypt: (c, iv) => decryptString(encKey, c, iv),
    mailDomain: env("MAIL_DOMAIN").trim().toLowerCase(),
    now: () => Date.now(),
    resolveMx: mxResolver(),
    nonceHmac: (n) => nonceHmac(addressKey, n),
    // The provider `billing-jobs` sends through (P9): its endpoint and key, named only.
    sendCanary: canarySender({
      url: env("EMAIL_API_URL"),
      key: env("EMAIL_API_KEY"),
      fetch: globalThis.fetch,
    }),
  };
}

let wired: Promise<(req: Request) => Promise<Response>> | null = null;

Deno.serve(async (req) => {
  try {
    wired ??= wire().then(mailHandler);
    const handler = await wired.catch((e) => {
      wired = null;
      throw e;
    });
    return await handler(req);
  } catch (e) {
    return asResponse(e);
  }
});
