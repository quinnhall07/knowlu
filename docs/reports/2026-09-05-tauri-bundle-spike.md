# Spike — `cargo tauri build` on the GNU toolchain: NSIS, the WebView2 bootstrapper, and the engine sidecar

Knowlu plan 4a, Task 1. Run 2026-09-05 on the laptop, in the worktree
`.claude/worktrees/knowlu-plan-1` on branch `worktree-knowlu-plan-4a`.

The question this spike exists to answer: **does `cargo tauri build` produce an NSIS installer with
the WebView2 bootstrapper and the engine sidecar, on `stable-x86_64-pc-windows-gnu`?** Task 9 (bundle
+ release script) branches on the answer.

---

## 1. `tauri-cli` version

Already installed; not reinstalled.

```
$ cargo tauri --version
tauri-cli 2.11.4
$ rustup show active-toolchain
stable-x86_64-pc-windows-gnu (default)
```

The CLI resolves and runs on the GNU host. `cargo install tauri-cli` was **not** re-run, so the
brief's Outcome D "will not install" branch was not exercised — it was already known good.

## 2. Engine exe, staged as the sidecar

```
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 43.16s
$ cp target/release/quinn-ops.exe app/binaries/quinn-ops-x86_64-pc-windows-gnu.exe
```

| file | bytes | MiB |
|---|---:|---:|
| `target/release/quinn-ops.exe` (engine) | 4,438,016 | 4.23 |
| `app/target/release/knowlu.exe` (shell) | 5,642,240 | 5.38 |
| `app/target/release/WebView2Loader.dll` | 160,320 | 0.15 |
| `%LOCALAPPDATA%\tauri\MicrosoftEdgeWebview2Setup.exe` (fetched by the bundler) | 1,783,000 | 1.70 |
| **payload, uncompressed** | **12,023,576** | **11.47** |

The target-triple suffix is required by Tauri on an `externalBin` file and is stripped at install
time, so the installed name is `quinn-ops.exe` — exactly the sibling name
`scheduler::engine_exe()` looks for after `KNOWLU_ENGINE_EXE`. Confirmed twice below: in
`app/target/release/` after the build, and in the generated NSIS script's install section.

## 3. The bundle config used (throwaway; reverted in step 7)

`app/tauri.conf.json`, the `"bundle"` object replaced with exactly this — nothing else in the file
touched, CRLF preserved (`git diff --stat` = `1 file changed, 7 insertions(+), 1 deletion(-)`):

```json
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"],
    "externalBin": ["binaries/quinn-ops"],
    "windows": { "webviewInstallMode": { "type": "embedBootstrapper" } }
  }
```

## 4. Build output (the whole log — it is 30 lines, fewer than the 40 asked for)

```
        Info Looking up installed tauri packages to check mismatched versions...
   Compiling tauri v2.11.5
   Compiling tauri-macros v2.6.3
   Compiling quinn-ops v0.1.0 (C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1)
   Compiling tauri-plugin-clipboard-manager v2.3.3
   Compiling tauri-plugin-autostart v2.5.1
   Compiling tauri-plugin-window-state v2.4.1
   Compiling quinn-ops-console v0.1.0 (C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1\app)
   Compiling tauri-plugin-single-instance v2.4.4
warning: linker stderr: C:/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin/../lib/gcc/x86_64-w64-mingw32/16.1.0/../../../../x86_64-w64-mingw32/bin/ld.exe: .rsrc merge failure: multiple non-default manifests
  |
  = note: `#[warn(linker_messages)]` on by default

warning: `quinn-ops-console` (bin "knowlu") generated 1 warning
    Finished `release` profile [optimized] target(s) in 1m 28s
       Built application at: C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1\app\target\release\knowlu.exe
        Info Patching C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1\app\target\release\knowlu.exe with bundle type information: nsis
        Info Verifying NSIS package
 Downloading https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
        Info validating hash
        Info extracting NSIS
 Downloading https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll
        Info validating hash
        Info Target: x64
 Downloading https://go.microsoft.com/fwlink/p/?LinkId=2124703
     Running makensis to produce C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1\app\target\release\bundle\nsis\Knowlu_0.1.0_x64-setup.exe
    Finished 1 bundle at:
        C:\Users\danie\GitHub\quinn-ops\.claude\worktrees\knowlu-plan-1\app\target\release\bundle\nsis\Knowlu_0.1.0_x64-setup.exe
```

Exit code 0. Wall clock ~2 minutes (the engine crate was warm; the tauri crates were not — they
still only took 1m 28s).

The one warning is the **pre-existing** `.rsrc merge failure: multiple non-default manifests`
already documented under "Known wrinkles, none blocking" in `app/README.md`. It is a warning, not an
error; the bundle built regardless. Measured here for the record: `knowlu.exe` contains
**2** occurrences of `urn:schemas-microsoft-com:asm.v1` and 1 `requestedExecutionLevel`, i.e. the
manifest count is still 2, not the 1 the README wants before shipping.

## 5. Outcome

**Outcome: A**

The NSIS installer built on the GNU toolchain, in one command, with no MSVC anywhere. The bundler
fetched NSIS 3.11, `nsis_tauri_utils.dll` and the WebView2 bootstrapper itself (into
`%LOCALAPPDATA%\tauri\`, hash-validated) and ran `makensis` without any `NSIS_PATH` help. Outcome B
(NSIS unfetchable), C (GNU refused / MSVC-only bundler) and D (CLI will not install) are all ruled
out.

## 6. Installer size and contents

```
app\target\release\bundle\nsis\Knowlu_0.1.0_x64-setup.exe
4,632,904 bytes (4.42 MiB)
```

`SetCompressor /SOLID "lzma"` over an 11.47 MiB payload — 38.5% of uncompressed.

**7-Zip is not installed on this machine** (`7z`/`7za`/`7zr` not on PATH; `C:\Program Files\7-Zip`,
`C:\Program Files (x86)\7-Zip` and `%LOCALAPPDATA%\Programs\7-Zip` all absent; no WinGet package).
The installer was **not run** — desktop safety. So the contents were read from the NSIS script the
bundler generated, `app/target/release/nsis/x64/installer.nsi`, which is the literal input
`makensis` compiled into that exe. Its install section:

```nsis
  ; Copy main executable
  File "${MAINBINARYSRCPATH}"                      ; ...\app\target\release\knowlu.exe

  ; Copy resources
    CreateDirectory "$INSTDIR\"
    File /a "/oname=WebView2Loader.dll" "...\app\target\release\WebView2Loader.dll"

  ; Copy external binaries
    File /a "/oname=quinn-ops.exe" "...\app\binaries\quinn-ops-x86_64-pc-windows-gnu.exe"
```

and, in `Section WebView2`, under `!if "${INSTALLWEBVIEW2MODE}" == "embedBootstrapper"`:

```nsis
    File "/oname=$TEMP\MicrosoftEdgeWebview2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
    ...
    ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
```

with `!define WEBVIEW2BOOTSTRAPPERPATH "C:\Users\danie\AppData\Local\tauri\MicrosoftEdgeWebview2Setup.exe"`
and `!define WEBVIEW2INSTALLERARGS "/silent"`. The bootstrapper section is guarded by three
registry probes (HKLM WOW6432Node, HKLM, HKCU `EdgeUpdate\Clients\{F3017226-...}`), so it is skipped
when a WebView2 runtime is already present.

All three of the things the decision table asked for are therefore present:

| expected | present | as |
|---|---|---|
| `knowlu.exe` | yes | `File "${MAINBINARYSRCPATH}"` → `$INSTDIR\knowlu.exe` |
| `quinn-ops.exe` | yes | `File /a /oname=quinn-ops.exe` → `$INSTDIR\quinn-ops.exe`, sibling of `knowlu.exe` |
| WebView2 bootstrapper | yes | embedded, extracted to `$TEMP` and run `/silent /install` when absent |

Two more facts from the generated script that Task 9 will want:

- `!define INSTALLMODE "currentUser"` — a **per-user** install. No UAC prompt, no admin, which is
  what the friends build wants; it also means `$INSTDIR` is under `%LOCALAPPDATA%\Programs`, a
  writable location, so the app updating itself in place is possible later.
- `!define BUNDLEID "app.knowlu.desktop"` — the installer inherits the identifier currently in
  `app/tauri.conf.json`. If the domain decision (`com.knowlu.desktop`) is meant to land, it has to
  land **before** the first installer reaches a friend, because the bundle id is the uninstall key.

The sidecar suffix strip was independently confirmed on disk: after the build,
`app/target/release/` holds both `knowlu.exe` and `quinn-ops.exe` (4,438,016 bytes, byte-identical
to the engine build) — the sibling layout `scheduler::engine_exe()` resolves.

## 7. What Task 9 should do

**Keep `cargo tauri build` as the one build command**: `scripts/release.ps1` builds the engine
(`cargo build --release`), copies it to `app/binaries/quinn-ops-x86_64-pc-windows-gnu.exe`, then
runs `cargo tauri build` from `app/` and picks up
`app/target/release/bundle/nsis/Knowlu_<version>_x64-setup.exe` — no hand-written `.nsi`, no
`NSIS_PATH`, no vendored NSIS.

## 8. Housekeeping

Committed: this report only.

Reverted, exactly as the brief required:

- `app/tauri.conf.json` — `git checkout --`; back to `"bundle": { "active": false, "icon": [...] }`.
- `app/binaries/` — deleted. The brief expected it left untracked, but it is not git-ignored yet
  (Task 9 does that) and it is a 4.2 MiB copy of build output in a tree that gets auto-committed, so
  leaving it there was the worse of the two. **Task 9 must re-create it** — one `cp` — and add
  `/app/binaries/` to `.gitignore`.

Not reverted (git-ignored build output, left as evidence): `app/target/release/bundle/`,
`app/target/release/nsis/`, `%LOCALAPPDATA%\tauri\` (NSIS 3.11 + the bootstrapper the bundler
cached).

## 9. Concerns for Task 9

1. **`cargo tauri build` touches `app/Cargo.toml`'s mtime.** It rewrites no bytes — the blob hash
   stayed identical to the index — but the stat cache is invalidated and `git status` then shows
   ` M app/Cargo.toml` until a `git update-index --refresh`. Combined with `core.autocrlf=true` and
   that file being bare LF in the working tree, a release script that asserts "clean tree" will trip
   on a file that has not changed. Assert with `git diff --quiet`, not `git status --porcelain`.
2. **The manifest count is still 2.** `app/README.md` says to fix this before shipping
   (`tauri-build`'s `WindowsAttributes::app_manifest`, verified by a count of 1). The bundle does not
   need it — but a signed installer around a binary with a merged-manifest warning is worth closing
   before the first friend gets one.
3. **The bundle id is `app.knowlu.desktop`**, not the `com.knowlu.desktop` the domain decision
   points at. Changing it after release orphans the uninstall entry.
4. **The installer is unsigned.** SmartScreen will warn every friend on first run. Trusted Signing
   is already started; the NSIS path takes a signing command per the Tauri config, so this is a
   config line, not a rework.
5. **The bundler downloads on a cold machine** (NSIS zip, `nsis_tauri_utils.dll`, the WebView2
   bootstrapper, all from GitHub/Microsoft, hash-validated). Fine here; on a machine without egress
   the build fails at "Verifying NSIS package". `release.ps1` should say so plainly if it does.
