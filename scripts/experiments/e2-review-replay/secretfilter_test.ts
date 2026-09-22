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
