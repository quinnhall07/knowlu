FAIL

# v0.4.0 wave A review: Home, Tasks with Details and the deck, Calendar

Base `d1844d04`. Diffs read with `git diff d1844d04..HEAD` in each worktree. Specs: plan §0, §1, §3, §4, §5;
note §4, §5, §8, §12; the mockups under `.superpowers/design/website/app/`. The main session runs the tests and checks;
this review ran git reads only. It looked at two shots (`t08-day` and `t11-calendar`, 1280x800 light).

## Verdicts

| Branch | Verdict | Why |
|---|---|---|
| `v040/t07-home` (T7, T8) | **FAIL** | Two Important findings, both small fixes in code: "Needs a date" opens an unfiltered Tasks, and the schedule rows carry the shell's `data-start` marker |
| `v040/t10-tasks` (T10, T14, T15) | **PASS** | Minor findings only, plus one decision (Sort by type) |
| `v040/t11-calendar` (T11) | **PASS** | Minor findings only, plus one decision (the `cm["from"]` workaround) |

## Important

**I1. Home's "Needs a date" link opens Tasks without the No date filter.** Fix in code, on the Home branch.
- `app/static/next/js/pages/home_aside.js:100` links `#tasks/undated`.
- Tasks applies a route argument only if it is in `DUE_ARGS = ["late", "today", "week", "none"]` (`tasks.js:29`, `:312`).
  `"undated"` is not in that list, so it is dropped.
- Note §4 gives the target as `#tasks?due=none`.
- Scenario: Waiting says "1 task needs a date" and the student presses it. Tasks opens on the full Open list with Due set
  to "Any due date", and the undated task sits in the "No date" group at the bottom, possibly below the fold.
- Both pins lock the mismatch in place. `ui_home_aside.rs:92` asserts `#tasks/undated`, `t08.py:196` checks the same
  href, and `ui_tasks.rs:158` pins the `none` value. Neither branch can see the other.
- Fix: link `#tasks/none` and update the pin and `t08.py:196`. Also add a cross-file check to T23a's `ui_cross.rs`: every
  `#tasks/<x>` link in a shipped script names one of Tasks' `DUE_ARGS`.

**I2. Home's schedule and free-time rows carry `data-start`, which plan §3 reserves for start controls.** Fix in code,
on the Home branch.
- `home_aside.js:165` (`<li class="ha-sc" data-start="10:10" ...>`) and `:197` (`<li class="ha-free-row" data-start=...>`).
- Plan §3 says `data-start` marks "every control that starts a source or a sync" and that each one takes
  `KnowluKit.owed()`.
- T20's re-homed `every_start_control_reads_the_owed_flag_and_a_throw_never_strands_it` checks every element marked
  `data-start` in any shipped script. `ui_cross.rs` holds the expected list.
- Scenario: when T20 or T23a lands, that pin goes red on `home_aside.js`. Someone then either allowlists a schedule row
  or adds `owed()` to it, which disables a list item while the privacy update is owed. Any owed-lock code that queries
  `[data-start]` also sweeps these rows.
- Fix: rename to `data-from` / `data-to` here and in `t08.py:145`, `:148`.

## Minor (fix in code unless marked)

- **M1. A card can be decided twice, once from the deck and once from its Details.** `deck.js:203-205`: `decideCard`
  checks only the clicked host's `dataset.busy`, never `BUSY[id]`. Scenario: Details for a card is open beside the deck.
  The student presses Approve on the deck card, then Reject in Details before the first answer returns. A second
  `decide` goes out, and its refusal comes back as an error toast. Fix: `if (BUSY[id]) { return; }` first.
- **M2. Tasks keeps the due filter across visits.** `tasks.js:312` sets `DUE` from the route argument and otherwise keeps
  the old value. Scenario: after following a due link, a later sidebar press on Tasks still shows only that group. Fix:
  reset `DUE` to `""` on mount when `ctx.arg` is empty.
- **M3. calendar.css styles generic class names with no `.cal` scope.** All page CSS loads into one document, and
  `calendar.css` defines `.day` (:125), `.lane` (:142), `.hr` (:172), `.off` (:174), `.now` (:177), `.menu-wrap` (:30) and
  `.obtn.ico` (:27-28) globally. Nothing collides yet. T18's Day editor or T13's class page using `.day`, `.now` or
  `.obtn ico` would pick up absolute positioning or a fixed width. Fix: scope these under `.cal`.
- **M4. Home's 10 s lane toast can lose its Undo early.** It is appended to `#toasts` by hand
  (`home_aside.js:226-240`). `KnowluKit.toast` evicts the oldest child once there are three (`kit.js:75`, shell). Scenario:
  the student removes a lane date, then checks three tasks within 10 s. The Removed/Undo toast is evicted, its timer still
  fires `remove_lane_date`, and Undo was never reachable. Accept for v0.4.0 and give `kit.toast` a duration after T23
  (note §11). That is a decision, not a page fix.
- **M5. Calendar shortcuts fire while focus is in the right panel.** `calendar.js:386-388` listens on `document`, and
  `typing()` skips only inputs. Scenario: Details is open with focus on its More button; pressing `1` switches the
  Calendar to one day. Fix: ignore keys whose target is inside `#panel` (or anywhere outside `#center`).
- **M6. The calendar envelope's `the_day` is not what the engine would produce.** In
  `envelopes/t11-calendar-{all,today}.json`, the free block 11:00-18:00 overlaps "Dentist" 13:00-14:00 and "Campus job"
  16:30-18:00, and the first free block has an unrounded `hours: 2.1666666666666665`. The generator says the free blocks
  are the base's and that the Calendar does not read them, which is true, so nothing drawn is wrong. But this is the one
  fixture where engine output was edited by hand. Fix: carve the free blocks around the inserted ones (or drop them), and
  never point a Home check at this set.
- **M7. The fixture server sends no CSP.** It sets no `Content-Security-Policy`, so the DOM checks cannot catch a CSP
  regression. Today's CSP allows inline styles (`tauri.conf.json:10`, `style-src 'self' 'unsafe-inline'`), so the
  Calendar's `style="--s:..."` attributes are fine. For T23 or after: have `serve.py` send the app's CSP. Not a page fix.

## Decisions for the main session (each with a recommendation)

1. **Tasks has no "Sort by type".**
   - The dispatch's ruling lists type, and the mockup has a Type column and option (`tasks/index.html:92`, `:105`).
   - Note §4's sort list leaves type out, and the engine's `Row` has no type field (`surface.rs:731-762`; `source_tag` is
     a source, not a type).
   - **Recommend:** ship without it for v0.4.0. A type sort would have to be invented on the page, and the page computes
     nothing.
2. **The Calendar dodges a shell scan with `cm["from"]`** (`calendar_data.js:59-60`).
   - `ui_shell.rs::no_mail_field_reaches_the_dom_without_h` flags any `.from` read outside `h()`, and a commitment's
     `from` is a date.
   - The workaround is harmless here: the value is compared, never drawn.
   - The smell is in the pin. A lexical scan that bracket access walks past also misses a real `c["from"]` in a mail
     renderer.
   - **Recommend:** keep the page as it is for wave A, with a one-line comment saying why. In T23a (`ui_cross.rs`,
     outside the shell's freeze):
     - narrow the mail-field read scan to card and row variables, the way the `title`/`why` scan already does with
       `c.`/`x.`;
     - flag any bracket read of a mail field (`["from"]`, `["evidence"]`, `["mail_date"]`), allowlisting only
       `calendar_data.js`;
     - then turn the Calendar back to `cm.from`.
3. **`window.KnowluMail.openApplied()` is a new contract between pages** (`home_aside.js:82`, `:277`). Plan §3 does not
   list it. Until T17 defines it, "From your email" never shows, even with a mailbox set up.
   **Recommend:** write it into T17's brief as a required export, and add a `ui_cross.rs` check that `mail.js` defines it.
4. **Home's own 10 s toast** (dispatch item). Acceptable as built: the pin `schedule_and_remove_lane_date_with_the_ten_second_toast`
   specifies it and `kit.toast` hard-codes 6 s. **Recommend:** a duration parameter in `kit.toast` after T23 (see M4).

## Checked and clean

- **Ownership.**
  - Each branch touches only its own placeholders, its own `ui_*.rs`, and its own `checks/tNN.py` and
    `envelopes/tNN-*.json`.
  - No shell file, no other page's file, nothing under `app/src` or the contract list.
  - `index.html` is untouched, so no `<script>` or `<link>` was added.
- **Security.**
  - Every `innerHTML` takes a module constant or a string built through `h()` (all six scripts read in full).
  - No inline handler, and no URL other than hash routes.
  - Every write goes through `KnowluData.call` (`set_fields`, `create_task`, `decide`, `answer_card`, `open_issue`,
    `set_body`, `delete_note`, `close_info`, `remove_lane_date`), all in data.js's `VIEWED` list.
  - Every check is `KnowluKit.checkHtml`, so it writes through the kit's `{progress: 100, status: "done"}` with Undo.
  - Details' progress edit writes `status: "done"` at 100 (`details.js:213`).
  - Mail fields reach markup only through `h()` (`deck.js:81-87`).
- **Rulings.**
  - No grade is drawn on these pages.
  - A done check fills with the class colour: shell `.ck[aria-pressed]` with the row's slot. A closed row's mark is ink
    because a close carries no course (A8).
  - Home has the domain row and today's schedule. Start by, slack and Won't fit appear only in Details (`details.js:111-119`).
  - Free time is `open_hours` plus the `free` blocks of the same `blocks` list. The shot agrees: 2 h 10 + 5 h 30 + 4 h
    against "11 h 42 min" is the engine's rounding.
  - Tasks has no Priority column and offers the Sort select (priority, due, name, class, estimate).
  - The Calendar is read-only (`ui_calendar.rs::calendar_is_read_only` is a real pin). Unconfirmed busy time shows on
    today only (`calendar.js:256`).
  - No chips, pills, rings, gradients, exclamation marks, "AI" or `title=` (`data-title` is data, never shown).
- **Rule 1.**
  - No real or mock names, classes or paths. The mockup codes CHEM 110, ENGL 210, MATH 220 and SPAN 201 do not appear.
  - The t07, t08, t10, t14 and t15 envelopes are built by running the engine on invented vaults
    (`make_base_courses.py` functions plus `knowlu-engine surface` and `write`).
  - t14's `note` answer is put together from engine outputs only.
  - t11's `your_week` has exactly the keys of `commitments::overview().to_json()` plus `week.rs`'s `registrar_label`, with
    invented commitments (ABC 101, DEF 210 by code, JKL 120, ZZZ 999 unmatched).
  - t11's `dropped_events` has `DroppedEvent`'s exact fields (`eventroster.rs:313-321`), using the engine's own reason
    strings.
- **Fit between pages.**
  - Details registers `task`, which Home, Tasks and the Calendar all open.
  - The deck registers through `KnowluSlots.addDeck`, and Home's Waiting calls `KnowluSlots.deck`.
  - The Calendar's `cal-thing` and Home's `overload` panels are each the page's own.
- **The DOM checks** run standalone because `run.py` belongs to T5c, and they reimplement its gates. Each covers all
  eight combos (2 sizes x 2 themes x 2 text sizes) and fails on a console error, a page error, page horizontal scroll or
  `#center` sideways overflow. t14 and t15 reuse t10's driver. Each then drives the key interactions:
  - Home: filter, over-by panel, Clear line, first run, no "Start by" text;
  - Tasks: Sort Due and back to the engine order, the filters, Done with Undo;
  - Details: edit and Escape, refused bounds, progress 100 with done, body editor and conflict, a poll leaving the draft
    alone, no typed text in `ui_event`;
  - deck: approve, reject, snooze, the double-click guard;
  - Calendar: 3 days below 1000 px, slots from `codes`, the keys, the stored Due fold, Not shown, check-off.

  These assert behaviour, not just that something mounted. `scripts/ui-fixture/README.md:33` still says page checks go
  into `run.py`. That is stale; T25 should correct it.

## Fidelity to the mockups

| Page | Departure | Acceptable for v0.4.0 |
|---|---|---|
| Home | The domain row filters Do today and Recommended, not Waiting (mockup `home/js/field.js:694` filters Waiting too) | Yes: note §4 rules |
| Home | The lead field's check is drawn in the field's text colour | Yes: identical to the mockup (`home/css/field.css:127`). The field itself is the class colour, so this is not a gray check (A8) |
| Tasks | A static column head plus a Sort select instead of sortable headers (`tasks/index.html:103-107`) | Yes |
| Tasks | No Type column and no Type sort | Decision 1 |
| Calendar | Controls in a sticky bar inside `#center`, not the top band (`calendar/index.html:60`, `.top.cal-top`) | Yes: pages may not edit the band (plan §3). It costs one row of scroll height |
| Calendar | Accepted events as rows in the Due lane, not timed blocks | Yes, and more honest: `ComingUp` has a `start` but no end (`surface.rs:1367-1382`), so a block would invent a length. `coming_up` is also the next five events only |
| Calendar | A sideways wheel jumps a whole day after half a column, not the mockup's smooth pan and snap (`calendar/js/calendar.js:842-857`) | Yes |
| Calendar | No Ctrl+wheel zoom (the mockup has it); Ctrl+= / - / 0 only | Yes: ruling 3 names the keys |
