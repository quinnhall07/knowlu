// Spec test 35, the Microsoft 365 mailbox half, reading (plan T10.2b; spec 4.2 step 5, 4.11 step 3): the
// dated search that leaves out Junk and Deleted Items and counts by `aria-setsize`, and the list read in
// the form the fingerprint uses (S2). Each test runs the compiled-in text from `m365/` against the
// synthetic mailbox in `m365_mailbox_pages.ts`.
import { assert, assertEquals } from "@std/assert";
import { convo, mailbox, ME, msg } from "./m365_mailbox_pages.ts";
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
  // The conversation's messages come in the page's order. A message whose row shows no subject has the
  // conversation's; one that shows its own keeps it. The mailbox's own reply (the folder pane names the
  // mailbox) is marked, never judged.
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

// T18c (2026-10-07, live): the inbox is in conversation view. A conversation's row shows no address, so the
// list read failed whole on it. Quinn's ruling for Gmail threads (2026-10-02) applies: it is opened and every
// message in it is read and fingerprinted.
const at = (n: number, h = 15, mi = 4) => new Date(2026, 9, n, h, mi).toISOString();
const TALK = convo("c1", "Group project", [
  msg("c1a", "ana.lopez@school.example.invalid", "Group project", 3),
  { ...msg("c1b", ME, "Group project", 4), ownSubject: "RE: Group project" },
  msg("c1c", "bo.chen@school.example.invalid", "Group project", 6),
]);

Deno.test("m365_a_conversation_row_is_expanded_and_every_message_in_it_is_read", async () => {
  const box = mailbox([MAIL[0], TALK, MAIL[1]]);
  const before = box.rows().length;
  const r = await run("m365/list.js", box.page);
  assertEquals(r.result, "ok");
  assertEquals(r.total, 3, "the list's aria-setsize counts the conversation once");
  assertEquals(r.drawn, 3);
  assertEquals(r.left, 0);
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
      sender: "bo.chen@school.example.invalid",
      subject: "Group project",
      received: "Thu 10/6/2026 3:04 PM",
      received_at: at(6),
      conversation: 1,
    },
    {
      sender: ME,
      subject: "RE: Group project",
      received: "Thu 10/4/2026 3:04 PM",
      received_at: at(4),
      conversation: 1,
      own: true,
    },
    {
      sender: "ana.lopez@school.example.invalid",
      subject: "Group project",
      received: "Thu 10/3/2026 3:04 PM",
      received_at: at(3),
      conversation: 1,
    },
  ]);
  // It is collapsed again: the list is as it was found.
  assertEquals(box.expands, ["c1", "c1"]);
  assertEquals(box.rows().length, before);
  assertEquals(box.el("r-c1").querySelector("[data-x]").getAttribute("aria-label"), "Expand conversation");
  // No button's title is ever a subject, and the display names never leave the page.
  const out = JSON.stringify(r);
  for (const word of ["Mark as read", "Flag this message", "Delete", "ana lopez", "bo chen"]) {
    assert(!out.includes(word), word);
  }
});

Deno.test("m365_a_conversation_it_cannot_read_whole_is_left_and_never_fails_the_read", async () => {
  // A message drawn with no time cannot be fingerprinted: the conversation is counted left, the rest is read.
  const broken = convo("c2", "Office hours", [
    msg("c2a", "ana.lopez@school.example.invalid", "Office hours", 3),
    { ...msg("c2b", "bo.chen@school.example.invalid", "Office hours", 4), at: "" },
  ]);
  const box = mailbox([MAIL[0], broken]);
  const r = await run("m365/list.js", box.page);
  assertEquals([r.result, r.total, r.drawn, r.left, r.rows.length], ["ok", 2, 2, 1, 1]);
  assertEquals(box.rows().length, 2, "collapsed again");
  // With no mailbox address in the folder pane, its own messages cannot be told apart: a conversation is
  // left rather than read with the student's own replies in it as if they were received mail.
  const anon = mailbox([MAIL[0], TALK], { noAccount: true });
  const a = await run("m365/list.js", anon.page);
  assertEquals([a.result, a.drawn, a.left, a.rows.length], ["ok", 2, 1, 1]);
  assertEquals(anon.expands, [], "nothing is expanded");
  // A conversation whose messages never draw is left, within the wait.
  const deaf = mailbox([TALK]);
  const x = deaf.el("r-c1").querySelector("[data-x]");
  x.replaceWith(x.cloneNode(true));
  const d = await run("m365/list.js", deaf.page, { wait_ms: 200 });
  assertEquals([d.result, d.left, d.rows.length], ["ok", 1, 0]);
});

Deno.test("m365_a_conversation_the_page_shows_open_is_put_away_before_the_list_is_read", async () => {
  // Its messages' rows are not the list's own: read as rows of the list they would fail the read whole.
  const box = mailbox([MAIL[0], TALK]);
  box.el("r-c1").querySelector("[data-x]").click();
  await new Promise((done) => setTimeout(done, 60));
  assertEquals(box.rows().length, 5, "the page opened it");
  const r = await run("m365/list.js", box.page);
  assertEquals([r.result, r.total, r.drawn, r.left, r.rows.length], ["ok", 2, 2, 0, 4]);
  assertEquals(box.rows().length, 2, "and leaves it put away");
});
