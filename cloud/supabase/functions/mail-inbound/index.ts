/**
 * The wire for `mail-inbound` (email plan T2g.1): the environment, the store over PostgREST, S3, SNS and
 * DNS, then `handler.ts`. `verify_jwt = false` (config.toml): SNS sends no bearer, and its signature is
 * the check. Secrets, named only (spec §5.1): `MAIL_DOMAIN`, `MAIL_ADDRESS_KEY`, `MAIL_ENC_KEY`,
 * `MAIL_BUCKET`, `MAIL_SNS_TOPIC_ARN`, `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`; and
 * `MAIL_JOBS_TOKEN` for D8's kick (T3d).
 *
 * The other outbound fetches are Gmail's confirmation (`confirm.ts`, T2g.2), which reaches only a link,
 * at most three redirect targets and a form action on `mail_bind.ts`'s fixed Google allow-list, and D8's
 * kick to this project's own `mail-process` (T3d).
 *
 * The store over PostgREST is `store.ts`'s, which logs nothing.
 */
import { importAesKey } from "../_shared/crypto.ts";
import { type Rest, restFromEnv } from "../_shared/db.ts";
import { dnsTxtResolver } from "../_shared/dkim.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { s3Client } from "../_shared/s3.ts";
import { mxResolver } from "../mail/provider.ts";
import { handle, type InboundConfig, type InboundDeps, type Kick, MAIL_CANARY_FROM } from "./handler.ts";
import { hmacKeyFromBase64, nonceHmac, tokenHmac } from "./keys.ts";
import { storeOver } from "./store.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

/** D8's kick (plan T3d): `mail-process` at this project's own functions URL, with `MAIL_JOBS_TOKEN`, the
 * token the cron sends. Run after the answer through `EdgeRuntime.waitUntil` where the runtime has it.
 * Without the token there is no kick, and the every-minute cron alone starts `mail-process`. */
function kickFromEnv(rest: Rest): Kick | undefined {
  const token = Deno.env.get("MAIL_JOBS_TOKEN");
  if (!token) return undefined;
  const runtime = (globalThis as { EdgeRuntime?: { waitUntil?(p: Promise<unknown>): void } }).EdgeRuntime;
  const waitUntil = runtime?.waitUntil?.bind(runtime);
  return {
    url: `${rest.url.replace(/\/+$/, "")}/functions/v1/mail-process`,
    token,
    fetch: (url, init) => fetch(url, init),
    ...(waitUntil === undefined ? {} : { waitUntil }),
  };
}

async function wire(): Promise<{ cfg: InboundConfig; deps: InboundDeps }> {
  const rest = restFromEnv();
  const addressKey = await hmacKeyFromBase64(env("MAIL_ADDRESS_KEY"));
  const encKey = await importAesKey(env("MAIL_ENC_KEY"));
  const cfg: InboundConfig = {
    topicArn: env("MAIL_SNS_TOPIC_ARN"),
    mailDomain: env("MAIL_DOMAIN"),
    bucket: env("MAIL_BUCKET"),
    canaryFrom: MAIL_CANARY_FROM,
  };
  const s3 = s3Client({
    bucket: cfg.bucket,
    region: env("AWS_REGION"),
    accessKeyId: env("AWS_ACCESS_KEY_ID"),
    secretAccessKey: env("AWS_SECRET_ACCESS_KEY"),
  }, { fetch: globalThis.fetch });

  const deps: InboundDeps = {
    sns: { fetch: globalThis.fetch },
    s3,
    bind: {
      resolveTxt: dnsTxtResolver(),
      resolveMx: mxResolver(),
      nonceHmac: (n) => nonceHmac(addressKey, n),
    },
    tokenHmac: (t) => tokenHmac(addressKey, t),
    now: () => Date.now(),
    store: storeOver(rest, encKey),
    confirmFetch: (url, init) => fetch(url, init),
    kick: kickFromEnv(rest),
  };
  return { cfg, deps };
}

let wired: Promise<{ cfg: InboundConfig; deps: InboundDeps }> | null = null;

Deno.serve(async (req) => {
  try {
    wired ??= wire();
    const { cfg, deps } = await wired.catch((e) => {
      wired = null;
      throw e;
    });
    return await handle(req, cfg, deps);
  } catch (e) {
    return asResponse(e);
  }
});
