# T5 shots (headless Edge, 1280x1100, `?fixture=`)

Both fixtures are real engine `surface` output (invented data, scratch vaults), not hand-staged.
The hand-staged `ds5-staged.json` / `ds5.png` are removed.

- `demo-d-2100.png` (dark), `demo-d-2100-light.png` (light) - `demo-d-2100.json`: three tasks due 23:59, read
  at 21:00. `spill_blocks` is 0, `spill_items` is Task Y (2.5h, with a `deadline`). Shows: Task Y's row line
  "Won't fit before 11:59 pm" (Task X and Task Z have none); the give line under the capacity sentence,
  "If something has to give: Task Y (2.5h). Everything else fits by its deadline."; the meter key
  "16 blocks fit inside the day" then "1 item won't make its deadline" (no "0 blocks do not"; no spill blocks).
- `demo-d-2100-why.png` - the same fixture with `.why` opened on Task Y's row: the why line plus
  `fit_why`, "2.5h left; doing it would leave other work due by 11:59 pm late."
- `demo-s1-1930.png` - `demo-s1-1930.json`: the s1 vault plus a task, read at 19:30. Must do in three runs
  (Overdue 1 / Due today 1 / Overdue 1), no row fit lines, no give line, no spill items; the key still reads
  "16 blocks fit inside the day" and "0 blocks do not" (unchanged: no spill items).
- Hover strings (titles, so not visible in a still; read from the DOM with `--dump-dom`):
  - demo-d-2100, key span: "Task Y · 2.5h · Won't fit before 11:59 pm" (the row's `fit_text`, looked up by id).
    There are no spill blocks, so no block titles.
  - demo-s1-1930: none (no spill items).
- Key wording (`renderMeter`): "0 blocks do not" is dropped only when `spill_blocks` is 0 and every spill item has
  a `deadline`. Deadline items read "1 item won't make its deadline" / "N items won't make their deadline";
  items without one keep "N items with nowhere to go"; a mix reads
  "N items: A with nowhere to go, B won't make their deadline".
- `ref-{s1,s1-migrated,full}-{before,after}.png` - the three reference fixtures with the pre-T5 console and
  the T5 console: pixel-identical. Re-rendered with the key fix and compared with the `-before` files:
  difference bbox None for all three.
- Not shot: the drawer (needs the engine's `note`); it reads `fit_text`/`fit_why` from the rendered row object.
