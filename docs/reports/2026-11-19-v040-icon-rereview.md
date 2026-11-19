PASS

# Re-review: v0.4.0 brand icon (feat/v040-icon)

Diff: `git diff b39f2744..9315c01c` (one commit, 9315c01c). First round: `docs/reports/2026-11-19-v040-icon-review.md`
(FAIL on I1, plus M1). The dispatch asked for this section to be appended to that report. Reviewer rules say each round is
a new file, so it is filed here instead.

Files: `app/icons/icon.ico`, `app/tests/brand_icons.rs`, `scripts/brand-icons.py`, `app/README.md`, and the round-one
report. No contract-list file, no `tauri.conf.json`, no engine file, no Tauri command, no workflow.

## I1: closed

**The directory rewrite is correct.** I parsed b39f2744's and 9315c01c's `icon.ico` field by field:

- Header: reserved 0, type 1, 10 entries, in both.
- New order: 32, 16, 24, 48, 64, 256, 20, 40, 96, 128. Entry 0 is 32x32.
- Each record's first 8 bytes (width, height, colour count, reserved, planes, bpp) are identical to the original record for
  that size. Reserved is 0 and bpp is 32 everywhere. Planes is 0 everywhere, as it was in the 11-17 original. That is valid
  for PNG entries, and it is not a change.
- Bytes-in-resource equals each blob's length. Offsets begin at 166 (6 + 16 x 10) and run back to back with no gap or
  overlap, and the last blob ends at 22913, the file length.
- Every blob is a PNG. Its SHA-256 matches the blob of the same size in the original, so no image was re-encoded.
- Pillow (`python -I`) opens the file and decodes all ten sizes as RGBA at their nominal sizes.

**The script is correct and idempotent.** Calling `ordered_ico` from 9315c01c's `scripts/brand-icons.py` on b39f2744's
file gives back the committed file byte for byte, and running it again on the committed file changes nothing. It exits
before writing on a bad header, an entry that points outside the file, or two entries of the same size. `tray-ok.rgba` and
`tray-warn.rgba` are unchanged from b39f2744. That is expected: the 32 px blob they derive from is the same bytes.

**The hash pin is sound.** `sha2` is a normal dependency of the app crate, so the integration test can use it. The test
collects every (size, SHA-256) pair and checks them against all ten `HIGH_DEF_ENTRIES` with exact equality. So an 11th
entry, a missing entry, a re-encoded entry or a swapped size fails it. I checked each of the ten pinned hashes against the
committed file's blobs, and the original's blobs carry the same hashes. `sizes.first() == Some(&32)` pins the property the
window icon depends on. The existing loop still checks every entry's offset and length against the file before hashing,
so a bad offset is reported as "points outside the file" and never panics on a slice.

The controller's live check (WM_GETICON ICON_SMALL now 32x32 and drawing the red tile) matches the file: entry 0 is the
32 px PNG.

## M1: closed

`app/README.md` now says "the nine menu items". The `icons/` row now describes the reorder and the script's part in it
correctly.

## Nothing else

The diff touches nothing outside the files listed above. The changed text files are LF in the index and the working tree,
`git diff --check` is clean, and `*.ico` and `*.rgba` stay `binary`. There are no single-user assumptions, no frozen
references, no engine or app invariants, nothing in cloud, and no new data class. I did not run `cargo test` (reviewer
rules), so the controller's green run is the gate for 0 warnings.

No findings.
