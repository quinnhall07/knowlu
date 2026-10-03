// Microsoft 365, Outlook on the web: the sign-in probe (spec 4.1 step 3, 4.2 step 1). The host's title
// signal says the page looks like a signed-in mailbox; this reads the page's half: the mail list is
// present, and the mailbox's own address, which becomes the mailbox's `mailbox_hint`. A body of an async
// function of `args` (it takes none). It returns one of
//   { result: "ok", signed_in: true, mailbox_hint: "<address, lower case>" }
//   { result: "ok", signed_in: false, mailbox_hint: null }     (no mail list: a sign-in or SSO page)
//   { result: "not found", control: "mailbox address" }          (signed in, address not on the page: guided)
// The address is read from the folder pane only (the tree's own title or label), never from the message
// list, where every address is somebody else's. The caller never logs it.
const list = document.querySelector('[role="listbox"][aria-label*="message list" i]');
if (!list) return { result: "ok", signed_in: false, mailbox_hint: null };

const EMAIL = /^[^\s@<>"]+@[^\s@<>"]+\.[^\s@<>"]+$/;
for (const tree of document.querySelectorAll('[role="tree"]')) {
  for (const item of tree.querySelectorAll('[role="treeitem"], [title], [aria-label]')) {
    for (const value of [item.getAttribute("title"), item.getAttribute("aria-label")]) {
      const v = String(value || "").trim();
      if (EMAIL.test(v)) return { result: "ok", signed_in: true, mailbox_hint: v.toLowerCase() };
    }
  }
}
return { result: "not found", control: "mailbox address" };
