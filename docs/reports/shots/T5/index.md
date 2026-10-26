# T5 shots (headless Edge, 1280x1100, `?fixture=`)

- `ds5.png` - staged state (`ds5-staged.json`, invented data, hand-staged from the s1 fixture): a row line
  ("Won't fit - due today ...") under two Must do titles, the `give_text` line under the capacity sentence,
  three spill blocks, and the key line "4.0h across 2 items with nowhere to go".
- Spill hover (a `title`, so not visible in a still; read from the DOM): the deadline item reads
  "<title> · 2.5h · Won't fit - due today 17:00; ..." (its row's `fit_text`, looked up by id); the other item
  reads "Demo Reading Notes · 1.5h · doesn't fit in today's hours".
- `ref-{s1,s1-migrated,full}-{before,after}.png` - the three reference fixtures rendered with the
  pre-change and post-change console: pixel-identical (difference bbox None for all three).
- Not shot: the drawer (needs the engine's `note`); it reads `fit_text`/`fit_why` from the rendered row object.
