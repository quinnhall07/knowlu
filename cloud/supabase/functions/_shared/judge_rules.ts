import type { Db } from "./judge_db.ts";
import type { Kind } from "./judge_validate.ts";

/// Tier 2. A `null` means "no rule answered"; the pipeline then reaches the model.
export interface RuleTable {
  lookup(account: string, kind: Kind, item: Record<string, unknown>): Promise<Record<string, unknown> | null>;
}

function s(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

/// The first three words of a title. **The promotion key, and the only part of a title that ever
/// leaves this function** — it is already the note's filename, so it is not new exposure.
export function titlePrefix(title: string): string {
  return s(title).split(/\s+/).slice(0, 3).join(" ");
}

/// The one field an email item is keyed on: **who sent it**.
///
/// C2 final review S-2: tier 2 was dead for `kind: "email"`. An email item carries `message_id`,
/// `subject`, `from`, `date` and `text` — no `title`, no `organizer`, no `source`, no `series_uid`
/// — so `features("email", …)` returned an empty array for every message ever judged, `lookup`
/// short-circuited to `null`, and no email rule could be looked up even if the promotion job had
/// managed to write one. The sender is mapped into the EXISTING `source` feature rather than a new
/// one, so `rules.feature`'s check constraint is unchanged and no migration is needed: "always
/// drop mail from the bursar's no-reply address" is exactly the shape tier 2 exists to promote.
///
/// **What is deliberately NOT a feature: the subject text.** A title prefix is a feature for tasks
/// and events because it is already the note's filename — public in the vault either way. An email
/// subject is not: it is message content, it never reaches `judgments.fields` (the privacy
/// tripwire in `judge_pipeline_test.ts` asserts that directly), and keying a promoted rule on it
/// would put a fragment of someone's mail in a `rules` row that outlives the message. The `date`
/// and the `message_id` are not features either, for the plainer reason that neither ever repeats.
function emailSender(item: Record<string, unknown>): string {
  return s(item.from);
}

/// The promotion features of §5.4 measure 1, extracted from an item, most specific first. Shared
/// with the nightly promotion job (through `judgments.fields`, which `featureMap` fills) so a rule
/// is always looked up by the same key it was promoted on.
export function features(kind: Kind, item: Record<string, unknown>): Array<[string, string]> {
  const prefix = titlePrefix(s(item.title));
  if (kind === "task") {
    const out: Array<[string, string]> = [];
    if (s(item.created_by) !== "" && prefix !== "") {
      out.push(["created_by+title_prefix", `${s(item.created_by)}|${prefix}`]);
    }
    if (prefix !== "") out.push(["title_prefix", prefix]);
    return out;
  }
  const out: Array<[string, string]> = [];
  if (s(item.organizer) !== "") out.push(["organizer", s(item.organizer)]);
  // S-2: the sender, under the existing `source` key — the only feature an email item has.
  const sender = kind === "email" ? emailSender(item) : s(item.source);
  if (sender !== "") out.push(["source", sender]);
  if (s(item.series_uid) !== "") out.push(["series", s(item.series_uid)]);
  if (prefix !== "") out.push(["title_prefix", prefix]);
  return out;
}

/// The same five values, flat, for `judgments.fields` — which is where `promote_rules` reads them
/// from, because the row carries no title and no body to recompute them from. For `kind: "email"`
/// that is the sender alone, under `source` (S-2); an email's subject is never a feature.
export function featureMap(kind: Kind, item: Record<string, unknown>): Record<string, string> {
  const out: Record<string, string> = {};
  const prefix = titlePrefix(s(item.title));
  if (prefix !== "") out.title_prefix = prefix;
  if (kind === "task") {
    if (s(item.created_by) !== "") out.created_by = s(item.created_by);
    return out;
  }
  if (s(item.organizer) !== "") out.organizer = s(item.organizer);
  // S-2: the same mapping `features` makes, so a rule is promoted on the key it is looked up on.
  const sender = kind === "email" ? emailSender(item) : s(item.source);
  if (sender !== "") out.source = sender;
  if (s(item.series_uid) !== "") out.series_uid = s(item.series_uid);
  return out;
}

/// The account's own active rules, then the hand-reviewed global ones (§11 R5). First feature that
/// matches wins, in the order `features` returns them — most specific first.
export function ruleTable(db: Db): RuleTable {
  return {
    async lookup(account, kind, item) {
      const pairs = features(kind, item);
      if (pairs.length === 0) return null;
      let rows: Array<
        { feature: string; value: string; verdict: Record<string, unknown>; account_id: string | null }
      >;
      try {
        rows = await db.select(
          `rules?kind=eq.${kind}&active=is.true&or=(account_id.eq.${account},account_id.is.null)&select=feature,value,verdict,account_id`,
        ) as typeof rows;
      } catch {
        // A rule table that cannot be read is not an error: the model answers instead, which is
        // what it did before any rule existed.
        return null;
      }
      for (const [feature, value] of pairs) {
        const own = rows.find((r) => r.feature === feature && r.value === value && r.account_id === account);
        if (own !== undefined) return own.verdict;
        const global = rows.find((r) => r.feature === feature && r.value === value && r.account_id === null);
        if (global !== undefined) return global.verdict;
      }
      return null;
    },
  };
}
