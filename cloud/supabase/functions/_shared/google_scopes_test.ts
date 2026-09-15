import { assert, assertEquals } from "@std/assert";
import { CALENDAR_SCOPE, GMAIL_SCOPE, GOOGLE_NOT_CONFIGURED } from "./google_scopes.ts";

const HERE = new URL(".", import.meta.url);
const FUNCTIONS = new URL("..", HERE);
/** The device half, read as TEXT — the same direction `ingest-coursework`'s tests already read
 * `engine/tests/fixtures/`. Rust cannot read this file's exported const, and `cloudmodel.rs`'s own
 * constant is private to that module, so the pin goes ONE way: from here, into the Rust source. */
const CLOUDMODEL_RS = new URL("../../../../engine/src/cloudmodel.rs", import.meta.url);

Deno.test("the not-configured 503 has exactly one copy, and the engine's matches it", async () => {
  // C2 final review S-4. This sentence is an exact-match contract, not copy: `cloudmodel.rs`
  // compares it character for character (R-C2-E45 (3)) to tell "Google sign-in was never
  // configured on this deployment" — a quiet, named skip the device prints once — from an ordinary
  // platform 503, which means "try later". It used to be written out five times; a rewording in
  // any one of them silently turned the skip into a failure line.
  const rust = await Deno.readTextFile(CLOUDMODEL_RS);
  const pinned = /const GMAIL_NOT_CONFIGURED_DETAIL: &str = "([^"]*)";/.exec(rust);
  assert(pinned !== null, "cloudmodel.rs no longer declares GMAIL_NOT_CONFIGURED_DETAIL");
  assertEquals(pinned[1], GOOGLE_NOT_CONFIGURED);

  // And nothing else writes the sentence out for itself any more — `google_scopes.ts`, which
  // declares it, is the one file allowed to contain the literal.
  const offenders: string[] = [];
  for await (const dir of Deno.readDir(FUNCTIONS)) {
    if (!dir.isDirectory) continue;
    for await (const file of Deno.readDir(new URL(`${dir.name}/`, FUNCTIONS))) {
      if (!file.isFile || !file.name.endsWith(".ts")) continue;
      const path = `${dir.name}/${file.name}`;
      if (path === "_shared/google_scopes.ts") continue;
      const source = await Deno.readTextFile(new URL(path, FUNCTIONS));
      if (source.includes(`"${GOOGLE_NOT_CONFIGURED}"`)) offenders.push(path);
    }
  }
  assertEquals(offenders, [], "import GOOGLE_NOT_CONFIGURED instead of spelling it again");
});

Deno.test("the two scope literals are the exact strings Google and the device both check", () => {
  // `read_google_grant(p_scope)` matches on these, `app/src/account.rs` reports the connection by
  // testing `scopes` for them, and §11a's whole sensitive/restricted ordering rests on which of the
  // two a consent asked for. They are literals in three places and must stay these literals.
  assertEquals(CALENDAR_SCOPE, "https://www.googleapis.com/auth/calendar.readonly");
  assertEquals(GMAIL_SCOPE, "https://www.googleapis.com/auth/gmail.readonly");
});
