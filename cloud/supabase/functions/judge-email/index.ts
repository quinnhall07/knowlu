// cloud/supabase/functions/judge-email/index.ts
//
// `origin: "device"` here, not "gmail_api": this endpoint is the one a NON-Gmail caller uses (the
// eval harness's parity check, and §13's forwarding fallback if it is ever built). The Gmail path
// judges inside `gmail-read`, where the origin is `gmail_api` and the export filter keys on it.
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps } from "../_shared/judge_deps.ts";
import { judgeHandler } from "../_shared/judge_handler.ts";

Deno.serve(judgeHandler("email", requireActiveEntitlement, (kind) => liveDeps(kind, "device")));
