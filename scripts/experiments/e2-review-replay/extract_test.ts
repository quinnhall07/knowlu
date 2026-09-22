import { assertEquals } from "@std/assert";
import { extractCorpus } from "./extract.ts";
import { findLeaks } from "./severity.ts";
import { mentionsSecret } from "./secretfilter.ts";

function pathExists(path: string): boolean {
  try {
    Deno.statSync(path);
    return true;
  } catch {
    return false;
  }
}

const CORPUS_AVAILABLE = pathExists(
  "C:\\Users\\danie\\GitHub\\knowlu\\.claude\\worktrees\\c1b-sign-in\\.superpowers\\sdd\\2026-09-17-c1b-sign-in-plan",
);

Deno.test({
  name:
    "extractCorpus finds 60 raw findings, drops exactly the 11 that name a secret/token/session/credential",
  ignore: !CORPUS_AVAILABLE,
  fn: async () => {
    const { kept, dropped, summary } = await extractCorpus();

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
    const { kept } = await extractCorpus();
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
    const { kept } = await extractCorpus();
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
    const { kept } = await extractCorpus();
    const setA = kept.filter((f) => f.set === "A");
    assertEquals(setA.length, 19);
    assertEquals(setA.filter((f) => f.severity === "critical").length, 3);
    assertEquals(setA.filter((f) => f.severity === "important").length, 7);
    assertEquals(setA.filter((f) => f.severity === "minor").length, 9);
  },
});
