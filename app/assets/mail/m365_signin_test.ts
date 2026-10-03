// Spec test 35, the Microsoft 365 sign-in probe (plan T10.2a, spec 4.1 step 3 and 4.2 step 1): the
// in-page half of "done when the title reads as a signed-in mailbox and the mail list is present", and
// the signed-in mailbox's own address, read from the page, for `mailbox_hint`. Synthetic pages only.
import { assertEquals } from "@std/assert";
import { load, run } from "./test_dom.ts";

const MINE = "Student.Box@School.Example.Invalid";

const LIST = `<div role="listbox" aria-label="Message list"><div role="option">A message</div></div>`;
const FOLDERS = (title: string) =>
  `<div role="tree" aria-label="Folders">
  <div role="treeitem" title="${title}" aria-label="${title}">${title}</div>
  <div role="treeitem" title="Inbox">Inbox</div></div>`;

Deno.test("m365_the_sign_in_probe_reads_the_list_and_the_mailbox_address", async () => {
  const page = load(FOLDERS(MINE) + LIST);
  assertEquals(await run("m365/signin.js", page), {
    result: "ok",
    signed_in: true,
    mailbox_hint: "student.box@school.example.invalid",
  });
});

Deno.test("m365_the_sign_in_probe_says_not_signed_in_on_a_page_with_no_mail_list", async () => {
  const page = load(`<form><input type="email" aria-label="Email"><button>Next</button></form>`);
  assertEquals(await run("m365/signin.js", page), { result: "ok", signed_in: false, mailbox_hint: null });
});

Deno.test("m365_the_sign_in_probe_names_a_missing_address_not_found", async () => {
  // A list with no mailbox address anywhere: guided, never a guess.
  const page = load(FOLDERS("Inbox") + LIST);
  assertEquals(await run("m365/signin.js", page), { result: "not found", control: "mailbox address" });
});

Deno.test("m365_the_sign_in_probe_ignores_an_address_that_is_not_the_mailbox", async () => {
  // A message sender's address is in the list, not in the folder tree: it is never the mailbox.
  const page = load(`${FOLDERS("Inbox")}<div role="listbox" aria-label="Message list">
    <div role="option"><span title="sender@other.example.invalid">Sender</span></div></div>`);
  assertEquals(await run("m365/signin.js", page), { result: "not found", control: "mailbox address" });
});
