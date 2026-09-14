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

function pickZybooks(source: Obj, timeZone: string, warnings: string[]): Assignment[] {
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
    requireSuccess(book.payload, `assignment payload for ${code}`);
    const routing = routeZybook(code, courses, ignore);
    if (routing.kind === "ignored") continue;
    if (routing.kind === "unmapped") {
      warnings.push(`zybook ${code} not in config; skipped`);
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
      for (const raw of Array.isArray(body.sources) ? body.sources : []) {
        const source = obj(raw);
        const name = String(source.name ?? "");
        const own: string[] = [];
        let items: Assignment[] = [];
        try {
          if (name === "zybooks") items = pickZybooks(source, timeZone, own);
          else if (name === "vhl") items = parseDashboard(String(source.html ?? ""), obj(source.config), own);
          else {
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
          warnings.push(`${name}: 0 assignments parsed; treating as failure`);
          continue;
        }
        assignments.push(...items);
      }
      return Response.json({ assignments, warnings });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-coursework: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "ingest failed" }, { status: 500 });
    }
  };
}
