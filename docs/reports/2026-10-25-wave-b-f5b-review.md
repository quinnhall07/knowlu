# F5b review: the shell's handler list

Reviewer: `reviewer` (read-only; the controller filed this report). Range: `ed0288e..9dd8280`. **PASS**, with two Minors.

## What changed
Ten names left the shell's list, which goes from 31 to 21:
- **Moved:** three, which were already on the console's list.
- **Retired:** three, which are now on neither list.
- **The plan's "may go" four:** removed exactly as listed.

## What the reviewer checked
- **Callers.** No file under `app/static` calls a removed name. The shell loads the same `index.html`, so no shell or account flow loses a command it uses.
- **Warnings.** None. The functions are `pub` in a library crate.
- **Pins and encoding.** No assertion was loosened, and `handler_lists.rs` is valid UTF-8 with LF line endings.

## Minor
1. **`capture_courses` stays on the shell's list.** It is uncalled and can only fail. The controller ruled to keep it, as recorded in the plan.
2. **The plan said "five commands move".** It now says three.
