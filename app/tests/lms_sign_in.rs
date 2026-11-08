//! C8 (onboarding replan §7): the LMS sign-in link and the hidden read's new ending.
//!
//! Nothing here opens a window, reads a cookie or leaves the machine. The page fetch is tested on a
//! listener bound to `127.0.0.1:0`; the hidden read is tested over injected closures. Every host is
//! invented (`lms.example.edu`, `idp.example.edu`).
use knowlu::grades::{
    hidden_outcome, hidden_start_url, open_hidden_with, read_hidden_with, CaptureError, LastRead, SETTLE,
};
use knowlu::lms_link::{fetch_sign_in_link, sign_in_link, LoginPageError};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

const HOST: &str = "lms.example.edu";
const PREFIX: &str = "/auth-saml/saml/login";
const LINK: &str = "https://lms.example.edu/auth-saml/saml/login?idp=_1_1&next=%2Fultra";

fn link_of(html: &str) -> Option<String> {
    sign_in_link(html, HOST, PREFIX)
}

// ---- `sign_in_link`, pure ---------------------------------------------------------------------

#[test]
fn one_absolute_link_on_the_host_passes() {
    let html = r#"<html><a class="x" href="https://lms.example.edu/auth-saml/saml/login?idp=_1_1&next=%2Fultra">Sign in</a></html>"#;
    assert_eq!(link_of(html).as_deref(), Some(LINK));
}

#[test]
fn a_page_with_no_link_gives_none() {
    assert_eq!(link_of("<html><a href=\"/help\">Help</a></html>"), None);
    assert_eq!(link_of(""), None);
}

#[test]
fn two_distinct_links_give_none_and_two_copies_of_one_give_it() {
    let two = r#"<a href="/auth-saml/saml/login?idp=_1_1">a</a><a href="/auth-saml/saml/login?idp=_2_1">b</a>"#;
    assert_eq!(link_of(two), None, "a page Knowlu does not understand is never guessed at");
    let same = r#"<a href="/auth-saml/saml/login?idp=_1_1">a</a><a href="/auth-saml/saml/login?idp=_1_1">b</a>"#;
    assert_eq!(link_of(same).as_deref(), Some("https://lms.example.edu/auth-saml/saml/login?idp=_1_1"));
}

#[test]
fn an_entity_encoded_query_is_decoded() {
    let html = r#"<a href="https://lms.example.edu/auth-saml/saml/login?idp=_1_1&amp;next=%2Fultra&amp;x=1">Sign in</a>"#;
    assert_eq!(
        link_of(html).as_deref(),
        Some("https://lms.example.edu/auth-saml/saml/login?idp=_1_1&next=%2Fultra&x=1"),
    );
}

#[test]
fn a_root_relative_link_is_made_absolute_on_the_host() {
    let html = r#"<a href="/auth-saml/saml/login?idp=_1_1">Sign in</a>"#;
    assert_eq!(link_of(html).as_deref(), Some("https://lms.example.edu/auth-saml/saml/login?idp=_1_1"));
}

#[test]
fn a_path_outside_the_prefix_is_refused() {
    assert_eq!(link_of(r#"<a href="/ultra/">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="https://lms.example.edu/other/auth-saml/saml/login">x</a>"#), None);
}

#[test]
fn another_host_is_refused() {
    assert_eq!(link_of(r#"<a href="https://idp.example.edu/auth-saml/saml/login?x=1">x</a>"#), None);
}

#[test]
fn a_look_alike_suffix_host_is_refused() {
    assert_eq!(link_of(r#"<a href="https://lms.example.edu.evil.test/auth-saml/saml/login">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="https://evil-lms.example.edu/auth-saml/saml/login">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="https://lms.example.edu:8443/auth-saml/saml/login">x</a>"#), None);
}

#[test]
fn http_is_refused() {
    assert_eq!(link_of(r#"<a href="http://lms.example.edu/auth-saml/saml/login?x=1">x</a>"#), None);
}

#[test]
fn a_protocol_relative_link_is_refused() {
    assert_eq!(link_of(r#"<a href="//lms.example.edu/auth-saml/saml/login">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="//evil.test/auth-saml/saml/login">x</a>"#), None);
}

#[test]
fn userinfo_is_refused() {
    assert_eq!(link_of(r#"<a href="https://evil.test@lms.example.edu/auth-saml/saml/login">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="https://lms.example.edu@evil.test/auth-saml/saml/login">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="/auth-saml/saml/login?next=a@b">x</a>"#), None, "any @ is refused");
}

#[test]
fn a_backslash_is_refused() {
    assert_eq!(link_of(r#"<a href="/auth-saml/saml/login\evil">x</a>"#), None);
    assert_eq!(link_of(r#"<a href="https://lms.example.edu\.evil.test/auth-saml/saml/login">x</a>"#), None);
}

#[test]
fn a_refused_link_beside_a_good_one_does_not_make_two() {
    let html = r#"<a href="https://evil.test/auth-saml/saml/login">x</a><a href="/auth-saml/saml/login?idp=_1_1">y</a>"#;
    assert_eq!(link_of(html).as_deref(), Some("https://lms.example.edu/auth-saml/saml/login?idp=_1_1"));
}

// ---- the page fetch, on a local listener -------------------------------------------------------

/// Serve one scripted response per connection, in order, and hand back every request head. Gives up
/// waiting after 5 s, so an implementation that connects less than the script expects fails the
/// assertions rather than hanging.
fn serve(responses: Vec<String>) -> (u16, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        for response in responses {
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
                    Err(_) => return seen,
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
            seen.push(String::from_utf8_lossy(&buf).to_string());
            let _ = stream.write_all(response.as_bytes());
        }
        seen
    });
    (port, handle)
}

fn ok(body: &str) -> String {
    format!("HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len())
}

fn redirect(to: &str) -> String {
    format!("HTTP/1.1 302 Found\r\nlocation: {to}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
}

fn status(line: &str) -> String {
    format!("HTTP/1.1 {line}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
}

const PAGE: &str = r#"<a href="/auth-saml/saml/login?idp=_1_1">Sign in</a>"#;
const LINK_ON_LISTENER: &str = "https://127.0.0.1:{port}/auth-saml/saml/login?idp=_1_1";

fn fetch(port: u16) -> Result<Option<String>, LoginPageError> {
    let host = format!("127.0.0.1:{port}");
    fetch_sign_in_link(&format!("http://{host}"), &host, PREFIX)
}

#[test]
fn the_get_asks_for_the_root_and_sends_no_cookie() {
    let (port, server) = serve(vec![ok(PAGE)]);
    let got = fetch(port).unwrap();
    assert_eq!(got.as_deref(), Some(LINK_ON_LISTENER.replace("{port}", &port.to_string()).as_str()));
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].starts_with("GET / HTTP/1.1\r\n"), "asked for: {}", seen[0].lines().next().unwrap_or(""));
    assert!(!seen[0].to_ascii_lowercase().contains("\r\ncookie:"), "the anonymous GET must carry no cookie");
}

#[test]
fn a_same_host_redirect_is_followed() {
    let (port, server) = serve(vec![redirect("/welcome"), ok(PAGE)]);
    let got = fetch(port).unwrap();
    assert!(got.is_some(), "the link on the page the redirect led to");
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen[1].starts_with("GET /welcome HTTP/1.1\r\n"));
    assert!(!seen[1].to_ascii_lowercase().contains("\r\ncookie:"));
}

#[test]
fn three_hops_are_followed_and_a_fourth_gives_none() {
    let (port, server) = serve(vec![redirect("/a"), redirect("/b"), redirect("/c"), ok(PAGE)]);
    assert!(fetch(port).unwrap().is_some(), "the third redirect's target is read");
    assert_eq!(server.join().unwrap().len(), 4);

    let (port, server) = serve(vec![redirect("/a"), redirect("/b"), redirect("/c"), redirect("/d"), ok(PAGE)]);
    assert_eq!(fetch(port), Ok(None), "a fourth hop is not followed");
    assert!(server.join().unwrap().len() <= 4, "the fourth hop's target is never requested");
}

#[test]
fn a_redirect_to_any_other_host_gives_none_and_is_never_requested() {
    // 127.0.0.2 is loopback too, so a wrongly followed hop would reach no listener and fail loudly
    // as Unreachable rather than as the `Ok(None)` asserted here.
    let (port, server) = serve(vec![redirect("http://127.0.0.2:9/x"), ok(PAGE)]);
    assert_eq!(fetch(port), Ok(None));
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn a_redirect_to_a_look_alike_name_gives_none() {
    let (port, server) = serve(vec![redirect("http://lms.example.edu.evil.test/"), ok(PAGE)]);
    assert_eq!(fetch(port), Ok(None));
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn a_page_with_no_link_is_none_not_unreachable() {
    let (port, server) = serve(vec![ok("<html>maintenance</html>")]);
    assert_eq!(fetch(port), Ok(None));
    server.join().unwrap();
}

#[test]
fn a_5xx_is_unreachable() {
    let (port, server) = serve(vec![status("503 Service Unavailable")]);
    assert_eq!(fetch(port), Err(LoginPageError::Unreachable));
    server.join().unwrap();
}

#[test]
fn a_transport_failure_is_unreachable() {
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    assert_eq!(fetch(port), Err(LoginPageError::Unreachable), "nothing listens, so the connection is refused");
}

// ---- the start page ----------------------------------------------------------------------------

#[test]
fn a_row_with_no_prefix_starts_at_ultra_and_makes_no_get() {
    let dead = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let origin = format!("http://127.0.0.1:{dead}");
    // A GET here would fail as Unreachable, so Ok proves there was none.
    assert_eq!(hidden_start_url(&origin, &format!("127.0.0.1:{dead}"), None), Ok(format!("{origin}/ultra/")));
}

#[test]
fn a_link_is_the_start_page() {
    let (port, server) = serve(vec![ok(PAGE)]);
    let host = format!("127.0.0.1:{port}");
    let url = hidden_start_url(&format!("http://{host}"), &host, Some(PREFIX)).unwrap();
    assert_eq!(url, LINK_ON_LISTENER.replace("{port}", &port.to_string()));
    server.join().unwrap();
}

#[test]
fn none_falls_back_to_ultra() {
    let (port, server) = serve(vec![ok("<html>a page that changed</html>")]);
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://{host}");
    assert_eq!(hidden_start_url(&origin, &host, Some(PREFIX)), Ok(format!("{origin}/ultra/")));
    server.join().unwrap();
}

#[test]
fn a_page_that_cannot_be_fetched_is_unreachable_and_opens_no_window() {
    for response in [status("500 Internal Server Error"), status("502 Bad Gateway")] {
        let (port, server) = serve(vec![response]);
        let host = format!("127.0.0.1:{port}");
        let opened = Cell::new(0);
        let got = open_hidden_with(&format!("http://{host}"), &host, Some(PREFIX), |_| {
            opened.set(opened.get() + 1);
            Ok(())
        });
        assert_eq!(got, Err(CaptureError::Unreachable));
        assert_eq!(opened.get(), 0, "the LMS is down, so a window would cost 45 s and prove nothing");
        server.join().unwrap();
    }
    let dead = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let host = format!("127.0.0.1:{dead}");
    let opened = Cell::new(0);
    let got = open_hidden_with(&format!("http://{host}"), &host, Some(PREFIX), |_| {
        opened.set(opened.get() + 1);
        Ok(())
    });
    assert_eq!((got, opened.get()), (Err(CaptureError::Unreachable), 0), "a transport failure opens no window");
}

#[test]
fn the_window_opens_at_the_link_when_there_is_one() {
    let (port, server) = serve(vec![ok(PAGE)]);
    let host = format!("127.0.0.1:{port}");
    let at = RefCell::new(String::new());
    let got = open_hidden_with(&format!("http://{host}"), &host, Some(PREFIX), |u| {
        *at.borrow_mut() = u.to_string();
        Ok(())
    });
    assert_eq!(got, Ok(()));
    assert_eq!(*at.borrow(), LINK_ON_LISTENER.replace("{port}", &port.to_string()));
    server.join().unwrap();
}

// ---- `hidden_outcome`, §7.4's table, row by row -----------------------------------------------

#[test]
fn off_the_host_at_the_deadline_is_signed_out_whatever_last_holds() {
    for last in [
        LastRead::Empty,
        LastRead::Transport,
        LastRead::Status(200),
        LastRead::Status(401),
        LastRead::Status(403),
        LastRead::Status(500),
    ] {
        assert_eq!(hidden_outcome(false, last), CaptureError::SignedOut, "last: {last:?}");
    }
}

#[test]
fn on_the_host_with_401_or_403_is_signed_out() {
    assert_eq!(hidden_outcome(true, LastRead::Status(401)), CaptureError::SignedOut);
    assert_eq!(hidden_outcome(true, LastRead::Status(403)), CaptureError::SignedOut);
}

#[test]
fn on_the_host_with_any_other_last_is_unreachable() {
    for last in [LastRead::Empty, LastRead::Transport, LastRead::Status(404), LastRead::Status(500), LastRead::Status(503)] {
        assert_eq!(hidden_outcome(true, last), CaptureError::Unreachable, "last: {last:?}");
    }
}

#[test]
fn on_the_identity_provider_with_last_empty_is_signed_out() {
    // The row the link makes new: the window never polled users/me because it never left the idp.
    assert_eq!(hidden_outcome(false, LastRead::Empty), CaptureError::SignedOut);
}

#[test]
fn on_the_identity_provider_with_last_a_transport_failure_is_signed_out() {
    // The other new row: one transport failure on the LMS host earlier must not read as Unreachable
    // when the window ended at the idp.
    assert_eq!(hidden_outcome(false, LastRead::Transport), CaptureError::SignedOut);
}

// ---- `read_hidden_with`, over injected closures ---------------------------------------------------

/// A clock that moves one second per call, so SETTLE is reached after `SETTLE.as_secs()` reads.
fn ticking() -> impl FnMut() -> Duration {
    let t = Cell::new(0u64);
    move || {
        t.set(t.get() + 1);
        Duration::from_secs(t.get() - 1)
    }
}

const ON_LMS: &str = "https://lms.example.edu/ultra/course";
const ON_IDP: &str = "https://idp.example.edu/sso/login?SAMLRequest=abc";

fn bundle(user: &Value) -> Result<Value, CaptureError> {
    Ok(json!({"user": user}))
}

#[test]
fn users_me_200_within_settle_is_captured() {
    let polls = Cell::new(0);
    let got = read_hidden_with(
        HOST,
        || Some(ON_LMS.into()),
        || {
            polls.set(polls.get() + 1);
            if polls.get() < 3 { Err(None) } else { Ok(json!({"id": "_1_1"})) }
        },
        bundle,
        ticking(),
        || {},
    );
    assert_eq!(got, Ok(json!({"user": {"id": "_1_1"}})));
    assert_eq!(polls.get(), 3);
}

#[test]
fn a_window_that_comes_back_from_the_idp_is_captured() {
    let calls = Cell::new(0);
    let got = read_hidden_with(
        HOST,
        || {
            calls.set(calls.get() + 1);
            Some(if calls.get() < 4 { ON_IDP } else { ON_LMS }.into())
        },
        || Ok(json!({"id": "_1_1"})),
        bundle,
        ticking(),
        || {},
    );
    assert!(got.is_ok());
}

#[test]
fn a_window_that_stays_on_the_idp_is_signed_out_and_users_me_is_never_read() {
    let reads = Cell::new(0);
    let got = read_hidden_with(
        HOST,
        || Some(ON_IDP.into()),
        || {
            reads.set(reads.get() + 1);
            Err(None)
        },
        bundle,
        ticking(),
        || {},
    );
    assert_eq!(got, Err(CaptureError::SignedOut), "last empty, on the identity provider");
    assert_eq!(reads.get(), 0, "users/me is read only on the LMS host");
}

#[test]
fn a_window_that_reached_the_host_then_ended_on_the_idp_is_signed_out() {
    // On the LMS host for the first polls (a transport failure each), on the idp at the deadline.
    let seen = Cell::new(0u32);
    let got = read_hidden_with(
        HOST,
        || {
            seen.set(seen.get() + 1);
            Some(if seen.get() <= 3 { ON_LMS } else { ON_IDP }.into())
        },
        || Err(None),
        bundle,
        ticking(),
        || {},
    );
    assert_eq!(got, Err(CaptureError::SignedOut), "where the window is at the deadline, not where it has been");
}

#[test]
fn on_the_host_with_401_at_the_deadline_is_signed_out() {
    for code in [401, 403] {
        let got = read_hidden_with(HOST, || Some(ON_LMS.into()), || Err(Some(code)), bundle, ticking(), || {});
        assert_eq!(got, Err(CaptureError::SignedOut), "{code}");
    }
}

#[test]
fn on_the_host_with_a_5xx_or_no_answer_at_the_deadline_is_unreachable() {
    let got = read_hidden_with(HOST, || Some(ON_LMS.into()), || Err(Some(503)), bundle, ticking(), || {});
    assert_eq!(got, Err(CaptureError::Unreachable));
    let got = read_hidden_with(HOST, || Some(ON_LMS.into()), || Err(None), bundle, ticking(), || {});
    assert_eq!(got, Err(CaptureError::Unreachable));
}

#[test]
fn a_window_with_no_url_is_signed_out() {
    let got = read_hidden_with(HOST, || None, || Err(None), bundle, ticking(), || {});
    assert_eq!(got, Err(CaptureError::SignedOut));
}

#[test]
fn a_window_at_ultra_ends_as_it_does_today() {
    // The fallback start page: the window rests on `/ultra/` of the LMS host.
    let ultra = "https://lms.example.edu/ultra/";
    let got = read_hidden_with(HOST, || Some(ultra.into()), || Err(Some(401)), bundle, ticking(), || {});
    assert_eq!(got, Err(CaptureError::SignedOut));
    let got = read_hidden_with(HOST, || Some(ultra.into()), || Err(None), bundle, ticking(), || {});
    assert_eq!(got, Err(CaptureError::Unreachable));
    let got = read_hidden_with(HOST, || Some(ultra.into()), || Ok(json!({"id": "_1_1"})), bundle, ticking(), || {});
    assert!(got.is_ok());
}

#[test]
fn the_loop_stops_at_settle_and_pauses_between_polls() {
    let pauses = Cell::new(0u64);
    let _ = read_hidden_with(HOST, || Some(ON_LMS.into()), || Err(None), bundle, ticking(), || pauses.set(pauses.get() + 1));
    assert!(pauses.get() >= 1, "a poll that finds nothing waits before the next");
    assert!(pauses.get() <= SETTLE.as_secs() + 1, "the injected clock, not the wall clock, ends the read: {}", pauses.get());
}
