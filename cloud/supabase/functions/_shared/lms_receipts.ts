// Stream J Task T9: completion evidence from email, tier 2 — a deterministic recogniser for the
// templated "your submission was received" mail an LMS sends, run BEFORE the model so a receipt
// costs no model call (rules before the model, cloud design §5.2).
//
// It runs where the message text exists: `gmail-read`, server-side. The device never receives
// email text (D12, F-6), so there is no device-side twin of this file.
//
// **Only a submission receipt is completion evidence.** A "new grade posted" email is NOT: an
// instructor posting a zero for missing work sends exactly the same mail, and reading it as "done"
// would propose finishing precisely the work that was not done. "Overdue" and "due soon" mail is
// the opposite of evidence. Those shapes are left to whatever path they take today.
//
// Adding another LMS is a data change: one more row in `RECEIPT_TEMPLATES`. The code below never
// names a vendor.
import { validate } from "./judge_validate.ts";

export interface ReceiptTemplate {
  /** The LMS's name, as the card's `why` says it: "<label> submission receipt". */
  label: string;
  /** The vendor's own sending addresses, lower-case, compared exactly against the From address. */
  senders: string[];
  /** The subject, matched against the trimmed subject line. */
  subject: RegExp;
  /** The body, matched against the text with every whitespace run collapsed to one space. Must
   * capture the work's name as the named group `title`. */
  body: RegExp;
}

const WEEKDAY = "(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)";

export const RECEIPT_TEMPLATES: readonly ReceiptTemplate[] = [
  {
    // "<id>.<term> <course code>  Assessment submitted  <title>  Submitted: <weekday>, <date> <time>
    // <TZ>  Confirmation number: <hex>". The title is everything between the two markers, up to
    // the FIRST "Submitted: <weekday>," — a title may itself contain the word "Submitted".
    label: "Blackboard",
    senders: ["do-not-reply@blackboard.com"],
    subject: /^Submission received$/i,
    body: new RegExp(
      `\\bAssessment submitted (?<title>.+?) Submitted: ${WEEKDAY}, .*\\bConfirmation number: [0-9A-Fa-f-]+`,
    ),
  },
];

export interface ReceiptMessage {
  subject: string;
  from: string;
  text: string;
}

/** The address inside `Name <addr>`, or the whole header when there are no angle brackets. */
function senderAddress(from: string): string {
  const bracketed = from.match(/<([^<>]+)>\s*$/);
  return (bracketed ? bracketed[1] : from).trim().toLowerCase();
}

/** The work's name and the LMS that receipted it, or null when this is not a receipt we know. */
export function recogniseReceipt(message: ReceiptMessage): { title: string; label: string } | null {
  const sender = senderAddress(message.from);
  const subject = message.subject.trim();
  const text = message.text.replace(/\s+/g, " ").trim();
  for (const template of RECEIPT_TEMPLATES) {
    if (!template.senders.includes(sender) || !template.subject.test(subject)) continue;
    const title = text.match(template.body)?.groups?.title?.trim() ?? "";
    if (title !== "") return { title, label: template.label };
  }
  return null;
}

/**
 * A receipt as an email verdict, through the same `validate` every model answer goes through — so
 * the title is one-lined and clipped exactly as a model's would be. Null when the message is not a
 * receipt, and the caller asks the model as before. The `why` is templated: nothing of the body,
 * and never the confirmation number, reaches the verdict.
 */
export function receiptVerdict(
  message: ReceiptMessage,
  seed: Record<string, unknown>,
): Record<string, unknown> | null {
  const receipt = recogniseReceipt(message);
  if (receipt === null) return null;
  const checked = validate("email", {
    tier: "completion",
    title: receipt.title,
    course: null,
    due: null,
    effort_hours: null,
    importance: null,
    why: `${receipt.label} submission receipt`,
    confidence: 1,
  }, seed);
  return checked.ok && checked.verdict !== undefined ? checked.verdict : null;
}
