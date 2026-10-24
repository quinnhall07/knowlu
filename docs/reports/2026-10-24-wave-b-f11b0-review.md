# Wave B F11b0 review r1 (de7656d vs 639c2da): FAIL

Reviewer: `reviewer`. Filed by the controller. No file on the contract list is touched, and only `onboarding.js` changes.

## Important
- **I1. `feed_kept_on_device` makes a privacy claim the code can't guarantee.** The key comes from a *failed* store, and a timeout or 5xx error may have sent the link anyway. The sentence also reads like a chosen setting, while the privacy page says links are stored encrypted on Knowlu's servers. A student could fail to press Try again and lose the feed.
  - New words: "Knowlu couldn't save your calendar link to your account yet. It's kept on this computer until then. Press Try again."

## Minor
- **m1.** Panels can't clear `S.error` under the seam rule. Add `OB.clearError()`.
- **m2.** Poll errors now stay on screen until the next click. Keep them separate: `S.pollError` is set when `load()` gets `ok:false` and cleared when it gets `ok:true`. Display `S.error || S.pollError`.
- **m3.** `mail_not_yet`'s "will check again shortly" isn't true for `mail.first_pass`.
  - New words: "Knowlu is still waiting on your school email. It keeps checking."
- **m4.** The m2 pin is a single substring check. The controller strengthened it: any assignment to `S.error`, a `clearError` call or `Object.assign(S` inside `load` fails it.

## Checked
- Clearing on any click is fine.
- `mail_setup_cancelled` and `mail_setup_failed` are honest.
- The m3 `found` pin holds.
- The seam comment is correct.
- The m5 asserts each fail against 639c2da and pass now. Traced: mixed reads Done, all skipped reads Skipped, no records reads Not yet, and the account row reads Done.

## Fix round (f7a6c6f): controller-checked
I1 and m1-m3 are applied exactly as the review asked:
- `S.pollError` is set and cleared in `load()`, and `load()` never assigns `S.error`.
- `OB.clearError()` is exposed and named in the seam comment.
- The two reason sentences are reworded.

With the stronger m4 pin, `onboarding_static` runs 27 pass / 20 fail; the 20 belong to later tasks.
