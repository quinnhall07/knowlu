// Spec test 35, the Microsoft 365 mailbox half, sending and cleaning up (plan T10.2b; spec 4.2 step 5,
// 4.11 step 4, 10 and R4): one forward-as-attachment per call with the batch's nonce, a refusal seen in
// the page, and the Sent Items cleanup that touches only the copies Knowlu made. Synthetic pages only.
import { assert, assertEquals } from "@std/assert";
import { Box, mailbox, Msg, msg, NONCE, OURS } from "./m365_mailbox_pages.ts";
import { run } from "./test_dom.ts";

const MAIL = [
  msg("m1", "ana.lopez@school.example.invalid", "Quiz 3 moved", 2),
  msg("m2", "bo.chen@school.example.invalid", "Lab report due", 5),
];
const TARGET = {
  address: OURS,
  nonce: NONCE,
  sender: "bo.chen@school.example.invalid",
  subject: "Lab report due",
  received: "Thu 10/5/2026 3:04 PM",
};

Deno.test("m365_forward_as_attachment_sends_one_message_with_the_nonce_in_the_subject", async () => {
  const box = mailbox(MAIL);
  const r = await run("m365/forward.js", box.page, TARGET);
  assertEquals(r, { result: "ok", sent: true });
  assertEquals(box.sent.length, 1);
  // One message, to Knowlu's address alone, carrying the one message that was asked for as an attachment,
  // under a subject that holds the nonce and none of the original's words.
  assertEquals(box.sent[0].to, [OURS]);
  assertEquals(box.sent[0].attached, "m2");
  assert(box.sent[0].subject.includes(NONCE), box.sent[0].subject);
  assert(!box.sent[0].subject.includes("Lab report"), box.sent[0].subject);
  assertEquals(box.el("over").innerHTML, "", "the compose window closed");
});

Deno.test("m365_forward_names_a_message_or_control_that_is_not_on_the_page_and_sends_nothing", async () => {
  const gone = mailbox(MAIL);
  assertEquals(await run("m365/forward.js", gone.page, { ...TARGET, subject: "Something else" }), {
    result: "not found",
    control: "message",
  });
  const noMenu = mailbox(MAIL);
  noMenu.el("fwd").remove();
  assertEquals(await run("m365/forward.js", noMenu.page, TARGET), {
    result: "not found",
    control: "forward menu",
  });
  const noItem = mailbox(MAIL, { noAttachmentItem: true });
  assertEquals(await run("m365/forward.js", noItem.page, { ...TARGET, wait_ms: 200 }), {
    result: "not found",
    control: "forward as attachment",
  });
  for (const b of [gone, noMenu, noItem]) assertEquals(b.sent, []);
  // An argument that would send somewhere else, or under no nonce, is refused before anything is clicked.
  const bad = mailbox(MAIL);
  for (const args of [{ ...TARGET, address: "not an address" }, { ...TARGET, nonce: "" }]) {
    assertEquals((await run("m365/forward.js", bad.page, args)).result, "failed");
  }
  assertEquals(bad.el("over").innerHTML, "");
});

Deno.test("m365_forward_never_sends_a_compose_with_another_recipient_or_no_attachment", async () => {
  // Outlook offers someone else on the To line: the one address it may send to is Knowlu's.
  const other = mailbox(MAIL, { prefill: "friend@elsewhere.example.invalid" });
  assertEquals(await run("m365/forward.js", other.page, TARGET), {
    result: "failed",
    reason: "recipient not accepted",
  });
  const bare = mailbox(MAIL, { bare: true });
  assertEquals(await run("m365/forward.js", bare.page, TARGET), {
    result: "failed",
    reason: "no attachment",
  });
  assertEquals([...other.sent, ...bare.sent], []);
});

Deno.test("m365_a_non_delivery_report_or_send_failure_in_the_page_is_reported", async () => {
  // A banner that says the send failed, with the compose window left open.
  const banner = mailbox(MAIL, { send: "banner" });
  assertEquals(await run("m365/forward.js", banner.page, TARGET), {
    result: "refused",
    reason: "send failure",
  });
  // The same banner after the window closed.
  const closes = mailbox(MAIL, { send: "banner-closes" });
  assertEquals(await run("m365/forward.js", closes.page, TARGET), {
    result: "refused",
    reason: "send failure",
  });
  // A non-delivery report for this nonce arrives in the Inbox with the send.
  const ndr = mailbox(MAIL, { send: "ndr" });
  assertEquals(await run("m365/forward.js", ndr.page, TARGET), {
    result: "refused",
    reason: "non-delivery report",
  });
  // And one already there stops the next forward before it sends anything (the first refusal ends the routine).
  const later = mailbox(MAIL, { send: "ndr" });
  await run("m365/forward.js", later.page, TARGET);
  const next = await run("m365/forward.js", later.page, {
    ...TARGET,
    sender: "ana.lopez@school.example.invalid",
    subject: "Quiz 3 moved",
    received: "Thu 10/2/2026 3:04 PM",
  });
  assertEquals(next, { result: "refused", reason: "non-delivery report" });
  assertEquals(later.sent.length, 1, "the second forward was never sent");
  // A report for some other batch's nonce is not this batch's refusal.
  const other = mailbox([
    ...MAIL,
    msg("n9", "postmaster@school.example.invalid", "Undeliverable: Knowlu zz99", 3),
  ]);
  assertEquals(await run("m365/forward.js", other.page, TARGET), { result: "ok", sent: true });
});

const COPY = (id: string, to: string, subject: string): Msg => ({ ...msg(id, to, subject, 3, "Sent Items") });

function sentItems(): Box {
  return mailbox([
    COPY("c1", OURS, `Knowlu ${NONCE}`),
    COPY("c2", OURS, `Knowlu ${NONCE}`),
    // R4: the nonce, to someone else; Knowlu's address, without the nonce; a copy to both; the student's own mail.
    COPY("k1", "friend@elsewhere.example.invalid", `Knowlu ${NONCE}`),
    COPY("k2", OURS, "Knowlu a-different-batch"),
    COPY("k3", OURS, "Lunch on Friday"),
    COPY("k4", "friend@elsewhere.example.invalid", "Lunch on Friday"),
  ], { folder: "Sent Items" });
}

Deno.test("m365_sent_cleanup_deletes_only_copies_with_the_nonce_and_the_knowlu_recipient", async () => {
  const box = sentItems();
  const r = await run("m365/sent_cleanup.js", box.page, { address: OURS, nonce: NONCE });
  assertEquals(r, { result: "ok", deleted: 2 });
  assertEquals(box.deleted, ["c1", "c2"]);
});

Deno.test("m365_sent_cleanup_keeps_a_nonce_copy_to_another_recipient_and_a_knowlu_copy_without_the_nonce", async () => {
  const box = sentItems();
  await run("m365/sent_cleanup.js", box.page, { address: OURS, nonce: NONCE });
  // Every other item on the page survives: the two R4 cases and the student's own mail.
  assertEquals(box.msgs.map((m) => m.id), ["k1", "k2", "k3", "k4"]);
  assertEquals(box.rows().length, 4);
  // The nonce alone is not enough, and neither is the address alone.
  const wide = sentItems();
  assertEquals(await run("m365/sent_cleanup.js", wide.page, { address: OURS, nonce: "zz99-no-such" }), {
    result: "ok",
    deleted: 0,
  });
  assertEquals(wide.deleted, []);
  // A copy to Knowlu and to someone else is not one Knowlu made.
  const both = mailbox([COPY("b1", `${OURS}; friend@elsewhere.example.invalid`, `Knowlu ${NONCE}`)], {
    folder: "Sent Items",
  });
  assertEquals((await run("m365/sent_cleanup.js", both.page, { address: OURS, nonce: NONCE })).deleted, 0);
});

Deno.test("m365_sent_cleanup_rechecks_the_row_before_each_delete_when_the_list_reuses_a_node", async () => {
  // After c1 goes, the page reuses c2's node to draw the student's own "Lunch on Friday" (k3).
  const box = mailbox([
    COPY("c1", OURS, `Knowlu ${NONCE}`),
    COPY("c2", OURS, `Knowlu ${NONCE}`),
    COPY("k3", OURS, "Lunch on Friday"),
  ], { folder: "Sent Items", recycle: { node: "c2", as: "k3" } });
  const r = await run("m365/sent_cleanup.js", box.page, { address: OURS, nonce: NONCE });
  assertEquals(r, { result: "ok", deleted: 1 });
  assertEquals(box.deleted, ["c1"]);
  assert(box.msgs.some((m) => m.id === "k3"), "the student's own mail survives");
});

Deno.test("m365_sent_cleanup_never_runs_outside_sent_items_and_refuses_a_bad_nonce", async () => {
  // The Inbox holds a message that looks like one of ours (a reply quoting the nonce, from Knowlu's address).
  const inbox = mailbox([msg("r1", OURS, `Re: Knowlu ${NONCE}`, 4)]);
  assertEquals(await run("m365/sent_cleanup.js", inbox.page, { address: OURS, nonce: NONCE }), {
    result: "not found",
    control: "sent items",
  });
  assertEquals(inbox.deleted, []);
  const box = sentItems();
  assertEquals((await run("m365/sent_cleanup.js", box.page, { address: OURS, nonce: "" })).result, "failed");
  assertEquals((await run("m365/sent_cleanup.js", box.page, { address: "", nonce: NONCE })).result, "failed");
  assertEquals(box.deleted, []);
  const noDelete = sentItems();
  noDelete.el("del").remove();
  assertEquals(await run("m365/sent_cleanup.js", noDelete.page, { address: OURS, nonce: NONCE }), {
    result: "not found",
    control: "delete",
  });
});
