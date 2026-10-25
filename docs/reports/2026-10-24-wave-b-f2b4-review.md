# Wave B F2b4 review: the week and homework sites in the answers file and status

Reviewer: `reviewer` (read-only; the controller filed this report). Commit 063a8ad against
merge-base 56e3ea7. **Verdict: PASS**, with three Minor findings. No contract-list file is touched.

## The six rulings: all accepted
1. **`week` null unless both fields are stored:** §14.58.3 "`null` without both"; tested.
2. **A site whose courses are all removed is absent:** matches F7f's "no evidence is absent";
   tested.
3. **A shared code is named once per site:** F11p2 puts the codes into a sentence.
4. **`homework_sites` null when empty:** acceptable; see m-1.
5. **An unknown key is dropped:** it is stale or foreign.
6. **`unmatched` goes out literally:** F11p2 keeps it off the page (§14.58.5 m-3).

## Other checks
- **Old files:** they load and re-save byte-identically (serde defaults, skipped when empty; test 48).
- **`STATE_VERSION`:** still 1.
- **Determinism:** the BTreeMap sorts sites. Codes follow the stored key order, by LMS key rather
  than by code, so F11p2 must not assume alphabetical codes.
- **Privacy:** the fields stay in the local answers file only, reach only the local webview, and the
  fixtures are fictional.

## Minor, and what the controller did
- **m-1:** "no sites" had two shapes, `null` and `{}`. *Fixed:* `status_for` now filters on the
  mapped result, so it is always `null`. Pinned in test 48.
- **m-2:** rulings 3 and 5 and an unmatched-only site were untested. *Fixed:* test 48 gains a shared
  code (QLT 101 for two kept courses), a stray key (`_9999_1`) and `{"vhl": ["unmatched"]}`.
- **m-3, for F8b's brief:** `save_preview` must replace all three fields on every preview, clearing
  them when the new answer lacks them, never merging.
