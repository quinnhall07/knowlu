/**
 * PQ2's interop check (email plan T2d, "After it"): run `_shared/dkim.ts` over a directory of real
 * `.eml` files and print, per file, each signature's verdict and the topmost ARC instance's, for the
 * outer message and for each attached `message/rfc822` original. Lines carry fixed words, provider
 * domains and the file's own name only: no address, subject, body or header value (`interopReport`).
 *
 * The controller runs it on its own machine over S0's captures, which are never committed:
 *
 *   deno run --allow-read=<dir> --allow-net --config cloud/supabase/deno.json \
 *     cloud/supabase/scripts/dkim_interop.ts <dir>
 *
 * `--allow-net` is for DNS only (keys are fetched live, so a rotated key reads `no_key` or `signature`).
 * No test runs this file; its logic is `interopReport`, which `dkim_test.ts` covers.
 */
import { dnsTxtResolver, interopReport } from "../functions/_shared/dkim.ts";
import { attachedMessages } from "../functions/_shared/mime.ts";

const dir = Deno.args[0];
if (dir === undefined) {
  console.error("usage: dkim_interop.ts <directory of .eml files>");
  Deno.exit(2);
}
const deps = { resolveTxt: dnsTxtResolver() };
const files: string[] = [];
for await (const e of Deno.readDir(dir)) {
  if (e.isFile && e.name.toLowerCase().endsWith(".eml")) files.push(e.name);
}
files.sort();
for (const name of files) {
  const bytes = await Deno.readFile(`${dir}/${name}`);
  console.log(`${name}: outer`);
  for (const line of await interopReport(bytes, deps)) console.log(`  ${line}`);
  for (const [n, inner] of attachedMessages(bytes).entries()) {
    console.log(`${name}: attached #${n + 1}`);
    for (const line of await interopReport(inner, deps)) console.log(`  ${line}`);
  }
}
console.log(`${files.length} file(s)`);
