import { assertEquals } from "@std/assert";
import { scrub, scrubJson } from "./scrub.ts";

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
  // A long opaque run of key-ish characters.
  assertEquals(scrub("key=sk_live_51Hxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), "key=<token>");
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
  // A bare `key=`, with no `api` prefix, is not one of the named keywords and is not 20 characters
  // either — ordinary key=value prose (a log line, a JSON fragment) survives untouched.
  assertEquals(scrub("key=sk_live_abc123"), "key=sk_live_abc123");
});

// Fix round 1, item 4: `scrubJson` is documented as reaching "every string, at any depth" — a key
// is a string too, and a Gmail-derived key in a `payload` object is exactly the kind of stray PII
// this function exists to catch.
Deno.test("scrubJson scrubs object keys, not only their values", () => {
  assertEquals(scrubJson({ "ada@x.invalid": "v" }), { "<email>": "v" });
});
