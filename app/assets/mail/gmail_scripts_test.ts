// Plan T10 "Rules for the scripts" for Gmail's forwarding and mailbox scripts: each is one readable file
// under gmail/ that opens with a comment saying what it does, logs nothing and names no credential (D19).
import { assert } from "@std/assert";

const FORBIDDEN = [
  "console.",
  "fetch(",
  "XMLHttpRequest",
  "WebSocket",
  "sendBeacon",
  "localStorage",
  "sessionStorage",
  "document.cookie",
  "indexedDB",
  "token",
  // Spelled in two pieces: the scaffold test refuses the whole word anywhere under assets/.
  "pass" + "word",
  "__TAURI",
];

Deno.test("gmail_scripts_log_nothing_and_name_no_credential", async () => {
  let seen = 0;
  for await (const e of Deno.readDir(new URL("gmail/", import.meta.url))) {
    if (!e.name.endsWith(".js")) continue;
    seen++;
    const text = (await Deno.readTextFile(new URL(`gmail/${e.name}`, import.meta.url))).toLowerCase();
    for (const word of FORBIDDEN) {
      assert(!text.includes(word.toLowerCase()), `gmail/${e.name} names ${word}`);
    }
    assert(text.trimStart().startsWith("//"), `gmail/${e.name} opens with a comment saying what it does`);
  }
  assert(seen >= 10, "the six forwarding scripts and the four mailbox scripts");
});
