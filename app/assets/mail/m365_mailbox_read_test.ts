// Spec test 35, the Microsoft 365 mailbox half, reading (plan T10.2b; spec 4.2 step 5, 4.11 step 3): the
// dated search that leaves out Junk and Deleted Items and counts by `aria-setsize`, and the list read in
// the form the fingerprint uses (S2). Each test runs the compiled-in text from `m365/` against the
// synthetic mailbox in `m365_mailbox_pages.ts`.
import { assert, assertEquals } from "@std/assert";
import { mailbox, msg } from "./m365_mailbox_pages.ts";
import { run } from "./test_dom.ts";

const MAIL = [
  msg("m1", "ana.lopez@school.example.invalid", "Quiz 3 moved", 2),
  msg("m2", "bo.chen@school.example.invalid", "Lab report due", 5),
  msg("m3a", "reg@school.example.invalid", "Registration open", 9),
  msg("m4", "old.news@school.example.invalid", "Welcome back", 1),
  msg("j1", "spam@elsewhere.example.invalid", "You won", 6, "Junk Email"),
  msg("x1", "ana.lopez@school.example.invalid", "Dropped by mistake", 7, "Deleted Items"),
];

Deno.test("m365_search_since_a_date_excludes_junk_and_deleted_and_counts_by_aria_setsize", async () => {
  // Only two rows are drawn: the list is virtualised, and the count is what aria-setsize says.
  const box = mailbox(MAIL, { drawn: 2 });
  const r = await run("m365/search.js", box.page, { since: "2026-10-02" });
  assertEquals(r, { result: "ok", count: 3 });
  assertEquals(box.rows().length, 2, "the page drew two rows of three");
  assertEquals(box.queries.length, 1);
  const q = box.queries[0];
  assert(q.includes("received>=2026-10-02"), q);
  assert(q.includes('NOT folder:"Junk Email"') && q.includes('NOT folder:"Deleted Items"'), q);
});

Deno.test("m365_search_leaves_out_sent_items_and_drafts_which_are_not_mail_received", async () => {
  // Exchange gives a Sent Items row a received time, and a "Knowlu <nonce>" copy a failed cleanup left
  // there would be forwarded again: the student's own mail is never counted, listed or recovered.
  const box = mailbox([
    ...MAIL,
    msg("s1", "ana.lopez@school.example.invalid", "Re: Quiz 3 moved", 3, "Sent Items"),
    msg("s2", "in@example.invalid", "Knowlu bk7f3a9c2e41", 4, "Sent Items"),
    msg("d1", "bo.chen@school.example.invalid", "Re: Lab report due", 6, "Drafts"),
  ]);
  const r = await run("m365/search.js", box.page, { since: "2026-10-02" });
  assertEquals(r, { result: "ok", count: 3 }, "m1, m2 and m3a only");
  const q = box.queries[0];
  assert(q.includes('NOT folder:"Sent Items"') && q.includes('NOT folder:"Drafts"'), q);
  assertEquals(
    box.rows().map((row) => row.getAttribute("id")).sort(),
    ["r-m1", "r-m2", "r-m3a"],
    "no Sent or Drafts row is listed",
  );
});

Deno.test("m365_search_until_narrows_the_window_to_since_inclusive_until_exclusive", async () => {
  // T11.2: the routine narrows a window so the virtualised list can draw all of it. The span is
  // [since, until): m1 (day 2) and m2 (day 5) are in [2, 9), m3a (day 9) is not.
  const box = mailbox(MAIL);
  const r = await run("m365/search.js", box.page, { since: "2026-10-02", until: "2026-10-09" });
  assertEquals(r, { result: "ok", count: 2 });
  const q = box.queries[0];
  assert(q.includes("received>=2026-10-02 received<2026-10-09 NOT folder"), q);
  // An until that is not a later day is a bad date, and nothing is typed.
  const ok = mailbox(MAIL);
  for (const until of ["2026-10-02", "2026-10-01", "10/09/2026", "2026-13-40", "2026-10-09 OR x"]) {
    assertEquals(await run("m365/search.js", ok.page, { since: "2026-10-02", until }), {
      result: "failed",
      reason: "bad date",
    });
  }
  assertEquals(ok.queries, []);
});

Deno.test("m365_search_reports_no_results_as_zero_and_a_deaf_page_as_not_settled", async () => {
  const none = mailbox(MAIL);
  assertEquals(await run("m365/search.js", none.page, { since: "2026-12-01" }), { result: "ok", count: 0 });
  // A page that never answers is not a count of the list it was already showing.
  const deaf = mailbox(MAIL, { deaf: true });
  const r = await run("m365/search.js", deaf.page, { since: "2026-10-02", wait_ms: 200 });
  assertEquals(r, { result: "failed", reason: "search did not settle" });
});

Deno.test("m365_search_finds_the_live_type_less_combobox_drawn_after_the_script_starts", async () => {
  // T18c (2026-10-07, live): the box is `<input role="combobox">` with no type attribute, and Outlook
  // redirects mid-load (outlook.office.com to outlook.cloud.microsoft), so it is briefly absent. A checkbox
  // whose name says "search" is never the box.
  const box = mailbox(MAIL, { searchLate: 300 });
  box.el("searchbar").insertAdjacentHTML(
    "beforebegin",
    `<input type="checkbox" aria-label="Search all folders">`,
  );
  const r = await run("m365/search.js", box.page, { since: "2026-10-02" });
  assertEquals(r, { result: "ok", count: 3 });
  assertEquals(box.queries.length, 1);
  assertEquals(box.el("q").getAttribute("type"), null, "the page's box has no type attribute");
});

Deno.test("m365_search_names_a_missing_control_and_refuses_a_bad_date", async () => {
  const box = mailbox(MAIL);
  box.el("q").remove();
  assertEquals(await run("m365/search.js", box.page, { since: "2026-10-02", draw_ms: 200 }), {
    result: "not found",
    control: "search",
  });
  const ok = mailbox(MAIL);
  for (const since of ["", "10/02/2026", "2026-13-40", "2026-10-02 OR x"]) {
    assertEquals(await run("m365/search.js", ok.page, { since }), { result: "failed", reason: "bad date" });
  }
  assertEquals(ok.queries, [], "nothing was typed for a bad date");
});

Deno.test("m365_the_list_reads_sender_subject_and_received_time_in_s2s_form", async () => {
  const box = mailbox(MAIL);
  const r = await run("m365/list.js", box.page);
  assertEquals(r.result, "ok");
  assertEquals(r.total, 4);
  const at = (n: number) => new Date(2026, 9, n, 15, 4).toISOString();
  // The address comes from the sender element's title, whether the row shows a name or the address,
  // folded to lower case; the time is the time element's title, with the instant it names.
  assertEquals(r.rows, [
    {
      sender: "ana.lopez@school.example.invalid",
      subject: "Quiz 3 moved",
      received: "Thu 10/2/2026 3:04 PM",
      received_at: at(2),
    },
    {
      sender: "bo.chen@school.example.invalid",
      subject: "Lab report due",
      received: "Thu 10/5/2026 3:04 PM",
      received_at: at(5),
    },
    {
      sender: "reg@school.example.invalid",
      subject: "Registration open",
      received: "Thu 10/9/2026 3:04 PM",
      received_at: at(9),
    },
    {
      sender: "old.news@school.example.invalid",
      subject: "Welcome back",
      received: "Thu 10/1/2026 3:04 PM",
      received_at: at(1),
    },
  ]);
  // The display name is on the page and never leaves it.
  assert(!JSON.stringify(r).includes("ana lopez"));
});

Deno.test("m365_the_list_names_a_row_it_cannot_read_and_a_page_with_no_list", async () => {
  const box = mailbox(MAIL);
  box.rows()[1].querySelector("span[title*='@']").removeAttribute("title");
  assertEquals(await run("m365/list.js", box.page), { result: "not found", control: "list row" });
  const bare = mailbox(MAIL);
  bare.el("list").remove();
  assertEquals(await run("m365/list.js", bare.page), { result: "not found", control: "list" });
});
