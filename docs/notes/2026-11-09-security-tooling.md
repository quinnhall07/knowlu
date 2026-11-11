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
| Supabase Security Advisor's security lints, run by `supabase db advisors` (the CLI `cloud-rls` already pins and checksums) on the local database after pgTAP | `ci.yml`, job `cloud-rls` | Fails on any warning. Its lint set lacks 0028/0029 (definer functions a client role may execute); pgTAP's F1 and F2 read those |

`security.yml` is not a required check; `ci.yml`'s three stay the required set. Every workflow is
covered by `engine/tests/workflows.rs`'s SHA-pin and `pull_request_target` tests, which now read
the whole `.github/workflows/` folder.

2026-11-11, the first read of production's advisor: production and staging are identical. It
reports 26 informational `rls_enabled_no_policy` lints, which are the service-only tables, by
design, and one warning, `auth_leaked_password_protection`. That one is an Auth setting rather than
SQL, so the local gate does not cover it; it is left to HANDOFF's Pilot row "C1b's open sign-in
settings".

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

## Fixed 2026-11-11

- Client roles (`anon`, `authenticated`) no longer hold TRUNCATE, REFERENCES, TRIGGER or MAINTAIN on
  any `public` relation, nor through `postgres`'s default privileges for new tables. They also no
  longer hold EXECUTE on the two trigger functions `sync_notes_stamp_rev` and `sync_usage_bump`.
  Migration `20261111000100_client_table_rights.sql`, applied to staging and production on
  2026-11-11 (production 15:11Z); pgTAP checks C10 and C11 and the tightened F2 now hold this.
- The static definer guard in `migrations_test.ts` now requires PUBLIC as well as the two client
  roles in the revoke. It matches argument signatures (overloads) and fails on a later re-grant.
- Cargo-run test processes force `NO_PROXY=127.0.0.1,localhost` (`.cargo/config.toml`), so a
  developer's `HTTP_PROXY` cannot send loopback tests off the machine. Deno's loopback tests are not
  covered by cargo's `[env]`.
- GitHub secret-scanning alert #1 was resolved 2026-11-11 as used in tests. It was a fake key in the
  scrubber tests, already listed in `.gitleaksignore`.

## Findings

Leads from scanners and the breadth pass are tracked privately, never in this public repo, until
they are fixed. Once fixed, a finding is recorded in its commit message.
