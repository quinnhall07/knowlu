// The friends-testers case table (docs/specs/2026-10-18-friends-testers-design.md §9, under its
// Amendment 2026-10-20). Data only, no Deno.test: layer ii (testers_model_test.ts) and layer iii
// (proofs/testers_staging.sql) run these cases, and S3's fake lookup reads them. Every address is
// @example.invalid, never a person. Times are whole days from the moment a case is evaluated (the
// model's fixed now, the staging script's now()); a negative number is in the past.
// Retired by the Amendment 2026-10-20, reserved and never reused: P4, P7, P10 and P11 (they tested a
// password or an address match; the view reads neither). They are never data rows.
// S2b writes the P rows; S2c1 and S2c2 add the A rows, S2d R1, S2e1 the state column, S2e2 the V rows.

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
  { id: "P8", given: "sign-up none row, grant past ends_at", accounts: signedUp("p8"), grants: [{ account: 0, endsIn: -1 }], expect: [NONE] },
  { id: "P9", given: "sign-up none row, grant ended early", accounts: signedUp("p9"), grants: [{ account: 0, endsIn: 30, endedAgo: 1 }], expect: [NONE] },
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
  { id: "P2", given: "Stripe active, open grant: paid wins", accounts: paid("p2", { status: "active", plan: "academic_year", periodEnd: 200 }), grants: [{ account: 0, endsIn: 30 }], expect: [{ status: "active", plan: "academic_year", periodEnd: 200 }] },
  { id: "P5", given: "Stripe canceled, open grant", accounts: paid("p5", { status: "canceled", plan: "monthly", periodEnd: -3 }), grants: [{ account: 0, endsIn: 30 }], expect: [tester(30)] },
  { id: "P6", given: "Stripe past_due, open grant", accounts: paid("p6", { status: "past_due", plan: "monthly", periodEnd: 2 }), grants: [{ account: 0, endsIn: 30 }], expect: [tester(30)] },
  // No account at all: the foreign key refuses the grant, and no grant row is written.
  { id: "P16", given: "comp_grant naming an id with no accounts row", accounts: [], grants: [], calls: [{ fn: "comp_grant", account: "unknown", endsIn: 30, refused: "foreign key" }], expect: [] },
];
