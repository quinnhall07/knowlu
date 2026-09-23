import { assertEquals } from "@std/assert";
import {
  encodeCommand,
  noKeyMessage,
  OPENROUTER_KEY_TARGET,
  readCredentialManagerSecret,
  resolveKeyWith,
} from "./credentials.ts";

Deno.test("resolveKeyWith returns the first tier that supplies a non-empty value", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: () => null },
    { source: "tier2", probe: () => "  " }, // blank counts as absent
    { source: "tier3", probe: () => "the-key" },
  ]);
  assertEquals(result, { key: "the-key", source: "tier3" });
});

Deno.test("resolveKeyWith prefers an earlier tier over a later one that would also succeed", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: () => "first" },
    { source: "tier2", probe: () => "second" },
  ]);
  assertEquals(result, { key: "first", source: "tier1" });
});

Deno.test("resolveKeyWith never inspects tiers past the first success (order is the contract)", async () => {
  let tier2Called = false;
  await resolveKeyWith([
    { source: "tier1", probe: () => "first" },
    {
      source: "tier2",
      probe: () => {
        tier2Called = true;
        return "second";
      },
    },
  ]);
  assertEquals(tier2Called, false);
});

Deno.test("resolveKeyWith returns source 'none' and a null key when every tier is empty", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: () => null },
    { source: "tier2", probe: () => "" },
  ]);
  assertEquals(result, { key: null, source: "none" });
});

Deno.test("resolveKeyWith supports async probes", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: async () => await Promise.resolve(null) },
    { source: "tier2", probe: async () => await Promise.resolve("async-key") },
  ]);
  assertEquals(result, { key: "async-key", source: "tier2" });
});

Deno.test("noKeyMessage names the three places and never a value", () => {
  const msg = noKeyMessage();
  assertEquals(msg.includes("process environment"), true);
  assertEquals(msg.includes("USER-scope"), true);
  assertEquals(msg.includes(OPENROUTER_KEY_TARGET), true);
  assertEquals(msg.includes("--dry-run"), true);
});

Deno.test("noKeyMessage appends secret-free diagnostics when the caller supplies them", () => {
  const msg = noKeyMessage(["user-env:OPENROUTER_API_KEY — exit 1: access denied"]);
  assertEquals(msg.includes("user-env:OPENROUTER_API_KEY — exit 1: access denied"), true);
});

// The quoting trap this whole file exists to fix: a multi-line PowerShell script containing double
// quotes and a here-string, passed as `-Command <script>`, is re-parsed by Windows command-line
// quoting (CommandLineToArgvW) before PowerShell ever sees it, and comes out mangled. The fix is
// PowerShell's own documented quoting-proof form: `-EncodedCommand <base64 of UTF-16LE>`. This
// value was computed independently (PowerShell's own
// `[Convert]::ToBase64String([System.Text.Encoding]::Unicode.GetBytes(...))`, not this file's
// encoder) so the test cannot pass by mirroring a bug in the implementation.
Deno.test("encodeCommand produces the exact base64 of UTF-16LE for a known script", () => {
  const result = encodeCommand('Write-Output "hi"');
  assertEquals(result, "VwByAGkAdABlAC0ATwB1AHQAcAB1AHQAIAAiAGgAaQAiAA==");
});

// Real round-trip: create a throwaway GENERIC credential with `cmdkey`, read it back through the
// harness's own Credential Manager probe (the one `resolveOpenRouterKey`'s tier 3 uses), and delete
// it in a `finally` no matter what. Never touches `knowlu/dev/openrouter`. Windows only — `cmdkey`
// and Credential Manager do not exist elsewhere.
Deno.test({
  name:
    "readCredentialManagerSecret reads back a real generic credential written by cmdkey, through -EncodedCommand",
  ignore: Deno.build.os !== "windows",
  fn: async () => {
    const target = `knowlu/test/e2-${crypto.randomUUID().slice(0, 8)}`;
    const value = `throwaway-${crypto.randomUUID().slice(0, 8)}`;
    const create = new Deno.Command("cmdkey.exe", {
      args: [`/generic:${target}`, "/user:e2-harness-test", `/pass:${value}`],
      stdout: "piped",
      stderr: "piped",
    });
    const createResult = await create.output();
    assertEquals(createResult.code, 0, "cmdkey /generic should create the throwaway credential");
    try {
      const read = await readCredentialManagerSecret(target);
      assertEquals(read, value);
    } finally {
      const del = new Deno.Command("cmdkey.exe", {
        args: [`/delete:${target}`],
        stdout: "piped",
        stderr: "piped",
      });
      await del.output();
    }
  },
});

// Guards against a fix that silently drops the diagnostic entirely: a failing read (target does not
// exist) must still resolve to null, never throw, and never print the value it obviously doesn't
// have — but readCredentialManagerSecret's caller-visible diagnostics array (threaded from
// resolveOpenRouterKey) must gain a non-empty entry naming the target, not just "none".
Deno.test({
  name: "readCredentialManagerSecret returns null and records a diagnostic for a target that does not exist",
  ignore: Deno.build.os !== "windows",
  fn: async () => {
    const target = `knowlu/test/e2-missing-${crypto.randomUUID().slice(0, 8)}`;
    const diagnostics: string[] = [];
    const read = await readCredentialManagerSecret(target, diagnostics);
    assertEquals(read, null);
    assertEquals(diagnostics.length > 0, true);
    assertEquals(diagnostics[0].includes(target), true);
  },
});
