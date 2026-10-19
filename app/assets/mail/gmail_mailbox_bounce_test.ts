// Spec test 35, the Gmail mailbox half, the Inbox look for a non-delivery report (spec 4.2 step 5, risk 15;
// Quinn's ruling 2026-10-19 on the T18c gate "Gmail non-delivery report between sends"). `gmail/bounce.js`
// reads the shown Inbox for a report of this batch's forwards and answers in counts and fixed words. Synthetic
// pages only; every address is `.example.invalid`.
import { assert, assertEquals } from "@std/assert";
import { mailbox, Msg, msg, NONCE, OURS, report } from "./gmail_mailbox_pages.ts";
import { load, run } from "./test_dom.ts";

const FAST = { wait_ms: 300 };
const MAIL = [
  msg("m1", "ana.lopez@school.example.invalid", "Quiz 3 moved", 2),
  msg("m2", "bo.chen@school.example.invalid", "Lab report due", 5),
  msg("m3", "reg@school.example.invalid", "Registration open", 9),
];
// The batch opened on the 6th, at midnight (the page's own zone, as the rows' times are read).
const SINCE = new Date(2026, 9, 6, 0, 0).toISOString();
const ARGS = { address: OURS, nonce: NONCE, since: SINCE, ...FAST };
const REFUSED = { result: "refused", reason: "non-delivery report" };
// A report that names the address and quotes no subject, and one that quotes the nonce and names no address.
const NAMING = (day: number) => report("n1", day);
const QUOTING = (day: number): Msg =>
  report("n2", day, { snippet: `Your message wasn't delivered. Subject: Knowlu ${NONCE}` });

Deno.test("gmail_bounce_finds_no_report_in_an_inbox_without_one", async () => {
  const box = mailbox(MAIL);
  assertEquals(await run("gmail/bounce.js", box.page, ARGS), { result: "ok", rows: 3 });
  // It only reads: nothing is ticked, opened or sent.
  assertEquals([box.checked(), box.sent, box.deleted, box.discards], [[], [], [], 0]);
  assertEquals(box.el("over").innerHTML, "");
});

Deno.test("gmail_bounce_refuses_a_report_that_carries_the_batch_nonce", async () => {
  // The nonce is enough, whatever the report's time: it can only be a report of this batch.
  const box = mailbox([...MAIL, QUOTING(2)]);
  assertEquals(await run("gmail/bounce.js", box.page, ARGS), REFUSED);
  assertEquals(await run("gmail/bounce.js", box.page, { ...ARGS, since: undefined }), REFUSED);
});

Deno.test("gmail_bounce_refuses_a_report_naming_the_address_that_arrived_since_the_batch_opened", async () => {
  const newer = mailbox([...MAIL, NAMING(10)]);
  assertEquals(await run("gmail/bounce.js", newer.page, ARGS), REFUSED);
  // The same row from before the batch opened is an earlier batch's, or a stranger's: not ours.
  const older = mailbox([...MAIL, NAMING(4)]);
  assertEquals(await run("gmail/bounce.js", older.page, ARGS), { result: "ok", rows: 4 });
  // A row's time has no seconds: a report in the minute the batch opened still counts.
  const minute = mailbox([...MAIL, NAMING(6)]);
  const late = new Date(2026, 9, 6, 15, 4, 45).toISOString();
  assertEquals(await run("gmail/bounce.js", minute.page, { ...ARGS, since: late }), REFUSED);
  // Without a `since` or with one that is not a time, the address alone is never enough.
  assertEquals(await run("gmail/bounce.js", newer.page, { ...ARGS, since: undefined }), {
    result: "ok",
    rows: 4,
  });
  assertEquals(await run("gmail/bounce.js", newer.page, { ...ARGS, since: "not a time" }), {
    result: "ok",
    rows: 4,
  });
});

Deno.test("gmail_bounce_reads_only_the_shown_grid", async () => {
  // A search keeps the Inbox's grid in the document, hidden, and draws its own: the report is in the hidden one.
  const box = mailbox([...MAIL, QUOTING(2)]);
  box.page.location.hash = "#search/after%3A2026%2F10%2F03";
  await new Promise((done) => setTimeout(done, 100));
  assertEquals(
    box.page.document.querySelectorAll('table[role="grid"]').length,
    2,
    "the Inbox's grid is kept",
  );
  // The Inbox shows again (the hash is back at #inbox) over the search's grid, which is the one shown.
  box.page.location.hash = "#inbox";
  assertEquals(await run("gmail/bounce.js", box.page, ARGS), { result: "ok", rows: 2 });
  // And a hidden grid drawn after the shown one (the order is Gmail's to choose) is not read either.
  const late = mailbox(MAIL);
  late.el("views").insertAdjacentHTML(
    "beforeend",
    `<div style="display: none"><table role="grid"><tbody><tr role="row"><td><div role="checkbox"></div></td>
      <td><span class="bog">Delivery Status Notification (Failure)</span> Knowlu ${NONCE}</td></tr></tbody></table></div>`,
  );
  assertEquals(await run("gmail/bounce.js", late.page, ARGS), { result: "ok", rows: 3 });
});

Deno.test("gmail_bounce_ignores_a_row_with_the_nonce_and_no_failure_phrase", async () => {
  // An auto-reply that quotes the subject, and a note that names the address, say nothing of delivery.
  const box = mailbox([
    ...MAIL,
    { ...msg("r1", "ana.lopez@school.example.invalid", "Out of office", 9), snippet: `Re: Knowlu ${NONCE}` },
    {
      ...msg("r2", "bo.chen@school.example.invalid", "Your forwarding address", 10),
      snippet: `It is ${OURS}`,
    },
  ]);
  assertEquals(await run("gmail/bounce.js", box.page, ARGS), { result: "ok", rows: 5 });
});

Deno.test("gmail_bounce_knows_gmails_failure_phrases", async () => {
  for (
    const [subject, snippet] of [
      ["Delivery Status Notification (Failure)", "a preview"],
      ["Notification", "Address not found"],
      ["Notification", "Your message wasn't delivered to someone"],
      ["Undeliverable: hello", "a preview"],
    ]
  ) {
    const box = mailbox([...MAIL, {
      ...msg("n3", "mailer-daemon@mail.example.invalid", subject, 2),
      snippet: `${snippet} Knowlu ${NONCE}`,
    }]);
    assertEquals(await run("gmail/bounce.js", box.page, ARGS), REFUSED, `${subject} / ${snippet}`);
  }
});

Deno.test("gmail_bounce_says_when_no_inbox_is_drawn_and_refuses_bad_arguments", async () => {
  const none = load("<p>Loading</p>");
  assertEquals(await run("gmail/bounce.js", none, ARGS), { result: "not found", control: "inbox" });
  const box = mailbox(MAIL);
  assertEquals(await run("gmail/bounce.js", box.page, { ...ARGS, address: "" }), {
    result: "failed",
    reason: "no address",
  });
  assertEquals(await run("gmail/bounce.js", box.page, { ...ARGS, nonce: "" }), {
    result: "failed",
    reason: "no nonce",
  });
  assertEquals(await run("gmail/bounce.js", box.page, { ...ARGS, nonce: "two words" }), {
    result: "failed",
    reason: "no nonce",
  });
});

Deno.test("gmail_bounce_returns_counts_and_fixed_words_only", async () => {
  const box = mailbox([...MAIL, NAMING(10), QUOTING(11)]);
  for (const extra of [{}, { nonce: "zz99-no-such" }]) {
    const out = JSON.stringify(await run("gmail/bounce.js", box.page, { ...ARGS, ...extra }));
    for (const word of ["example.invalid", "Delivery", "Knowlu", NONCE, "Quiz", "mailer"]) {
      assert(!out.includes(word), `${out} names ${word}`);
    }
  }
});

Deno.test("gmail_bounce_and_forward_read_one_list_of_failure_phrases", async () => {
  // forward.js looks at the page it is on and bounce.js at the Inbox: a report one stops on, the other must.
  const phrases = async (name: string) => {
    const source = await Deno.readTextFile(new URL(`gmail/${name}`, import.meta.url));
    return /const REPORT =\s*(\/.*\/i);/.exec(source)?.[1];
  };
  const forward = await phrases("forward.js");
  assert(forward, "forward.js names its phrases");
  assertEquals(await phrases("bounce.js"), forward);
  assert(new RegExp(forward.slice(1, -2), "i").test("Address not found"), "Gmail's own heading");
});

const GRID = (row: string) =>
  `<div><table role="grid"><tbody><tr role="row"><td><div role="checkbox"></div></td>${row}</tr></tbody></table></div>`;

Deno.test("gmail_bounce_reads_every_shown_grid_not_the_last", async () => {
  // "Unread first", "Important first" and Priority Inbox draw several grids, all shown.
  const box = mailbox([...MAIL, QUOTING(2)]);
  box.el("views").insertAdjacentHTML(
    "beforeend",
    GRID(`<td><span class="bog">Quiz 4 moved</span> a preview</td>`),
  );
  assertEquals(box.page.document.querySelectorAll('table[role="grid"]').length, 2);
  assertEquals(await run("gmail/bounce.js", box.page, ARGS), REFUSED);
  // With no report in either, it counts the rows of both.
  const clean = mailbox(MAIL);
  clean.el("views").insertAdjacentHTML("beforeend", GRID(`<td><span class="bog">Quiz 4 moved</span></td>`));
  assertEquals(await run("gmail/bounce.js", clean.page, ARGS), { result: "ok", rows: 4 });
});

Deno.test("gmail_bounce_reads_only_the_inbox", async () => {
  // A search, Sent or a thread is not the Inbox, whatever grid is shown: nothing is read.
  const box = mailbox([...MAIL, QUOTING(2)]);
  box.page.location.hash = "#search/after%3A2026%2F10%2F01";
  await new Promise((done) => setTimeout(done, 100));
  const none = { result: "not found", control: "inbox" };
  assertEquals(await run("gmail/bounce.js", box.page, ARGS), none);
  for (const hash of ["#sent", "#label/School", "#inboxes", "", "#category/updates"]) {
    box.page.location.hash = hash;
    assertEquals(await run("gmail/bounce.js", box.page, ARGS), none, hash);
  }
  // The Inbox, its pages and an open view of it (a thread hides the list: no grid is shown then).
  for (const hash of ["#inbox", "#inbox/p2", "#inbox?compose=new"]) {
    const inbox = mailbox([...MAIL, QUOTING(2)]);
    inbox.page.location.hash = hash;
    assertEquals(await run("gmail/bounce.js", inbox.page, ARGS), REFUSED, hash);
  }
});
