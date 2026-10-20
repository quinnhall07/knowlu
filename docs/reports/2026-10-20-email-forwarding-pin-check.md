# Email forwarding: the pin check (T18b), 2026-10-20

Email spec §12.1 test 17, plan T18b. The controller ran it live against OpenRouter twice: once on the pins the
migrations held, then again after the re-pin. Every message and course batch is invented: example.edu,
example.com and example.net addresses, and the courses WDG 101, GZM 205, THG 310 and LNG 120. No student's
mail, course or address went into a request. The harness and both result files are outside the repo, in the
controller's session scratchpad (`t18b/`, with `results.json` and `results-repin.json`).

## How it ran

- **mail_claim:** 23 messages, built as RFC 822 bytes, parsed by production's `mime.ts`, then sent through
  production's `modelPath` (`mail-process/model.ts`) unchanged.
- **mail_resolve:** 12 course batches POSTed to production's `resolveHandler` (`mail-resolve/handler.ts`), in
  plan T4.5's wire (09fc1e2).
- **Requests:** 35 per run, against a hard cap of 50. Calls were sequential, with no retries, through a tap that
  refuses any host but OpenRouter.
- **Key:** the dev key at Credential Manager `knowlu/dev/openrouter`, read by the controller only. It was never
  printed or written.
- **Pins:** the migrations' values (no `--rows`). The route was `DeepInfra` only, `zdr`, no fallbacks, and
  DeepInfra served every call.

## Results

| | Run 1, old pins | Run 2, re-pinned |
|---|---|---|
| Started | 2026-10-20 01:15Z | 2026-10-20 02:17Z |
| mail_claim cap | 1024 | 1536 |
| mail_claim largest output | m21, 1008 | m21, 1025 |
| mail_claim valid | 22/23 | 22/23 |
| mail_claim matched | 19/23 | 19/23 |
| mail_resolve cap | 640 | 1280 |
| mail_resolve largest output | r10, 640 (`length`) | r10, 947 |
| mail_resolve truncated | 1 (r10: 0/15 answered) | 0 (r10: 13/15) |
| mail_resolve matched | 10/12 | 10/12 |
| Slowest call | 7.96 s | 6.05 s |
| Calls over production's 30 s limit | 0 | 0 |
| Cost of the run | $0.0092 | $0.0095 |

**Pin conclusions**

- **mail_resolve at 640 truncated.** A full 15-claim batch, the server's `MAX_CLAIMS`, stopped at `length` with
  nothing answered.
  - At 1280 it used 947, which is 74% of the cap.
- **mail_claim at 1024 was not safe.** The same five-claim stress message (m21) came in at 1008 in run 1 and 1025
  in run 2. Greedy decoding still varies by a token or two between calls, so 1025 is over the old cap.
  - At 1536 it used 1025, which is 67% of the cap.
- **The fix:** Quinn ruled the re-pin on 2026-10-20. It is `20261020000100_mail_max_tokens_repin.sql` (42d4d12):
  one statement, where only `max_tokens` and `since` move.

## What still misses (unchanged by the re-pin)

These misses are about accuracy, not the caps. They are the same messages in both runs:

- **m05, m07:** a due-date miss.
- **m16:** keeps a change claim the case forbids.
- **m20:** queues a row where none belongs.
- **m08:** the due resolver reads its date in UTC rather than the course's zone, already known.
- **r11:** answers null where the batch holds the target.
- **r10, run 2:**
  - one claim gets no answer where the target was there;
  - one claim is matched to a neighbouring item (`…e002` for `…e003`). A wrong target is worse than null: it
    proposes a change to the wrong note. It reaches the student only as an amend card, never a silent write.

## Where this is not production

The harness README lists the gaps. The ones that matter here:

- It used a 60 s timeout, not production's 30 s; no call came near that.
- It used the dev key, not staging's secret.
- The screen and templates are skipped.
- The resolve claims are fixed inputs, not chained from mail_claim's answers.
- The pins are the migrations' values. Before the prod push, read staging's rows (see below).

## Before the prod push

- Confirm on staging that `20261005000100` and `20261020000100` are applied. Then read the two `models` rows, and
  re-run with `--rows` if they differ from the migrations.
- Once `20261020000100` is applied, give it a SHA-256 "unchanged" test like the other applied mail migrations.
- Add `20261020000100` to HANDOFF's prod-push migration list, after `20261005000100_mail_claim_max_tokens.sql`.
