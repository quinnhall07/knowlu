// The friends-testers case table (docs/specs/2026-10-18-friends-testers-design.md §9, under its
// Amendment 2026-10-20). Data only, no Deno.test: layer ii (testers_model_test.ts) and layer iii
// (proofs/testers_staging.sql) run these cases, and S3's fake lookup reads them. Every address is
// @example.invalid, never a person. Times are whole days from the moment a case is evaluated (the
// model's fixed now, the staging script's now()); a negative number is in the past.
// Retired by the Amendment 2026-10-20, reserved and never reused: P4, P7, P10 and P11 (they tested a
// password or an address match; the view reads neither). They are never data rows.
// S2b writes the P rows; S2c1 and S2c2 add the A rows, S2d R1, S2e1 the state column (P2, P8, P9, P13),
// S2e2 the V rows.

export type Status = "active" | "trialing" | "past_due" | "canceled" | "none";
/** An `entitlements` row, or the view's answer: `status` / `plan` / `current_period_end`. */
export interface Row {
  status: Status;
  plan: string | null;
  periodEnd: number | null;
}
export interface Account {
  /** `accounts.email`. */
  email: string;
  /** The `auth.users` address, when an auth email change left it apart from `accounts.email` (P15). */
  authEmail?: string;
  /** The `entitlements` row; `null` when it was deleted by hand (P0x, P3x). */
  row: Row | null;
}
/** A grant on `accounts[account]`, open unless `endedAgo` says it was ended early that many days ago. */
export interface Grant {
  account: number;
  endsIn: number;
  endedAgo?: number;
}
/** An admin call made after the grants exist. `refused` is a substring the refusal must carry, or
 * `true` where the spec fixes no wording; absent, the call succeeds. `account: "unknown"` is a fresh
 * id with no `accounts` row (P16). */
export interface Call {
  fn: "comp_grant" | "comp_extend";
  account: number | "unknown";
  endsIn: number;
  refused?: string | true;
}
export interface TesterCase {
  id: string;
  given: string;
  accounts: Account[];
  grants: Grant[];
  calls?: Call[];
  /** Deleted after the calls (P15); its answer in `expect` is then `null`, no view row. */
  deleteAccount?: number;
  /** The view's answer per account before the calls and the deletion, where it differs from `expect`. */
  expectBefore?: Array<Row | null>;
  /** The view's answer per account at the end. */
  expect: Array<Row | null>;
  /** S2e1: `admin.testers`' row per grant at the end, in `grants` order (§5): its state and the
   * account's `paid` flag. Set on P2, P8, P9 and P13 (§9). */
  testers?: TestersRow[];
  /** As `testers`, before the calls, where it differs (P13). */
  testersBefore?: TestersRow[];
}
/** §5's three states, derived at read time in this order: ended, expired, live. */
export type GrantState = "ended" | "expired" | "live";
export interface TestersRow {
  state: GrantState;
  paid: boolean;
}

/** The sign-up row `handle_new_user` writes for every account (§2), and the answer it gives unchanged. */
const SIGNUP: Row = { status: "none", plan: null, periodEnd: null };
const NONE: Row = { status: "none", plan: null, periodEnd: null };
const tester = (endsIn: number): Row => ({ status: "active", plan: "tester", periodEnd: endsIn });
const signedUp = (id: string, n = 1): Account[] =>
  Array.from({ length: n }, (_, i) => ({ email: `${id}-${i}@example.invalid`, row: { ...SIGNUP } }));
const paid = (id: string, row: Row): Account[] => [{ email: `${id}-0@example.invalid`, row }];
const EXTEND_HINT = "admin.comp_extend"; // §3.3: P12's refusal names comp_extend

// The none-row cases first: the sign-up row is what every unpaid account and every real tester carries.
export const CASES: TesterCase[] = [
  { id: "P0", given: "sign-up none row, no grant: every unpaid account", accounts: signedUp("p0"), grants: [], expect: [NONE] },
  { id: "P3", given: "sign-up none row, open grant: every real tester", accounts: signedUp("p3"), grants: [{ account: 0, endsIn: 30 }], expect: [tester(30)] },
  { id: "P8", given: "sign-up none row, grant past ends_at", accounts: signedUp("p8"), grants: [{ account: 0, endsIn: -1 }], expect: [NONE], testers: [{ state: "expired", paid: false }] },
  { id: "P9", given: "sign-up none row, grant ended early", accounts: signedUp("p9"), grants: [{ account: 0, endsIn: 30, endedAgo: 1 }], expect: [NONE], testers: [{ state: "ended", paid: false }] },
  {
    id: "P12", given: "a second grant while one is open, live (first) or expired and never ended (second)",
    accounts: signedUp("p12", 2), grants: [{ account: 0, endsIn: 30 }, { account: 1, endsIn: -1 }],
    calls: [
      { fn: "comp_grant", account: 0, endsIn: 60, refused: EXTEND_HINT },
      { fn: "comp_grant", account: 1, endsIn: 60, refused: EXTEND_HINT },
    ],
    expect: [tester(30), NONE],
  },
  {
    id: "P13", given: "an expired open grant extended (a past date refused first); an ended grant not extendable",
    accounts: signedUp("p13", 2), grants: [{ account: 0, endsIn: -1 }, { account: 1, endsIn: 30, endedAgo: 1 }],
    calls: [
      { fn: "comp_extend", account: 0, endsIn: -2, refused: true },
      { fn: "comp_extend", account: 0, endsIn: 45 },
      { fn: "comp_extend", account: 1, endsIn: 45, refused: true },
    ],
    expectBefore: [NONE, NONE], expect: [tester(45), NONE],
    testersBefore: [{ state: "expired", paid: false }, { state: "ended", paid: false }],
    testers: [{ state: "live", paid: false }, { state: "ended", paid: false }],
  },
  { id: "P14", given: "two sign-up accounts, a grant on the first only", accounts: signedUp("p14", 2), grants: [{ account: 0, endsIn: 30 }], expect: [tester(30), NONE] },
  {
    id: "P15", given: "two accounts carrying one accounts.email, a grant on the first; then the second is deleted",
    accounts: [
      { email: "p15-0@example.invalid", row: { ...SIGNUP } },
      { email: "p15-0@example.invalid", authEmail: "p15-1@example.invalid", row: { ...SIGNUP } },
    ],
    grants: [{ account: 0, endsIn: 30 }], deleteAccount: 1,
    expectBefore: [tester(30), NONE], expect: [tester(30), null],
  },
  // The entitlements row deleted by hand: the coalesce answers none.
  { id: "P0x", given: "no entitlements row, no grant", accounts: [{ email: "p0x-0@example.invalid", row: null }], grants: [], expect: [NONE] },
  { id: "P3x", given: "no entitlements row, open grant", accounts: [{ email: "p3x-0@example.invalid", row: null }], grants: [{ account: 0, endsIn: 30 }], expect: [tester(30)] },
  // Stripe rows: paid passes through column for column and wins over a grant; unpaid ones yield to it.
  { id: "P1", given: "Stripe active, no grant: the paying-student guard", accounts: paid("p1", { status: "active", plan: "monthly", periodEnd: 20 }), grants: [], expect: [{ status: "active", plan: "monthly", periodEnd: 20 }] },
  { id: "P2", given: "Stripe active, open grant: paid wins", accounts: paid("p2", { status: "active", plan: "academic_year", periodEnd: 200 }), grants: [{ account: 0, endsIn: 30 }], expect: [{ status: "active", plan: "academic_year", periodEnd: 200 }], testers: [{ state: "live", paid: true }] },
  { id: "P5", given: "Stripe canceled, open grant", accounts: paid("p5", { status: "canceled", plan: "monthly", periodEnd: -3 }), grants: [{ account: 0, endsIn: 30 }], expect: [tester(30)] },
  { id: "P6", given: "Stripe past_due, open grant", accounts: paid("p6", { status: "past_due", plan: "monthly", periodEnd: 2 }), grants: [{ account: 0, endsIn: 30 }], expect: [tester(30)] },
  // No account at all: the foreign key refuses the grant, and no grant row is written.
  { id: "P16", given: "comp_grant naming an id with no accounts row", accounts: [], grants: [], calls: [{ fn: "comp_grant", account: "unknown", endsIn: 30, refused: "foreign key" }], expect: [] },
];

// S2c1's A rows (§3.5, §9): the audit row each grant function writes, the refusals that write none,
// the billing job that never lists a comp, and the service role's rights. Every grant here is made
// by an owner call, never set up by hand. S2c2's A3, A3s, A4 and A6 follow A7.
/** An owner call. `audit` is this account's `comp_audit` actions after it, compared as a multiset
 * (one transaction, one `now()`); every row carries the account's id and, but for a refusal, its
 * grant's id. `refused` is a substring the refusal must carry; absent, the call succeeds. */
export interface AuditCall {
  fn: "comp_grant" | "comp_extend" | "comp_end";
  account: number | "unknown";
  endsIn?: number;
  reason?: "done" | "asked" | "misuse" | "other";
  refused?: string;
  audit: Array<"grant" | "extend" | "end">;
}
/** A7: one statement as `service_role`, each in its own savepoint. */
export interface RoleStep {
  op: "insert" | "update" | "delete" | "select" | "call";
  on: "comp_grants" | "comp_audit" | "effective_entitlements" | "admin.comp_grant";
  /** For a select on the view: exactly the four columns the gate's lookup reads (§3.2). */
  columns?: string[];
  refused?: "permission denied";
}
export interface AdminCase {
  id: string;
  given: string;
  accounts: Account[];
  calls: AuditCall[];
  /** A7's statements, after the calls. */
  asServiceRole?: RoleStep[];
  /** A5: per account, whether `billing_subscribers` lists it at the end. */
  billingSubscribers?: boolean[];
  /** A3, A3s: per account, whether a `route_daily` row is seeded after the calls. */
  routeDaily?: boolean[];
  /** A3, A3s: deleted after the calls. `owner` deletes its `auth.users` row (the cascade runs from
   * there); `service_role` deletes its `accounts` row in a savepoint under `set local role
   * service_role`, as the purge's `restDelete` does. Its answer in `expect` is then `null`. */
  deleteAccount?: { account: number; as: "owner" | "service_role" };
  /** A4, A6: moved into the past as the owner after the calls, then `admin.comp_sweep()` is run.
   * `endedAgo` and `endsIn` (days) set the account's grant; `auditAgo` (a Postgres interval) sets
   * its audit rows' `at`. */
  sweep?: Array<{ account: number; endedAgo?: number; endsIn?: number; auditAgo?: string }>;
  /** Per account at the end: its `comp_grants`, `comp_audit` (`unlinked`: of them, `grant_id` null)
   * and `route_daily` rows. */
  rows?: Array<{ grants: number; audit: number; unlinked: number; routeDaily: number }>;
  /** The view's answer per account at the end. */
  expect: Array<Row | null>;
}

const DENIED = "permission denied" as const;
const grantNow = (endsIn = 30): AuditCall => ({ fn: "comp_grant", account: 0, endsIn, audit: ["grant"] });

export const ADMIN_CASES: AdminCase[] = [
  {
    id: "A1", given: "comp_grant, comp_extend, comp_end: one audit row each, on the grant's account", accounts: signedUp("a1"),
    calls: [
      grantNow(),
      { fn: "comp_extend", account: 0, endsIn: 60, audit: ["grant", "extend"] },
      { fn: "comp_end", account: 0, reason: "done", audit: ["grant", "extend", "end"] },
    ],
    expect: [NONE],
  },
  {
    id: "A2", given: "a refused comp_grant writes no audit row: a second open grant (P12), an unknown id (P16)", accounts: signedUp("a2"),
    calls: [
      grantNow(),
      { fn: "comp_grant", account: 0, endsIn: 60, refused: EXTEND_HINT, audit: ["grant"] },
      { fn: "comp_grant", account: "unknown", endsIn: 30, refused: "foreign key", audit: [] },
    ],
    expect: [tester(30)],
  },
  { id: "A5", given: "a comped account is absent from billing_subscribers", accounts: signedUp("a5"), calls: [grantNow()], billingSubscribers: [false], expect: [tester(30)] },
  {
    id: "A7", given: "as service_role: each write on both tables and the grant call refused; the three selects succeed", accounts: signedUp("a7"),
    calls: [grantNow()],
    asServiceRole: [
      ...(["comp_grants", "comp_audit"] as const).flatMap((on) => (["insert", "update", "delete"] as const).map((op) => ({ op, on, refused: DENIED }))),
      { op: "call", on: "admin.comp_grant", refused: DENIED },
      { op: "select", on: "comp_grants" },
      { op: "select", on: "comp_audit" },
      { op: "select", on: "effective_entitlements", columns: ["account_id", "plan", "status", "current_period_end"] },
    ],
    expect: [tester(30)],
  },
  // S2c2 (§3.1, §3.3, Amendment 2026-10-20): deletion by the cascade from accounts, and the sweep.
  deletion("A3", "owner"),
  deletion("A3s", "service_role"),
  {
    id: "A4", given: "the sweep: grants closed 31 days ago (ended, expired, expired then ended) deleted, 29 kept; audit stays",
    accounts: signedUp("a4", 5), calls: [0, 1, 2, 3, 4].map(grantOn).concat([0, 1, 4].map(endOn)),
    sweep: [
      { account: 0, endedAgo: 31 }, { account: 1, endedAgo: 29 }, { account: 2, endsIn: -31 }, { account: 3, endsIn: -29 },
      { account: 4, endsIn: -31, endedAgo: 10 },
    ],
    rows: [kept(0, 2, 2), kept(1, 2), kept(0, 1, 1), kept(1, 1), kept(0, 2, 2)], expect: [NONE, NONE, NONE, NONE, NONE],
  },
  {
    id: "A6", given: "the sweep: an audit row three years and a day old deleted, three years less a day kept; the grants stay",
    accounts: signedUp("a6", 2), calls: [grantOn(0), grantOn(1)],
    sweep: [{ account: 0, auditAgo: "3 years 1 day" }, { account: 1, auditAgo: "3 years -1 day" }],
    rows: [kept(1, 0), kept(1, 1)], expect: [tester(30), tester(30)],
  },
];

function kept(grants: number, audit: number, unlinked = 0, routeDaily = 0) {
  return { grants, audit, unlinked, routeDaily };
}
function grantOn(account: number): AuditCall {
  return { fn: "comp_grant", account, endsIn: 30, audit: ["grant"] };
}
function endOn(account: number): AuditCall {
  return { fn: "comp_end", account, reason: "done", audit: ["grant", "end"] };
}
/** A3 and A3s: the first account, holding a grant (its audit row's `grant_id` set) and a `route_daily`
 * row, is deleted, so one delete fires the cascade from `accounts` and the set-null from
 * `comp_grants`; the second account's grant, audit and `route_daily` rows are untouched. */
function deletion(id: "A3" | "A3s", as: "owner" | "service_role"): AdminCase {
  return {
    id, given: `an account with a grant and a route_daily row deleted as ${as === "owner" ? "the owner" : "production does"}`,
    accounts: signedUp(id.toLowerCase(), 2), calls: [grantOn(0), grantOn(1)], routeDaily: [true, true],
    deleteAccount: { account: 0, as }, rows: [kept(0, 0), kept(1, 1, 0, 1)], expect: [null, tester(30)],
  };
}

// S2d's R1 (§4, §9): count_route, the one writer of route_daily, as the gate's service role calls it.
/** One `count_route(p_account, p_route)` call on the case's account, each in its own savepoint.
 * `refused` is a substring the refusal must carry; absent, the call succeeds. */
export interface RouteStep {
  route: string;
  as: "service_role" | "authenticated";
  refused?: string;
}
export interface RouteCase {
  id: string;
  given: string;
  accounts: Account[];
  steps: RouteStep[];
  /** The account's `route_daily` rows at the end, today's day: `calls` per route. */
  calls: Record<string, number>;
}

export const ROUTE_CASES: RouteCase[] = [
  {
    id: "R1", given: "count_route twice, then a bad route, then as authenticated", accounts: signedUp("r1"),
    steps: [
      { route: "judge-task", as: "service_role" },
      { route: "judge-task", as: "service_role" },
      { route: "Judge_Task", as: "service_role", refused: "check constraint" },
      { route: "judge-task", as: "authenticated", refused: DENIED },
    ],
    calls: { "judge-task": 2 },
  },
];
