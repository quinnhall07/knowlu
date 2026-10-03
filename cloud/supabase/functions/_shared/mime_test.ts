// Email plan T2c: the parse intake and `mail-process` rest on (spec §4.6 steps 5 and 7, §4.7 step 1,
// D20). Every message here is synthetic: each address, id and domain is invented for this file.
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import { attachedMessages, bodyText, mailUid, messageKey, parseHeaders } from "./mime.ts";

/** Lines joined by CRLF, as mail is on the wire. */
const crlf = (lines: string[]) => lines.join("\r\n");
const latin1 = (b: Uint8Array) => Array.from(b, (c) => String.fromCharCode(c)).join("");
const utf8 = (s: string) => new TextEncoder().encode(s);

Deno.test("parses_from_date_subject_message_id_list_headers_and_precedence", () => {
  const raw = crlf([
    "Received: by mx.example.test with SMTP id x1; Thu, 01 Oct 2026 14:31:05 +0000",
    'From: "Doe, Jane" (staff) <Jane.Doe@Example.TEST>',
    "To: someone@in.example.test",
    "Subject: =?UTF-8?B?Q2Fmw6k=?= hours",
    " =?ISO-8859-1?Q?=E9t=E9?=",
    "Date: Thu, 1 Oct 2026 09:30:00 -0500 (CDT)",
    "Message-ID: <abc.123@mail.example.test>",
    "List-Unsubscribe: <mailto:leave@lists.example.test>",
    "List-ID: Course News <news.example.test>",
    "Precedence: Bulk",
    "Content-Type: text/plain",
    "",
    "Body.",
  ]);
  for (const input of [raw, raw.replaceAll("\r\n", "\n"), utf8(raw)]) {
    const h = parseHeaders(input);
    assertEquals(h.fromName, "Doe, Jane");
    assertEquals(h.fromAddress, "jane.doe@example.test");
    assertEquals(h.date, "Thu, 1 Oct 2026 09:30:00 -0500 (CDT)");
    assertEquals(h.dateAt, Date.UTC(2026, 9, 1, 14, 30, 0));
    assertEquals(h.subject, "Café hours été");
    assertEquals(h.messageId, "abc.123@mail.example.test");
    assertEquals(h.list, {
      "list-unsubscribe": "<mailto:leave@lists.example.test>",
      "list-id": "Course News <news.example.test>",
    });
    assertEquals(h.precedence, "bulk");
    // Every field is kept in order, with its exact bytes, for the signature checks (T2d).
    assertEquals(h.headers.map((f) => f.name.toLowerCase()).slice(0, 4), [
      "received",
      "from",
      "to",
      "subject",
    ]);
  }
  const folded = parseHeaders(raw).headers[3];
  assertEquals(folded.raw, "Subject: =?UTF-8?B?Q2Fmw6k=?= hours\r\n =?ISO-8859-1?Q?=E9t=E9?=");

  // A plain message: none of the optional headers, an address alone, adjacent encoded words joined.
  const plain = parseHeaders(
    "From: Prof@Dept.Example.EDU\nSubject: =?UTF-8?Q?Due_?= =?UTF-8?Q?Fri=C3=A9day?=\n\nhi\n",
  );
  assertEquals(plain.fromName, null);
  assertEquals(plain.fromAddress, "prof@dept.example.edu");
  assertEquals(plain.subject, "Due Friéday");
  assertEquals(plain.messageId, null);
  assertEquals(plain.date, null);
  assertEquals(plain.dateAt, null);
  assertEquals(plain.list, {});
  assertEquals(plain.precedence, null);

  // An encoded display name, and raw UTF-8 in a header (RFC 6532), and a mailbox list.
  const intl = parseHeaders(
    utf8("From: =?UTF-8?Q?Jos=C3=A9?= <jose@example.test>, other@example.test\nSubject: Ré: plan\n\n"),
  );
  assertEquals(intl.fromName, "José");
  assertEquals(intl.fromAddress, "jose@example.test");
  assertEquals(intl.subject, "Ré: plan");

  // A From with no usable address names none.
  assertEquals(parseHeaders("From: undisclosed\n\n").fromAddress, null);
});

Deno.test("takes_the_first_text_plain_part_else_reduces_html_to_text", () => {
  // multipart/mixed > (multipart/alternative > text/plain QP, text/html), then a text attachment.
  const alt = crlf([
    "From: a@example.test",
    'Content-Type: multipart/mixed; boundary="outer"',
    "",
    "preamble",
    "--outer",
    "Content-Type: multipart/alternative; boundary=inner",
    "",
    "--inner",
    "Content-Type: text/plain; charset=utf-8",
    "Content-Transfer-Encoding: quoted-printable",
    "",
    "Caf=C3=A9 essay due Friday=",
    " at noon.",
    "--inner",
    "Content-Type: text/html; charset=utf-8",
    "",
    "<p>not this one</p>",
    "--inner--",
    "--outer",
    "Content-Type: text/plain; name=notes.txt",
    "Content-Disposition: attachment; filename=notes.txt",
    "",
    "attached notes",
    "--outer--",
    "epilogue",
  ]);
  assertEquals(bodyText(alt), "Café essay due Friday at noon.");

  // HTML only, base64: scripts, styles, comments and tags go; entities and blocks read as text.
  const html = "<html><head><title>T</title><style>p{color:red}</style></head><body>" +
    "<!-- hidden --><script>var x = '<p>no</p>';</script>" +
    "<h1>Lab&nbsp;3</h1><p>Due <b>Oct&nbsp;9</b> &amp; worth 5&#37;</p>" +
    "<div>Room&#x20;B&lt;2&gt;</div><br>Bring &quot;goggles&quot; &unknown;</body></html>";
  const b64 = btoa(latin1(utf8(html)));
  const htmlOnly = crlf([
    "From: a@example.test",
    "Content-Type: text/html; charset=UTF-8",
    "Content-Transfer-Encoding: base64",
    "",
    b64.slice(0, 40),
    b64.slice(40),
  ]);
  assertEquals(bodyText(htmlOnly), 'Lab 3\nDue Oct 9 & worth 5%\nRoom B<2>\nBring "goggles" &unknown;');

  // The only text/plain is inside an attached message: it is not opened, so the HTML part is read.
  const nested = crlf([
    "Content-Type: multipart/mixed; boundary=b1",
    "",
    "--b1",
    "Content-Type: text/html",
    "",
    "<p>outer html</p>",
    "--b1",
    "Content-Type: message/rfc822",
    "",
    "Subject: inner",
    "Content-Type: text/plain",
    "",
    "inner plain text",
    "--b1--",
  ]);
  assertEquals(bodyText(nested), "outer html");

  // A text/plain marked `attachment` is not the body, even when it comes first.
  const attachedFirst = crlf([
    "Content-Type: multipart/mixed; boundary=b2",
    "",
    "--b2",
    "Content-Type: text/plain",
    'Content-Disposition: attachment; filename="a.txt"',
    "",
    "attached text",
    "--b2",
    "Content-Type: text/html",
    "",
    "<div>the body</div>",
    "--b2--",
  ]);
  assertEquals(bodyText(attachedFirst), "the body");

  // A named file is an attachment whatever its disposition (Apple Mail and Outlook mark files
  // `inline`): an inline text/plain with a filename, a text/plain with only a `name=`, and an
  // RFC 2231 `filename*` are never the body, so the HTML part is read.
  const namedFile = (fileHeaders: string[]) =>
    crlf([
      "Content-Type: multipart/mixed; boundary=b3",
      "",
      "--b3",
      "Content-Type: text/html",
      "",
      "<p>the html body</p>",
      "--b3",
      ...fileHeaders,
      "",
      "roster contents",
      "--b3--",
    ]);
  for (
    const fileHeaders of [
      ['Content-Type: text/plain; name="roster.txt"', 'Content-Disposition: inline; filename="roster.txt"'],
      ['Content-Type: text/plain; name="roster.txt"'],
      ["Content-Type: text/plain", "Content-Disposition: inline; filename*=utf-8''roster.txt"],
      ["Content-Type: text/plain; name*0=ros; name*1=ter.txt"],
    ]
  ) assertEquals(bodyText(namedFile(fileHeaders)), "the html body");
  // An inline disposition with no file name is still a body.
  assertEquals(
    bodyText(namedFile(["Content-Type: text/plain", "Content-Disposition: inline"])),
    "roster contents",
  );

  // No Content-Type is text/plain; a declared single-byte charset is honoured; CRLF reads as LF.
  assertEquals(bodyText("Subject: x\r\n\r\nline one\r\nline two\r\n"), "line one\nline two");
  const latin = new Uint8Array([...utf8("Content-Type: text/plain; charset=iso-8859-1\n\nr"), 0xe9, 0x73]);
  assertEquals(bodyText(latin), "rés");

  // Nothing readable is an empty text, never an error.
  assertEquals(bodyText("Content-Type: image/png\n\n\x89PNG"), "");
});

Deno.test("extracts_each_message_rfc822_attachment_and_opens_no_other_part", () => {
  // An original that itself carries an attached message: that inner one is not a work item.
  const first = crlf([
    "Message-ID: <first@example.test>",
    "Subject: first",
    "Content-Type: multipart/mixed; boundary=f1",
    "",
    "--f1",
    "Content-Type: text/plain",
    "",
    "first body",
    "--f1",
    "Content-Type: message/rfc822",
    "",
    "Message-ID: <inside-first@example.test>",
    "",
    "never extracted",
    "--f1--",
  ]);
  const second = crlf(["Message-ID: <second@example.test>", "Subject: second", "", "second body"]);
  const third = crlf(["Message-ID: <third@example.test>", "Subject: third", "", "third body"]);
  const batch = crlf([
    "From: me@school.example.edu",
    "Subject: knowlu batch n0nce",
    'Content-Type: multipart/mixed; boundary="==b=="',
    "",
    "--==b==",
    "Content-Type: text/plain",
    "",
    "outer text",
    "--==b==",
    "Content-Type: message/rfc822",
    "Content-Disposition: attachment; filename=first.eml",
    "",
    first,
    "--==b==",
    // Not a message: never decoded, so its broken base64 cannot matter.
    "Content-Type: application/pdf",
    "Content-Transfer-Encoding: base64",
    "",
    "!!!not base64!!!",
    "--==b==",
    // A message sent base64-encoded (some clients do) is decoded back to its bytes.
    "Content-Type: message/rfc822",
    "Content-Transfer-Encoding: base64",
    "",
    btoa(second),
    "--==b==",
    // A container is walked for messages; a message inside a message is not.
    "Content-Type: multipart/mixed; boundary=deeper",
    "",
    "--deeper",
    "Content-Type: message/rfc822",
    "",
    third,
    "--deeper--",
    "--==b==--",
  ]);
  const got = attachedMessages(batch);
  assertEquals(got.map(latin1), [first, second, third]);
  assertEquals(got.map((m) => parseHeaders(m).messageId), [
    "first@example.test",
    "second@example.test",
    "third@example.test",
  ]);

  // A single-part message, and a multipart without a boundary, carry none.
  assertEquals(attachedMessages("Subject: x\n\nbody"), []);
  assertEquals(attachedMessages("Content-Type: multipart/mixed\n\n--x\n\nbody\n--x--"), []);
  // multipart/digest's parts are messages by default (RFC 2046 §5.1.5).
  const digest = "Content-Type: multipart/digest; boundary=d\n\n--d\n\nSubject: in digest\n\nhi\n--d--\n";
  assertEquals(attachedMessages(digest).map(latin1), ["Subject: in digest\n\nhi"]);
});

Deno.test("reads_the_topmost_received_time", () => {
  const raw = crlf([
    "Received: by 2002:a05:6000:1 with SMTP id q1csp2;",
    "        Fri, 2 Oct 2026 07:12:13 -0700 (PDT)",
    "Received: from relay.example.test (relay.example.test [192.0.2.1]) by mx.example.test",
    "        with ESMTPS id r2; Fri, 02 Oct 2026 14:12:10 +0000",
    "Received: from origin.example.test; Thu, 1 Oct 2026 23:59:59 GMT",
    "Date: Fri, 2 Oct 2026 14:11:00 +0000",
    "",
    "",
  ]);
  const h = parseHeaders(raw);
  assertEquals(h.receivedAt, Date.UTC(2026, 9, 2, 14, 12, 13));
  assertEquals(h.deliveredAt, h.receivedAt);

  // No Received: the delivery time is the Date.
  const noReceived = parseHeaders("Date: 2 Oct 2026 08:00 EDT\n\n");
  assertEquals(noReceived.receivedAt, null);
  assertEquals(noReceived.dateAt, Date.UTC(2026, 9, 2, 12, 0, 0));
  assertEquals(noReceived.deliveredAt, noReceived.dateAt);

  // An unreadable topmost stamp falls back to the Date, never to a lower (sender-side) hop.
  const unreadable = parseHeaders(crlf([
    "Received: by mx.example.test; yesterday-ish",
    "Received: from origin.example.test; Thu, 1 Oct 2026 23:59:59 +0000",
    "Date: Thu, 1 Oct 2026 20:00:00 +0000",
    "",
    "",
  ]));
  assertEquals(unreadable.receivedAt, null);
  assertEquals(unreadable.deliveredAt, Date.UTC(2026, 9, 1, 20, 0, 0));

  // Dates that do not exist are unreadable, not rolled over.
  assertEquals(parseHeaders("Date: Mon, 31 Feb 2026 10:00:00 +0000\n\n").dateAt, null);
  assertEquals(parseHeaders("Date: Mon, 1 Feb 2026 25:00:00 +0000\n\n").dateAt, null);
  assertEquals(parseHeaders("Date: soon\n\n").deliveredAt, null);
});

Deno.test("a_message_without_a_message_id_keys_on_date_from_and_subject", async () => {
  // With a Message-ID the key is the id alone: sha256("abc") pins the uid's form (D20).
  const a = parseHeaders("Message-ID: <abc>\nSubject: one\n\n");
  const b = parseHeaders("Message-Id:   <abc>  \nSubject: two\nFrom: x@example.test\n\n");
  assertEquals(messageKey(a), "abc");
  assertEquals(messageKey(b), "abc");
  assertEquals(await mailUid(a), "mail:ba7816bf8f01cfea414140de5dae2223");
  assertEquals(await mailUid(b), await mailUid(a));

  // Without one: Date, From and Subject. The same three by another route key the same.
  const lines = (date: string, from: string, subject: string, body: string) =>
    `Date: ${date}\nFrom: ${from}\nSubject: ${subject}\n\n${body}`;
  const c = parseHeaders(lines("Fri, 2 Oct 2026 10:00:00 -0400", "Prof <P@Example.EDU>", "Quiz", "1"));
  const d = parseHeaders(
    lines("2 Oct 2026 14:00:00 +0000", '"Prof" <p@example.edu>', "=?UTF-8?Q?Quiz?=", "2"),
  );
  assertEquals(messageKey(c), messageKey(d));
  assertEquals(await mailUid(c), await mailUid(d));
  assert(/^mail:[0-9a-f]{32}$/.test(await mailUid(c)));

  // Any one of the three changing changes the key.
  const others = [
    lines("Fri, 2 Oct 2026 10:01:00 -0400", "Prof <p@example.edu>", "Quiz", "1"),
    lines("Fri, 2 Oct 2026 10:00:00 -0400", "Prof <q@example.edu>", "Quiz", "1"),
    lines("Fri, 2 Oct 2026 10:00:00 -0400", "Prof <p@example.edu>", "Quiz 2", "1"),
  ];
  for (const other of others) assertNotEquals(messageKey(parseHeaders(other)), messageKey(c));
  // A key without an id can never equal an id's key (an id cannot begin with `[`).
  assert(messageKey(c).startsWith("["));
  assertEquals(parseHeaders('Message-ID: <["x","y","z"]>\n\n').messageId, null);
});

Deno.test("hostile_structure_is_bounded_and_never_throws", () => {
  // Nesting far past the depth cap, an unterminated boundary, headers with no colon, NUL bytes.
  let deep = "Content-Type: message/rfc822\n\nSubject: bottom\n\nbottom";
  for (let i = 0; i < 200; i++) {
    deep = `Content-Type: multipart/mixed; boundary=l${i}\n\n--l${i}\n${deep}\n--l${i}--`;
  }
  assertEquals(attachedMessages(deep), []);
  assertEquals(bodyText(deep), "");
  const broken = "no colon here\n\tcontinued\nContent-Type: multipart/mixed; boundary=z\n\n--z\n" +
    "Content-Type: text/plain\n\nunterminated \u0000 text";
  assertEquals(bodyText(broken), "unterminated \u0000 text");
  assertEquals(parseHeaders(broken).headers.map((f) => f.name), ["Content-Type"]);
  const many = "Content-Type: multipart/mixed; boundary=m\n\n" +
    "--m\nContent-Type: message/rfc822\n\nSubject: s\n\nx\n".repeat(5000) + "--m--";
  assert(attachedMessages(many).length <= 500);
  // Markup built to make a backtracking stripper quadratic is read in one pass.
  const started = performance.now();
  bodyText(`Content-Type: text/html\n\n${"<a<b".repeat(250_000)}${"<script>".repeat(50_000)}`);
  assert(performance.now() - started < 2_000);
  for (const junk of ["", "\r\n\r\n", ":", 'Content-Type: multipart/x; boundary=""\n\n--\n']) {
    parseHeaders(junk);
    bodyText(junk);
    attachedMessages(junk);
  }
});
