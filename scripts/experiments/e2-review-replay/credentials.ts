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

async function runPowerShell(script: string): Promise<string | null> {
  try {
    const cmd = new Deno.Command("powershell.exe", {
      args: ["-NoProfile", "-NonInteractive", "-Command", script],
      stdout: "piped",
      stderr: "piped",
    });
    const { code, stdout } = await cmd.output();
    if (code !== 0) return null;
    const text = new TextDecoder().decode(stdout).trim();
    return text.length > 0 ? text : null;
  } catch {
    // powershell.exe not on PATH, or --allow-run was not granted: treat as "no value", never throw —
    // this must fall through to --dry-run, not crash the experiment.
    return null;
  }
}

async function readUserEnvVar(name: string): Promise<string | null> {
  return await runPowerShell(`[Environment]::GetEnvironmentVariable('${name}', 'User')`);
}

const CRED_MAN_SCRIPT = (target: string) => `
Add-Type -Namespace E2 -Name CredMan -MemberDefinition @'
[DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern bool CredRead(string target, int type, int reservedFlag, out IntPtr credentialPtr);
[DllImport("advapi32.dll", SetLastError = true)]
public static extern void CredFree(IntPtr cred);
[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct CREDENTIAL {
  public uint Flags;
  public uint Type;
  public IntPtr TargetName;
  public IntPtr Comment;
  public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
  public uint CredentialBlobSize;
  public IntPtr CredentialBlob;
  public uint Persist;
  public uint AttributeCount;
  public IntPtr Attributes;
  public IntPtr TargetAlias;
  public IntPtr UserName;
}
'@
$credPtr = [IntPtr]::Zero
$ok = [E2.CredMan]::CredRead("${target}", 1, 0, [ref]$credPtr)
if (-not $ok) { exit 1 }
try {
  $cred = [System.Runtime.InteropServices.Marshal]::PtrToStructure($credPtr, [E2.CredMan+CREDENTIAL])
  $size = [int]$cred.CredentialBlobSize
  if ($size -le 0) { exit 1 }
  $bytes = New-Object byte[] $size
  [System.Runtime.InteropServices.Marshal]::Copy($cred.CredentialBlob, $bytes, 0, $size)
  [System.Text.Encoding]::Unicode.GetString($bytes)
} finally {
  [E2.CredMan]::CredFree($credPtr)
}
`;

async function readCredentialManagerSecret(target: string): Promise<string | null> {
  return await runPowerShell(CRED_MAN_SCRIPT(target));
}

export const OPENROUTER_KEY_TARGET = "knowlu/dev/openrouter";

export async function resolveOpenRouterKey(): Promise<KeyLookupResult> {
  return await resolveKeyWith([
    { source: "env:OPENROUTER_API_KEY", probe: () => Deno.env.get("OPENROUTER_API_KEY") ?? null },
    { source: "user-env:OPENROUTER_API_KEY", probe: () => readUserEnvVar("OPENROUTER_API_KEY") },
    {
      source: `credential-manager:${OPENROUTER_KEY_TARGET}`,
      probe: () => readCredentialManagerSecret(OPENROUTER_KEY_TARGET),
    },
  ]);
}

/** The one-line message when no tier supplies a key — names the three places, never a value. */
export function noKeyMessage(): string {
  return (
    "No OPENROUTER_API_KEY found in: (1) the process environment, (2) the Windows USER-scope " +
    "environment variable, (3) Credential Manager target " +
    `"${OPENROUTER_KEY_TARGET}". Running --dry-run.`
  );
}
