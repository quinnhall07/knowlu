// The entry point. Five lines, and the only file in this directory — it imports C1's entitlement
// module and the shared handler, and reaches a project through neither of its own.
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps } from "../_shared/judge_deps.ts";
import { judgeHandler } from "../_shared/judge_handler.ts";

Deno.serve(judgeHandler("task", requireActiveEntitlement, (kind) => liveDeps(kind)));
