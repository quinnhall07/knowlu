import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { eventsHandler } from "./handler.ts";

Deno.serve(eventsHandler(requireActiveEntitlement));
