# Security tooling (2026-11-09)

Source: the October 2026 "Knowlu Ship-Readiness Toolkit" research report (kept outside the repo).
Its rule: before the pilot, spend nothing — free scanners in CI, then a depth pass on our own surface.
This note records what is switched on, where it runs, and every exception with its reason.

## Switched on

| What | Where | Gate |
|---|---|---|
| CodeQL default setup (Rust, JS/TS, Actions, Python), extended suite | GitHub repo settings | Alerts in the Security tab |
| Secret scanning + push protection | GitHub repo settings | A push carrying a known key is refused |
| Dependabot alerts (no automatic PRs) | GitHub repo settings | Alerts only |
| `release` environment limited to `v*` tags | GitHub repo settings | A branch workflow naming `environment: release` gets no secrets |
| zizmor 1.32.0 (workflow security) | `security.yml` | Fails on any finding |
| actionlint 1.7.12 (binary, SHA-256 checked) | `security.yml` | Fails on any finding |
| gitleaks 8.30.1 (full history, binary, SHA-256 checked) | `security.yml` | Fails on any finding |
| cargo-deny (`deny.toml`) | `security.yml` | `bans licenses sources` always fail; `advisories` fail on main and the Monday run, report-only on a PR |

`security.yml` is not a required check; `ci.yml`'s three stay the required set. Every workflow is
covered by `engine/tests/workflows.rs`'s SHA-pin and `pull_request_target` tests, which now read
the whole `.github/workflows/` folder.

## Exceptions, each reviewed

- **gitleaks** (`.gitleaksignore`): 18 historical hits, all false positives — test keys in
  `*_test.ts`, a test AES key that decodes to "not-a-secret-32-byte-test-key-!!", placeholder JWTs
  in two plans, and the Supabase anon key in `app/src/account.rs` (role `anon`, public by design).
- **cargo-deny advisories** (`deny.toml`): RUSTSEC-2026-0367/0368/0369/0429, all in `zip`'s
  `extract`. The engine moved to zip 9 on 2026-11-09; only `tauri-plugin-updater`'s zip 4.x remains,
  which installs the NSIS .exe on Windows and checks the minisign signature first. Remove when the
  updater reaches zip >= 9.
- **zizmor `cache-poisoning`** on `release.yml`: fixed, not excepted. Quinn ruled (2026-11-09) that the release build
  restores no cache, so nothing a main CI run wrote reaches the signing job; `workflows.rs` asserts it.

## Fixed in this pass

- `rustls` 0.23.43 -> 0.23.46, `rustls-webpki` 0.103.15 -> 0.103.16, `webpki-roots` 1.0.9 -> 1.0.11
  (six advisories: distrusted intermediates, a key-exchange downgrade, handshake and Debug leaks).
- `clipboard-win` 5.4.1 -> 5.4.3 (the old version was yanked).
- `persist-credentials: false` on every `actions/checkout` (nothing after checkout uses git
  credentials; `gh release` reads `GH_TOKEN`).

## Findings

Leads from scanners and the breadth pass are tracked privately, never in this public repo, until
they are fixed. Once fixed, a finding is recorded in its commit message.
