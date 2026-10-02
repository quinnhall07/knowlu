/**
 * Which provider hosts a mail domain (spec §4.1 step 1 and §13 Q4; Quinn's per-school ruling PS: a curated
 * campus row's `mail_provider` wins on the device, else this MX reading). Nothing here names a school.
 *
 * - `gmail.com` and `googlemail.com` are Google's own, and are never looked up.
 * - Any other domain is read by its MX records, the most preferred first: an exchange under
 *   `mail.protection.outlook.com` (or Microsoft's newer `mx.microsoft`) is `m365`, one of Google's MX hosts
 *   is `google`, anything else is `other`, and no MX at all is `null` (nothing can be sent there).
 *
 * T4.1's hint check (`a_hint_is_set_once_and_must_fit_the_providers_domains`) and T4.2's `GET /provider`
 * and `POST /canary` read this one table. The resolver is injected, so no test queries DNS.
 */
export type MailProvider = "m365" | "google" | "other";
export const PROVIDERS: readonly MailProvider[] = ["m365", "google", "other"];

/** A domain's MX exchanges, most preferred first, lower-case with no trailing dot; `[]` when none. Throws
 * when DNS could not answer (a timeout, a server failure), which is not the same as no MX. */
export type ResolveMx = (domain: string) => Promise<string[]>;

/** The two Gmail domains (§4.10's "`gmail.com` or `googlemail.com` for Gmail"). */
export const GMAIL_DOMAINS: readonly string[] = ["gmail.com", "googlemail.com"];

const DNS_TIMEOUT_MS = 5_000;
/** A lower-case dotted domain of letters, digits and hyphens. */
export const DOMAIN = /^(?=.{4,253}$)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/;

const under = (host: string, suffix: string) => host === suffix || host.endsWith(`.${suffix}`);

/** The provider one MX exchange belongs to. */
export function providerOfMx(host: string): MailProvider {
  const h = host.trim().toLowerCase().replace(/\.$/, "");
  if (under(h, "mail.protection.outlook.com") || under(h, "mx.microsoft")) return "m365";
  if (h === "smtp.google.com" || under(h, "aspmx.l.google.com") || under(h, "googlemail.com")) {
    return "google";
  }
  return "other";
}

/** The provider hosting `domain`, or `null` when it has no MX. Throws when DNS could not answer. */
export async function providerOf(domain: string, resolveMx: ResolveMx): Promise<MailProvider | null> {
  const d = domain.trim().toLowerCase();
  if (GMAIL_DOMAINS.includes(d)) return "google";
  if (!DOMAIN.test(d)) return null;
  const hosts = await resolveMx(d);
  return hosts.length === 0 ? null : providerOfMx(hosts[0]);
}

/** Production's resolver: `Deno.resolveDns`'s MX records, sorted by preference. No such name, or no MX
 * records, is `[]`; any other failure throws a bare error naming neither the domain nor DNS's answer. */
export function mxResolver(timeoutMs = DNS_TIMEOUT_MS): ResolveMx {
  return async (domain: string) => {
    try {
      const records = await Deno.resolveDns(domain, "MX", { signal: AbortSignal.timeout(timeoutMs) });
      return [...records]
        .sort((a, b) => a.preference - b.preference)
        .map((r) => r.exchange.toLowerCase().replace(/\.$/, ""))
        .filter((h) => h !== "");
    } catch (e) {
      if (e instanceof Deno.errors.NotFound) return [];
      throw new Error("dns");
    }
  };
}
