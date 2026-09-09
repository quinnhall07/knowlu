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
  // A bearer token, and a JWT in particular.
  assertEquals(
    scrub("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"),
    "authorization: Bearer <token>",
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
