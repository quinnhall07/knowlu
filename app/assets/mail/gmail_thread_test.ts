// Spec test 35, the Gmail mailbox half, conversation rows (plan T11.2; Quinn 2026-10-02: a list row that
// shows a message count is opened and every message in it fingerprinted): `gmail/thread.js` against the
// synthetic mailbox in `gmail_mailbox_pages.ts`. No address is real.
import { assert, assertEquals } from "@std/assert";
import { mailbox, msg, SELF, when } from "./gmail_mailbox_pages.ts";
import { El, run } from "./test_dom.ts";

const FAST = { wait_ms: 300 };
const ANA = "ana.lopez@school.example.invalid";
const BO = "Bo.Chen@school.example.invalid";

const thread = () => ({
  ...msg("t1", ANA, "Group project", 9),
  count: 3,
  convo: [
    { who: ANA, at: when(2) },
    { who: BO, at: when(5) },
    { who: ANA, at: when(9) },
  ],
});
const ROW = { sender: ANA, subject: "Group project", received: when(9) };

Deno.test("gmail_thread_reads_every_message_oldest_first_and_goes_back_to_the_list", async () => {
  const box = mailbox([thread(), msg("m2", "reg@school.example.invalid", "Registration open", 4)]);
  const r = await run("gmail/thread.js", box.page, { ...ROW, messages: 3, ...FAST });
  const at = (n: number) => new Date(2026, 9, n, 15, 4).toISOString();
  // The middle message was collapsed: Expand all drew it. Addresses come from the email attribute, folded.
  assertEquals(r, {
    result: "ok",
    messages: [
      { sender: ANA, subject: "Group project", received: when(2), received_at: at(2) },
      { sender: BO.toLowerCase(), subject: "Group project", received: when(5), received_at: at(5) },
      { sender: ANA, subject: "Group project", received: when(9), received_at: at(9) },
    ],
  });
  // The list is back, nothing is ticked, nothing was sent or deleted.
  assertEquals(box.rows().length, 2);
  assert(!box.el("grid").hasAttribute("hidden"));
  assertEquals([box.checked(), box.sent, box.deleted, box.discards], [[], [], [], 0]);
  // The display names are on the page and never leave it.
  assert(!JSON.stringify(r).includes("ana lopez"));
});

Deno.test("gmail_thread_goes_back_by_the_titled_back_button", async () => {
  // Live: on a search the back control is titled "Back to Search Results", has no aria-label, and ignores
  // click(); a press works.
  const box = mailbox([thread(), msg("m2", "reg@school.example.invalid", "Registration open", 4)]);
  await run("gmail/search.js", box.page, { since: "2026-10-01", ...FAST });
  box.rows().find((r: El) => r.getAttribute("data-id") === "t1").click();
  const back = box.el("back");
  assertEquals([back.getAttribute("title"), back.getAttribute("aria-label")], [
    "Back to Search Results",
    null,
  ]);
  back.click();
  assert(box.el("grid").hasAttribute("hidden"), "click() did not go back");
  for (const t of ["mousedown", "mouseup"]) back.dispatchEvent(new box.page.Event(t, { bubbles: true }));
  assert(!box.el("grid").hasAttribute("hidden"));
  const r = await run("gmail/thread.js", box.page, { ...ROW, messages: 3, ...FAST });
  assertEquals([r.result, r.messages.length], ["ok", 3]);
  assert(!box.el("grid").hasAttribute("hidden"), "the script went back to the results");
  // The search's grid is the one read: the hidden inbox grid beside it holds the same rows.
  assertEquals(box.page.document.querySelectorAll('table[role="grid"]').length, 2);
});

Deno.test("gmail_thread_of_two_needs_no_expanding_and_a_wrong_row_is_not_found", async () => {
  const two = { ...thread(), count: 2, convo: thread().convo!.slice(1) };
  const box = mailbox([two]);
  const r = await run("gmail/thread.js", box.page, { ...ROW, ...FAST });
  assertEquals(r.messages.map((m: { sender: string }) => m.sender), [BO.toLowerCase(), ANA]);
  assertEquals(await run("gmail/thread.js", box.page, { ...ROW, subject: "Another", ...FAST }), {
    result: "not found",
    control: "message",
  });
  // Nothing was opened for the wrong row, and the list is still showing.
  assert(!box.el("grid").hasAttribute("hidden"));
  const bare = mailbox([thread()]);
  bare.el("grid").remove();
  assertEquals(await run("gmail/thread.js", bare.page, { ...ROW, ...FAST }), {
    result: "not found",
    control: "list",
  });
});

Deno.test("gmail_thread_returns_nothing_when_fewer_messages_draw_than_the_row_counts", async () => {
  // A partial read would call a held message missing and forward the whole conversation again.
  const box = mailbox([thread()], { stuck: true });
  const r = await run("gmail/thread.js", box.page, { ...ROW, messages: 3, ...FAST });
  assertEquals(r, { result: "failed", reason: "not every message drawn" });
  assert(!box.el("grid").hasAttribute("hidden"), "and the list is back");
  // Without a count to check, what drew is what is read.
  const blind = mailbox([thread()], { stuck: true });
  const b = await run("gmail/thread.js", blind.page, { ...ROW, ...FAST });
  assertEquals(b.messages.length, 2);
});

Deno.test("gmail_thread_names_a_conversation_that_will_not_open_or_a_list_that_will_not_return", async () => {
  const flat = mailbox([msg("f1", ANA, "Group project", 9)]);
  assertEquals(await run("gmail/thread.js", flat.page, { ...ROW, ...FAST }), {
    result: "failed",
    reason: "conversation did not open",
  });
  const noBack = mailbox([thread()], { noBack: true });
  assertEquals(await run("gmail/thread.js", noBack.page, { ...ROW, ...FAST }), {
    result: "not found",
    control: "back",
  });
});

Deno.test("gmail_thread_marks_the_students_own_messages_and_says_so_only_for_them", async () => {
  // The student's own reply is in the conversation but the service never received it (it was never
  // forwarded to the student's own mailbox): the routine must not judge it missing.
  const replied = {
    ...msg("t1", SELF.toUpperCase(), "Group project", 9),
    count: 2,
    convo: [{ who: ANA, at: when(2) }, { who: SELF.toUpperCase(), at: when(9) }],
  };
  const box = mailbox([replied]);
  const row = { sender: SELF, subject: "Group project", received: when(9) };
  const r = await run("gmail/thread.js", box.page, { ...row, messages: 2, ...FAST });
  assertEquals(r.result, "ok");
  assertEquals(r.messages.map((m: { sender: string; own?: boolean }) => [m.sender, m.own]), [
    [ANA, undefined],
    [SELF, true],
  ]);
  assertEquals(Object.keys(r.messages[0]), ["sender", "subject", "received", "received_at"]);
});

Deno.test("gmail_thread_reads_nothing_when_it_cannot_tell_whose_account_it_is", async () => {
  // Without the account chip the student's replies cannot be told from received mail: nothing is opened.
  const box = mailbox([thread()], { account: null });
  assertEquals(await run("gmail/thread.js", box.page, { ...ROW, messages: 3, ...FAST }), {
    result: "not found",
    control: "account",
  });
  assert(!box.el("grid").hasAttribute("hidden"), "the list is still showing");
});

Deno.test("gmail_thread_reads_the_account_from_the_live_chip_label_that_continues_after_the_address", async () => {
  // Live 2026-10-18: "Google Account: <name> (<address>), Google membership". The address does not end the
  // label, and it is the last parenthesised address that names the account.
  const box = mailbox([thread()]);
  const chip = box.page.document.querySelector('[aria-label^="Google Account"]');
  assertEquals(
    chip.getAttribute("aria-label"),
    `Google Account: Student Name (${SELF}), Google membership`,
  );
  const r = await run("gmail/thread.js", box.page, { ...ROW, messages: 3, ...FAST });
  assertEquals(r.result, "ok");
  // A name with its own parenthesised address before the real one: the last one counts. The label is set
  // before the script runs, and the conversation holds a message from each address.
  const DECOY = "sam@other.example.invalid";
  const both = {
    ...msg("t1", ANA, "Group project", 9),
    count: 3,
    convo: [{ who: ANA, at: when(2) }, { who: DECOY, at: when(5) }, { who: SELF, at: when(9) }],
  };
  const again = mailbox([both]);
  again.page.document.querySelector('[aria-label^="Google Account"]').setAttribute(
    "aria-label",
    `Google Account: Sam (${DECOY}) (they/them) (${SELF}), Google membership`,
  );
  const s = await run("gmail/thread.js", again.page, { ...ROW, messages: 3, ...FAST });
  assertEquals(s.result, "ok");
  assertEquals(s.messages.map((m: { sender: string; own?: boolean }) => [m.sender, m.own]), [
    [ANA, undefined],
    [DECOY, undefined],
    [SELF, true],
  ]);
});
