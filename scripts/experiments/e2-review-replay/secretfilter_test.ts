import { assertEquals } from "@std/assert";
import { mentionsSecret, secretMentionTerms } from "./secretfilter.ts";

Deno.test("mentionsSecret is false over ordinary review prose", () => {
  assertEquals(
    mentionsSecret(
      "the deadline is not checked on the path where connections keep arriving indefinitely",
    ),
    false,
  );
});

Deno.test("mentionsSecret catches a plain word mention", () => {
  assertEquals(mentionsSecret("a same-user process can already read knowlu/pending/session"), true);
  assertEquals(mentionsSecret("out of Credential Manager"), true);
  assertEquals(mentionsSecret("the public anon key"), true);
  assertEquals(mentionsSecret('`secret = "env(GOOGLE_SECRET)"`'), true);
});

Deno.test("mentionsSecret catches a term embedded in a snake_case identifier", () => {
  assertEquals(
    mentionsSecret("`a_six_digit_code_from_the_email_becomes_a_session_on_this_machine`"),
    true,
  );
});

Deno.test("secretMentionTerms names what it matched, for the audit trail, never the surrounding text", () => {
  const terms = secretMentionTerms("the challenge is not secret, and a_session_on_this_machine");
  assertEquals(terms, ["secret", "session"]);
});

// Fix round 1, finding 1 (Important): every SECRET_TERMS pattern was `\bTERM\b` on the singular
// only, so a plural/inflected mention (as B-final-F2's kept text has: "the port and the tokens")
// slipped past the filter. The binding rule — "drop, never redact, any finding that names a
// secret, a token or a session" — is applied mechanically, so a plural must be caught exactly like
// the singular, not judged case by case.
Deno.test("mentionsSecret catches the plain plural of every single-word term", () => {
  assertEquals(mentionsSecret("about the port and the tokens, not the attestation"), true);
  assertEquals(mentionsSecret("two sessions raced past the guard"), true);
  assertEquals(mentionsSecret("no secrets, coupon ids or percentages"), true);
  assertEquals(mentionsSecret("the three credentials never left the device"), true);
});

Deno.test("mentionsSecret catches the plain plural of every multi-word term", () => {
  assertEquals(mentionsSecret("both anon keys were rotated on the same day"), true);
  assertEquals(mentionsSecret("neither credential managers entry was touched"), true);
  assertEquals(mentionsSecret("both client ids are compiled in, never fetched"), true);
  assertEquals(mentionsSecret("rotate the api keys before the pilot"), true);
});

Deno.test("mentionsSecret catches a plural term embedded in a snake_case identifier", () => {
  assertEquals(mentionsSecret("`every_route_needs_a_bearer_tokens_check`"), true);
});

Deno.test("secretMentionTerms reports the matched plural form, not a normalised singular", () => {
  const terms = secretMentionTerms("about the port and the tokens, not the attestation");
  assertEquals(terms, ["tokens"]);
});
