// R-C2-E24: `read_google_grant` returns the REFRESH token; a calendar (or Gmail) read needs an
// ACCESS token. This is the one place that exchange happens, so a calendar read and a mail read
// never each grow their own copy of it.
const TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";

export interface GoogleTokenDeps {
  clientId: string;
  clientSecret: string;
  fetch: typeof fetch;
}

/**
 * A fresh access token from a stored refresh token.
 *
 * Returns `null` on `invalid_grant` — a revoked grant, or the 7-day testing-mode expiry (§9) — so
 * `calendarTokenFor` can fold that into the same "not connected" answer it already gives for no
 * grant at all, and the caller's existing 409 covers it with no new status to add.
 *
 * Throws on any other non-OK answer, with a message that names the HTTP status and **never** the
 * refresh token or the response body: a token-exchange error body can echo either.
 */
export async function accessTokenFromRefresh(
  refreshToken: string,
  deps: GoogleTokenDeps,
): Promise<string | null> {
  const response = await deps.fetch(TOKEN_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "refresh_token",
      refresh_token: refreshToken,
      client_id: deps.clientId,
      client_secret: deps.clientSecret,
    }),
  });
  if (!response.ok) {
    let code: unknown;
    try {
      code = (await response.json() as { error?: unknown }).error;
    } catch {
      code = undefined;
    }
    if (code === "invalid_grant") return null;
    throw new Error(`google token refresh failed (HTTP ${response.status})`);
  }
  const body = await response.json() as { access_token?: string };
  if (typeof body.access_token !== "string" || body.access_token === "") {
    throw new Error("google token refresh returned no access_token");
  }
  return body.access_token;
}
