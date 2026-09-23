// The paid run's key lookup, per the coordinator's addition to the E2 brief (2026-09-22): try, in
// order, (1) the process env var OPENROUTER_API_KEY, (2) the Windows USER-scope environment
// variable of the same name read live (it may be set after this shell started, so a cached
// Deno.env read at process start is not enough), (3) a Windows Credential Manager generic
// credential at target `knowlu/dev/openrouter`, password field. Never print, log or write the key
// anywhere — only which of the three supplied it. If none does, callers fall back to --dry-run and
// name the three places in one line.
//
// Tier 3 mirrors the byte-vs-UTF-16 trap `engine/src/wincred.rs` documents: CredentialBlobSize is a
// count of BYTES, not UTF-16 code units. `System.Text.Encoding.Unicode.GetString(byte[])` takes the
// whole array and its own .Length is already a byte count, so passing it the exact CredentialBlobSize
// bytes (not half, not double) decodes it correctly — the same fact wincred.rs's
// `blob_size_is_bytes_not_code_units` test pins in Rust.

export interface KeyTier {
  source: string;
  probe: () => Promise<string | null> | string | null;
}

export interface KeyLookupResult {
  key: string | null;
  source: string;
  /** One secret-free line per probe that failed with a non-zero exit or a thrown error — empty
   * when a tier succeeded before any failing probe ran, or when nothing failed. Only
   * `resolveOpenRouterKey` populates this; `resolveKeyWith` is generic and never touches it. */
  diagnostics?: string[];
}

/** The ordering/short-circuit logic, decoupled from the OS-touching probes so it is unit-testable. */
export async function resolveKeyWith(tiers: KeyTier[]): Promise<KeyLookupResult> {
  for (const tier of tiers) {
    const val = await tier.probe();
    if (val && val.trim().length > 0) {
      return { key: val, source: tier.source };
    }
  }
  return { key: null, source: "none" };
}

/**
 * PowerShell's own documented quoting-proof invocation form: base64 of the script's UTF-16LE
 * bytes, passed as `-EncodedCommand`. `-Command <script>` instead hands the script through Windows
 * command-line quoting (CommandLineToArgvW) before PowerShell ever parses it, which mangles a
 * multi-line script containing double quotes and a here-string (exactly what `CRED_MAN_SCRIPT` is)
 * — the actual cause of the Credential Manager tier silently returning null through Deno even
 * though the identical script run directly in PowerShell succeeds. Exported and pure so it can be
 * tested against a base64 computed independently of this file.
 */
export function encodeCommand(script: string): string {
  let binary = "";
  for (let i = 0; i < script.length; i++) {
    const code = script.charCodeAt(i);
    binary += String.fromCharCode(code & 0xff);
    binary += String.fromCharCode((code >> 8) & 0xff);
  }
  return btoa(binary);
}

interface PowerShellResult {
  value: string | null;
  /** Secret-free: an exit code and, if present, the first line of stderr. Never the script's
   * output, so a Credential Manager failure can never leak a partial secret here. */
  diagnostic: string | null;
}

async function runPowerShell(script: string): Promise<PowerShellResult> {
  try {
    const cmd = new Deno.Command("powershell.exe", {
      args: ["-NoProfile", "-NonInteractive", "-EncodedCommand", encodeCommand(script)],
      stdout: "piped",
      stderr: "piped",
    });
    const { code, stdout, stderr } = await cmd.output();
    if (code !== 0) {
      const errFirstLine = new TextDecoder().decode(stderr).trim().split("\n")[0]?.trim() ?? "";
      return { value: null, diagnostic: errFirstLine ? `exit ${code}: ${errFirstLine}` : `exit ${code}` };
    }
    const text = new TextDecoder().decode(stdout).trim();
    return { value: text.length > 0 ? text : null, diagnostic: null };
  } catch (err) {
    // powershell.exe not on PATH, or --allow-run was not granted: treat as "no value", never throw —
    // this must fall through to --dry-run, not crash the experiment.
    const message = err instanceof Error ? err.message : String(err);
    return { value: null, diagnostic: message.split("\n")[0]?.trim() ?? "powershell.exe unavailable" };
  }
}

async function readUserEnvVar(name: string, diagnostics?: string[]): Promise<string | null> {
  const { value, diagnostic } = await runPowerShell(
    `[Environment]::GetEnvironmentVariable('${name}', 'User')`,
  );
  if (diagnostic) diagnostics?.push(`user-env:${name} — ${diagnostic}`);
  return value;
}

const CRED_MAN_SCRIPT = (target: string) => `
Add-Type -Namespace E2 -Name CredMan -MemberDefinition @'
[DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern bool CredRead(string target, int type, int reservedFlag, out IntPtr credentialPtr);
[DllImport("advapi32.dll", SetLastError = true)]
public static extern void CredFree(IntPtr cred);
'@
$credPtr = [IntPtr]::Zero
$ok = [E2.CredMan]::CredRead("${target}", 1, 0, [ref]$credPtr)
if (-not $ok) { exit 1 }
try {
  # CREDENTIALW's fields are read by fixed byte offset rather than Marshal.PtrToStructure(IntPtr,
  # Type): on this PowerShell build, PtrToStructure rejects an Add-Type-defined struct — even one
  # with every field blittable (IntPtr/uint only) — with "must be blittable or have layout
  # information", confirmed in isolation against a two-int struct, so it cannot be a CharSet or
  # FILETIME artifact. CredentialBlobSize (DWORD) and CredentialBlob (pointer) are the only two
  # fields this probe needs; their offsets follow wincred.h's CREDENTIALW layout after
  # Flags/Type (4+4 bytes), TargetName/Comment (one pointer each) and LastWritten/FILETIME
  # (8 bytes) — verified against a real cmdkey-written credential before this shipped.
  # CredentialBlobSize is a count of BYTES, not UTF-16 code units — the same fact
  # engine/src/wincred.rs's blob_size_is_bytes_not_code_units test pins in Rust.
  $ptrSize = [IntPtr]::Size
  $sizeOffset = if ($ptrSize -eq 8) { 32 } else { 24 }
  $blobPtrOffset = if ($ptrSize -eq 8) { 40 } else { 28 }
  $size = [System.Runtime.InteropServices.Marshal]::ReadInt32($credPtr, $sizeOffset)
  if ($size -le 0) { exit 1 }
  $blobPtr = [System.Runtime.InteropServices.Marshal]::ReadIntPtr($credPtr, $blobPtrOffset)
  $bytes = New-Object byte[] $size
  [System.Runtime.InteropServices.Marshal]::Copy($blobPtr, $bytes, 0, $size)
  [System.Text.Encoding]::Unicode.GetString($bytes)
} finally {
  [E2.CredMan]::CredFree($credPtr)
}
`;

export async function readCredentialManagerSecret(
  target: string,
  diagnostics?: string[],
): Promise<string | null> {
  const { value, diagnostic } = await runPowerShell(CRED_MAN_SCRIPT(target));
  if (diagnostic) diagnostics?.push(`credential-manager:${target} — ${diagnostic}`);
  return value;
}

export const OPENROUTER_KEY_TARGET = "knowlu/dev/openrouter";

export async function resolveOpenRouterKey(): Promise<KeyLookupResult> {
  const diagnostics: string[] = [];
  const result = await resolveKeyWith([
    { source: "env:OPENROUTER_API_KEY", probe: () => Deno.env.get("OPENROUTER_API_KEY") ?? null },
    {
      source: "user-env:OPENROUTER_API_KEY",
      probe: () => readUserEnvVar("OPENROUTER_API_KEY", diagnostics),
    },
    {
      source: `credential-manager:${OPENROUTER_KEY_TARGET}`,
      probe: () => readCredentialManagerSecret(OPENROUTER_KEY_TARGET, diagnostics),
    },
  ]);
  return { ...result, diagnostics };
}

/** The one-line message when no tier supplies a key — names the three places, never a value.
 * `diagnostics` (from `resolveOpenRouterKey`'s result) appends the secret-free reason each probe
 * that actually ran gave, so a "none" result can say why instead of just that it happened. */
export function noKeyMessage(diagnostics: string[] = []): string {
  const base = "No OPENROUTER_API_KEY found in: (1) the process environment, (2) the Windows USER-scope " +
    "environment variable, (3) Credential Manager target " +
    `"${OPENROUTER_KEY_TARGET}". Running --dry-run.`;
  if (diagnostics.length === 0) return base;
  return base + "\nDiagnostics:\n" + diagnostics.map((d) => `  - ${d}`).join("\n");
}
