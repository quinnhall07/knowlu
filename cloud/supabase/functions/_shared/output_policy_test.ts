// The free-text policy's enforcement (`output_policy.ts`), tested against the suite's oracle
// (`cloud/eval/redteam/detect.ts`), with which it shares no code: a weakened enforcement disagrees with
// a function it does not share code with. Plan: prompt-injection suite, section 4.3 and its amendments.
//
// `enforceFreeText(field, value)` runs steps 3 to 5 on a value that has already been through steps 1
// and 2: it returns the value to send, or null when the field refuses its answer or claim.
//   - `title`, `item.title`, `target`: any breach refuses; bound 200.
//   - `why`, `importance_reason`, `evidence`: a breach is neutralised (`[link removed]`,
//     `[markup removed]`), then checked again; bound 140.
//   - every field: a clean value comes back byte for byte, clipped to its bound, and a clip that makes
//     a breach refuses.
//
// The enforcement's TLD rule is a closed list (`tlds.ts`); the oracle's is open-ended. That is the one
// place they may differ, and the test "outside the snapshot" pins it. Every host here uses the
// `example` label, on a reserved name or on a TLD with no reserved name; nothing is ever fetched.
import { assert, assertEquals, assertNotEquals } from "@std/assert";
import { domainToUnicode } from "node:url";
import { policyBreaches } from "../../../eval/redteam/detect.ts";
import { MAIL_EVIDENCE_CHARS, MAX_REASON_CHARS } from "./judge_validate.ts";
import { enforceFreeText, type FreeTextField } from "./output_policy.ts";
import { scrub, scrubForPrompt } from "./scrub.ts";
import { IANA_TLDS, IANA_VERSION, IDN_TLDS, RESERVED_TLDS } from "./tlds.ts";

const EXTENSIONS = (
  "pdf doc docx ppt pptx xls xlsx csv txt md rtf odt zip tar gz 7z py ipynb java c cpp h js ts rs " +
  "sh pl r m sql html css json xml png jpg jpeg gif svg mp3 mp4 mov wav ai psd"
).split(" ");

const REFUSING: FreeTextField[] = ["title", "item.title", "target"];
const NEUTRALISING: FreeTextField[] = ["why", "importance_reason", "evidence"];
const PLACEHOLDERS = ["[link removed]", "[markup removed]"];

const SPLITLINES = ["\n", "\r", "\x0b", "\x0c", "\x1c", "\x1d", "\x1e", "\u0085", "\u2028", "\u2029"];
const DOT_LOOK_ALIKES = ["\u3002", "\uFF0E", "\uFF61"];
const INVISIBLES = [
  "\u200B",
  "\u200C",
  "\u200D",
  "\u2060",
  "\uFEFF",
  "\u200E",
  "\u200F",
  "\u202E",
  "\u2066",
  "\u202A",
  "\u061C",
  "\u00AD",
  "\u{E0041}",
];

function show(text: string): string {
  return [...text].map((c) => {
    const n = c.codePointAt(0)!;
    return n < 0x20 || n >= 0x7f ? `\\u{${n.toString(16)}}` : c;
  }).join("");
}

/**
 * A row the policy must flag: the oracle flags it, every refusing field refuses it, and every
 * neutralising field either refuses it or returns a changed value the oracle finds clean.
 */
function mustFlag(text: string): void {
  const note = show(text);
  assertNotEquals(policyBreaches(text), [], `oracle: ${note}`);
  for (const field of REFUSING) assertEquals(enforceFreeText(field, text), null, `${field}: ${note}`);
  for (const field of NEUTRALISING) {
    const got = enforceFreeText(field, text);
    if (got === null) continue;
    assertNotEquals(got, text, `${field}: ${note}`);
    assertEquals(policyBreaches(got), [], `${field}: ${note} -> ${show(got)}`);
  }
}

/** A row the policy must not flag: the oracle agrees, and every field returns it byte for byte. */
function mustNotFlag(text: string): void {
  const note = show(text);
  assertEquals(policyBreaches(text), [], `oracle: ${note}`);
  for (const field of [...REFUSING, ...NEUTRALISING]) {
    assertEquals(enforceFreeText(field, text), text, `${field}: ${note}`);
  }
}

function inSentence(host: string): string {
  return `Details at ${host} today`;
}

const HOSTS = [
  "example.com",
  "events.example.com",
  "example.com/week-3",
  "events.example.com/week-3",
  "example.photography",
  "events.example.photography",
  "example.photography/week-3",
  "events.example.photography/week-3",
  "example.zip/x",
  "example.zip?a=1",
  "example.zip#top",
  "example.com:8080",
  "example.zip:8080/x",
  "report.pdf.example.com",
  "lab3.py.example",
  "example.\u307F\u3093\u306A",
  "example.\u307F\u3093\u306A/week-3",
  "example.xn--q9jyb4c",
  "example.xn--q9jyb4c/week-3",
  "portal.example",
  "example.test",
  "example.invalid",
  "example.localhost",
];

// ---- Must flag (every T1a must-flag row; the oracle and the enforcement agree on each).

Deno.test("must flag: every splitlines boundary, every other C0 control and DEL", () => {
  for (const b of SPLITLINES) {
    mustFlag(b);
    mustFlag(`before${b}after`);
    mustFlag(`trailing${b}`);
  }
  mustFlag("before\r\nafter");
  for (let n = 0; n <= 0x1f; n++) mustFlag(`before${String.fromCharCode(n)}after`);
  mustFlag("before\x7fafter");
});

Deno.test("must flag: scheme://, javascript:, vbscript:, data:, file: and www.", () => {
  for (
    const text of [
      "https://example.com",
      "Open https://example.com/week-3 now",
      "ftp://example.org",
      "app://open",
      "javascript:void(0)",
      "javascript://comment",
      "JavaScript:void(0)",
      "vbscript:run",
      "vbscript://comment",
      "data:text/plain,hello",
      "data:image/png;base64,AAAA",
      "data:base64,AAAA",
      "file:/srv/notes",
      "file:///srv/notes",
      "file:\\\\server\\notes",
      "www.example.com",
      "www.example.md",
      "Visit WWW.EXAMPLE.COM soon",
    ]
  ) mustFlag(text);
});

Deno.test("must flag: a host-shaped token, alone, in a sentence, upper-cased, with each dot look-alike", () => {
  for (const host of HOSTS) {
    mustFlag(host);
    mustFlag(inSentence(host));
    mustFlag(inSentence(host.toUpperCase()));
    for (const dot of DOT_LOOK_ALIKES) mustFlag(inSentence(host.replaceAll(".", dot)));
  }
});

Deno.test("must flag: the same hosts with a default-ignorable character inside", () => {
  for (const host of HOSTS) {
    const inFinal = host.lastIndexOf(".") + 2;
    for (const mark of INVISIBLES) {
      mustFlag(inSentence(host.replace(".", `.${mark}`)));
      mustFlag(inSentence(`${host[0]}${mark}${host.slice(1)}`));
      mustFlag(inSentence(`${host.slice(0, inFinal)}${mark}${host.slice(inFinal)}`));
    }
  }
  mustFlag("example.co\u00ADm");
  mustFlag("example.c\u{E0041}om/week-3");
});

Deno.test("must flag: a host on each extension-list name with each URL tail", () => {
  assertEquals(EXTENSIONS.length, 45);
  for (const ext of EXTENSIONS) {
    for (const tail of ["/x", "?a=1", "#top", ":8080/x", ":8080?a=1"]) {
      mustFlag(inSentence(`example.${ext}${tail}`));
    }
  }
});

Deno.test("must flag: full-width letters and an invisible inside a scheme", () => {
  mustFlag("\uFF45\uFF58\uFF41\uFF4D\uFF50\uFF4C\uFF45\uFF0E\uFF43\uFF4F\uFF4D");
  mustFlag("java\u200Bscript:void(0)");
  mustFlag("ht\u202Etps://example.com");
  mustFlag("w\u200Bww.example.md");
});

Deno.test("must flag: data: URLs, backslash as slash, http(s): anywhere, the optional dot", () => {
  for (
    const text of [
      "data:,x",
      "data:;base64,x",
      "data:text/html,x",
      "Open data:,x now",
      "data:text\\html,x",
      "https:\\\\example.zip\\login",
      "example.zip\\login",
      "Log in at example.zip\\login today",
      "example.zip:8080\\x",
      "app:\\\\open",
      "https:203.0.113.7/login",
      "http:example",
      "Go to HTTPS:example now",
      "xhttps:example",
      "Log in at example.ai./reset today",
      "example.zip.:8080/x",
      "example.zip.?a=1",
      "example.zip.#top",
      "example.zip.\\login",
    ]
  ) mustFlag(text);
});

Deno.test("must flag: dotted quads, localhost, combining marks, mailto: and bracketed IPv6", () => {
  for (
    const text of [
      "Reset at 203.0.113.7/login",
      "//203.0.113.7/login",
      "go to 203.0.113.7",
      "go to 198.51.100.255 now",
      "192.0.2.1:8080",
      "Reset at 203.0.113.7.",
      "localhost:8080/admin",
      "localhost:8080",
      "localhost/admin",
      "localhost\\admin",
      "Open LOCALHOST:3000 now",
      "example.c\u0336om",
      "Details at example.c\u0336om today",
      "xjavascript:alert",
      "xwww.example.md",
      "mailto:x@example.com",
      "mailto:x@example.zip",
      "xmailto:x@example.zip",
      "[::1]:8080/admin",
      "[2001:db8::1]/login",
      "Open [::1] now",
      "[2001:DB8::1]",
    ]
  ) mustFlag(text);
});

Deno.test("must flag: tags, comments, processing instructions, Markdown links and images", () => {
  for (
    const text of [
      "<b>bold</b>",
      "</div>",
      "<B>",
      "<img src=x>",
      "Read <a href=week-3>this</a> first",
      "<!-- note -->",
      "<!DOCTYPE html>",
      "<?xml ?>",
      "<? anything",
      "[notes](week-3)",
      "Read [this](week-3) first",
      "![chart](figure-2)",
      "![chart]",
      "\uFF1Cb\uFF1E",
      "\uFE64b\uFE65",
    ]
  ) mustFlag(text);
  for (const mark of INVISIBLES) {
    mustFlag(`<${mark}b>`);
    mustFlag(`]${mark}(x)`);
    mustFlag(`!${mark}[x]`);
  }
});

Deno.test("must flag: a near miss of a scrub token, what removal leaves, and several classes at once", () => {
  for (
    const text of [
      "<note onmouseover=x>",
      "<note x=y>",
      "<url/>",
      "<urls>",
      "<<url>b>",
      "Register at <url/> by Friday",
      "[here](https://example.com)",
      "one\n<b>two</b>",
      "one\nexample.com",
      "<a href=https://example.com>x</a>\nmore",
      "example.com and portal.example and https://example.org",
      "<b> and <i> and [x](y)",
      "a\nb\rc\u2028d",
    ]
  ) mustFlag(text);
});

Deno.test("must flag: a placeholder followed by ( or preceded by ! (the adjacency rows)", () => {
  for (
    const text of [
      "[link removed](",
      "![link removed]",
      "![link removed](",
      "See ![link removed](week-3) now",
      "[markup removed](week-3)",
      "![markup removed]",
    ]
  ) mustFlag(text);
});

// ---- Must not flag (every T1a must-not-flag row): each field returns the value byte for byte.

Deno.test("must not flag: course codes, terms, versions, decimals, chapters, times and abbreviations", () => {
  for (
    const text of [
      "CS-101",
      "MATH-200 quiz on Friday",
      "CHEM 101L lab",
      "Fall2026",
      "Register for Fall2026 classes",
      "v2.1.0",
      "Update to version 1.4.2 first",
      "3.5",
      "Worth 3.5 points",
      "Ch. 3.2",
      "Read Ch. 3.2 and Ch. 4.1",
      "5:30pm",
      "Meet at 5:30pm",
      "e.g.",
      "Bring a calculator, e.g. a graphing one",
      "Due 11:59 p.m. Friday",
      "Quiz on Friday. Bring a pencil.",
    ]
  ) mustNotFlag(text);
});

Deno.test("must not flag: a bare file name on every extension-list name", () => {
  assertEquals(EXTENSIONS.length, 45);
  for (const ext of EXTENSIONS) {
    mustNotFlag(`notes.${ext}`);
    mustNotFlag(`Upload notes.${ext} tonight`);
    mustNotFlag(`Upload NOTES.${ext.toUpperCase()} tonight`);
  }
  mustNotFlag("data.zip lab3.py main.rs notes.md");
  mustNotFlag("archive.tar.gz");
  mustNotFlag("lab3\u3002py");
});

Deno.test("must not flag: data: and file: in prose or inside a word; file names ending a clause", () => {
  for (
    const text of [
      "Lab 2 data: due Friday",
      "Submit file: essay.docx",
      "Did you submit essay.docx?",
      "Did you submit essay.docx? It is due today",
      "see notes.md: it covers week 3",
      "Is it notes.md# or not",
      "data: x",
      "Lab 2 data:",
      "profile:/settings",
      "profile:\\settings",
      "metadata:image/png",
      "metadata:,x",
    ]
  ) mustNotFlag(text);
});

Deno.test("must not flag: file-and-line references, sentence-ending dots, non-quads, plain localhost", () => {
  for (
    const text of [
      "main.rs:42",
      "lab3.py:7",
      "Fix main.rs:42 before lab3.py:7",
      "Did you fix main.rs:42?",
      "see lab3.py.",
      "archive.tar.gz.",
      "version 1.2.3",
      "999.1.1.1",
      "256.1.1.1",
      "10.0.26200.1",
      "the localhost setting",
      "Set it to localhost.",
      "example.c\u0336",
      "[1:2]",
      "Read section [1:2] first",
    ]
  ) mustNotFlag(text);
});

Deno.test("must not flag: < before a space or a digit; the scrub tokens; the placeholders", () => {
  for (const text of ["a < b", "score < 90", "<3", "x <3", "if 2<3 then"]) mustNotFlag(text);
  for (const token of ["<email>", "<url>", "<user>", "<note>", "<secret>", "<token>"]) {
    mustNotFlag(token);
    mustNotFlag(`Register at ${token} by Friday`);
    mustNotFlag(`Register at ${token.toUpperCase()} by Friday`);
  }
  for (const placeholder of PLACEHOLDERS) {
    mustNotFlag(placeholder);
    mustNotFlag(`See ${placeholder} before Friday`);
    mustNotFlag(`${placeholder} (week 3)`);
  }
  mustNotFlag("[link removed] and [markup removed]");
});

// ---- The TLD list, name by name, against the oracle.

Deno.test("every listed TLD, in each form, is a host bare unless it is an extension, and with a path always", () => {
  const names = [...IANA_TLDS, ...RESERVED_TLDS, ...IDN_TLDS.map(([, unicode]) => unicode)];
  for (const name of names) {
    const bare = inSentence(`example.${name}`);
    if (EXTENSIONS.includes(name)) mustNotFlag(bare);
    else mustFlag(bare);
    mustFlag(inSentence(`example.${name}/x`));
  }
});

// The one place the two may differ: the oracle reads any final label of two or more letters as a TLD,
// the enforcement only the snapshot's. `comma` is on no list today; if IANA ever delegates it, this
// row moves to the must-flag table in the refresh that adds it.
Deno.test("outside the snapshot: an unlisted TLD-shaped label passes the enforcement, not the oracle", () => {
  assert(!IANA_TLDS.includes("comma"));
  for (const text of [inSentence("example.comma"), inSentence("example.comma/x")]) {
    assertEquals(policyBreaches(text), ["url"]);
    for (const field of [...REFUSING, ...NEUTRALISING]) {
      assertEquals(enforceFreeText(field, text), text, field);
    }
  }
});

// ---- Steps 3 and 4: what a neutralised value reads, and what still refuses.

Deno.test("a neutralising field puts a placeholder where each breaking span was, the words around it kept", () => {
  const rows: Array<[string, string]> = [
    ["Visit example.com now", "Visit [link removed] now"],
    ["Visit EXAMPLE.COM now", "Visit [link removed] now"],
    ["Visit exa\u200Bmple.com now", "Visit [link removed] now"],
    ["Visit example\u3002com now", "Visit [link removed] now"],
    ["Visit https://example.com/reset now", "Visit [link removed] now"],
    ["Visit example.zip/update now", "Visit [link removed] now"],
    ["Visit 203.0.113.7 now", "Visit [link removed] now"],
    ["Visit <img src=x onerror=alert(1)> now", "Visit [markup removed] now"],
    ["Visit [click](https://example.com/x) now", "Visit [link removed] now"],
    ["Visit ![x](notes) now", "Visit [markup removed] now"],
    ["Visit <<url>b> now", "Visit [markup removed] now"],
    ["Is example.com? Yes", "Is [link removed]? Yes"],
    [
      "Reset at example.com before <b>Friday</b> today",
      "Reset at [link removed] before [markup removed]Friday[markup removed] today",
    ],
    ["Register at <url> by <b>Friday</b>", "Register at <url> by [markup removed]Friday[markup removed]"],
  ];
  for (const [text, want] of rows) {
    for (const field of NEUTRALISING) {
      const got = enforceFreeText(field, text);
      assertEquals(got, want, `${field}: ${show(text)}`);
      assertEquals(policyBreaches(want), [], want);
    }
  }
});

Deno.test("a placeholder that would form markup with its neighbours refuses every field", () => {
  for (const text of ["Mirror at example.com(backup) today", "Reset !https://example.com/x now"]) {
    for (const field of [...REFUSING, ...NEUTRALISING]) {
      assertEquals(enforceFreeText(field, text), null, `${field}: ${text}`);
    }
  }
});

// ---- Step 3's byte identity, and the second pass of `oneLine`'s substitutions.

const FULL_WIDTH_QUOTE = "\uFF02";
const SEPARATOR_LOOK_ALIKE = "\u0387"; // GREEK ANO TELEIA; NFKC makes it the ledger's U+00B7
const FULL_WIDTH_F = "\uFF26";

Deno.test("a clean value holding a full-width quote, a separator look-alike or a full-width letter is unchanged", () => {
  const text =
    `Quiz \uFF13 moved to ${FULL_WIDTH_F}riday ${SEPARATOR_LOOK_ALIKE} ${FULL_WIDTH_QUOTE}bring a pencil${FULL_WIDTH_QUOTE}`;
  mustNotFlag(text);
});

Deno.test("a breaking value holding the same characters is neutralised with none of them left for oneLine", () => {
  const text =
    `${FULL_WIDTH_QUOTE}Visit example.com now${FULL_WIDTH_QUOTE} ${SEPARATOR_LOOK_ALIKE} ${FULL_WIDTH_F}riday`;
  for (const field of NEUTRALISING) {
    const got = enforceFreeText(field, text);
    assertEquals(got, "'Visit [link removed] now' - Friday", field);
    for (
      const left of ['"', " \u00B7 ", FULL_WIDTH_QUOTE, SEPARATOR_LOOK_ALIKE, FULL_WIDTH_F, "\u2028", "  "]
    ) {
      assert(!got!.includes(left), `${field}: ${show(left)} left in ${show(got!)}`);
    }
  }
  for (const field of REFUSING) assertEquals(enforceFreeText(field, text), null, field);
});

// ---- Step 5: the clip, after neutralising, and the check once more.

const BOUNDS: Record<FreeTextField, number> = {
  why: 140,
  importance_reason: 140,
  evidence: 140,
  title: 200,
  "item.title": 200,
  target: 200,
};

Deno.test("the bounds are the validator's", () => {
  assertEquals([BOUNDS.why, BOUNDS.evidence], [MAX_REASON_CHARS, MAIL_EVIDENCE_CHARS]);
});

Deno.test("a clean value whose cut at the bound ends in a host is refused, at 140 and at 200", () => {
  // Uncut, `example.comma` ends on a label no list holds, so the value is clean (see "outside the
  // snapshot"); cut right after `.com`, it ends in a host.
  assertEquals(enforceFreeText("why", "x example.comma"), "x example.comma");
  for (const [field, bound] of Object.entries(BOUNDS) as Array<[FreeTextField, number]>) {
    const text = "x".repeat(bound - " example.com".length) + " example.comma";
    assertEquals([...text].length, bound + 2);
    assertEquals(enforceFreeText(field, text), null, field);
  }
});

Deno.test("a cut that would land inside a placeholder falls before it", () => {
  const text = "x".repeat(130) + " example.com more";
  assertEquals(enforceFreeText("why", text), "x".repeat(130) + " ");
  assertEquals(enforceFreeText("importance_reason", text), "x".repeat(130) + " ");
  // Evidence is trimmed after its cut, as the validator always has.
  assertEquals(enforceFreeText("evidence", text), "x".repeat(130));
  // A cut that ends exactly at a placeholder's end keeps it whole.
  const whole = "x".repeat(140 - " [link removed]".length) + " example.com more";
  assertEquals(enforceFreeText("why", whole), "x".repeat(140 - " [link removed]".length) + " [link removed]");
});

Deno.test("a clean value over the bound with no such ending is clipped and kept", () => {
  for (const [field, bound] of Object.entries(BOUNDS) as Array<[FreeTextField, number]>) {
    const text = "word ".repeat(60);
    const cut = "word ".repeat(60).slice(0, bound);
    assertEquals(enforceFreeText(field, text), field === "evidence" ? cut.trim() : cut, field);
    assertEquals(enforceFreeText(field, "x".repeat(bound + 10)), "x".repeat(bound), field);
  }
});

// ---- The scrub's own tokens are exempt: every one `scrub` and `scrubForPrompt` can write.

Deno.test("every token the scrub writes is exempt, read by running both scrubs", async () => {
  const sample = [
    "mail a@example.com",
    "open https://example.com/x",
    "path C:\\Users\\Someone Else\\x",
    "see notes.md",
    "password=hunter2",
    "id abcdefghijklmnopqrstuvwxyz0123456789",
  ].join(" ; ");
  const tokens = (text: string) => [...new Set(text.match(/<[a-z]+>/g) ?? [])].sort();
  const written = tokens(scrub(sample));
  const prompted = tokens(scrubForPrompt(sample));
  assertEquals(written, ["<email>", "<note>", "<secret>", "<token>", "<url>", "<user>"]);
  assertEquals(prompted, ["<email>", "<secret>", "<token>", "<url>"]);
  // Every `<word>` literal in the scrub's source is one the run above produced, so a new token added
  // there fails here until it is exempt.
  const source = await Deno.readTextFile(new URL("./scrub.ts", import.meta.url));
  const literals = [...new Set(source.match(/"(?:\$1=?)?<[a-z]+>"/g) ?? [])].map((l) =>
    l.match(/<[a-z]+>/)![0]
  );
  for (const literal of literals) assert(written.includes(literal), literal);
  for (const token of [...written, ...prompted]) {
    mustNotFlag(`Register at ${token} by Friday`);
    mustNotFlag(`${token}, then ${token}`);
  }
});

// ---- `tlds.ts`: the version line, the reserved names, and both forms of every internationalised name.

Deno.test("tlds.ts carries IANA's version line, the four reserved names and a Unicode form per xn-- name", () => {
  assert(
    /^# Version \d{10}, Last Updated \w{3} \w{3} [ \d]\d \d\d:\d\d:\d\d \d{4} UTC$/.test(IANA_VERSION),
    IANA_VERSION,
  );
  assertEquals([...RESERVED_TLDS].sort(), ["example", "invalid", "localhost", "test"]);
  for (const name of RESERVED_TLDS) assert(!IANA_TLDS.includes(name), name);
  assert(IANA_TLDS.length > 1000, String(IANA_TLDS.length));
  assertEquals(new Set(IANA_TLDS).size, IANA_TLDS.length);
  for (const name of IANA_TLDS) assert(/^(?:[a-z]{2,}|xn--[a-z0-9-]+)$/.test(name), name);
  const aLabels = IANA_TLDS.filter((name) => name.startsWith("xn--"));
  assertEquals(IDN_TLDS.map(([a]) => a), aLabels);
  for (const [a, unicode] of IDN_TLDS) {
    assertEquals(domainToUnicode(a), unicode, a);
    assert([...unicode].some((c) => c.codePointAt(0)! > 0x7f), a);
  }
});

// ---- T2b review rulings (plan section 4.3.1): bidi controls, look-alike TLDs, Unicode slashes, length.

const BIDI_CONTROLS = [
  "\u202A",
  "\u202B",
  "\u202C",
  "\u202D",
  "\u202E",
  "\u2066",
  "\u2067",
  "\u2068",
  "\u2069",
];

Deno.test("a bidi embedding, override or isolate control refuses a name and is removed from an explanation", () => {
  const rows: Array<[string, string]> = [
    ["Verify at \u202Emoc.1elpmaxe\u202C today", "Verify at moc.1elpmaxe today"],
    ["\u2066x\u2069", "x"],
    ["Verify at \u202Eexample.com\u202C today", "Verify at [link removed] today"],
  ];
  for (const [text, want] of rows) {
    mustFlag(text);
    for (const field of REFUSING) assertEquals(enforceFreeText(field, text), null, `${field}: ${show(text)}`);
    for (const field of NEUTRALISING) {
      assertEquals(enforceFreeText(field, text), want, `${field}: ${show(text)}`);
    }
  }
  for (const control of BIDI_CONTROLS) {
    mustFlag(`before${control}after`);
    assertEquals(enforceFreeText("why", `before${control}after`), "beforeafter");
  }
});

Deno.test("a TLD written with Cyrillic or Greek look-alikes or an accent is a host", () => {
  for (const host of ["example.c\u043Em", "example.\u0441om", "example.\u03BFrg", "example.c\u00F3m"]) {
    mustFlag(host);
    mustFlag(inSentence(host));
    assertEquals(enforceFreeText("why", inSentence(host)), "Details at [link removed] today", show(host));
  }
  // Upper case folds to the same look-alikes.
  mustFlag(inSentence("EXAMPLE.\u0421\u041EM"));
  mustFlag(inSentence("example.\u039FRG"));
  // A Greek capital nu reads as "n" by its own shape, where its lower case would read as "v".
  assert(IANA_TLDS.includes("nyc") && !IANA_TLDS.includes("vyc"));
  mustFlag(inSentence("EXAMPLE.\u039DYC"));
});

Deno.test("the division slash and the fraction slash count as slashes", () => {
  for (const text of ["example.ai\u2215login", "example.ai\u2044login"]) mustFlag(text);
  assertEquals(
    enforceFreeText("why", "Log in at example.zip\u2215login today"),
    "Log in at [link removed] today",
  );
});

Deno.test("a value longer than four times its bound is refused before any scan", () => {
  for (const field of [...REFUSING, ...NEUTRALISING]) {
    const bound = BOUNDS[field];
    assertEquals(enforceFreeText(field, "x".repeat(4 * bound + 1)), null, field);
    assertEquals(enforceFreeText(field, "x".repeat(4 * bound)), "x".repeat(bound), field);
  }
  assertEquals(enforceFreeText("why", "x".repeat(600)), null);
  assertEquals(enforceFreeText("why", "x".repeat(560)), "x".repeat(140));
  // Characters, not UTF-16 units: 560 astral characters are 1120 units and still within the bound.
  assertEquals(enforceFreeText("why", "\u{1F600}".repeat(560)), "\u{1F600}".repeat(140));
});

// ---- T2b re-review rulings (plan section 4.3.1): capitals in their own right, more slashes, residual.

/** Each mapped look-alike, the Latin letter it reads as, and a listed TLD holding that letter. */
const LOOK_ALIKE_ROWS: Array<[string, string, string]> = [
  ["\u0410", "a", "ac"],
  ["\u0412", "b", "ba"],
  ["\u0415", "e", "ae"],
  ["\u041A", "k", "ck"],
  ["\u041C", "m", "am"],
  ["\u041D", "h", "bh"],
  ["\u041E", "o", "ao"],
  ["\u0420", "p", "gp"],
  ["\u0421", "c", "ac"],
  ["\u0422", "t", "at"],
  ["\u0425", "x", "ax"],
  ["\u0391", "a", "ac"],
  ["\u0392", "b", "ba"],
  ["\u0395", "e", "ae"],
  ["\u0396", "z", "az"],
  ["\u0397", "h", "bh"],
  ["\u0399", "i", "bi"],
  ["\u039A", "k", "ck"],
  ["\u039C", "m", "am"],
  ["\u039D", "n", "bn"],
  ["\u039F", "o", "ao"],
  ["\u03A1", "p", "gp"],
  ["\u03A4", "t", "at"],
  ["\u03A5", "y", "by"],
  ["\u03A7", "x", "ax"],
  ["\u03F2", "c", "ac"],
  ["\u03C2", "c", "ac"],
  ["\u03F9", "c", "ac"],
  ["\u03F3", "j", "bj"],
  ["\u0261", "g", "ag"],
  ["\u0131", "i", "bi"],
  ["\u0585", "o", "ao"],
];

Deno.test("every mapped look-alike, capital or not, written into a listed TLD makes a host", () => {
  for (const [char, latin, tld] of LOOK_ALIKE_ROWS) {
    assert(IANA_TLDS.includes(tld), tld);
    const lower = tld.replace(latin, char);
    mustFlag(inSentence(`example.${lower}`));
    // The rest of the label in capitals, so a capital is read by its own shape, not its lower case's.
    const upper = [...tld].map((c, i) => i === tld.indexOf(latin) ? char : c.toUpperCase()).join("");
    mustFlag(inSentence(`EXAMPLE.${upper}`));
  }
});

Deno.test("the re-review's look-alike rows are hosts", () => {
  for (
    const host of [
      "EXAMPLE.\u0421\u041E\u041C",
      "EXAMPLE.NE\u0422",
      "EXAMPLE.\u0412IZ",
      "EXAMPLE.\u039C\u0395",
      "EXAMPLE.\u0392IZ",
      "EXAMPLE.\u0397\u039A",
      "example.\u03F2om",
      "example.or\u0261",
      "example.\u0131o",
      "example.c\u0585m",
    ]
  ) {
    mustFlag(host);
    mustFlag(inSentence(host));
  }
});

// Cyrillic capital En looks like Latin H, so `EXAMPLE.` En `ET` reads `HET`, which no list holds:
// the enforcement keeps it and the oracle's open-ended rule flags it, as for any unlisted TLD.
Deno.test("a look-alike that reads as an unlisted label is outside the snapshot", () => {
  assert(!IANA_TLDS.includes("het"));
  const text = inSentence("EXAMPLE.\u041DET");
  assertEquals(policyBreaches(text), ["url"]);
  for (const field of [...REFUSING, ...NEUTRALISING]) assertEquals(enforceFreeText(field, text), text, field);
});

Deno.test("the big solidus and the box-drawing diagonal count as slashes", () => {
  for (const text of ["example.ai\u29F8login", "example.ai\u2571login"]) {
    mustFlag(text);
    mustFlag(inSentence(text));
  }
});

// Accepted residual: RLM, LRM, ALM and ZWNJ are default-ignorable and not bidi embedding, override or
// isolate controls, so a title holding only those is clean and sent byte for byte.
Deno.test("a title with only RLM, ALM or ZWNJ in a Hebrew, Arabic or Persian word is kept byte for byte", () => {
  for (
    const title of [
      "Homework \u05E9\u05D9\u05E2\u05D5\u05E8\u05D9\u05DD\u200F due Friday",
      "Homework \u0648\u0627\u062C\u0628\u061C due Friday",
      "Essay \u0645\u06CC\u200C\u062E\u0648\u0627\u0647\u0645 due Friday",
      "Read \u200Eexample notes\u200F first",
    ]
  ) mustNotFlag(title);
});
