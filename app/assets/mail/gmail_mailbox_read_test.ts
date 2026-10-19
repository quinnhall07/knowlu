// Spec test 35, the Gmail mailbox half, reading (plan T10.3b; spec 4.3 step 7, 4.11 step 3): the dated search
// that leaves out Spam and Trash, the pages walked oldest first, and the list read in the form the
// fingerprint uses (S2). Each test runs the compiled-in text from `gmail/` against the synthetic mailbox in
// `gmail_mailbox_pages.ts`. No address is real.
import { assert, assertEquals } from "@std/assert";
import { mailbox, msg, when } from "./gmail_mailbox_pages.ts";
import { El, run } from "./test_dom.ts";

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
  // "1–2 of many": the script pressed "Next results" to the last page, where the total is exact, and left
  // the page there, on its one row.
  assertEquals(r, { result: "ok", count: 3, exact: true, at_oldest: true });
  assertEquals(box.rows().length, 1, "the walk ended on the oldest page, one row of three");
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
  assertEquals(r, { result: "ok", count: 3, exact: true });
  assertEquals(
    box.page.location.hash,
    "#search/after%3A2026%2F10%2F02+-in%3Aspam+-in%3Atrash+-in%3Asent+-in%3Adrafts",
  );
  assertEquals(box.el("q").value ?? "", "", "nothing was typed into the box");
  // The same search again leaves the hash where it is (no hashchange, no redraw): it still answers.
  assertEquals(await run("gmail/search.js", box.page, { since: "2026-10-02", ...FAST }), {
    result: "ok",
    count: 3,
    exact: true,
  });
  assertEquals(box.queries.length, 1, "the page searched once");
  // A different day after a search is a new search.
  assertEquals(await run("gmail/search.js", box.page, { since: "2026-10-06", ...FAST }), {
    result: "ok",
    count: 1,
    exact: true,
  });
});

Deno.test("gmail_search_does_not_read_the_previous_views_range_as_its_count", async () => {
  // The rows redraw at once but the range text follows later; the inbox's "1-4 of 4" must not be read.
  const box = mailbox(MAIL, { slowRange: true });
  assertEquals(box.range().includes("of 4"), true, "the inbox shows its own range first");
  const r = await run("gmail/search.js", box.page, { since: "2026-10-05", ...FAST });
  assertEquals(r, { result: "ok", count: 2, exact: true });
});

Deno.test("gmail_search_reports_no_results_as_zero_and_a_deaf_page_as_not_settled", async () => {
  const none = mailbox(MAIL);
  assertEquals(await run("gmail/search.js", none.page, { since: "2026-12-01", ...FAST }), {
    result: "ok",
    count: 0,
    exact: true,
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

Deno.test("gmail_search_reads_the_shown_grid_not_the_hidden_previous_view", async () => {
  // Live 2026-10-18/19: after the hash moves to a search, the inbox's grid and its "1–4 of 4" stay in the
  // document, hidden, and the search draws its own grid and range after them.
  const box = mailbox(MAIL);
  const r = await run("gmail/search.js", box.page, { since: "2026-10-05", ...FAST });
  assertEquals(r, { result: "ok", count: 2, exact: true });
  const grids = [...box.page.document.querySelectorAll('table[role="grid"]')];
  assertEquals(grids.length, 2, "the inbox's grid is still in the document");
  assertEquals(grids[0].offsetParent, null, "and hidden");
  assertEquals(grids[0].querySelectorAll('tr[role="row"]').length, 4);
  const ranges = [...box.page.document.querySelectorAll(".Dj")].map((s: { textContent: string }) =>
    s.textContent.replace(/\s+/g, " ")
  );
  assertEquals(ranges, ["1–4 of 4", "1–2 of 2"]);
  assert(grids[1].offsetParent !== null && box.rows().length === 2);
  // The list reads the shown one too.
  const list = await run("gmail/list.js", box.page, FAST);
  assertEquals([list.page, list.rows.length], [{ from: 1, to: 2, total: 2 }, 2]);
});

const MANY = (n: number) =>
  Array.from(
    { length: n },
    (_, i) => msg(`w${i}`, `s${i}@school.example.invalid`, `Subject ${i}`, (i % 28) + 1),
  );

Deno.test("gmail_search_walks_an_of_many_result_to_its_exact_total", async () => {
  // Seven results, two to a page: "1–2 of many" until the fourth page, "7–7 of 7".
  const box = mailbox(MANY(7), { size: 2 });
  const r = await run("gmail/search.js", box.page, { since: "2026-10-01", ...FAST });
  assertEquals(r, { result: "ok", count: 7, exact: true, at_oldest: true });
  // The page is left on the oldest page, reached by presses (the hash follows the pager, never set to it).
  assertEquals(box.range(), "7–7 of 7");
  assert(box.page.location.hash.endsWith("/p4"), box.page.location.hash);
  // Large totals read with their commas: 1,001 results, 500 to a page.
  const big = mailbox(MANY(1001), { size: 500 });
  const b = await run("gmail/search.js", big.page, { since: "2026-10-01", ...FAST });
  assertEquals(b, { result: "ok", count: 1001, exact: true, at_oldest: true });
  assertEquals(big.range(), "1,001–1,001 of 1,001");
  // A press that never redraws the page is not a page walked.
  const stuck = mailbox(MANY(7), { size: 2, stuckPager: true });
  assertEquals(await run("gmail/search.js", stuck.page, { since: "2026-10-01", ...FAST }), {
    result: "failed",
    reason: "page did not change",
  });
});

Deno.test("gmail_search_stops_the_walk_at_its_budget", async () => {
  // No time to walk: the rows counted so far are a lower bound, and the page stays on page 1.
  const box = mailbox(MANY(7), { size: 2 });
  const r = await run("gmail/search.js", box.page, { since: "2026-10-01", max_ms: 0, ...FAST });
  assertEquals(r, { result: "ok", count: 2, exact: false, at_oldest: false });
  assertEquals(box.range(), "1–2 of many");
  // The walk resumes from there with the list: a whole budget reaches the oldest page.
  const rest = await run("gmail/list.js", box.page, { go: "oldest", ...FAST });
  assertEquals([rest.page, rest.moved, rest.at_oldest], [{ from: 7, to: 7, total: 7 }, true, true]);
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

Deno.test("gmail_list_reads_the_bx0_message_count", async () => {
  // Live: the count is digits alone in a span.bx0 ("2", no parentheses), the sender cell ending " 2", and
  // the two senders may name one address.
  const box = mailbox([{ ...MAIL[0], also: [MAIL[0].who], count: 2 }]);
  const cell = box.rows()[0].querySelector("span[email]").closest("td");
  assert(/ 2$/.test(cell.textContent.replace(/\s+/g, " ").trim()), cell.textContent);
  assertEquals(box.rows()[0].querySelector("span.bx0").textContent, "2");
  const row = (await run("gmail/list.js", box.page, FAST)).rows[0];
  assertEquals([row.sender, row.thread, row.messages], [MAIL[0].who, true, 2]);
  // One address twice and no count is one sender: not a thread.
  const same = mailbox([{ ...MAIL[0], also: [MAIL[0].who] }]);
  assertEquals((await run("gmail/list.js", same.page, FAST)).rows[0].thread, false);
  // The older "(3)" form is still read.
  const old = mailbox([{ ...MAIL[0], count: 3 }]);
  old.rows()[0].querySelector("span.bx0").replaceWith(old.page.document.createTextNode("(3)"));
  const o = (await run("gmail/list.js", old.page, FAST)).rows[0];
  assertEquals([o.thread, o.messages], [true, 3]);
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
  // Five in a folder, two to a page: newest first on screen, so the oldest page is the last one. A folder
  // draws Newest/Newer/Older/Oldest and an exact range; "oldest" is one press of Oldest.
  const mail = [1, 2, 3, 4, 5].map((n) =>
    msg(`p${n}`, `s${n}@school.example.invalid`, `Subject ${n}`, n + 10)
  );
  const box = mailbox(mail, { size: 2 });
  assertEquals(box.range(), "1–2 of 5");
  const seen: string[] = [];
  const first = await run("gmail/list.js", box.page, { go: "oldest", ...FAST });
  assertEquals(first.page, { from: 5, to: 5, total: 5 });
  assertEquals([first.moved, first.at_oldest], [true, true]);
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
  const still = await run("gmail/list.js", again.page, { go: "oldest", ...FAST });
  assertEquals([still.moved, still.at_oldest], [false, true]);
  // "newest" is one press of Newest; "older" one of Older.
  assertEquals((await run("gmail/list.js", again.page, { go: "newest", ...FAST })).page.from, 1);
  assertEquals((await run("gmail/list.js", again.page, { go: "older", ...FAST })).page, {
    from: 3,
    to: 4,
    total: 5,
  });
});

Deno.test("gmail_list_pages_by_previous_and_next_results", async () => {
  // A search draws "Previous results"/"Next results" and no Oldest or Newest; its range reads "of many"
  // until the last page. search.js leaves the page on the oldest page.
  const mail = [1, 2, 3, 4, 5].map((n) =>
    msg(`p${n}`, `s${n}@school.example.invalid`, `Subject ${n}`, n + 10)
  );
  const box = mailbox(mail, { size: 2 });
  await run("gmail/search.js", box.page, { since: "2026-10-01", ...FAST });
  const labels = [...box.page.document.querySelectorAll(".vw:last-child [role=button]")].map((b: El) =>
    b.getAttribute("aria-label")
  );
  assertEquals(labels, ["Previous results", "Next results", "Show more messages"]);
  const here = await run("gmail/list.js", box.page, { go: "oldest", ...FAST });
  assertEquals([here.page, here.moved, here.at_oldest], [{ from: 5, to: 5, total: 5 }, false, true]);
  // Newer is "Previous results"; a page that is not the last reads "of many": no total.
  const second = await run("gmail/list.js", box.page, { go: "newer", ...FAST });
  assertEquals([second.page, second.moved], [{ from: 3, to: 4, total: null }, true]);
  assertEquals(second.rows.map((r: { subject: string }) => r.subject), ["Subject 2", "Subject 3"]);
  const third = await run("gmail/list.js", box.page, { go: "newest", ...FAST });
  assertEquals([third.page, third.moved], [{ from: 1, to: 2, total: null }, true]);
  // Previous results is disabled on page 1: nowhere newer to go.
  const end = await run("gmail/list.js", box.page, { go: "newer", ...FAST });
  assertEquals([end.page.from, end.moved], [1, false]);
  // "oldest" walks Next results until it is disabled.
  const back = await run("gmail/list.js", box.page, { go: "oldest", ...FAST });
  assertEquals([back.page, back.moved, back.at_oldest], [{ from: 5, to: 5, total: 5 }, true, true]);
  assertEquals((await run("gmail/list.js", box.page, { go: "older", ...FAST })).moved, false);
  // Out of budget, the walk stops where it is and says so.
  await run("gmail/list.js", box.page, { go: "newest", ...FAST });
  const short = await run("gmail/list.js", box.page, { go: "oldest", max_ms: 0, ...FAST });
  assertEquals([short.page.from, short.moved, short.at_oldest], [1, false, false]);
});

Deno.test("gmail_the_pager_answers_a_press_not_a_click", async () => {
  const mail = [1, 2, 3].map((n) => msg(`p${n}`, `s${n}@school.example.invalid`, `Subject ${n}`, n + 10));
  const box = mailbox(mail, { size: 2 });
  // Live: Gmail's pager ignores click().
  box.page.document.querySelector('[aria-label="Older"]').click();
  await new Promise((done) => setTimeout(done, 100));
  assertEquals(box.range(), "1–2 of 3");
  const r = await run("gmail/list.js", box.page, { go: "older", ...FAST });
  assertEquals([r.page, r.moved], [{ from: 3, to: 3, total: 3 }, true]);
  assertEquals(box.page.location.hash, "#inbox/p2");
  // One press is one page, never two.
  const many = mailbox(MANY(7), { size: 2 });
  assertEquals((await run("gmail/list.js", many.page, { go: "older", ...FAST })).page.from, 3);
  // A pager that takes the press and draws nothing is named.
  const stuck = mailbox(mail, { size: 2, stuckPager: true });
  assertEquals(await run("gmail/list.js", stuck.page, { go: "older", ...FAST }), {
    result: "failed",
    reason: "page did not change",
  });
});

Deno.test("gmail_the_pager_is_named_when_it_cannot_reach_the_oldest_page", async () => {
  const mail = [1, 2, 3].map((n) => msg(`p${n}`, `s${n}@school.example.invalid`, `Subject ${n}`, n + 10));
  // No Oldest drawn: "oldest" walks Older to the end.
  const walk = mailbox(mail, { size: 2, noOldest: true });
  const w = await run("gmail/list.js", walk.page, { go: "oldest", ...FAST });
  assertEquals([w.page, w.moved, w.at_oldest], [{ from: 3, to: 3, total: 3 }, true, true]);
  // Neither Oldest nor Older: the pager is named.
  const none = mailbox(mail, { size: 2, noOldest: true });
  none.page.document.querySelector('[aria-label="Older"]').remove();
  assertEquals(await run("gmail/list.js", none.page, { go: "oldest", ...FAST }), {
    result: "not found",
    control: "pager",
  });
  // One page has no oldest to reach and needs no button.
  const one = mailbox(mail, { noOldest: true });
  assertEquals((await run("gmail/list.js", one.page, { go: "oldest", ...FAST })).result, "ok");
  assertEquals((await run("gmail/list.js", one.page, { go: "sideways", ...FAST })).result, "failed");
});
