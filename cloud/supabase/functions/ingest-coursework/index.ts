import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { ingestHandler } from "./handler.ts";

Deno.serve(ingestHandler(requireActiveEntitlement));
