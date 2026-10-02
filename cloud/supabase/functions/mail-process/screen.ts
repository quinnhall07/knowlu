/**
 * §4.7 step 2, the screen (email plan T3d): four rules, no model, in order; the first that fires decides
 * and the item is `noise`. Nothing here logs, and nothing of the message leaves it but the reason.
 *
 * 1. **Bulk or list mail** (`List-Unsubscribe`, or `Precedence: bulk` or `list`) from a sender off the
 *    allow-list: any `.edu`, the school's own domain, and the LMS and homework-platform vendors' domains.
 *    The server never holds the campus row's `mail_domain` (it is the device's), so the school's domain is
 *    the domain of the account's own mailboxes, less Google's personal-mail domains (personal Gmail is
 *    not a school). The vendors' domains are the template rows' senders' (data, never named here).
 * 2. **Microsoft's bulk score** (`BCL` in `X-Microsoft-Antispam`, the topmost one), where it exists, at 7
 *    or above.
 * 3. **The account's promoted rules** (`judge_rules.ts`, tier 2) answering noise: an `email` rule, keyed on
 *    the sender as `gmail-read` keys it, whose tier is `information` (the email tiers' noise).
 * 4. **The student's own mail coming back**: From is this mailbox's own address, outside a batch. Only
 *    this mailbox: mail from another of the account's mailboxes passes (PQ8 (ii) only; the device catches
 *    that duplicate, T5.6).
 */
import type { RuleTable } from "../_shared/judge_rules.ts";
import type { MailTemplate } from "../_shared/mail_templates.ts";
import type { MailHeaders } from "../_shared/mime.ts";

export type NoiseReason = "bulk" | "bulk score" | "rule" | "own mail";
/** Microsoft's bulk complaint level at which mail is noise. */
export const BULK_SCORE_NOISE = 7;
/** Google's personal-mail domains: a mailbox there is never the school's (provider-level, no school). */
const PERSONAL_DOMAINS: readonly string[] = ["gmail.com", "googlemail.com"];

export interface ScreenMailbox {
  mailboxHint: string | null;
}
export interface ScreenInput {
  headers: MailHeaders;
  /** The From header's value, decoded: the key a promoted email rule is looked up by. */
  from: string;
  accountId: string;
  /** `live`, or a batch's phase. */
  phase: "live" | "backfill" | "recover";
  /** The mailbox the item arrived on. */
  mailbox: ScreenMailbox;
  /** `allowDomains` for the account. */
  allow: readonly string[];
}

/** The domain of `local@domain`, lower-cased; null without one. */
export function domainOf(address: string | null): string | null {
  if (address === null) return null;
  const at = address.lastIndexOf("@");
  const domain = at < 0 ? "" : address.slice(at + 1).trim().toLowerCase().replace(/\.$/, "");
  return domain === "" ? null : domain;
}

/** The allow-list's named domains: the account's mailboxes' (less personal Gmail) and the vendors'. */
export function allowDomains(
  mailboxes: readonly ScreenMailbox[],
  templates: readonly MailTemplate[],
): string[] {
  const out = new Set<string>();
  for (const m of mailboxes) {
    const d = domainOf(m.mailboxHint);
    if (d !== null && !PERSONAL_DOMAINS.includes(d)) out.add(d);
  }
  for (const t of templates) {
    for (const s of t.senders) {
      const d = domainOf(s);
      if (d !== null) out.add(d);
    }
  }
  return [...out].sort();
}

/** On the allow-list: any `.edu`, or a named domain or a subdomain of one. */
export function allowed(address: string | null, allow: readonly string[]): boolean {
  const d = domainOf(address);
  if (d === null) return false;
  return d.endsWith(".edu") || allow.some((a) => d === a || d.endsWith(`.${a}`));
}

function bulk(h: MailHeaders): boolean {
  return "list-unsubscribe" in h.list || h.precedence === "bulk" || h.precedence === "list";
}

/** The topmost `X-Microsoft-Antispam`'s `BCL`; null where there is none. */
export function bulkScore(h: MailHeaders): number | null {
  const field = h.headers.find((f) => f.name.trim().toLowerCase() === "x-microsoft-antispam");
  const m = field === undefined ? null : /(?:^|;)\s*BCL:(\d{1,2})\s*(?:;|$)/i.exec(field.value);
  return m === null ? null : Number(m[1]);
}

/** Rule 3: an `email` rule for this sender whose tier is `information`. A rule table that cannot be read
 * answers nothing, as tier 2 does on every path. */
async function ruleSaysNoise(input: ScreenInput, rules: RuleTable): Promise<boolean> {
  if (input.from.trim() === "") return false;
  try {
    const verdict = await rules.lookup(input.accountId, "email", { from: input.from });
    return verdict?.tier === "information";
  } catch {
    return false;
  }
}

/** The four rules in order: the first that fires, or null when the item goes on. */
export async function screen(input: ScreenInput, rules: RuleTable): Promise<NoiseReason | null> {
  const from = input.headers.fromAddress;
  if (bulk(input.headers) && !allowed(from, input.allow)) return "bulk";
  const score = bulkScore(input.headers);
  if (score !== null && score >= BULK_SCORE_NOISE) return "bulk score";
  if (await ruleSaysNoise(input, rules)) return "rule";
  const own = input.mailbox.mailboxHint?.trim().toLowerCase() ?? null;
  if (input.phase === "live" && from !== null && own !== null && own !== "" && from === own) {
    return "own mail";
  }
  return null;
}
