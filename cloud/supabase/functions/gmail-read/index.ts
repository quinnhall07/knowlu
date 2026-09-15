import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps, sharedDb } from "../_shared/judge_deps.ts";
import { accessTokenFromRefresh } from "../_shared/google_token.ts";
import { GMAIL_SCOPE } from "../_shared/google_scopes.ts";
import { GmailApiError, type GmailApi, READ_BUDGET_MS, readHandler, type TokenLookup } from "./handler.ts";

const GMAIL = "https://gmail.googleapis.com/gmail/v1/users/me";

/** The first `text/plain` part, base64url-decoded. Walks nested `multipart/*` and stops at text. */
function firstTextPart(part: Record<string, unknown> | undefined): string {
  if (part === undefined) return "";
  const mime = typeof part.mimeType === "string" ? part.mimeType : "";
  const body = part.body as { data?: string } | undefined;
  if (mime === "text/plain" && typeof body?.data === "string") {
    const b64 = body.data.replace(/-/g, "+").replace(/_/g, "/");
    return new TextDecoder().decode(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
  }
  for (const child of (part.parts as Array<Record<string, unknown>> | undefined) ?? []) {
    const found = firstTextPart(child);
    if (found !== "") return found;
  }
  return "";
}

// There is no `attachments` method here and no call to `.../attachments/`. Adding one would mean
// adding a method to `GmailApi`, which the handler test forbids by scanning the handler's source.
const api: GmailApi = {
  async list(accessToken, q) {
    const url = `${GMAIL}/messages?q=${encodeURIComponent(q)}&maxResults=100`;
    const response = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
    if (!response.ok) throw new GmailApiError(response.status, `gmail list ${response.status}`);
    const body = await response.json() as { messages?: Array<{ id: string }> };
    return (body.messages ?? []).map((m) => m.id);
  },
  async message(accessToken, id) {
    const response = await fetch(`${GMAIL}/messages/${id}?format=full`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    if (!response.ok) throw new GmailApiError(response.status, `gmail message ${response.status}`);
    const body = await response.json() as { payload?: Record<string, unknown> };
    const headers = (body.payload?.headers as Array<{ name: string; value: string }> | undefined) ?? [];
    const header = (name: string) =>
      headers.find((h) => h.name.toLowerCase() === name)?.value ?? "";
    return {
      subject: header("subject"),
      from: header("from"),
      date: header("date"),
      text: firstTextPart(body.payload),
    };
  },
};

Deno.serve(readHandler(requireActiveEntitlement, {
  api,
  async accessTokenFor(accountId): Promise<TokenLookup> {
    // R-C2-E41: a missing P2 (no Google client configured on this deployment at all) is checked
    // first and answers `missing: "config"` — never per-account, and never worth a DB round trip.
    const clientId = Deno.env.get("GOOGLE_CLIENT_ID") ?? "";
    if (clientId === "") return { missing: "config" };
    // The Gmail scope specifically: a grant that carries only `calendar.readonly` must read no
    // mail, and `read_google_grant` refuses rather than this function remembering to check.
    // R-C2-E39: reuse Task 10's shared token exchange — `invalid_grant` (a revoked grant, or the
    // 7-day testing-mode expiry) comes back as null and folds into `missing: "grant"`; any other
    // failure throws and surfaces as the handler's generic 500.
    const refresh = await sharedDb().rpc("read_google_grant", {
      p_account: accountId,
      p_scope: GMAIL_SCOPE,
    });
    if (typeof refresh === "string" && refresh !== "") {
      const token = await accessTokenFromRefresh(refresh, {
        clientId,
        clientSecret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
        fetch,
      });
      return token === null ? { missing: "grant" } : { token };
    }
    // The RPC answered nothing: either this account never connected Google at all, or it did and
    // the grant simply does not carry the Gmail scope (`p_scope` is checked inside the RPC —
    // Task 10 step 7). The two read as different things to the device (R-C2-E41): a calendar-only
    // grant must never be marked revoked over a Gmail step the student never took, so this
    // distinguishes them by whether a live `google_accounts` row exists at all.
    const rows = await sharedDb().select(
      `google_accounts?account_id=eq.${accountId}&status=neq.revoked&select=scopes`,
    ) as Array<{ scopes: string[] }>;
    return rows.length > 0 ? { missing: "scope" } : { missing: "grant" };
  },
  async excludedLabels(accountId) {
    const rows = await sharedDb().select(
      `google_accounts?account_id=eq.${accountId}&select=excluded_labels`,
    ) as Array<{ excluded_labels: string[] }>;
    return rows[0]?.excluded_labels ?? [];
  },
  async markRevoked(accountId) {
    await sharedDb().update(`google_accounts?account_id=eq.${accountId}`, { status: "revoked" });
  },
  async seen(accountId) {
    const rows = await sharedDb().select(
      `gmail_seen?account_id=eq.${accountId}&select=uid`,
    ) as Array<{ uid: string }>;
    return new Set(rows.map((r) => r.uid));
  },
  async markSeen(accountId, uid) {
    await sharedDb().insert("gmail_seen", { account_id: accountId, uid }, false);
  },
  async enqueue(accountId, uid, tier, payload, judgmentId) {
    await sharedDb().insert(
      "gmail_queue",
      { account_id: accountId, uid, tier, payload, judgment_id: judgmentId },
      false,
    );
  },
  async undelivered(accountId) {
    return await sharedDb().select(
      `gmail_queue?account_id=eq.${accountId}&delivered_at=is.null&select=uid,tier,payload&order=queued_at`,
    ) as Array<{ uid: string; tier: string; payload: Record<string, unknown> }>;
  },
  async deliver(accountId, uids) {
    const list = uids.map((u) => `"${u}"`).join(",");
    await sharedDb().update(
      `gmail_queue?account_id=eq.${accountId}&uid=in.(${list})&delivered_at=is.null`,
      { delivered_at: new Date().toISOString() },
    );
  },
  async knownCourses(accountId) {
    // Derived from the account's own history — there is no course table server-side, and inventing
    // one would mean the device syncing its `courses/` folder, which is C3's problem. A new
    // account has none, every `course` comes back null, and the first coursework sync fixes it.
    const since = new Date(Date.now() - 120 * 86_400_000).toISOString();
    const rows = await sharedDb().select(
      `judgments?account_id=eq.${accountId}&judged_at=gte.${since}&select=fields`,
    ) as Array<{ fields: Record<string, string> }>;
    const out = new Set<string>();
    for (const row of rows) {
      const course = row.fields?.course;
      if (typeof course === "string" && course !== "" && course !== "null") out.add(course);
    }
    return [...out];
  },
  pipeline: () => liveDeps("email", "gmail_api"),
  budgetMs: READ_BUDGET_MS,
  clock: () => Date.now(),
}));
