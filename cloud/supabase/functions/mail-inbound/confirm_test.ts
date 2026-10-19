// Email spec §4.6 step 5 as amended 2026-10-18 (R2, Quinn): Gmail's confirmation follows at most three
// redirects, each cookie-less with `redirect: "manual"`, and only to a target on `confirmUrlAllowed`'s
// list; each hop is logged by host and path, never its query or fragment. The fetch is a fake function, so
// nothing here reaches the network. Every link and code is invented: the `vf-` codes are synthetic, and a
// host off Google's list is `example.invalid`.
import { assert, assertEquals } from "@std/assert";
import { completeConfirmation, type ConfirmFetch } from "./confirm.ts";

const CODE = "%5BTESTCODE-0001%5D-AbCdEf";
const LINK = `https://mail-settings.google.com/mail/vf-${CODE}`;
const hop = (n: number) => `https://mail.google.com/mail/vf-${CODE}-${n}`;
const ACTION = `https://mail.google.com/mail/vf-${CODE}-post`;

const page = (action: string) =>
  `<!doctype html><html><body><form method="POST" action="${action}">` +
  `<input type="hidden" name="vf" value="synthetic-1"><input type="submit" value="Confirm"></form></body></html>`;
const html = (action = ACTION) =>
  new Response(page(action), { headers: { "content-type": "text/html", "set-cookie": "GMAIL_RTT=page" } });
/** A 3xx to `location`, setting a cookie a browser's jar would carry to the next hop. */
const moved = (location: string | null, status = 302, n = 0) => {
  const headers = new Headers({ "set-cookie": `SID=hop-cookie-${n}; Secure; HttpOnly` });
  if (location !== null) headers.set("location", location);
  return new Response(null, { status, headers });
};

interface Sent {
  method: string;
  url: string;
  init: RequestInit;
}
/** A fake Google: `routes` answers by `METHOD url`; anything unrouted is a 404. */
function google(routes: Record<string, () => Response>) {
  const sent: Sent[] = [];
  const logs: string[] = [];
  const fetchImpl: ConfirmFetch = (url, init) => {
    const method = init.method ?? "GET";
    sent.push({ method, url, init });
    const r = routes[`${method} ${url}`];
    return Promise.resolve(r ? r() : new Response("not here", { status: 404 }));
  };
  const run = () => completeConfirmation(LINK, fetchImpl, (l) => logs.push(l));
  return { sent, logs, run, fetchImpl };
}
const calls = (sent: Sent[]) => sent.map((s) => `${s.method} ${s.url}`);
/** No line carries the code, a query, a fragment or a cookie. */
function clean(logs: string[]) {
  for (const l of logs) {
    assert(!l.includes("TESTCODE") && !l.includes("?") && !l.includes("#") && !l.includes("cookie"), l);
  }
}

Deno.test("one_on_list_redirect_then_the_form_then_the_post_confirms", async () => {
  // The form's action is relative: it resolves against the page's final URL, not the mailed link.
  const g = google({
    [`GET ${LINK}`]: () => moved(hop(1)),
    [`GET ${hop(1)}`]: () => html(`vf-${CODE}-post`),
    [`POST ${ACTION}`]: () => new Response("<p>Confirmation Success!</p>"),
  });
  assertEquals(await g.run(), "confirmed");
  assertEquals(calls(g.sent), [`GET ${LINK}`, `GET ${hop(1)}`, `POST ${ACTION}`]);
  assertEquals(g.logs, ["confirm hop 1: mail.google.com/mail/vf-(code)"]);
  clean(g.logs);
});

Deno.test("three_on_list_hops_confirm_and_a_fourth_refuses", async () => {
  // Hop 2 is a relative Location, resolved against hop 1.
  const three = google({
    [`GET ${LINK}`]: () => moved(hop(1)),
    [`GET ${hop(1)}`]: () => moved(`vf-${CODE}-2`, 301),
    [`GET ${hop(2)}`]: () => moved(hop(3), 307),
    [`GET ${hop(3)}`]: () => html(),
    [`POST ${ACTION}`]: () => new Response("ok"),
  });
  assertEquals(await three.run(), "confirmed");
  assertEquals(three.logs.length, 3);
  assertEquals(three.logs[2], "confirm hop 3: mail.google.com/mail/vf-(code)");

  const four = google({
    [`GET ${LINK}`]: () => moved(hop(1)),
    [`GET ${hop(1)}`]: () => moved(hop(2)),
    [`GET ${hop(2)}`]: () => moved(hop(3)),
    [`GET ${hop(3)}`]: () => moved(hop(4)),
    [`GET ${hop(4)}`]: () => html(),
  });
  assertEquals(await four.run(), "redirect");
  assertEquals(calls(four.sent), [`GET ${LINK}`, `GET ${hop(1)}`, `GET ${hop(2)}`, `GET ${hop(3)}`]);
  assertEquals(four.logs.at(-1), "confirm hop too many: mail.google.com/mail/vf-(code)");
  clean(four.logs);
});

Deno.test("a_hop_to_a_sign_in_page_refuses_off_list_and_logs_its_host_and_path", async () => {
  const signIn = "https://accounts.google.com/ServiceLogin?continue=https%3A%2F%2Fmail.google.com%2Fsecret-q";
  const g = google({ [`GET ${LINK}`]: () => moved(signIn) });
  assertEquals(await g.run(), "redirect_off_list");
  assertEquals(calls(g.sent), [`GET ${LINK}`]);
  assertEquals(g.logs, ["confirm hop off list: accounts.google.com/ServiceLogin (query)"]);
  for (const l of g.logs) assert(!l.includes("continue") && !l.includes("secret-q"), l);
});

Deno.test("a_hop_carrying_a_query_is_refused_and_logs_the_marker_never_the_query", async () => {
  const g = google({ [`GET ${LINK}`]: () => moved(`${hop(1)}?token=SECRETQ#frag-SECRETF`) });
  assertEquals(await g.run(), "redirect_off_list");
  assertEquals(calls(g.sent), [`GET ${LINK}`]);
  assertEquals(g.logs, ["confirm hop off list: mail.google.com/mail/vf-(code) (query) (fragment)"]);
  clean(g.logs);
  for (const l of g.logs) assert(!l.includes("SECRET") && !l.includes("token"), l);
});

Deno.test("a_redirect_without_a_location_or_off_https_is_refused_off_list", async () => {
  const none = google({ [`GET ${LINK}`]: () => moved(null) });
  assertEquals(await none.run(), "redirect_off_list");
  assertEquals(none.logs, ["confirm hop off list: (no location)"]);
  const plain = google({ [`GET ${LINK}`]: () => moved(`http://mail.google.com/mail/vf-${CODE}-1`) });
  assertEquals(await plain.run(), "redirect_off_list");
  assertEquals(plain.logs, ["confirm hop off list: http://mail.google.com/mail/vf-(code)"]);
  assertEquals(plain.sent.length, 1);
  const data = google({ [`GET ${LINK}`]: () => moved("data:text/html,<b>payload-text</b>") });
  assertEquals(await data.run(), "redirect_off_list");
  assertEquals(data.logs, ["confirm hop off list: data: (not http)"]);
});

Deno.test("an_off_list_paths_address_or_long_opaque_segment_is_not_logged", async () => {
  const opaque = "Zx9".repeat(20);
  const g = google({
    [`GET ${LINK}`]: () => moved(`https://example.invalid/u/jane%40example.invalid/${opaque}/end`),
  });
  assertEquals(await g.run(), "redirect_off_list");
  assertEquals(g.logs, ["confirm hop off list: example.invalid/u/(address)/(long)/end"]);
});

Deno.test("every_hop_is_cookie_less_and_manual_and_no_set_cookie_reaches_the_next", async () => {
  const g = google({
    [`GET ${LINK}`]: () => moved(hop(1), 302, 1),
    [`GET ${hop(1)}`]: () => moved(hop(2), 303, 2),
    [`GET ${hop(2)}`]: () => html(),
    [`POST ${ACTION}`]: () => moved(hop(3), 303, 3),
    [`GET ${hop(3)}`]: () => new Response("<p>Confirmation Success!</p>"),
  });
  assertEquals(await g.run(), "confirmed");
  assertEquals(g.sent.length, 5);
  for (const s of g.sent) {
    assertEquals(s.init.credentials, "omit", s.url);
    assertEquals(s.init.redirect, "manual", s.url);
    assert(s.init.signal instanceof AbortSignal, s.url);
    const headers = new Headers(s.init.headers);
    assertEquals(headers.get("cookie"), null, s.url);
    for (const [, v] of headers) assert(!v.includes("hop-cookie") && !v.includes("GMAIL_RTT"), s.url);
  }
  clean(g.logs);
});

Deno.test("a_post_answered_303_to_an_on_list_page_that_answers_2xx_confirms", async () => {
  const g = google({
    [`GET ${LINK}`]: () => html(),
    [`POST ${ACTION}`]: () => moved(hop(9), 303),
    [`GET ${hop(9)}`]: () => new Response("<p>Confirmation Success!</p>"),
  });
  assertEquals(await g.run(), "confirmed");
  assertEquals(calls(g.sent), [`GET ${LINK}`, `POST ${ACTION}`, `GET ${hop(9)}`]);
  // The Post/Redirect/Get hop is a GET with no body.
  assertEquals(g.sent[2].init.body, undefined);
  assertEquals(g.logs, ["confirm hop 1: mail.google.com/mail/vf-(code)"]);
});

Deno.test("a_post_redirect_to_an_on_list_page_that_fails_reads_post", async () => {
  const g = google({
    [`GET ${LINK}`]: () => html(),
    [`POST ${ACTION}`]: () => moved(hop(9), 303),
    [`GET ${hop(9)}`]: () => new Response("no", { status: 500 }),
  });
  assertEquals(await g.run(), "post");
});

Deno.test("a_post_answered_302_off_list_reads_post_redirect", async () => {
  const g = google({
    [`GET ${LINK}`]: () => html(),
    [`POST ${ACTION}`]: () => moved("https://example.invalid/done?x=1"),
  });
  assertEquals(await g.run(), "post_redirect");
  assertEquals(calls(g.sent), [`GET ${LINK}`, `POST ${ACTION}`]);
  assertEquals(g.logs, ["confirm hop off list: example.invalid/done (query)"]);
});

Deno.test("the_three_hops_are_one_budget_across_the_page_and_the_post", async () => {
  const g = google({
    [`GET ${LINK}`]: () => moved(hop(1)),
    [`GET ${hop(1)}`]: () => moved(hop(2)),
    [`GET ${hop(2)}`]: () => html(),
    [`POST ${ACTION}`]: () => moved(hop(3), 303),
    [`GET ${hop(3)}`]: () => moved(hop(4)),
    [`GET ${hop(4)}`]: () => new Response("ok"),
  });
  assertEquals(await g.run(), "post_redirect");
  assertEquals(g.sent.length, 5);
  assertEquals(g.logs.at(-1), "confirm hop too many: mail.google.com/mail/vf-(code)");
});

Deno.test("a_link_off_the_list_is_refused_before_any_fetch_and_no_logger_is_needed", async () => {
  let n = 0;
  const f: ConfirmFetch = () => (n++, Promise.resolve(new Response("x")));
  assertEquals(await completeConfirmation("https://example.invalid/mail/vf-x", f), "link");
  assertEquals(n, 0);
  // Without a logger a redirect is still refused, quietly.
  const g = google({ [`GET ${LINK}`]: () => moved("https://example.invalid/") });
  assertEquals(await completeConfirmation(LINK, g.fetchImpl), "redirect_off_list");
  assertEquals(g.sent.length, 1);
});
