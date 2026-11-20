PASS with changes

# v0.4.1 pricing: r5 review, round 2 (last round)

Reviewer (spec/plan mode), 2026-11-20. Branch `docs/v041-pricing` at `cdfac539`. Scope: the r4 findings
(`2026-11-20-v041-pricing-r4-review.md`) against note §10 and plan §12 *Revision r5*, then only what r5
introduced. Authority: Amendment 2026-11-17 on `origin/main` (cloud design `:1234-1550`). Code claims checked in this worktree.

## Round-1 findings

| # | Status | Cite |
|---|---|---|
| I1 | resolved | note D13 `:502-506`; plan T1a `:349-351`, T5 `:660-666`. The legal spec's terms text has no other `$` figure (its liability cap says "zero"), so the tie can pass. |
| I2 | resolved | plan S2 (e) `:945-958` (paid period, N = 22, three reads, `account_id` still present); ledger `:827` |
| I3 | resolved as text, decision still open | note D14 `:539-544`, §8 item 15 `:1152-1159`; plan T3 `:494-501`, stop rule `:532-536`; S2 (d) `:940-944` |
| I4 | resolved | note D14 `:532-538`; plan T3 `:486-490` (both edges, the date source) |
| I5 | partly | one tag, `concurrency` and `environment: release` are in (note `:598-606`; plan T10 `:734-741`). How the job sees the tags is not specified: see N1 |
| I6 | resolved, with one gap | note D15 `:614-619`, D16 step 6 `:674-675`; plan T6 `:717-722`, §9 `:1085-1086`, step 14.4 `:1044-1047`. The hand restore skips the privacy guard: see N2 |
| m1 | resolved | note D9 `:348-357` (`UNREACHABLE` is `account.rs:175`; `provider_error` is `:210-217`); plan T4 `:563-569`, `:590-593` |
| m2 | resolved | note D13 `:462-476`, D8 `:320-321`, D16 `:659-663`; plan T1 `:398-400`, T4 `:579-580`. The scan is green today: no match for `\$\d+\.\d\d` in any `.js` or `.html` file under `app/static` |
| m3 | resolved | plan T3 `:509-514`. The wording elsewhere still overclaims: see N5 |
| m4 | resolved | note D16 step 1 `:648-651`; plan T0r question 10 `:209-210` |
| m5 | resolved | note D16 step 7 `:676-681` |
| m6 | resolved | note D14 `:551-553`, `:561-564`; plan T3 `:483-485`, `:505-508`, `:523` |
| m7 | resolved | plan T0r question 9 `:206-207`; checkpoint A `:329-330` |
| m8 | resolved | note `:28-41`. The cites match `origin/main`, including (8) to (10) at `:1444-1462`. No "draft" or "open item 2" is left |
| m9 | resolved | plan step 14.1 `:1039-1040`; note D16 step 0 `:640-641` |
| m10 | resolved | plan T3 `:491-493` |
| m11 | resolved | note D16 `:635-637` |

## New in r5

**N1 (Important, text fix). The newer-tag refusal can pass without checking anything.** The check is in note D15
`:598-601` and plan T10 `:737-738` ("any `v*` tag sorts after the Release's tag"). Neither says where the job gets
its tags or how it orders them. `actions/checkout`'s default `fetch-depth: 1` fetches no tags, and `release.yml`'s
publish job uses exactly that (`release.yml:287`).
- *Failure:* `site.yml` checks out `main` as `release.yml:287` does. `git tag -l 'v*'` then lists nothing, so the
  check never refuses. A failed v0.4.2 tag that carries a privacy bump then gets past the check, which is r4's I5(a)
  again.
- A lexical sort fails the same way once versions reach v0.10: "v0.10.0" sorts before "v0.9.0".
- The privacy comparison also needs the tag, but `git show <tag>:site/privacy.html` errors when the tag is missing,
  so that check fails closed.
- *Fix:* read the tags from the remote (`git ls-remote --tags origin`, or `fetch-depth: 0`), compare by version
  (`--sort=v:refname`), and pin both in `site_yml_refuses_a_newer_tag_or_a_running_release`.

**N2 (Important, text fix). The hand restore can publish a privacy page before any build carries it.** The
fallback is in note D15 `:615-617`, plan T6 `:719-720` and §9 `:1085`: "a hand restore … into a `main`
checkout's `site/releases/`, then the same deploy". It runs neither of `site.yml`'s guards.
- *Failure:* main holds a merged privacy bump that is waiting for its release, a state D16 step 0 (`:640-641`)
  admits. A mistaken hand deploy then drops the installer. `site.yml` refuses because the privacy pages differ.
  The named fallback deploys main's `privacy.html`, and that page is live while every installed build carries the
  old `PRIVACY_VERSION`. This is the C25 hazard that I5 closed.
- Step 14.4 is safe only because step 14.1 checks first.
- *Fix:* the hand restore takes `site/privacy.html` from the Release's tag, or stops when main's copy differs. Say
  so in D15, T6, §9 and T11.

**N3 (Minor, text fix). `environment: release` is the signing credential's OIDC subject.** The comment at
`release.yml:144-156` gives the Azure Trusted Signing federated credential's subject as
`repo:…:environment:release`.
- T10's permissions (`:755`) leave out `id-token`, so `site.yml` cannot mint a token today. But no pin holds this.
  A later edit that adds `id-token: write` would give a hand-dispatched run from main the release signer's
  identity.
- *Fix:* add a T10 pin that the permissions are exactly `contents: read, actions: read`. Note in D15 that a tag-only
  deployment rule on the `release` environment would refuse `site.yml`.

**N4 (Minor, text fix). "A cut moves a trial" holds only when N is small.** D14 `:542-543` says "the student pays
less than they agreed to". That happens only when the trial end is at least N × 24h away when the cut lands. With
14-day trials and N ≥ 15, no trial is ever inside the window, so the first charge is the old price, which is still
what the consent row records. Say this in D14 and in §8 item 15, so Quinn rules knowing it.

**N5 (Minor, text fix). D16 step 2 still says CI "finds any amount left in the billing code" (`:659-660`).** Since
m3, a bare cents literal such as `1299` is not scanned. Narrow the sentence to `$` figures, decimal figures and
`STRIPE_PRICE_` names. The handler tests' invented catalogs (1399) cover hard-coded amounts in behaviour.

**Checked and clean:**
- **The window.** It is exact. A 4-weekly renewal stays reachable after a trial converts for every N from 7 to 24.
  S2 (e)'s N = 22 puts the 28-day renewal inside the window.
- **The terms tie.** It is satisfiable.
- **The app scan.** It is green today.
- **`concurrency` and the release check.** They leave only a start-of-run race. A tag pushed after the check
  publishes only after its build, so it lands later and wins.
- **`workflows.rs:190`.** It counts `environment: release` in `release.yml` only, so `site.yml` does not trip it.
- **Privacy.** No private detail appears in either document.

## For Quinn

- **Q15 (I3), Quinn decision.** Does `move` change a trial's first charge? *Recommend:* as written: not on a raise,
  yes on a cut (N4's caveat applies).
- **Q14, Quinn decision.** It blocks T5 and T10. *Recommend:* yes, once N1 and N2 are written into D15 and T10.
- **N1 to N5: text fixes.** None needs a third round. The controller applies them, and `contract-reviewer` checks
  N1 and N3 in T10's diff.
