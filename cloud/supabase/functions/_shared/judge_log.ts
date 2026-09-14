import type { Db } from "./judge_db.ts";
import type { Cause, Kind } from "./judge_validate.ts";

/// One judgment, ready to record. **There is nowhere in this type to put a title, a body, a prompt
/// or a reply**, and that is the design (§5.6): the privacy property is structural rather than a
/// matter of care at each call site. `fields` is field name -> the literal that was (or would have
/// been) written, plus the four promotion features tier 2 is keyed on.
export interface JudgmentRow {
  account_id: string;
  kind: Kind;
  item_id: string;
  tier: number;
  outcome: string;
  cause: Cause | null;
  confidence: number;
  fields: Record<string, string>;
  model: string | null;
  prompt_version: string | null;
  grammar_version: string | null;
  prompt_hash: string | null;
  ms: number;
  origin: "device" | "gmail_api" | "events";
}

export interface JudgmentSink {
  /** The new row's id, so the reply can carry it and a later correction can name it. */
  write(row: JudgmentRow): Promise<string | null>;
}

export function judgmentSink(db: Db): JudgmentSink {
  return {
    async write(row) {
      // A log that cannot be written must not stop the judgment: the judgment is the work and the
      // row is the record of it. The error is reported, never thrown, and never carries the row.
      try {
        const written = await db.insert("judgments", row as unknown as Record<string, unknown>);
        return typeof written?.id === "string" ? written.id : null;
      } catch (e) {
        console.error(`judgments insert failed: ${e instanceof Error ? e.message : "unknown"}`);
        return null;
      }
    },
  };
}
