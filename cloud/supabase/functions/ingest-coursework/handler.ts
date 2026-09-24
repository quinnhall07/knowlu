// POST /ingest-coursework — the device's raw portal payload in, reconciled items out.
//
// Never a credential: the device signs in with the student's own password out of Credential
// Manager and stops before parsing (D11, §4.3, §9). This function has no way to fetch anything and
// no place to keep a password, and the engine's `the_coursework_payload_carries_no_credential`
// test is the other half of the proof.
//
// *An empty parse is a failure, never an empty semester* — the rule `collect` has always kept: a
// 200 yielding nothing is far more likely to be a dead session than a term with no homework.
import type { Entitle } from "../_shared/judge_handler.ts";
import {
  type Assignment,
  NotLoggedIn,
  parseAssignments,
  requireSuccess,
  routeZybook,
} from "./parse_zybooks.ts";
import { parseDashboard } from "./parse_vhl.ts";

type Obj = Record<string, unknown>;

function obj(value: unknown): Obj {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Obj : {};
}

/// One thing the account's mapping does not know about (R-OB-1).
export interface MapProposal {
  source: "zybooks" | "vhl";
  /** What goes in the config: the zybook code, or the VHL section id. */
  key: string;
  /**
   * What a human reads on the card — **display only**. It is the vendor's own name for the book
   * (`UACS100Fall2026`, `VHL section 2102121`), which is exactly what the config's `label:` must
   * NOT be: that one prefixes every title the parser produces (`CS 100 HW 01`), so `write_mapping`
   * derives it from the course slug the student confirmed and never from this.
   */
  label: string;
  /** A guess the student confirms or replaces, or null when there is nothing to guess from. */
  suggested_course: string | null;
}

/** zyBooks' term words. Their presence at the end is what marks an institution-hosted code. */
const TERM = /(?:Spring|Summer|Fall|Winter)\s?\d{4}\s*$/i;

/**
 * A course slug from a zybook code, or null.
 *
 * zyBooks mints an **institution-hosted** book as `<II><DEPT><number><Term><Year>` —
 * `UACS100Fall2026` is the University of Alabama's CS 100 — and a plain catalogue book as
 * `<DEPT><number>`, `MATH125`. **The term suffix is the only marker there is**: nothing inside the
 * letter run says where `UA` stops and `CS` starts, and an unconditional strip would turn `MATH125`
 * into `th-125`. So the two-letter institution prefix comes off only when the code carries a term
 * AND the letter run is longer than a two-letter department could be. That leaves `MATH125` and
 * `PH106Spring2027` alone and turns `UAMATH120Fall2026` into `math-120`.
 *
 * A three-letter institution abbreviation would still guess wrong, and that is **allowed**: the
 * card shows the guess, the student confirms or replaces it, and `write_mapping` never writes a
 * value nobody confirmed. A wrong guess costs one tap.
 *
 * `null` when nothing matches, because an invented slug on a card is worse than a blank one: the
 * student would have to notice it was wrong rather than fill it in.
 */
export function suggestCourse(name: string): string | null {
  const m = /^([A-Za-z]{2,8})[\s_-]?(\d{3,4})(.*)$/.exec(name.trim());
  if (m === null) return null;
  const [, letters, number, tail] = m;
  const dept = TERM.test(tail) && letters.length > 2 ? letters.slice(2) : letters;
  return `${dept.toLowerCase()}-${number}`;
}

/** `section <id> not in config; skipped` — the one VHL warning that is really a proposal. */
const VHL_UNMAPPED = /^section (\d+) not in config; skipped$/;

function pickZybooks(
  source: Obj,
  timeZone: string,
  warnings: string[],
  proposals: MapProposal[],
): Assignment[] {
  const cfg = obj(source.config);
  const courses = obj(cfg.courses);
  const ignore = Array.isArray(cfg.ignore) ? cfg.ignore.map(String) : [];
  const out: Assignment[] = [];
  for (const raw of Array.isArray(source.books) ? source.books : []) {
    const book = obj(raw);
    const code = String(book.code ?? "");
    // Checked before routing, not after: a session can die between the device's item-list call
    // and this book's own assignment fetch, and that signal must not be swallowed by an unmapped
    // or ignored code — a dead session is a property of the session, not of any one course.
    requireSuccess(book.payload, `assignment fetch for ${code}`);
    const routing = routeZybook(code, courses, ignore);
    // `ignored` stays silent: `HowToUseZyBooks2` is zyBooks' own onboarding book, and a proposal
    // there would fire on every healthy run for every student.
    if (routing.kind === "ignored") continue;
    if (routing.kind === "unmapped") {
      // R-OB-1: never a warning. A book that authenticated and returned a payload and then
      // produced nothing is the failure the deck exists to surface.
      proposals.push({ source: "zybooks", key: code, label: code, suggested_course: suggestCourse(code) });
      continue;
    }
    out.push(...parseAssignments(
      book.payload,
      String(routing.mapping.course ?? ""),
      String(routing.mapping.label ?? ""),
      cfg,
      timeZone,
      warnings,
    ));
  }
  return out;
}

export function ingestHandler(entitle: Entitle): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      await entitle(req);
      let body: Obj;
      try {
        body = obj(await req.json());
      } catch {
        return Response.json({ error: "the body is not JSON" }, { status: 400 });
      }
      const timeZone = typeof body.timezone === "string" && body.timezone !== ""
        ? body.timezone
        : "America/Chicago";
      const assignments: Assignment[] = [];
      const warnings: string[] = [];
      const proposals: MapProposal[] = [];
      for (const raw of Array.isArray(body.sources) ? body.sources : []) {
        const source = obj(raw);
        const name = String(source.name ?? "");
        const own: string[] = [];
        let items: Assignment[] = [];
        // R-C1c-10: `proposals` is shared across every source in this request, so whether THIS
        // source found a proposal is what changed between these two counts — never whether some
        // other source in the same request did.
        const proposalsBefore = proposals.length;
        try {
          if (name === "zybooks") items = pickZybooks(source, timeZone, own, proposals);
          else if (name === "vhl") {
            items = parseDashboard(String(source.html ?? ""), obj(source.config), own);
            // The parser reports an unmapped section as a warning because that is what the Python
            // it was ported from did, and its behaviour is frozen against a reference. R-OB-1 is a
            // policy about what to DO with that, so it is applied here: the warning becomes a
            // proposal and is dropped from the list.
            //
            // One warning line per unmapped assignment row means the same section id can repeat
            // many times over (13, for this fixture) — `seenSections` is what turns that into one
            // card, while every matching line still comes out of `own`.
            const seenSections = new Set<string>();
            for (let i = own.length - 1; i >= 0; i--) {
              const m = VHL_UNMAPPED.exec(own[i]);
              if (m === null) continue;
              if (!seenSections.has(m[1])) {
                seenSections.add(m[1]);
                proposals.push({
                  source: "vhl",
                  key: m[1],
                  // The dashboard carries a section id and due dates and no course name at all, so
                  // there is nothing to guess from — and a guess would be worse than a question.
                  label: `VHL section ${m[1]}`,
                  suggested_course: null,
                });
              }
              own.splice(i, 1);
            }
          } else {
            warnings.push(`${name}: unknown source; nothing changed`);
            continue;
          }
        } catch (e) {
          const message = e instanceof Error ? e.message : String(e);
          warnings.push(
            e instanceof NotLoggedIn
              ? `${name}: session invalid (${message}); nothing changed`
              : `${name}: parse failed (${message}); nothing changed`,
          );
          continue;
        }
        warnings.push(...own.map((w) => `${name}: ${w}`));
        if (items.length === 0) {
          // R-C1c-10: an unmapped book or section is a question, not a failed parse — the parser
          // found work, and this source's own proposal (just pushed above) is the honest channel
          // for it. Only a source that produced neither an item nor a proposal is the dead
          // session or empty semester this rule exists to catch (coursework spec §9).
          if (proposals.length === proposalsBefore) {
            warnings.push(`${name}: 0 assignments parsed; treating as failure`);
          }
          continue;
        }
        assignments.push(...items);
      }
      return Response.json({ assignments, warnings, proposals });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-coursework: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "ingest failed" }, { status: 500 });
    }
  };
}
