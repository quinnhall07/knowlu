// The gate's third condition (e2-brief.md, e2-report note §2 item 3): findings naming a live
// secret, a token, a session or a credential are DROPPED from the corpus, not redacted. This is a
// mechanical, auditable policy rather than a per-item judgment call: any finding whose body
// mentions one of the listed concepts — even generically, even inside a snake_case identifier —
// is dropped outright, because the cheapest way to honour "no secret in a prompt" is to not send
// the finding at all.

// Every pattern ends the last word with `s?` so the plain plural (or, for "id", "ids") of the term
// matches too — fix round 1, finding 1 (Important): the binding rule ("drop, never redact, any
// finding that names a secret, a token or a session") is applied mechanically, so a plural mention
// (B-final-F2's kept text: "the port and the tokens") must be caught exactly like the singular, not
// left to a human to notice case by case. None of these words pluralise irregularly (no -es, no
// -ies), so appending `s?` is the whole fix — it is not a general English-plural rule.
const SECRET_TERMS: RegExp[] = [
  /\bsessions?\b/gi,
  /\bjwts?\b/gi,
  /\banon keys?\b/gi,
  /\banon_keys?\b/gi,
  /\bcredential managers?\b/gi,
  /\bcredentials?\b/gi,
  /\bsecrets?\b/gi,
  /\btokens?\b/gi,
  /\bclient_ids?\b/gi,
  /\bclient ids?\b/gi,
  /\bapi keys?\b/gi,
];

/**
 * Normalises underscores and hyphens to spaces before scanning, so a term embedded in a
 * snake_case test name (e.g. `a_six_digit_code_from_the_email_becomes_a_session_on_this_machine`)
 * still counts as naming what it names — the identifier is English with underscores for spaces.
 */
function normalizeForScan(text: string): string {
  return text.replace(/[_-]/g, " ");
}

/** Returns the distinct matched terms (lowercased) if `text` names a secret/token/session/credential. */
export function secretMentionTerms(text: string): string[] {
  const normalized = normalizeForScan(text);
  const hits = new Set<string>();
  for (const pattern of SECRET_TERMS) {
    const re = new RegExp(pattern.source, pattern.flags);
    let m: RegExpExecArray | null;
    while ((m = re.exec(normalized)) !== null) {
      hits.add(m[0].toLowerCase());
    }
  }
  return [...hits].sort();
}

export function mentionsSecret(text: string): boolean {
  return secretMentionTerms(text).length > 0;
}
