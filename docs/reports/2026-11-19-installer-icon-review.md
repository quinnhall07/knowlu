PASS

# Installer icon review (2026-11-19)

Branch `fix/installer-icon` (PR #81), `git diff origin/main..HEAD`, one commit `da7f16c5`. Size: small.
Files: `app/tauri.conf.json` (1 line), `app/tests/bundle_config.rs` (+6). No contract-list file.

## Findings

None.

## What was checked

1. **Right field.** tauri-utils 2.9.3 `src/config.rs:844-866`: `NsisConfig` is
   `#[serde(rename_all = "camelCase", deny_unknown_fields)]` and has
   `installer_icon: Option<PathBuf>` (serialized `installerIcon`; alias `install-icon`). The
   spelling in `app/tauri.conf.json:20` is the one that type accepts. The path is resolved against
   the tauri dir (`app/`), the same base as `bundle.icon`: the generated
   `target/x86_64-pc-windows-gnu/release/nsis/x64/installer.nsi` (from the controller's 15:39 dry
   run) has `!define INSTALLERICON "...\app\icons\icon.ico"` at line 42, which becomes `MUI_ICON`
   at line 127. That absolute path exists only in the generated build output, which is not in the repo.
   `app/icons/icon.ico` is tracked and marked binary.
2. **Updater.** The icon is compiled into the setup exe by makensis. Signing happens after that,
   and so does the minisign `.sig`, which covers the exe's final bytes. The artifact's shape (a bare
   `*-setup.exe` plus `.sig`, from `createUpdaterArtifacts: true`), its name, the `plugins.updater`
   endpoint and the pubkey are unchanged. release.ps1's `$wantsUpdater`/`$hasUpdaterPlugin` logic
   (lines 160-173) reads only `createUpdaterArtifacts` and `plugins`. Note: `-DryRun` turns updater
   artifacts off through `--config`, so the dry run did not build a `.sig`. Nothing in this diff
   touches that path.
3. **Signing and the uninstaller.** `signCommand` is unchanged. The generated script still has
   `!define UNINSTALLERICON ""` (line 45), so `MUI_UNICON` stays undefined and the uninstaller keeps
   the MUI default icon it had before. `!uninstfinalize` (line 96) and the `DisplayIcon`
   registry value (line 676, the main exe) do not depend on the installer icon.
4. **Other readers of the nsis block.** `.github/workflows/release.yml` does not read
   `tauri.conf.json`'s nsis block. `scripts/release.ps1:54` parses the file and reads only `version`,
   `createUpdaterArtifacts` and `plugins`. The `--config` override deep-merges over `bundle`, so
   the `nsis` keys survive. `scripts/sign.ps1` takes a fixed argv.
5. **Gates.** The test edit adds assertions only: it pins `installerIcon` and that
   `bundle.icon[0]` is the same file, and it changes no existing assertion. Neither file has a CR,
   so both stay LF. The diff adds no `#[ignore]` and no workflow change.
6. **Rules 1, 2 and privacy.** The diff names no user, vault, machine or credential. It touches no
   fixture and no new data class.

The controller's evidence was a dry run (exit 0, 6.52 MB) in which every extracted icon size,
16 to 256 px, draws the brand mark. That matches the generated script.
