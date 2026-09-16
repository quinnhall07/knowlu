import { assert, assertEquals } from "@std/assert";
import { scrub, scrubForPrompt, scrubJson } from "./scrub.ts";

Deno.test("the seven things a report must never carry", () => {
  // An address — somebody else's, usually.
  assertEquals(scrub("mailed a.student@crimson.ua.edu twice"), "mailed <email> twice");
  // A capability URL: the LMS feed link is a password with a scheme in front of it.
  assertEquals(
    scrub("fetching https://lms.example.invalid/feed/abc123.ics failed"),
    "fetching <url> failed",
  );
  assertEquals(scrub("http://10.0.0.1/x"), "<url>");
  // A bearer token, and a JWT in particular. Fix round 1, item 3: the credential rule now claims
  // the whole `authorization: Bearer …` header as one match (the scheme name is as much a fact
  // about the secret as the JWT itself), so this is `<secret>` rather than the plain `<token>` a
  // bare opaque-run match used to leave here.
  assertEquals(
    scrub("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"),
    "authorization=<secret>",
  );
  // A note's filename is its title, slugified.
  assertEquals(scrub("1 unreadable: read-chapter-3-of-calculus.md)"), "1 unreadable: <note>)");
  // A long opaque run of key-ish characters. Fix round 1b: `key` joined the credential keyword
  // list, so this is claimed by CREDENTIAL before TOKEN ever sees it — `<secret>`, not `<token>`.
  assertEquals(scrub("key=sk_live_51Hxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), "key=<secret>");
  // The Windows account name, which is usually a person's name.
  assertEquals(
    scrub("could not open C:\\Users\\Ada\\Knowlu\\Fall 2026\\tasks\\a.md"),
    "could not open C:\\Users\\<user>\\Knowlu\\Fall 2026\\tasks\\<note>",
  );
  // …and ordinary prose survives intact.
  assertEquals(scrub("the run at 12:00 exited 1 after 4 steps"), "the run at 12:00 exited 1 after 4 steps");
});

Deno.test("scrubJson reaches every string, at any depth, and leaves the shape alone", () => {
  assertEquals(
    scrubJson({ a: "see https://x.invalid/y", b: [{ c: "me@x.invalid" }], d: 3, e: null, f: true }),
    { a: "see <url>", b: [{ c: "<email>" }], d: 3, e: null, f: true },
  );
});

// Fix round 1, item 1: an ICS capability URL is not always `http(s)://` — `webcal://` is the same
// scheme Google Calendar subscriptions and the LMS feed both hand out, and it is the exact thing
// legal §9 names. Any `scheme://` must go, not only the two web ones.
Deno.test("any URL scheme is caught, not just http(s)", () => {
  assertEquals(scrub("subscribe at webcal://calendar.google.com/x.ics"), "subscribe at <url>");
  assertEquals(scrub("ftp://h/x"), "<url>");
  // The two schemes already covered keep working under the broadened pattern.
  assertEquals(
    scrub("fetching https://lms.example.invalid/feed/abc123.ics failed"),
    "fetching <url> failed",
  );
  assertEquals(scrub("http://10.0.0.1/x"), "<url>");
});

// Fix round 1, item 2: the old pattern stopped at the first whitespace, so a two-word Windows
// account name ("Ada Lovelace") lost only its first word and left the surname sitting in the
// output. The fix consumes up to the next path separator or quote, spaces included, and works with
// either slash — `docs/notes` on this drive still get authored with a mix of both.
Deno.test("a two-word Windows account name is scrubbed whole, forward slashes too, and stays a fixed point", () => {
  const twoWord = String.raw`could not open C:\Users\Ada Lovelace\Knowlu\tasks\a.md`;
  const scrubbed = scrub(twoWord);
  assertEquals(scrubbed, String.raw`could not open C:\Users\<user>\Knowlu\tasks\<note>`);
  // A second pass over already-scrubbed text changes nothing: `<user>` and `<note>` do not
  // themselves look like a `Users\` path segment or a `.md` filename.
  assertEquals(scrub(scrubbed), scrubbed);

  assertEquals(scrub("C:/Users/ada/x"), "C:/Users/<user>/x");

  // The original single-word case still passes unchanged.
  assertEquals(
    scrub(String.raw`could not open C:\Users\Ada\Knowlu\Fall 2026\tasks\a.md`),
    String.raw`could not open C:\Users\<user>\Knowlu\Fall 2026\tasks\<note>`,
  );
});

// Fix round 1, item 3: a named credential is a secret whatever its length — the 20-character floor
// on TOKEN exists to catch *opaque* runs, but `password=hunter2` never needed to be opaque to be a
// password. `bearer` is itself one of the named keywords, so an `Authorization: Bearer <short
// token>` header collapses to one redaction rather than leaving "Bearer" standing next to a token
// too short for TOKEN to have caught on its own.
Deno.test("a named credential is redacted whatever its length", () => {
  assertEquals(scrub("password=hunter2"), "password=<secret>");
  assertEquals(scrub("Authorization: Bearer abc123"), "Authorization=<secret>");
  // Fix round 1b (ruled): bare `key` joins the keyword list too — `key=sk_live_abc123` in a pasted
  // log is a credential shape, and prose with `key=` is rare enough that over-scrubbing is the
  // safe side.
  assertEquals(scrub("key=sk_live_abc123"), "key=<secret>");
});

// Fix round 1, item 4: `scrubJson` is documented as reaching "every string, at any depth" — a key
// is a string too, and a Gmail-derived key in a `payload` object is exactly the kind of stray PII
// this function exists to catch.
Deno.test("scrubJson scrubs object keys, not only their values", () => {
  assertEquals(scrubJson({ "ada@x.invalid": "v" }), { "<email>": "v" });
});

// Task 16 (`app/src/report.rs`) fix round 1, ruling R-C1-46: the Rust twin's first round was a
// hand-written scanner, and a differential fuzz run against a faithful JS port of these same six
// patterns found 1,109 / 40,000 adversarial and 2,673 / 30,000 log-shaped disagreements, one of
// them an under-redaction (a capability URL left unscrubbed on the wire). Round 2 rewrote the Rust
// side as a literal `regex` crate transcription of these same patterns, so the fix belongs on that
// end — but the eight inputs that exposed it are pinned here too, so both ends are tested against
// the same cases rather than merely re-implemented against the same intent (M1, a shared
// input→expected fixture read by both ends, is a follow-up for the close, not this round).
Deno.test("the eight inputs that exposed the Rust twin's round-1 scanner divergences", () => {
  // A capability URL right after a punctuation character that is a scheme-continuation character
  // (`+`, `.`, `-`) but not a `\w` — a single-backtrack hand scanner gave up one character early
  // and left the whole URL on the wire.
  assertEquals(scrub("-https://lms.example.invalid/feed"), "-<url>");
  assertEquals(scrub(".https://lms.example.invalid/feed"), ".<url>");
  assertEquals(scrub("+https://x/y"), "+<url>");
  assertEquals(scrub("2026-https://x/y"), "2026-<url>");
  // `\S+` needs at least one character — a scheme with nothing after `://` is not a URL.
  assertEquals(scrub("x://"), "x://");
  // CREDENTIAL claims `token://` (its own keyword, `:` separator, `//` as the opaque value) before
  // URL_RE's own (correctly empty) attempt ever gets a look at it.
  assertEquals(scrub("token://"), "token=<secret>");
  // NOTE's `\b` sits at the first `\w` inside the `[\w.-]` run, not at the run's own start — a run
  // that opens on `-` keeps that leading punctuation outside the match.
  assertEquals(scrub("-a.md"), "-<note>");
  // `(?:bearer\s+)?` backtracks when the value after it would be empty: the trailing space here
  // leaves nothing for `\S+`, so the optional group is dropped and `\S+` matches `Bearer` itself,
  // claiming the header but leaving the trailing space outside the match.
  assertEquals(scrub("authorization: Bearer "), "authorization=<secret> ");
});

// Task 16 fix round 2, ruling R-C1-49 (N1): the Rust twin's round-2 rewrite picked a different
// (wrong) approximation of `\s`/`\S` at each of the four places this file uses them — ASCII-only
// for the two positive `\s*` occurrences here in CREDENTIAL (missing NBSP and the rest of Unicode
// `Zs`, so a non-breaking space next to the keyword made the whole match fail to fire, the password
// surviving unredacted) and Rust's own Unicode default for the negated `\S+` occurrences (which
// admits U+0085/NEL, which this file's `\s` — ECMA-262's `WhiteSpace`/`LineTerminator`, independent
// of the `u` flag — does not, so a match ended one character early and left the rest on the wire).
// These three inputs are the ones that exposed both directions; pinned here with this file's own
// output as the source of truth, exactly as the eight above were.
Deno.test("a non-breaking space and NEL are this file's whitespace, in both directions", () => {
  // A non-breaking space (U+00A0) before the `:` separator — routine in text pasted from a web
  // page or a Word document — must not let the password through.
  assertEquals(scrub("password : hunter2"), "password=<secret>");
  // A non-breaking space inside the `Bearer ` prefix — the whole header is still claimed whole.
  assertEquals(
    scrub("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"),
    "authorization=<secret>",
  );
  // NEL (U+0085) inside a URL's tail: this file's `\s` does not include it, so the whole
  // capability URL — NEL and all — is claimed, not just the part before it.
  assertEquals(scrub("see https://x.invalid/secrettail more"), "see <url> more");
});

// Provider swap Task 3: `scrubForPrompt` is the narrower scrub a judge prompt goes through
// (`judge_prompts.ts`'s email branch) -- URLs, email addresses, named credentials and bearer
// tokens, and opaque runs that carry a digit, but NOT the plain 20-character floor `scrub`'s
// `TOKEN` rule uses, which a student's own long word can clear on its own.
Deno.test("scrubForPrompt catches URLs, email addresses, named credentials and digit-bearing tokens", () => {
  assertEquals(scrubForPrompt("mailed a.student@crimson.ua.edu twice"), "mailed <email> twice");
  assertEquals(
    scrubForPrompt("fetching https://lms.example.invalid/feed/abc123.ics failed"),
    "fetching <url> failed",
  );
  assertEquals(scrubForPrompt("password=hunter2"), "password=<secret>");
  assertEquals(
    scrubForPrompt("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdef"),
    "authorization=<secret>",
  );
  // A digit-bearing opaque run -- a confirmation code, a session id -- is still caught.
  assertEquals(scrubForPrompt("your code is abcdEFGH12345678901234wxyz today"), "your code is <token> today");
});

Deno.test("scrubForPrompt leaves a plain long word alone, unlike scrub's TOKEN rule", () => {
  const sentence = "Please see the Hausaufgabenbesprechungstermin tomorrow.";
  // `scrub` over-redacts: the word alone clears TOKEN's 20-character floor.
  assert(scrub(sentence).includes("<token>"), "sanity: scrub's plain TOKEN rule does redact this word");
  // `scrubForPrompt` requires a digit in the run, so an ordinary long word survives.
  assertEquals(scrubForPrompt(sentence), sentence);
});

Deno.test("scrubForPrompt does not scrub a Windows path or a note filename (not a prompt concern)", () => {
  assertEquals(scrubForPrompt(String.raw`C:\Users\Ada\notes.md`), String.raw`C:\Users\Ada\notes.md`);
});
