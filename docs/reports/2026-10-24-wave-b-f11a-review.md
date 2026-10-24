# Wave B F11a review r1 (f070dee vs c4b410f): FAIL

Reviewer: `reviewer`. Filed by the controller. No contract-list file is touched, and all line endings are LF.

## Critical
- **C1. The page never enters onboarding** (`onboarding.js:311`).
  - `launch_state` answers `"console"` in `run_console` even when `LaunchMode::Onboarding` is managed, because the `Onboarding` struct exists only in `run_shell`. So an unfinished setup boots the normal console, which has no tray, scheduler or queue.
  - `onboarding_status` is the wrong signal: it is not on a handler list yet, it answers `{ok:false}` until F4c, and it has no mode field.
  - Fix:
    - `launch_state`'s `None` arm answers `"onboarding"` when `LaunchMode::Onboarding` is managed.
    - `console.js`'s dispatcher returns early on `"onboarding"`.
    - Add pins for both.
    - Drop the z-index-40 stopgap.

## Important
- **I1. The 1 s tick re-enables every button.** `renderClosing` sets `disabled = closing` on every button, which undoes F11d's Open Knowlu gate and any in-flight disable. Fix: touch only buttons the frame itself disabled.
- **I2. A rail row can show "Not yet" or "Waiting" in done styling.** `m.cls` starts as `"done"` when `past` is set and is never reset. This is the rail's version of reading `(not yet:` as done.
- **I3. One failing leave hook blocks the exits.** A rejected promise makes `Promise.all` reject, so `onboarding_quit` never runs, and a hook that never resolves hangs the page. Fix: catch each hook and cap the wait at a few seconds.

## Minor
- **M1.** `openKnowluEnabled` reads `ok`, `gate.open` and `!exiting` as well as `finish_ready`, against §14.52.6. The F11t pin requires the `gate.open` case. The planner reconciles this before F11d.
- **M2.** `status.found` is optional, nothing produces it, and its reason is shown raw instead of through `reasonWords`.
- **M3.** Reason keys with no words: `mail_setup_cancelled`, `mail_setup_failed`, `mail_not_yet`, `feed_kept_on_device` (F11b).
- **M4.** `retryJob` drops a refusal silently.
- **M5.** `renderPane` rewrites `#ob-ask` and wipes what the student typed when a panel refreshes from `onStatus`.
- **M6.** `.su-row.done .su-st` is unscoped and recolours F10's shell rail. Scope it under `.ob`.
- **M7.** `revealModel` drops `gate.failed[].fix` (F11d).

## Holding
- Display only.
- The panel seam is enough once I1, I3 and M5 are fixed.
- The `console.js` change is safe for the normal console.
