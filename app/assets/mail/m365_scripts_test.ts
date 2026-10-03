// Plan T10.2a's rules for the scripts: each is one readable file, acts and reads back, logs nothing,
// and names no credential (D19, T10 "Rules for the scripts").
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

Deno.test("m365_scripts_log_nothing_and_name_no_credential", async () => {
  let seen = 0;
  for await (const e of Deno.readDir(new URL("m365/", import.meta.url))) {
    if (!e.name.endsWith(".js")) continue;
    seen++;
    const text = (await Deno.readTextFile(new URL(`m365/${e.name}`, import.meta.url))).toLowerCase();
    for (const word of FORBIDDEN) {
      assert(!text.includes(word.toLowerCase()), `m365/${e.name} names ${word}`);
    }
    assert(text.trimStart().startsWith("//"), `m365/${e.name} opens with a comment saying what it does`);
  }
  assert(seen >= 4, "the sign-in probe and the three forwarding scripts");
});
