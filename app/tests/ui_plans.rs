//! v0.4.1 T4: the plans the app sells, read across the page (note D9, D1, D13). The app holds no price: the
//! wizard's subscribe step and the upgrade sheet show the server's offer as text and send the plan pressed with
//! that plan's `amount_cents` and the offer's `terms_version`. The wizard's own pins are `ui_wizard.rs`'s and
//! Settings' plan words are `ui_settings.rs`'s; these pins read the sheet (`js/carried/account.js`), every shipped
//! script and page, and the price catalog the server sells from (`_shared/price_catalog.ts`, read as text).
#[path = "support/ui_common.rs"]
mod ui_common;

use std::fs;
use ui_common::*;

const ACCOUNT: &str = "js/carried/account.js";

/// The sheet's script with its comments stripped: what a pin reads is what runs.
fn account() -> String { strip_comments(&read(ACCOUNT)) }

/// The price catalog the server sells from, as text.
fn catalog() -> String {
    let p = app_dir().join("../cloud/supabase/functions/_shared/price_catalog.ts");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Every `key: "<k>"` inside the catalog's `export const <name>: … = [ … ];`, in order.
fn catalog_keys(name: &str) -> Vec<String> {
    let cat = catalog();
    let head = format!("export const {name}: ");
    let body = cat.split(&head).nth(1).and_then(|s| s.split("];").next()).unwrap_or_else(|| panic!("price_catalog.ts declares {head}… = [ … ];"));
    body.split("key: \"").skip(1).map(|s| s.split('"').next().unwrap_or("").to_string()).collect()
}

/// Every key the catalog's type can name: `export type PlanKey = "a" | "b";`.
fn plan_keys() -> Vec<String> {
    let cat = catalog();
    let union = cat.split("export type PlanKey = ").nth(1).and_then(|s| s.split(';').next()).expect("price_catalog.ts declares `export type PlanKey = …;`");
    union.split('|').map(|k| k.trim().trim_matches('"').to_string()).filter(|k| !k.is_empty()).collect()
}

/// The words no string of a file may carry, lower-cased: the wizard's rule (`ui_wizard.rs`, `PRICE_WORDS`).
const PRICE_WORDS: [&str; 18] = [
    "9.99", "12.99", "84.37", "6.49", "69.99", "a year", "per year", "7-day", "7 day", "seven day", "14-day", "14 days",
    "fourteen", "per month", "/month", "a month", "/mo", "free first",
];

/// True when `s` holds a decimal money figure: digits, a dot, then exactly two digits.
fn has_money_figure(s: &str) -> bool {
    let b = s.as_bytes();
    (1..b.len()).any(|i| {
        b[i] == b'.' && b[i - 1].is_ascii_digit()
            && b.get(i + 1).is_some_and(u8::is_ascii_digit) && b.get(i + 2).is_some_and(u8::is_ascii_digit)
            && !b.get(i + 3).is_some_and(u8::is_ascii_digit)
    })
}

/// True when `s` holds `$`, one or more digits, a dot and two digits.
fn has_dollar_amount(s: &str) -> bool {
    let b = s.as_bytes();
    (0..b.len()).any(|i| {
        if b[i] != b'$' { return false; }
        let mut j = i + 1;
        while j < b.len() && b[j].is_ascii_digit() { j += 1; }
        j > i + 1 && b.get(j) == Some(&b'.') && b.get(j + 1).is_some_and(u8::is_ascii_digit) && b.get(j + 2).is_some_and(u8::is_ascii_digit)
    })
}

/// The `if (…) { … }` arm of the sheet's click handler that names `sel`, up to the next arm.
fn arm<'a>(listener: &'a str, sel: &str) -> &'a str {
    let at = listener.find(sel).unwrap_or_else(|| panic!("the sheet's handler names {sel}"));
    let rest = &listener[at..];
    &rest[..rest[1..].find("\n    if (").map(|i| i + 1).unwrap_or(rest.len())]
}

/// Note D9 and the amendment's (9): the upgrade sheet offers both plans, Yearly first, inside `#up-subscribe`,
/// the wrapper that is hidden and shown (its id stays, so the sheet's other pins hold). Each button sends its
/// signed key with that plan's `amount_cents` from the offer and the offer's `terms_version`.
#[test]
fn the_upgrade_sheet_sells_both_plans_yearly_first() {
    let js = account();
    let sheet = fn_body(&js, "function upgradeHtml(");
    let wrap = sheet.find("<div class=\"sh-doors\" id=\"up-subscribe\" hidden>").expect("#up-subscribe is the wrapper the sheet hides and shows");
    let year = sheet.find("<button class=\"btn\" type=\"button\" id=\"up-sub-year\" disabled>Yearly</button>").expect("Yearly is the sheet's red button (btn)");
    let month = sheet.find("<button class=\"obtn\" type=\"button\" id=\"up-sub-month\" disabled>Every 4 weeks</button>").expect("Every 4 weeks is the outlined button (obtn)");
    assert!(wrap < year && year < month, "inside the wrapper, Yearly comes before Every 4 weeks");
    assert!(!sheet.contains(">Subscribe</button>"), "the one Subscribe button is gone");
    for empty in ["id=\"up-sub-year-lines\"></p>", "id=\"up-sub-month-lines\"></p>", "id=\"up-sub-note\"></p>"] {
        assert!(sheet.contains(empty), "the markup carries no price line: {empty}");
    }
    assert!(sheet.contains("id=\"up-sub-retry\">Try again</button>") && sheet.contains("id=\"up-sub-why\""), "Try again, and the sentence beside it");
    let listener = fn_body(&js, "function onUpgradeClick(");
    assert!(listener.contains("var plan = e.target.closest(\"#up-sub-year\") ? \"yearly\" : e.target.closest(\"#up-sub-month\") ? \"four_weekly\" : \"\";"), "each button sends its signed plan");
    let sub = fn_body(&js, "function subscribe(");
    assert!(js.contains("function subscribe(plan) {"), "subscribe takes the plan pressed");
    assert!(sub.contains("KnowluData.call(\"open_checkout\", { plan: plan, amountCents: offer.plans[plan].amount_cents, termsVersion: offer.terms_version })"), "with that plan's cents and the offer's terms version");
    assert!(sub.contains("if (!r.ok) { el(\"up-error\").textContent = r.error || UNREACHABLE; askOffer(); return; }"), "a refused checkout says the server's sentence and asks for the offer again");
    let after = fn_body(&js, "function afterUpgradeSignIn(");
    assert!(after.contains("if (!yes) { el(\"up-subscribe\").hidden = false; return askOffer(); }"), "where the sheet showed Subscribe it asks for the offer");
    let asker = fn_body(&js, "function askOffer(");
    assert!(asker.contains("KnowluData.call(\"checkout_offer\", {})"), "the offer comes from checkout_offer, with no arguments");
    let draw = fn_body(&js, "function drawOffer(");
    for to in ["el(\"up-sub-year-lines\").textContent = ", "el(\"up-sub-month-lines\").textContent = ", "el(\"up-sub-note\").textContent = ", "el(\"up-sub-why\").textContent = "] {
        assert!(draw.contains(to), "the offer is written as text: {to}");
    }
    assert!(draw.contains("el(\"up-sub-year\").disabled = !held || UP.checkoutBusy;") && draw.contains("el(\"up-sub-month\").disabled = !held || UP.checkoutBusy;"), "both buttons wait for an offer, and while a press is in flight");
    for line in js.lines().filter(|l| l.contains("innerHTML")) {
        for field in ["offer", ".lines", "amount_cents", "terms_version"] {
            assert!(!line.contains(field), "an innerHTML takes an offer field ({field}): {}", line.trim());
        }
    }
    let from = fn_body(&js, "function offerFrom(");
    assert!(js.contains("var NO_CONNECTION = \"Prices could not be loaded. Check your connection, then try again.\";"), "the unreachable sentence");
    assert!(js.contains("var UPDATE_KNOWLU = \"Prices could not be loaded. Update Knowlu, then try again.\";"), "the missing-plan sentence");
    assert!(from.contains("indexOf(UNREACHABLE) === 0") && from.contains("? NO_CONNECTION : e"), "an unreachable service asks for the connection; any other refusal is said as given");
    assert!(from.contains("!plans.yearly || !plans.four_weekly") && from.contains("UPDATE_KNOWLU"), "an answer without both plans asks for an update");
    assert!(from.contains("Array.isArray(p.lines) && p.lines.length > 0 &&"), "a plan without a line is not offered (review M4)");
    assert!(!js.contains("\"monthly\"") && !js.contains("\"academic_year\""), "the retired plan keys are gone from the sheet");
    assert!(!js.contains("localStorage") && !js.contains("sessionStorage"), "the sheet keeps nothing");
}

/// Note D9: the sheet holds no price, by the wizard's literal rule; "Every 4 weeks" is its one button label.
#[test]
fn the_sheet_holds_no_price() {
    let text = read(ACCOUNT);
    for s in string_literals(&text) {
        let low = s.to_lowercase();
        assert!(!s.contains('$'), "{ACCOUNT} shows a price: {s:?}");
        assert!(!has_money_figure(&s), "{ACCOUNT} holds a money figure: {s:?}");
        if s != "trialing" { assert!(!low.contains("trial"), "{ACCOUNT} names a trial: {s:?}"); }
        for bad in PRICE_WORDS {
            assert!(!low.contains(bad), "{ACCOUNT} names a price or a trial length ({bad}): {s:?}");
        }
        for (i, _) in low.match_indices("pell") {
            assert!(i > 0 && low.as_bytes()[i - 1].is_ascii_alphabetic(), "{ACCOUNT} names Pell: {s:?}");
        }
        assert!(!s.contains("every 4 weeks"), "{ACCOUNT} names a cadence as words (only the label \"Every 4 weeks\" may): {s:?}");
    }
    let code = account();
    assert_eq!(code.matches("Every 4 weeks").count(), 1, "\"Every 4 weeks\" is a plan word, the sheet's button label only");
    assert_eq!(code.matches(">Every 4 weeks</button>").count(), 1, "and it is the button's label");
}

/// r5: m2: no script or page under `app/static` writes a dollar amount (`$` digits `.` two digits). The SVG logos
/// are not read. The prices on screen are the server's, set as text when they arrive.
#[test]
fn no_static_script_or_page_writes_a_dollar_amount() {
    let files = shipped_with(&["js", "html"]);
    assert!(files.iter().any(|(f, _)| f == "index.html") && files.iter().any(|(f, _)| f == "js/wizard.js"), "the scan reads the shipped page and its scripts");
    for (f, text) in files {
        assert!(!has_dollar_amount(&text), "{f} writes a dollar amount");
    }
    assert!(has_dollar_amount("billed $128.31 once") && !has_dollar_amount("$ 12") && !has_dollar_amount("$12"), "the scan matches a dollar amount and nothing else");
}

/// Note D9: only the two plan buttons start a checkout. The handler takes the plan from the button pressed and
/// never from `closest("#up-subscribe")`, so a click on the lines, the note or the wrapper starts nothing.
#[test]
fn only_the_two_plan_buttons_start_checkout() {
    let js = account();
    let listener = fn_body(&js, "function onUpgradeClick(");
    assert!(!listener.contains("closest(\"#up-subscribe\")"), "a click anywhere in the wrapper must not start a checkout");
    assert!(!js.contains("subscribe()"), "no checkout starts without a plan");
    let calls: Vec<usize> = listener.match_indices("subscribe(").map(|(i, _)| i).collect();
    assert_eq!(calls.len(), 1, "the handler starts a checkout in one place");
    let plan_at = listener.find("var plan = e.target.closest(\"#up-sub-year\")").expect("the plan comes from the button pressed");
    assert!(plan_at < calls[0], "the plan is read from the button before the checkout starts");
    assert!(listener[plan_at..calls[0]].contains("if (plan)"), "only a plan button starts it");
    let retry = arm(listener, "closest(\"#up-sub-retry\")");
    assert!(retry.contains("askOffer()") && !retry.contains("subscribe("), "Try again asks for the offer, and starts no checkout");
    let sub = fn_body(&js, "function subscribe(");
    assert!(sub.contains("if (!offer || UP.checkoutBusy || !offer.plans[plan]) { return; }"), "no offer held, or a press in flight, no checkout");
}

/// T4 review M2-M3: the sheet's plan press is one at a time. Both buttons wait from the press until `open_checkout`
/// answers, and a chain that rejects ends the press and says the unreachable clause rather than leaving the buttons
/// locked.
#[test]
fn a_sheet_press_is_one_at_a_time_and_always_says_why() {
    let js = account();
    assert!(js.contains("var UP = { root: null, api: null, done: false, offer: null, offerSeq: 0, checkoutBusy: false };"), "UP carries the press in flight");
    for head in ["function renderUpgrade(", "function onUpgradeClosed("] {
        assert!(fn_body(&js, head).contains("UP.checkoutBusy = false;"), "{head} starts with no press in flight");
    }
    let sub = fn_body(&js, "function subscribe(");
    let busy = sub.find("UP.checkoutBusy = true;").expect("the press is marked in flight");
    let ask = sub.find("checkEntitled(").expect("the entitlement ask");
    assert!(busy < ask && sub[busy..ask].contains("drawOffer(\"\");"), "marked, and both buttons drawn waiting, before the first await");
    // Re-review N1: every way out of a press that keeps the sheet up draws the buttons back. An entitled answer goes
    // on to attach_account, which can fail and leave the sheet up, so the buttons are drawn back before it.
    assert!(sub.contains("if (yes) { UP.checkoutBusy = false; drawOffer(\"\"); return finishUpgrade(); }"), "an entitled answer ends the press and draws the buttons back");
    let open = sub.find("KnowluData.call(\"open_checkout\"").expect("open_checkout");
    let answered = sub[open..].find("UP.checkoutBusy = false;").map(|i| i + open).expect("open_checkout's answer ends the press");
    let refused = sub.find("if (!r.ok)").expect("the refusal");
    assert!(answered < refused, "the press ends before the refusal or the success is drawn");
    let refused_line = &sub[refused..sub[refused..].find('\n').map(|i| i + refused).unwrap_or(sub.len())];
    assert!(refused_line.contains("askOffer();"), "a refusal draws the buttons through a fresh ask");
    let success = sub.find("say(\"\");").expect("the success path");
    assert!(sub[success..].starts_with("say(\"\");\n        drawOffer(\"\");"), "a payment page that opened draws the buttons back");
    assert!(sub.contains(".catch(function () { UP.checkoutBusy = false; say(UNREACHABLE); drawOffer(\"\"); });"), "a rejected chain ends the press and says why");
}

/// Note D1, D13: the two keys the app sends are the two the catalog sells, read from `SELLING` as text.
#[test]
fn every_plan_the_app_sends_is_one_the_catalog_sells() {
    let selling = catalog_keys("SELLING");
    assert_eq!(selling, vec!["yearly".to_string(), "four_weekly".to_string()], "the catalog sells Yearly first, then Every 4 weeks");
    let wizard = strip_comments(&read("js/wizard.js"));
    let sheet = account();
    for key in ["yearly", "four_weekly"] {
        assert!(selling.iter().any(|k| k == key), "the app sends {key}, which the catalog does not sell");
        assert!(wizard.contains(&format!("\"{key}\"")) && sheet.contains(&format!("\"{key}\"")), "the wizard and the sheet both send {key}");
    }
}

/// Note D1, D9: every key the catalog can name (its `PlanKey` type, and every entry it sells or keeps) has a word in
/// Settings, so a row billing writes never reads as no plan.
#[test]
fn every_plan_the_catalog_can_name_has_a_word() {
    let keys = plan_keys();
    assert!(keys.len() >= 2, "price_catalog.ts's PlanKey names the plans: {keys:?}");
    let settings = strip_comments(&read("js/pages/settings.js"));
    let words = settings.split("var PLAN_WORDS = {").nth(1).and_then(|s| s.split("};").next()).expect("settings.js needs `var PLAN_WORDS = { … };`");
    let mut named = keys.clone();
    named.extend(catalog_keys("SELLING"));
    named.extend(catalog_keys("KEPT"));
    for key in named {
        assert!(keys.contains(&key), "the catalog names {key}, which PlanKey does not");
        assert!(words.split(',').any(|w| w.trim().starts_with(&format!("{key}: \""))), "Settings has no word for the catalog's {key}");
    }
}
