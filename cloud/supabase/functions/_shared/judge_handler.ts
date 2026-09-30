// The one handler all three `/judge-*` functions are. It computes nothing: it checks the
// entitlement, reads the body, hands it to the one pipeline and returns what came back. A handler
// that computed anything would be a second place to fix a bug, and there are three of these.
import { judge, type JudgeRequest, type Kind, type PipelineDeps } from "./judge_pipeline.ts";

export type Entitle = (req: Request) => Promise<{ account_id: string }>;

export function judgeHandler(
  kind: Kind,
  entitle: Entitle,
  deps: (kind: Kind) => Promise<PipelineDeps>,
): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") {
      return Response.json({ error: "POST only" }, { status: 405 });
    }
    try {
      const { account_id } = await entitle(req);
      let body: JudgeRequest;
      try {
        body = await req.json() as JudgeRequest;
      } catch {
        return Response.json({ error: "the body is not JSON" }, { status: 400 });
      }
      if (body?.kind !== kind || typeof body.item !== "object" || body.item === null) {
        return Response.json({ error: `expected kind '${kind}' and an item object` }, { status: 400 });
      }
      const seed = typeof body.heuristics_seed === "object" && body.heuristics_seed !== null
        ? body.heuristics_seed
        : {};
      // Only a list of strings declares anything (final review item 2's `unsure` gate).
      const accepts = Array.isArray(body.accepts)
        ? body.accepts.filter((word): word is string => typeof word === "string")
        : [];
      // Only a non-empty string names a timezone; the due resolver ignores one `Intl` does not know.
      const timezone = typeof body.timezone === "string" && body.timezone !== "" ? body.timezone : undefined;
      const reply = await judge(
        account_id,
        { kind, item: body.item, heuristics_seed: seed, accepts, timezone },
        await deps(kind),
      );
      return Response.json(reply);
    } catch (e) {
      // `requireActiveEntitlement` throws a Response (401 / 402); everything else is ours.
      if (e instanceof Response) return e;
      // The class, never the message: a provider's error string and a connection error's text are
      // the two places a request body can come back out, and this line reaches a log (§5.6).
      console.error(`judge-${kind}: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "judgment failed" }, { status: 500 });
    }
  };
}
