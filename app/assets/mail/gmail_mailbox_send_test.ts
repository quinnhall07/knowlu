// Spec test 35, the Gmail mailbox half, sending and cleaning up (plan T10.3b; spec 4.3 step 7, 4.11 step 4,
// 10 and R4): a page's missing messages forwarded as attachments in one send under 25 MB with the batch's
// nonce, a refusal seen in the page, and the Sent cleanup that touches only the copies Knowlu made.
// Synthetic pages only.
import { assertEquals } from "@std/assert";
import { Box, mailbox, Msg, msg, NONCE, OURS } from "./gmail_mailbox_pages.ts";
import { run } from "./test_dom.ts";

const FAST = { wait_ms: 300 };
const MB = 10_000_000;
const MAIL = [
  msg("m1", "ana.lopez@school.example.invalid", "Quiz 3 moved", 2, "Inbox", MB),
  msg("m2", "bo.chen@school.example.invalid", "Lab report due", 5, "Inbox", MB),
  msg("m3", "reg@school.example.invalid", "Registration open", 9, "Inbox", MB),
];
const row = (m: Msg) => ({ sender: m.who, subject: m.subject, received: m.at });
const TARGET = { address: OURS, nonce: NONCE, rows: [row(MAIL[0]), row(MAIL[1])], ...FAST };

Deno.test("gmail_forward_as_attachment_keeps_each_send_under_25_mb_with_the_nonce", async () => {
  const box = mailbox(MAIL);
  const r = await run("gmail/forward.js", box.page, TARGET);
  assertEquals(r, { result: "ok", sent: true, count: 2 });
  // One message, to Knowlu's address alone, carrying the two messages asked for as attachments, under a
  // subject that holds the nonce and none of the originals' words.
  assertEquals(box.sent.length, 1);
  assertEquals(box.sent[0].to, [OURS]);
  assertEquals(box.sent[0].attached, ["m1", "m2"]);
  assertEquals(box.sent[0].subject, `Knowlu ${NONCE}`);
  assertEquals(box.el("over").innerHTML, "", "the compose window closed");
  assertEquals(box.checked(), [], "nothing is left selected");
  // Three of these are 30 MB: nothing is sent, the draft is discarded and the selection let go.
  const big = mailbox(MAIL);
  const over = await run("gmail/forward.js", big.page, { ...TARGET, rows: MAIL.map(row) });
  assertEquals(over, { result: "failed", reason: "over 25 MB" });
  assertEquals([big.sent, big.discards, big.checked()], [[], 1, []]);
  // The limit is the caller's to lower.
  const low = mailbox(MAIL);
  assertEquals(
    (await run("gmail/forward.js", low.page, { ...TARGET, max_bytes: 15_000_000 })).reason,
    "over 25 MB",
  );
  assertEquals(low.sent, []);
  // Gmail's own offer to send Drive links instead is never accepted, whatever sizes the chips show.
  const drive = mailbox([{ ...MAIL[0], size: 1000 }], { driveOffer: true });
  assertEquals(await run("gmail/forward.js", drive.page, { ...TARGET, rows: [row(MAIL[0])] }), {
    result: "failed",
    reason: "over 25 MB",
  });
  assertEquals(drive.sent, []);
});

Deno.test("gmail_forward_of_a_conversation_row_expects_one_attachment_per_message_in_it", async () => {
  // T11.2: a conversation row of three messages, forwarded with the row's count, attaches three; the plain
  // row beside it attaches one. Four attachments for two rows is what the page is expected to show.
  const small = MAIL.map((m) => ({ ...m, size: 1_000_000 }));
  const convo = {
    ...small[1],
    count: 3,
    convo: [small[1], small[1], small[1]].map((m) => ({ who: m.who, at: m.at })),
  };
  const box = mailbox([small[0], convo]);
  const r = await run("gmail/forward.js", box.page, {
    ...TARGET,
    rows: [row(MAIL[0]), { ...row(MAIL[1]), messages: 3 }],
  });
  assertEquals(r, { result: "ok", sent: true, count: 4 });
  assertEquals(box.sent[0].attached, ["m1", "m2#0", "m2#1", "m2#2"]);
  // Without the count the same send is refused: three attachments for one row is not what was asked.
  const blind = mailbox([small[0], convo]);
  assertEquals(await run("gmail/forward.js", blind.page, { ...TARGET, rows: [row(MAIL[0]), row(MAIL[1])] }), {
    result: "failed",
    reason: "attachments do not match the rows",
  });
  assertEquals([blind.sent, blind.discards, blind.checked()], [[], 1, []]);
  // If "Forward as attachment" attaches only the newest message of a conversation, the count does not
  // match, nothing is sent and the draft is discarded: the routine leaves that row (T18c's gate).
  const newest = mailbox([small[0], convo], { newestOnly: true });
  assertEquals(
    await run("gmail/forward.js", newest.page, {
      ...TARGET,
      rows: [row(MAIL[0]), { ...row(MAIL[1]), messages: 3 }],
    }),
    { result: "failed", reason: "attachments do not match the rows" },
  );
  assertEquals([newest.sent, newest.discards, newest.checked()], [[], 1, []]);
});

Deno.test("gmail_forward_names_a_message_or_control_that_is_not_on_the_page_and_sends_nothing", async () => {
  const gone = mailbox(MAIL);
  const missing = { ...row(MAIL[1]), subject: "Something else" };
  assertEquals(await run("gmail/forward.js", gone.page, { ...TARGET, rows: [row(MAIL[0]), missing] }), {
    result: "not found",
    control: "message",
  });
  assertEquals(gone.checked(), [], "nothing was selected for a message that is not there");
  const noMore = mailbox(MAIL);
  noMore.el("more").remove();
  assertEquals(await run("gmail/forward.js", noMore.page, TARGET), {
    result: "not found",
    control: "more menu",
  });
  const noItem = mailbox(MAIL, { noAttachmentItem: true });
  assertEquals(await run("gmail/forward.js", noItem.page, { ...TARGET, wait_ms: 200 }), {
    result: "not found",
    control: "forward as attachment",
  });
  for (const b of [noMore, noItem]) assertEquals([b.sent, b.checked()], [[], []]);
  // An argument that would send somewhere else, under no nonce, or for no rows, is refused before a click.
  const bad = mailbox(MAIL);
  for (
    const args of [
      { ...TARGET, address: "not an address" },
      { ...TARGET, nonce: "" },
      { ...TARGET, rows: [] },
      { ...TARGET, rows: Array(101).fill(row(MAIL[0])) },
    ]
  ) {
    assertEquals((await run("gmail/forward.js", bad.page, args)).result, "failed");
  }
  assertEquals([bad.checked(), bad.el("over").innerHTML], [[], ""]);
  // A box already ticked by someone is not ours to send along with the rest.
  const ticked = mailbox(MAIL);
  ticked.rows()[2].querySelector('[role="checkbox"]').click();
  assertEquals(await run("gmail/forward.js", ticked.page, TARGET), {
    result: "failed",
    reason: "rows already selected",
  });
  assertEquals(ticked.sent, []);
});

Deno.test("gmail_forward_never_sends_a_compose_with_another_recipient_or_the_wrong_attachments", async () => {
  // Gmail offers someone else on the To line: the one address it may send to is Knowlu's.
  const other = mailbox(MAIL, { prefill: "friend@elsewhere.example.invalid" });
  assertEquals(await run("gmail/forward.js", other.page, TARGET), {
    result: "failed",
    reason: "recipient not accepted",
  });
  const bare = mailbox(MAIL, { bare: true });
  assertEquals(await run("gmail/forward.js", bare.page, TARGET), {
    result: "failed",
    reason: "no attachment",
  });
  const short = mailbox(MAIL, { short: true });
  assertEquals(await run("gmail/forward.js", short.page, TARGET), {
    result: "failed",
    reason: "attachments do not match the rows",
  });
  for (const b of [other, bare, short]) assertEquals([b.sent, b.discards, b.checked()], [[], 1, []]);
});

Deno.test("gmail_a_send_failure_is_reported", async () => {
  // A banner that says the send failed, with the compose window left open.
  const banner = mailbox(MAIL, { send: "banner" });
  assertEquals(await run("gmail/forward.js", banner.page, TARGET), {
    result: "refused",
    reason: "send failure",
  });
  // The same banner after the window closed.
  const closes = mailbox(MAIL, { send: "banner-closes" });
  assertEquals(await run("gmail/forward.js", closes.page, TARGET), {
    result: "refused",
    reason: "send failure",
  });
  // A non-delivery report for this nonce arrives in the list with the send.
  const ndr = mailbox(MAIL, { send: "ndr" });
  assertEquals(await run("gmail/forward.js", ndr.page, TARGET), {
    result: "refused",
    reason: "non-delivery report",
  });
  // And one already there stops the next forward before it sends anything (the first refusal ends the routine).
  const later = mailbox(MAIL, { send: "ndr" });
  await run("gmail/forward.js", later.page, TARGET);
  assertEquals(await run("gmail/forward.js", later.page, { ...TARGET, rows: [row(MAIL[2])] }), {
    result: "refused",
    reason: "non-delivery report",
  });
  assertEquals(later.sent.length, 1, "the second forward was never sent");
  // A report for some other batch's nonce is not this batch's refusal.
  const report = msg("n9", "mailer-daemon@googlemail.com", "Delivery Status Notification (Failure)", 3);
  const other = mailbox([...MAIL, {
    ...report,
    snippet: "Your message wasn't delivered. Subject: Knowlu zz99",
  }]);
  assertEquals(await run("gmail/forward.js", other.page, TARGET), { result: "ok", sent: true, count: 2 });
});

const COPY = (id: string, to: string, subject: string, also?: string[]): Msg => ({
  ...msg(id, to, subject, 3, "Sent"),
  also,
});

function sentItems(size = 50): Box {
  return mailbox([
    COPY("c1", OURS, `Knowlu ${NONCE}`),
    COPY("c2", OURS, `Knowlu ${NONCE}`),
    // R4: the nonce, to someone else; Knowlu's address, without the nonce; a copy to both; the student's own mail.
    COPY("k1", "friend@elsewhere.example.invalid", `Knowlu ${NONCE}`),
    COPY("k2", OURS, "Knowlu a-different-batch"),
    COPY("k3", OURS, "Lunch on Friday"),
    COPY("k4", "friend@elsewhere.example.invalid", "Lunch on Friday"),
    COPY("k5", OURS, `Knowlu ${NONCE}`, ["friend@elsewhere.example.invalid"]),
  ], { view: "sent", size });
}

Deno.test("gmail_sent_cleanup_deletes_only_its_own_copies", async () => {
  const box = sentItems();
  assertEquals(await run("gmail/sent_cleanup.js", box.page, { address: OURS, nonce: NONCE, ...FAST }), {
    result: "ok",
    deleted: 2,
  });
  assertEquals([...box.deleted].sort(), ["c1", "c2"]);
  // The list is drawn again after each Delete, and the next copy may be on a page it was not on before.
  const paged = sentItems(2);
  assertEquals(
    (await run("gmail/sent_cleanup.js", paged.page, { address: OURS, nonce: NONCE, ...FAST })).deleted,
    2,
  );
  assertEquals([...paged.deleted].sort(), ["c1", "c2"]);
  // `max` bounds one call.
  const capped = sentItems();
  assertEquals(
    (await run("gmail/sent_cleanup.js", capped.page, { address: OURS, nonce: NONCE, max: 1, ...FAST }))
      .deleted,
    1,
  );
});

Deno.test("gmail_sent_cleanup_keeps_a_nonce_copy_to_another_recipient_and_a_knowlu_copy_without_the_nonce", async () => {
  const box = sentItems();
  await run("gmail/sent_cleanup.js", box.page, { address: OURS, nonce: NONCE, ...FAST });
  // Every other item on the page survives: the R4 cases, a copy to Knowlu and someone else, the student's mail.
  const left = box.msgs.filter((m) => m.folder === "Sent").map((m) => m.id).sort();
  assertEquals(left, ["k1", "k2", "k3", "k4", "k5"]);
  assertEquals(box.rows().length, 5);
  // The nonce alone is not enough, and neither is the address alone.
  const wide = sentItems();
  assertEquals(
    await run("gmail/sent_cleanup.js", wide.page, { address: OURS, nonce: "zz99-no-such", ...FAST }),
    {
      result: "ok",
      deleted: 0,
    },
  );
  assertEquals(wide.deleted, []);
});

Deno.test("gmail_sent_cleanup_never_runs_outside_sent_and_refuses_a_bad_nonce", async () => {
  // The Inbox holds a message that looks like one of ours (a reply quoting the nonce, from Knowlu's address).
  const inbox = mailbox([msg("r1", OURS, `Knowlu ${NONCE}`, 4)]);
  assertEquals(await run("gmail/sent_cleanup.js", inbox.page, { address: OURS, nonce: NONCE, ...FAST }), {
    result: "not found",
    control: "sent items",
  });
  assertEquals(inbox.deleted, []);
  const box = sentItems();
  assertEquals((await run("gmail/sent_cleanup.js", box.page, { address: OURS, nonce: "" })).result, "failed");
  assertEquals(
    (await run("gmail/sent_cleanup.js", box.page, { address: "", nonce: NONCE })).result,
    "failed",
  );
  assertEquals(box.deleted, []);
  const noDelete = sentItems();
  noDelete.el("del").remove();
  assertEquals(await run("gmail/sent_cleanup.js", noDelete.page, { address: OURS, nonce: NONCE, ...FAST }), {
    result: "not found",
    control: "delete",
  });
  // A Delete that does nothing is named, with what went before it.
  const stuck = sentItems();
  stuck.el("del").replaceWith(stuck.el("del").cloneNode(true));
  assertEquals(
    await run("gmail/sent_cleanup.js", stuck.page, { address: OURS, nonce: NONCE, wait_ms: 200 }),
    {
      result: "failed",
      reason: "delete did not settle",
      deleted: 0,
    },
  );
});

Deno.test("gmail_sent_cleanup_rechecks_the_row_before_each_delete_when_the_page_changes_it", async () => {
  // The moment c1 is ticked the page draws the student's own "Lunch on Friday" (k3) in its row.
  const box = mailbox([
    COPY("c1", OURS, `Knowlu ${NONCE}`),
    COPY("c2", OURS, `Knowlu ${NONCE}`),
    COPY("k3", OURS, "Lunch on Friday"),
  ], { view: "sent", swap: { id: "c1", as: "k3" } });
  const r = await run("gmail/sent_cleanup.js", box.page, { address: OURS, nonce: NONCE, ...FAST });
  // The row that showed k3 was let go, unticked and skipped (c2 went first); c1 itself is ours and goes when
  // the page draws it as it is. The student's own mail is never ticked for a Delete.
  assertEquals(r, { result: "ok", deleted: 2 });
  assertEquals(box.deleted, ["c2", "c1"]);
  assertEquals(box.checked(), []);
});
