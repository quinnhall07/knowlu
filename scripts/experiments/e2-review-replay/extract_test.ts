import { assertEquals, assertThrows } from "@std/assert";
import { corpusRootFromEnv, extractCorpus, PLAN_REVIEW_URL, resolvePaths } from "./extract.ts";
import { findLeaks } from "./severity.ts";
import { mentionsSecret } from "./secretfilter.ts";

// The corpus is the C1b review documents, which live outside this repository (git-ignored
// `.superpowers/` of whichever checkout holds them). These tests run only when `E2_CORPUS_ROOT`
// names that folder (and `--allow-env=E2_CORPUS_ROOT` lets this file read it); otherwise they
// ignore themselves rather than fail — no machine path is written here (CLAUDE.md rule 1).
const CORPUS_ROOT = corpusRootFromEnv();
const CORPUS_AVAILABLE = CORPUS_ROOT !== undefined;
const PATHS = { corpusRoot: CORPUS_ROOT ?? "", workspace: "", planReview: PLAN_REVIEW_URL };

Deno.test("resolvePaths takes the corpus root and workspace from flags, then the environment", () => {
  const env = (name: string) =>
    ({ E2_CORPUS_ROOT: "env-corpus", E2_WORKSPACE: "env-ws" } as Record<string, string>)[name];
  assertEquals(resolvePaths([], env), {
    corpusRoot: "env-corpus",
    workspace: "env-ws",
    planReview: PLAN_REVIEW_URL,
  });
  assertEquals(
    resolvePaths(["--corpus-root", "flag-corpus", "--workspace", "flag-ws", "--dry-run"], env),
    { corpusRoot: "flag-corpus", workspace: "flag-ws", planReview: PLAN_REVIEW_URL },
  );
});

Deno.test("resolvePaths names what is missing instead of guessing a machine path", () => {
  const none = () => undefined;
  assertThrows(() => resolvePaths([], none), Error, "E2_CORPUS_ROOT");
  assertThrows(() => resolvePaths(["--corpus-root", "c"], none), Error, "E2_WORKSPACE");
  assertThrows(() => resolvePaths(["--workspace", "w"], none), Error, "--corpus-root");
});

Deno.test("the plan-review document is read from this repository, not from a worktree", () => {
  assertEquals(PLAN_REVIEW_URL.href.endsWith("/docs/reports/2026-09-17-c1b-sign-in-plan-review.md"), true);
  assertEquals(Deno.statSync(PLAN_REVIEW_URL).isFile, true);
});

Deno.test({
  name:
    "extractCorpus finds 60 raw findings, drops exactly the 11 that name a secret/token/session/credential",
  ignore: !CORPUS_AVAILABLE,
  fn: async () => {
    const { kept, dropped, summary } = await extractCorpus(PATHS);

    assertEquals(summary.rawFound, 60);
    assertEquals(summary.expected, 59);
    assertEquals(kept.length + dropped.length, 60);
    assertEquals(dropped.length, 11);
    assertEquals(kept.length, 49);

    // Every id here was checked by hand against its source document, 2026-09-22 (see
    // docs/reports/2026-09-22-e2-review-replay-prep.md): each genuinely names the concept its
    // matched term says, including two that read as a false positive at first glance and are not —
    // A-C6 matches on Stripe's own proper noun "Checkout Session" (still, literally, naming a
    // session), and A-M4 matches on the literal env-var name `KNOWLU_ANON_KEY`. B-final-F2 is fix
    // round 1's own finding: it names "the tokens" (plural), which SECRET_TERMS's singular-only
    // patterns missed until the plural fix.
    const droppedIds = dropped.map((d) => d.id).sort();
    assertEquals(droppedIds, [
      "A-C2",
      "A-C5",
      "A-C6",
      "A-I2",
      "A-M4",
      "B-final-F11",
      "B-final-F2",
      "B-final-F7",
      "B-final-F8",
      "B-task1-2",
      "B-task3-3",
    ]);
  },
});

Deno.test({
  name: "every kept finding is leak-free and mentions no secret/token/session/credential",
  ignore: !CORPUS_AVAILABLE,
  fn: async () => {
    const { kept } = await extractCorpus(PATHS);
    for (const f of kept) {
      assertEquals(findLeaks(f.text), [], `leak in ${f.id}`);
      assertEquals(mentionsSecret(f.text), false, `secret mention survived in ${f.id}`);
    }
  },
});

Deno.test({
  name: "every kept finding has a disposition in one of the four named values",
  ignore: !CORPUS_AVAILABLE,
  fn: async () => {
    const { kept } = await extractCorpus(PATHS);
    const allowed = new Set(["fixed", "ruled_against", "handed_off", "deferred"]);
    for (const f of kept) {
      assertEquals(allowed.has(f.disposition), true, `${f.id} has disposition ${f.disposition}`);
    }
  },
});

Deno.test({
  name: "Set A keeps its 6/8/10 Critical/Important/Minor split minus the 5 dropped (C2, C5, C6, I2, M4)",
  ignore: !CORPUS_AVAILABLE,
  fn: async () => {
    const { kept } = await extractCorpus(PATHS);
    const setA = kept.filter((f) => f.set === "A");
    assertEquals(setA.length, 19);
    assertEquals(setA.filter((f) => f.severity === "critical").length, 3);
    assertEquals(setA.filter((f) => f.severity === "important").length, 7);
    assertEquals(setA.filter((f) => f.severity === "minor").length, 9);
  },
});
