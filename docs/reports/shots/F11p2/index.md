# F11p2 shots: the week strip and step 8's one-site form

Headless Edge, 1280x800, a scratch copy of `app/static`, the status staged through
`KNOWLU_SHOTS.startOnboarding(step, status)`. Invented data only (BIO 101, CS 201; counts
2/0/3/1/4/0/5, later 9; the week of October 19, today Wednesday). Prototype screens are
`5b-departures-board/index.html?shot=1&theme={dark|light}#{id}`. Each pair is `{name}-{dark|light}-built.png`
and `{name}-{dark|light}-prototype.png`.

| Name | Built | Prototype screen |
|---|---|---|
| `f3-school-strip` | step 3 with the strip | `o2p-proof` |
| `f5-email-strip` | step 5 with the strip | `o4-school-email` |
| `f6-google-strip` | step 6 with the strip | `o5-google` |
| `f8-found-and-notfound` | step 8, zyBooks found (codes in the order given) | `o7-zybooks` |
| `f8-vhl-first-found` | step 8, only VHL found, so VHL shows first | `o7-vhl` |
| `f8-vhl-not-found` | step 8, VHL reached by "Show VHL", no Found line | `o7-vhl` |
| `f8-unmatched-only` | step 8, zyBooks with `unmatched` only: the plain sentence | `o7-zybooks` |

## Intended differences

- Faces are Zilla Slab and PT Serif Caption, not the prototype's; wraps and heights differ.
- The strip is a row of bordered day cells with the counts beside the day name; past days (Mon, Tue)
  are dimmed and dashed with no count. The prototype's strip carries its own styling and, in places, titles; the built one is counts only.
- Steps 5 and 6 show a title and the why line only: their panels are F11d's and not built yet.
  Step 3's body is likewise F11b's; only the strip is new on all three.
- The label reads "Week of October 19" from `week.of`, in the strip's header.
- Step 8 shows one site's form; the other is a "Show {site}" link under the frame. The prototype's
  "Knowlu's form" tag is built as a small caps label inside the frame.
- The frame ends at the stored-only line; the "If you skip ..." line and the Sign in and Skip buttons sit outside it, as in the prototype. Two found codes read "CS 201 and BIO 101".
- "unmatched" never appears; the unmatched-only site gets "some assignments at your school".
- The prototype's syllabi sub-step and its rail sub-rows are not built.
