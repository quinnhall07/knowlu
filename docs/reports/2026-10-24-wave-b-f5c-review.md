# Wave B F5c review: console commands for discovery and credential storage

Reviewer: `contract-reviewer` (read-only; the controller filed this report). Diff `34d652a..435c94e`.
**Verdict: PASS.** No contract-list file is touched. A trial merge onto 069c72d is clean: the
console list has 74 names and the shell list 31.

## Checked
- **The secret** is moved into `store_credentials` without a copy and lives as long as it does in
  the shell's command. No reply, log or format carries it.
- **The page can't pick a path.** The wrappers take no `vault`, so the target is exactly
  `target_for(id_for(cs.vault), source)`.
- **An unknown source is refused before any write**, including `session` and the pending-link
  names.
- **The target shape** is unchanged (`credentials::target_for`), and `wincred.rs` is untouched.
- **Handler lists:** the three new names are console-only, and the path-taking originals stay
  shell-only (test 43).
- **Async safety:** no lock, no await, and a profile switch can't race (`commands.rs:681`).
- **The tests** never write a real credential. Merging before F5 is safe.

## Findings and outcome
1. **Important, in the plan: F5 and F5b still move the path-taking `store_credentials` and
   `discover_coursework` onto the console list,** against §14.57.1 and F5c's pin. *Sent to the
   planner:* F5 and F5b are to be amended before F5 is dispatched.
2. **Minor:** `id_for(cs.vault)` against `settings.profile_id` could split for a non-derived profile
   id. *Carried to the plan* for whichever task wires `run_config_ingest_in`: use one derivation.
3. **Minor:** test 43's refusal case calls the real `store_credentials` with no `CREDMAN_LOCK` or
   guard, and stays off the Credential Manager only while the allowlist holds. Optional; *carried to
   the plan* for the same follow-up.
