import { assert, assertEquals } from "@std/assert";

const FUNCTIONS = new URL("..", new URL(".", import.meta.url));
const CONFIG_TOML = new URL("../config.toml", FUNCTIONS);

Deno.test("every deployed function has its own verify_jwt = false entry in config.toml", async () => {
  // C2 final review S-6. Supabase's gateway verifies the Authorization header as a Supabase JWT
  // BEFORE a function runs unless that function opts out — and every function here authenticates
  // for itself (`requireActiveEntitlement`, or, for `google-callback`, the single-use `state`
  // nonce; `stripe-webhook` verifies a Stripe signature). A function added without an entry here
  // would be gateway-verified: the device's own session JWT would still get through, so nothing
  // would look broken on the happy path, and `stripe-webhook`/`google-callback` — which are called
  // by someone else's server and someone's browser, neither holding a Supabase JWT — would answer
  // 401 to callers who cannot be told why. The entry is easy to forget and impossible to notice, so
  // it is a test rather than a convention.
  const toml = await Deno.readTextFile(CONFIG_TOML);
  const declared = new Set(
    [...toml.matchAll(/^\[functions\.([a-z0-9-]+)\]/gm)].map((m) => m[1]),
  );
  const missing: string[] = [];
  const notOptedOut: string[] = [];
  const deployed: string[] = [];
  for await (const entry of Deno.readDir(FUNCTIONS)) {
    // `_shared` is imported, never deployed as a function of its own.
    if (!entry.isDirectory || entry.name === "_shared") continue;
    deployed.push(entry.name);
    if (!declared.has(entry.name)) {
      missing.push(entry.name);
      continue;
    }
    // The entry AND its flag: a section with no `verify_jwt` line is the same gap as no section.
    const section = new RegExp(`^\\[functions\\.${entry.name}\\][^[]*`, "m").exec(toml)?.[0] ?? "";
    if (!/verify_jwt\s*=\s*false/.test(section)) notOptedOut.push(entry.name);
  }
  assertEquals(missing, [], "every function directory needs a [functions.<name>] section");
  assertEquals(notOptedOut, [], "every section needs verify_jwt = false");
  // And nothing is declared that is not deployed — a stale section names a function that is gone.
  assertEquals([...declared].filter((d) => !deployed.includes(d)), []);
  assert(deployed.length >= 19, `expected every function directory, found ${deployed.length}`);
});

Deno.test("config.toml declares no auth provider, and says why — the Google provider is the dashboard's", async () => {
  // R-C1b-5. A block here would put a client id in the repo and need `GOOGLE_SECRET` in every
  // pushing shell — and a push from a shell without it would DISABLE a provider that works. The
  // file therefore declares none, and the comment is what a reviewer reads instead: "we chose not
  // to write a block" is otherwise invisible on a diff.
  const toml = await Deno.readTextFile(CONFIG_TOML);
  assert(!/^\[auth\.external\./m.test(toml), "no auth provider may be declared in the repo");
  assert(
    toml.includes("The Google provider is the DASHBOARD's, on both projects"),
    "…and the file has to say so, or the next reader adds one",
  );
  // The other two halves of the same decision: D5, and the number that bounds what D5 costs (I2).
  assert(toml.includes("enable_confirmations = false"), "D5: email confirmations are off");
  assert(/^\[auth\.rate_limit\]/m.test(toml), "[auth.rate_limit] is declared, not defaulted");
  assert(/^email_sent = \d+$/m.test(toml), "…with a number on the diff");
  // **And the table starts AFTER `double_confirm_changes`** (review R3). A `[auth.rate_limit]`
  // header placed one line too early does not fail to parse — it silently adopts the key below it,
  // so `[auth.email]` loses `double_confirm_changes` and a push carries that loss to the project.
  assert(
    toml.indexOf("double_confirm_changes") < toml.indexOf("[auth.rate_limit]"),
    "[auth.rate_limit] must not capture double_confirm_changes",
  );
});
