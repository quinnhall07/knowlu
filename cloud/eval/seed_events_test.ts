// T2 (stream J): the synthetic event seed. Every case in `cloud/eval/seed/events.jsonl` must
// clear the two checks `loadSeed` (`./loader.ts`) already enforces on every committed seed line
// — `validateSeedRecord` (the record shape) and `scrubViolations` (no `@`, no URL, no home path,
// no run of 7+ digits, no string over 200 characters, no banned `request.item` key) — and the
// brief's own coverage bullets (T2 brief, `.superpowers/sdd/2026-09-22-judgment-quality-plan/
// t2-brief.md`): the two drop rules, a clear obligation/opportunity/drop, an empty description, a
// description clipped at the seed's own 200-character ceiling, a Spanish-language description,
// and — because J-1 ruled design (a), the fourth verdict word `unsure` — the unanswerable case.
//
// `loadSeed` already throws (naming the file and line) on any violation, so a plain `await
// loadSeed()` that resolves is itself proof every committed line passed both checks; this file
// additionally runs `validateSeedRecord`/`scrubViolations` per record directly, so a failure here
// names the record and the violation instead of only the file and line number.
import { assert, assertEquals } from "@std/assert";
import { loadSeed } from "./loader.ts";
import { scrubViolations, type SeedRecord, validateSeedRecord } from "./schema.ts";

async function eventSeed(): Promise<SeedRecord[]> {
  const all = await loadSeed();
  return all.filter((r) => r.kind === "event");
}

Deno.test("the event seed loads and is between 20 and 40 cases (T2 brief)", async () => {
  const events = await eventSeed();
  assert(
    events.length >= 20 && events.length <= 40,
    `expected 20-40 event cases, got ${events.length}`,
  );
});

Deno.test("every event case passes validateSeedRecord and scrubViolations individually", async () => {
  const events = await eventSeed();
  assert(events.length > 0, "no event cases were loaded");
  for (const record of events) {
    const shape = validateSeedRecord(record);
    assertEquals(shape, [], `seed-shape violations in ${record.id}: ${shape.join("; ")}`);
    const scrub = scrubViolations(record);
    assertEquals(scrub, [], `scrub violations in ${record.id}: ${scrub.join("; ")}`);
  }
});

Deno.test("every event id is unique and carries the seed- prefix", async () => {
  const events = await eventSeed();
  const ids = new Set<string>();
  for (const record of events) {
    assert(record.id.startsWith("seed-"), `${record.id} is missing the seed- prefix`);
    assert(!ids.has(record.id), `duplicate id ${record.id}`);
    ids.add(record.id);
  }
});

Deno.test("the audience drop rule is covered: faculty, staff, alumni or graduate students", async () => {
  const events = await eventSeed();
  const hit = events.filter((r) => {
    const audiences = r.request.item.audiences;
    if (!Array.isArray(audiences)) return false;
    const text = audiences.join(" ").toLowerCase();
    const namesRule = ["faculty", "staff", "alumni", "graduate"].some((w) => text.includes(w));
    return namesRule && r.theirs.verdict === "drop";
  });
  assert(hit.length >= 1, "no case covers the faculty/staff/alumni/graduate-student drop rule");
});

Deno.test("the standing/office-hours/recurring drop-in rule is covered", async () => {
  const events = await eventSeed();
  const hit = events.filter((r) => {
    const item = r.request.item;
    const seriesUid = typeof item.series_uid === "string" ? item.series_uid : "";
    const description = typeof item.description === "string" ? item.description : "";
    const title = typeof item.title === "string" ? item.title : "";
    const text = `${title} ${description}`.toLowerCase();
    const standing = seriesUid !== "" ||
      ["standing", "office hours", "drop-in", "recurring", "permanent"].some((w) => text.includes(w));
    return standing && r.theirs.verdict === "drop";
  });
  assert(hit.length >= 1, "no case covers the standing/office-hours/recurring-drop-in drop rule");
});

Deno.test("a clear obligation, a clear opportunity and a clear drop are each covered", async () => {
  const events = await eventSeed();
  for (const verdict of ["obligation", "opportunity", "drop"]) {
    const n = events.filter((r) => r.theirs.verdict === verdict).length;
    assert(n >= 1, `no case labels a clear '${verdict}'`);
  }
});

Deno.test("the unanswerable case (J-1 design (a): the 'unsure' verdict) is covered", async () => {
  const events = await eventSeed();
  const n = events.filter((r) => r.theirs.verdict === "unsure").length;
  assert(n >= 1, "no case labels the 'unsure' verdict");
});

Deno.test("an event with an empty description is covered", async () => {
  const events = await eventSeed();
  const n = events.filter((r) => r.request.item.description === "").length;
  assert(n >= 1, "no case has an empty description");
});

Deno.test("an event with a description clipped at the seed's own body limit is covered", async () => {
  const events = await eventSeed();
  const n = events.filter((r) => {
    const d = r.request.item.description;
    return typeof d === "string" && d.length === 200 && d.endsWith("[truncated]");
  }).length;
  assert(n >= 1, "no case has a description clipped at the 200-character seed ceiling");
});

Deno.test("a Spanish-language description is covered", async () => {
  const events = await eventSeed();
  const spanishWords = ["sesion", "informativa", "estudiante", "esta", "creditos"];
  const n = events.filter((r) => {
    const d = r.request.item.description;
    if (typeof d !== "string") return false;
    const text = d.toLowerCase();
    return spanishWords.filter((w) => text.includes(w)).length >= 2;
  }).length;
  assert(n >= 1, "no case has a Spanish-language description");
});

Deno.test("every event case's request.item carries exactly cloudmodel.rs's event_request keys", async () => {
  const events = await eventSeed();
  const expected = [
    "uid", "title", "start", "end", "source", "organizer", "location", "url",
    "description", "categories", "audiences", "series_uid",
  ].sort();
  for (const record of events) {
    const keys = Object.keys(record.request.item).sort();
    assertEquals(keys, expected, `${record.id}: request.item keys do not match event_request`);
  }
});
