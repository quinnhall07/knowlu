/**
 * The delivery check's one message (email spec §4.10; plan P9, T4.2): sent through Resend's HTTP API, the
 * provider that already sends Knowlu's account mail (`billing-jobs` and the sign-in code), from a fixed
 * address on Knowlu's verified domain, to the mailbox's own address (its `mailbox_hint`). The mailbox's
 * forwarding brings it back to intake, which knows it by that sender, its DKIM, and the nonce its signed
 * Subject carries (`mail_bind.ts`'s `nonceTag`). The nonce also rides in one header.
 *
 * `mail` (T4.2's `POST /canary`) and `mail-canary` (T4.3's periodic check) send through this one file.
 * **Nothing here logs.** A refused send throws a bare error naming only the provider's status: never the
 * address, the nonce or the key. The response body is cancelled unread.
 */
import { nonceTag } from "./mail_bind.ts";

/** The canary's sender. Intake's `MAIL_CANARY_FROM` is the same address, and a test holds them equal. */
export const CANARY_FROM = "canary@knowlu.com";
export const CANARY_SUBJECT = "Knowlu delivery check — no need to open this";
/** The header that carries the nonce beside the Subject. */
export const CANARY_HEADER = "X-Knowlu-Check";
/** One sentence: what it is, and where to turn it off. */
export const CANARY_TEXT = "Knowlu sent this to check that your email forwarding still reaches it, " +
  "and you can turn email off in Knowlu under Settings → Email.";

const NONCE = /^[0-9a-f]{32}$/;
const SEND_TIMEOUT_MS = 10_000;

/** The JSON body Resend's `POST /emails` takes. */
export interface CanaryMail {
  from: string;
  to: string[];
  subject: string;
  text: string;
  headers: Record<string, string>;
}

/** The canary to `to` carrying `nonce` (32 lower-case hex digits, `mail_bind.ts`'s `newNonce`). */
export function canaryMail(to: string, nonce: string): CanaryMail {
  if (!NONCE.test(nonce)) throw new Error("canary: not a nonce");
  return {
    from: `Knowlu <${CANARY_FROM}>`,
    to: [to],
    subject: `${CANARY_SUBJECT} ${nonceTag(nonce)}`,
    text: CANARY_TEXT,
    headers: { [CANARY_HEADER]: nonce },
  };
}

export interface CanaryConfig {
  /** `EMAIL_API_URL`, Resend's `/emails` endpoint (the one `billing-jobs` posts to). */
  url: string;
  /** `EMAIL_API_KEY`. */
  key: string;
  fetch: typeof fetch;
  timeoutMs?: number;
}
export type SendCanary = (to: string, nonce: string) => Promise<void>;

/** A sender over Resend. Resolves once Resend accepted the message; throws `canary send <status>` else. */
export function canarySender(cfg: CanaryConfig): SendCanary {
  return async (to, nonce) => {
    const body = JSON.stringify(canaryMail(to, nonce));
    const abort = new AbortController();
    const timer = setTimeout(() => abort.abort(), cfg.timeoutMs ?? SEND_TIMEOUT_MS);
    let res: Response;
    try {
      res = await cfg.fetch(cfg.url, {
        method: "POST",
        headers: { authorization: `Bearer ${cfg.key}`, "content-type": "application/json" },
        body,
        signal: abort.signal,
      });
    } catch {
      throw new Error("canary send failed");
    } finally {
      clearTimeout(timer);
    }
    await res.body?.cancel();
    if (!res.ok) throw new Error(`canary send ${res.status}`);
  };
}
