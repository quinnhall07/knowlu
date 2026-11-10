# Prompt-injection suite plan: review r2

**Verdict: FAIL.** No critical findings and 8 important ones, most of them in §4.3. The reviewer's
full text, with line references, went to the planner for r3. What follows is the summary.

**r1 findings.** These are resolved: C1, I3, I4 (as Q6), I5, I6, M1, M2, M3 and M5. Two are partly
resolved: I1 (see I7 below) and M4 (see I2 below). I2 is not resolved (see I1 below).

## Important

- **I1. The TLD rule contradicts itself.** None of the eight colliding names is on the TLD list. Make
  the rule explicit for a final label followed by `/`.
- **I2. A placeholder can recreate markup.** A placeholder next to `(` or `!` forms new markup. Fix:
  run the policy again after neutralising and refuse the answer if anything is still flagged. Add rows
  for these adjacent characters.
- **I3. The scrub's own tokens read as markup.** `scrub.ts` writes tokens such as `<url>` and
  `<email>`, which the markup rule matches. Exempt those exact tokens in both the oracle and the
  enforcement, or honest tasks will be refused.
- **I4. Ordinary prose trips the scheme rule.** `data:` and `file:` without slashes appear in plain
  text. Flag `data:` only before a MIME-type shape or `base64`, and `file:` only before `/` or `\`.
- **I5. Normalise before matching, and use a full TLD list.** Fold case, apply NFKC and strip
  zero-width and bidi characters, as `plantedIn` does. Use a dated IANA root-zone snapshot for
  enforcement and an open-ended TLD rule in the oracle.
- **I6. The line-conversion step has no owner.** Make it an explicit T2b step that runs before
  `oneLine`, and describe it as target behaviour.
- **I7. Holding back the push doesn't prevent disclosure.** Squash the branch. Land T2b as its own PR
  and deploy it before the suite branch is pushed, or deploy to staging on the same day.
- **I8. Totals still reveal new fixtures.** CI prints only "critical: yes/no", calls and spend.
  Fixtures are added in batches after a local `--per-case` run.

## Minor

- **M1.** T2c and T10: derive values from production, and carry the boundary forward in HANDOFF.
- **M2.** Clip to 140 characters after neutralising, on the server.
- **M3.** H9 also covers a template's `item.title`. H9 is green from the start.
- **M4.** Reword §4.3 at line 189 as a scope decision.
- **M5.** The H5 `evidence` case must survive the verbatim check.
