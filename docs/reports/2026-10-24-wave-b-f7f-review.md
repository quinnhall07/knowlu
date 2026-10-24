# Wave B F7f review r1 (2ae4a08 vs fb6b104): FAIL

Reviewer: `reviewer`. Filed by the controller.

## Important
- **I-1:** with no course list, `homework_sites` was built over an empty list. Every link landed in `unmatched`, which F11p2 would render as "some assignments at your school link to…" from a preview with no courses. Ruling 4 says it should be `{}`.

## Minor
- m-1: a zone that jiff can't map on Windows reports `"UTC"`, not `null`. Accepted: it is truthful about the zone the counts used. F13 notes it.
- m-2: 98 lines against about 60. Accepted, as one coherent change.
- m-3: the authority didn't end at `,`, `;` or `!`.
- m-4: test 47 gaps (the `tz` argument, ruling 4, trailing dot, near midnight). Ruling 4 is now covered by a new test.

## Checked
- The preview is read-only, always exits 0, and writes through `dumps_value` with sorted keys.
- Bucketing is deterministic, and the past filter and the weekday counts use the same zone.
- The host parser handles userinfo, ports, case, embedded links and trailing dots. Nothing is a substring match.
- There are no reason strings, so nothing guessed is shown.
- No zone literal in src, no fixture touched, and no test edited.

## Fix round (dcedfef): controller-checked
- Two new tests went red first: `homework_sites_is_empty_without_a_readable_course_list` and `a_host_ends_at_a_comma_a_semicolon_or_a_bang`.
- The fix is two one-line changes in onboarding.rs: `map_or(json!({}), …)` and `,;!` added to the authority split.
- onboarding_cmd is 35/35, and the whole engine is green. One timing test, `judge_roster_paging_time_counts_against_its_budget`, flaked once under load; it passes alone and on a rerun.
