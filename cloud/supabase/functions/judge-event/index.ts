// cloud/supabase/functions/judge-event/index.ts
//
// `origin: "events"`, so a judgment made for the events pass is distinguishable in `judgments`
// from one made for a task — which is what the eval's per-kind metrics and the correction-rate
// dashboards key on.
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps } from "../_shared/judge_deps.ts";
import { judgeHandler } from "../_shared/judge_handler.ts";

Deno.serve(judgeHandler("event", requireActiveEntitlement, (kind) => liveDeps(kind, "events")));
