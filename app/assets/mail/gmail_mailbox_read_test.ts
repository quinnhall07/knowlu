// Spec test 35, the Gmail mailbox half, reading (plan T10.3b; spec 4.3 step 7, 4.11 step 3): the dated search
// that leaves out Spam and Trash, the pages walked oldest first, and the list read in the form the
// fingerprint uses (S2). Each test runs the compiled-in text from `gmail/` against the synthetic mailbox in
// `gmail_mailbox_pages.ts`. No address is real.
import { assert, assertEquals } from "@std/assert";
import { mailbox, msg, when } from "./gmail_mailbox_pages.ts";
import { run } from "./test_dom.ts";

const FAST = { wait_ms: 300 };
const MAIL = [
  msg("m1", "ana.lopez@school.example.invalid", "Quiz 3 moved", 2),
  msg("m2", "Bo.Chen@school.example.invalid", "Lab report due", 5),
  msg("m3a", "reg@school.example.invalid", "Registration open", 9),
  msg("m4", "old.news@school.example.invalid", "Welcome back", 1),
  msg("j1", "spam@elsewhere.example.invalid", "You won", 6, "Spam"),
  msg("x1", "ana.lopez@school.example.invalid", "Dropped by mistake", 7, "Trash"),
  msg("s1", "friend@elsewhere.example.invalid", "My own note", 8, "Sent"),
];

Deno.test("gmail_search_uses_after_date_minus_spam_minus_trash", async () => {
  // Two rows to a page: the count is the pager's total, not the rows drawn.
  const box = mailbox(MAIL, { size: 2 });
  const r = await run("gmail/search.js", box.page, { since: "2026-10-02", ...FAST });
  assertEquals(r, { result: "ok", count: 3 });
  assertEquals(box.rows().length, 2, "the page drew two rows of three");
  assertEquals(box.queries.length, 1);
  const q = box.queries[0];
  assert(q.includes("after:2026/10/02"), q);
  assert(q.includes("-in:spam") && q.includes("-in:trash"), q);
  // The student's own sent mail and drafts are not mail received.
  assert(q.includes("-in:sent") && q.includes("-in:drafts"), q);
});

Deno.test("gmail_search_runs_by_url_and_never_types_into_the_box", async () => {
  // Live 2026-10-18: the search box is labelled "Ask Gmail" and may route to Gemini, so the script sets the
  // hash instead: `#search/` + the query, percent-encoded with spaces as plus signs.
  const box = mailbox(MAIL);
  const label = box.el("q").getAttribute("aria-label");
  assertEquals(label, "Ask Gmail");
  const r = await run("gmail/search.js", box.page, { since: "2026-10-02", ...FAST });
  assertEquals(r, { result: "ok", count: 3 });
  assertEquals(
    box.page.location.hash,
    "#search/after%3A2026%2F10%2F02+-in%3Aspam+-in%3Atrash+-in%3Asent+-in%3Adrafts",
  );
  assertEquals(box.el("q").value ?? "", "", "nothing was typed into the box");
  // The same search again leaves the hash where it is (no hashchange, no redraw): it still answers.
  assertEquals(await run("gmail/search.js", box.page, { since: "2026-10-02", ...FAST }), {
    result: "ok",
    count: 3,
  });
  assertEquals(box.queries.length, 1, "the page searched once");
  // A different day after a search is a new search.
  assertEquals(await run("gmail/search.js", box.page, { since: "2026-10-06", ...FAST }), {
    result: "ok",
    count: 1,
  });
});

Deno.test("gmail_search_does_not_read_the_previous_views_range_as_its_count", async () => {
  // The rows redraw at once but the range text follows later; the inbox's "1-4 of 4" must not be read.
  const box = mailbox(MAIL, { slowRange: true });
  assertEquals(box.range().includes("of 4"), true, "the inbox shows its own range first");
  const r = await run("gmail/search.js", box.page, { since: "2026-10-05", ...FAST });
  assertEquals(r, { result: "ok", count: 2 });
});

Deno.test("gmail_search_reports_no_results_as_zero_and_a_deaf_page_as_not_settled", async () => {
  const none = mailbox(MAIL);
  assertEquals(await run("gmail/search.js", none.page, { since: "2026-12-01", ...FAST }), {
    result: "ok",
    count: 0,
  });
  // A page that never answers is not a count of the list it was already showing.
  const deaf = mailbox(MAIL, { deaf: true });
  assertEquals(await run("gmail/search.js", deaf.page, { since: "2026-10-02", wait_ms: 200 }), {
    result: "failed",
    reason: "search did not settle",
  });
});

Deno.test("gmail_search_names_a_missing_list_and_refuses_a_bad_date", async () => {
  // No grid ever draws: the page has no list to search into.
  const box = mailbox(MAIL, { deaf: true });
  box.el("grid").remove();
  assertEquals(await run("gmail/search.js", box.page, { since: "2026-10-02", wait_ms: 200 }), {
    result: "not found",
    control: "search",
  });
  const ok = mailbox(MAIL);
  for (const since of ["", "10/02/2026", "2026-13-40", "2026-10-02 OR x"]) {
    assertEquals(await run("gmail/search.js", ok.page, { since }), { result: "failed", reason: "bad date" });
  }
  assertEquals(ok.queries, [], "nothing was searched for a bad date");
  // Results with no count to read are not a count of zero.
  const blind = mailbox(MAIL, { blind: true });
  assertEquals(await run("gmail/search.js", blind.page, { since: "2026-10-02", ...FAST }), {
    result: "not found",
    control: "result count",
  });
});

Deno.test("gmail_the_list_reads_sender_subject_and_received_time_in_s2s_form", async () => {
  const box = mailbox(MAIL.slice(0, 4));
  const r = await run("gmail/list.js", box.page, FAST);
  assertEquals(r.result, "ok");
  assertEquals(r.page, { from: 1, to: 4, total: 4 });
  const at = (n: number) => new Date(2026, 9, n, 15, 4).toISOString();
  // The address is the span[email] attribute, whether the row shows a name or the address, folded to lower
  // case; the time is the time element's title, with the instant it names. Oldest first.
  assertEquals(r.rows, [
    {
      sender: "old.news@school.example.invalid",
      subject: "Welcome back",
      received: when(1),
      received_at: at(1),
      thread: false,
    },
    {
      sender: "ana.lopez@school.example.invalid",
      subject: "Quiz 3 moved",
      received: when(2),
      received_at: at(2),
      thread: false,
    },
    {
      sender: "bo.chen@school.example.invalid",
      subject: "Lab report due",
      received: when(5),
      received_at: at(5),
      thread: false,
    },
    {
      sender: "reg@school.example.invalid",
      subject: "Registration open",
      received: when(9),
      received_at: at(9),
      thread: false,
    },
  ]);
  // The display name is on the page and never leaves it.
  assert(!JSON.stringify(r).includes("ana lopez"));
  // A conversation row lists several senders: the newest is read, and the row says it is a thread.
  const t = mailbox([{ ...MAIL[0], also: ["bo.chen@school.example.invalid"] }]);
  const row = (await run("gmail/list.js", t.page, FAST)).rows[0];
  assertEquals([row.sender, row.thread], ["bo.chen@school.example.invalid", true]);
  assertEquals(row.messages, null, "no count shown, so none read");
  // Quinn 2026-10-02: a row that shows a message count is a thread even when one person wrote them all,
  // and the count is read (the routine opens it and reads each message, thread.js).
  const counted = mailbox([{ ...MAIL[0], count: 3 }, MAIL[1]]);
  const rows = (await run("gmail/list.js", counted.page, FAST)).rows;
  const named = rows.find((r: { subject: string }) => r.subject === "Quiz 3 moved");
  assertEquals([named.thread, named.messages], [true, 3]);
  const plain = rows.find((r: { subject: string }) => r.subject === "Lab report due");
  assertEquals([plain.thread, "messages" in plain], [false, false]);
});

Deno.test("gmail_the_list_names_a_row_it_cannot_read_and_a_page_with_no_list", async () => {
  const box = mailbox(MAIL);
  box.rows()[1].querySelector("span[email]").removeAttribute("email");
  assertEquals(await run("gmail/list.js", box.page, FAST), { result: "not found", control: "list row" });
  const bare = mailbox(MAIL);
  bare.el("grid").remove();
  assertEquals(await run("gmail/list.js", bare.page, FAST), { result: "not found", control: "list" });
  const empty = mailbox([]);
  assertEquals(await run("gmail/list.js", empty.page, FAST), {
    result: "ok",
    page: { from: 0, to: 0, total: 0 },
    moved: false,
    rows: [],
  });
});

Deno.test("gmail_pages_are_selected_oldest_first", async () => {
  // Five results, two to a page: newest first on screen, so the oldest page is the last one.
  const mail = [1, 2, 3, 4, 5].map((n) =>
    msg(`p${n}`, `s${n}@school.example.invalid`, `Subject ${n}`, n + 10)
  );
  const box = mailbox(mail, { size: 2 });
  assertEquals((await run("gmail/search.js", box.page, { since: "2026-10-01", ...FAST })).count, 5);
  const seen: string[] = [];
  const first = await run("gmail/list.js", box.page, { go: "oldest", ...FAST });
  assertEquals(first.page, { from: 5, to: 5, total: 5 });
  assertEquals(first.moved, true);
  seen.push(...first.rows.map((r: { subject: string }) => r.subject));
  // Then toward the newest, a page at a time; each page's rows are oldest first too.
  const second = await run("gmail/list.js", box.page, { go: "newer", ...FAST });
  assertEquals(second.page, { from: 3, to: 4, total: 5 });
  seen.push(...second.rows.map((r: { subject: string }) => r.subject));
  const third = await run("gmail/list.js", box.page, { go: "newer", ...FAST });
  assertEquals(third.page, { from: 1, to: 2, total: 5 });
  seen.push(...third.rows.map((r: { subject: string }) => r.subject));
  assertEquals(seen, ["Subject 1", "Subject 2", "Subject 3", "Subject 4", "Subject 5"]);
  // At the newest page there is nowhere newer to go: the page is read again, and it says it did not move.
  const end = await run("gmail/list.js", box.page, { go: "newer", ...FAST });
  assertEquals([end.page.from, end.moved], [1, false]);
  // Already on the oldest page: no click, the page as it is.
  const again = mailbox(mail, { size: 2 });
  await run("gmail/list.js", again.page, { go: "oldest", ...FAST });
  assertEquals((await run("gmail/list.js", again.page, { go: "oldest", ...FAST })).moved, false);
});

Deno.test("gmail_the_pager_is_named_when_it_cannot_reach_the_oldest_page", async () => {
  const mail = [1, 2, 3].map((n) => msg(`p${n}`, `s${n}@school.example.invalid`, `Subject ${n}`, n + 10));
  const none = mailbox(mail, { size: 2, noOldest: true });
  assertEquals(await run("gmail/list.js", none.page, { go: "oldest", ...FAST }), {
    result: "not found",
    control: "pager",
  });
  // One page has no oldest to reach and needs no button.
  const one = mailbox(mail, { noOldest: true });
  assertEquals((await run("gmail/list.js", one.page, { go: "oldest", ...FAST })).result, "ok");
  assertEquals((await run("gmail/list.js", one.page, { go: "sideways", ...FAST })).result, "failed");
});
