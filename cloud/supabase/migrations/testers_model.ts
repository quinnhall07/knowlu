// Layer ii of docs/specs/2026-10-18-friends-testers-design.md §9 (task S2m): a pure model of
// public.effective_entitlements' precedence (§3.2) and admin.testers' state order (§5), under the
// Amendment 2026-10-20 (grants keyed by account, no address read). It registers no test: S3's fake lookup
// imports this file and testers_cases.ts, never a _test.ts file. Days are relative to the model's
// fixed now, day 0, as in the case table. It proves the table self-consistent, not the SQL.
import type { Account, Call, Grant, GuardTestersRow, Row, TesterCase } from "./testers_cases.ts";
import { EXTEND_NOT_FUTURE, EXTEND_TOO_FAR, GRANT_NOT_FUTURE, GRANT_TOO_FAR } from "./testers_cases.ts";

export type State = "ended" | "expired" | "live";
/** The statuses a grant never overrides: `not in ('active', 'trialing')` is the grant branch (§3.2). */
export const PAID: ReadonlyArray<string> = ["active", "trialing"];
/** §3.3: P12's refusal names comp_extend. */
export const OPEN_GRANT_REFUSAL = "an open grant exists for this account; use admin.comp_extend";

/** `coalesce(e.status, 'none') in ('active', 'trialing')`: admin.testers' `paid` flag. */
export const isPaid = (row: Row | null): boolean => PAID.includes(row?.status ?? "none");

/** The view's grant match: `g.ended_at is null and now() < g.ends_at`. */
export const entitles = (g: Grant): boolean => g.endedAgo === undefined && 0 < g.endsIn;

/** §5's order: `ended` (ended_at set), then `expired` (now() >= ends_at), otherwise `live`. */
export function grantState(g: Grant): State {
  if (g.endedAgo !== undefined) return "ended";
  if (0 >= g.endsIn) return "expired";
  return "live";
}

/** One account's view row from its `entitlements` row (null when deleted by hand) and its grants. */
export function viewRow(row: Row | null, grants: Grant[]): Row {
  const g = isPaid(row) ? undefined : grants.find(entitles);
  if (g) return { status: "active", plan: "tester", periodEnd: g.endsIn };
  return { status: row?.status ?? "none", plan: row?.plan ?? null, periodEnd: row?.periodEnd ?? null };
}

/** The S6 follow-up's ceiling, `now() + interval '1 year'`, in whole days: 365 is inside on every
 * date and 367 outside; 366 depends on the calendar, and no case uses it. */
export const YEAR_DAYS = 365;

/** admin.testers' row for a grant, with the follow-up's two columns: `coalesce(e.status, 'none')` and
 * `e.stripe_subscription_id is not null` (false with no row). */
export function testersRow(a: Account, g: Grant): GuardTestersRow {
  return {
    state: grantState(g),
    paid: isPaid(a.row),
    entitlementStatus: a.row?.status ?? "none",
    hasSubscription: a.row !== null && a.subscription === true,
  };
}

export interface Outcome {
  /** The view's answer per account before the calls and the deletion; null for no view row. */
  before: Array<Row | null>;
  after: Array<Row | null>;
  /** Per call: null when it succeeded, else the refusal. */
  refusals: Array<string | null>;
  grantsBefore: Grant[];
  grants: Grant[];
}

/** Runs one case: its grants, then its admin calls in order, then its account deletion (cascade). */
export function run(c: TesterCase): Outcome {
  const alive = c.accounts.map(() => true);
  let grants: Grant[] = c.grants.map((g) => ({ ...g }));
  const answers = () =>
    c.accounts.map((a, i) => alive[i] ? viewRow(a.row, grants.filter((g) => g.account === i)) : null);
  const grantsBefore = grants.map((g) => ({ ...g }));
  const before = answers();
  const refusals = (c.calls ?? []).map((call) => {
    const r = apply(call, alive, grants);
    if (r.grants) grants = r.grants;
    return r.refused;
  });
  if (c.deleteAccount !== undefined) {
    alive[c.deleteAccount] = false;
    grants = grants.filter((g) => g.account !== c.deleteAccount);
  }
  return { before, after: answers(), refusals, grantsBefore, grants };
}

function apply(call: Call, alive: boolean[], grants: Grant[]): { refused: string | null; grants?: Grant[] } {
  const account = call.account;
  const open = account === "unknown" ? undefined : grants.find((g) => g.account === account && g.endedAgo === undefined);
  if (call.fn === "comp_grant") {
    // The S6 follow-up: the date first, as the SQL checks it before the insert.
    if (call.endsIn <= 0) return { refused: GRANT_NOT_FUTURE };
    if (call.endsIn > YEAR_DAYS) return { refused: GRANT_TOO_FAR };
    if (account === "unknown" || !alive[account]) {
      return { refused: 'insert on table "comp_grants" violates foreign key constraint' };
    }
    if (open) return { refused: OPEN_GRANT_REFUSAL };
    return { refused: null, grants: [...grants, { account, endsIn: call.endsIn }] };
  }
  // comp_extend (§3.3): accepts an open grant whether or not its ends_at has passed; refuses a new
  // ends_at not in the future, then (the S6 follow-up) one past a year, then an ended grant or none,
  // in the SQL's order (the date first).
  if (call.endsIn <= 0) return { refused: EXTEND_NOT_FUTURE };
  if (call.endsIn > YEAR_DAYS) return { refused: EXTEND_TOO_FAR };
  if (!open) return { refused: "no open grant for this account" };
  return { refused: null, grants: grants.map((g) => g === open ? { ...g, endsIn: call.endsIn } : g) };
}
