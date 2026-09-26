//! The download page (plan 4a, Task 10; spec §7). Plain HTML and CSS, **no scripts**, both pages
//! carrying the privacy sentence, and no claim the product cannot back.
//!
//! **This test pins the sentence against its own literal, and that is all it does.** The
//! wizard↔site cross-check — the one that fails when a word is edited on either side — is
//! `app/tests/static_assets.rs::the_wizards_privacy_sentence_is_the_sites_privacy_sentence`
//! (R-P4a-21), which reads the sentence out of `site/privacy.html` and looks for exactly it in
//! `console.js`. Two different guarantees: this one says the site still promises what it promised,
//! that one says the app promises the same thing.
use std::fs;

/// `site/` sits at the workspace root, one level above this crate.
fn site(rel: &str) -> std::path::PathBuf { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("site").join(rel) }

const PRIVACY: &str = "Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on opens on the same day; our servers keep them encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.";

#[test]
fn the_site_is_plain_html_and_carries_the_privacy_sentence_on_both_pages() {
    let index = fs::read_to_string(site("index.html")).expect("site/index.html");
    let privacy = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
    for (name, text) in [("index.html", &index), ("privacy.html", &privacy)] {
        assert!(!text.contains("<script"), "{name} must carry no script (decision 11)");
        assert!(text.contains("site.css"), "{name} uses the one stylesheet");
    }
    assert!(privacy.contains(PRIVACY), "the privacy page carries the exact sentence");
    assert!(index.contains(PRIVACY), "so does the download page");
    // Phase 3 (spec §4): the school window fetches three things now, the class schedule among them.
    assert!(privacy.contains("your calendar link, your course list and, at schools Knowlu supports, your class schedule"),
        "the privacy page names the registrar fetch");
    // M5: the stable name `release.ps1` copies beside the versioned installer, so the page's link
    // never has to change with a version. The versioned file stays there too.
    assert!(index.contains("releases/Knowlu-setup.exe"), "the download button points at the stable name");
    // No claim the product cannot back (product plan §8: scraping ToS, minors and FERPA are
    // Quinn's external dependency and the page makes no claim about them).
    for banned in ["FERPA", "compliant", "certified", "guarantee"] {
        assert!(!index.contains(banned) && !privacy.contains(banned), "the site must not claim {banned}");
    }
    assert!(site("releases/.gitkeep").is_file(), "the drop folder exists in git");
}

/// C3′ Task 11: the bullet said, in published words, that note bodies are never stored and that if
/// that ever changed it would be "a separate switch, off by default, with its own screen". Ruling 2
/// changed it and there is no switch: the copy is what an account IS. A page still promising the
/// switch would be the one thing in this stream that cannot be fixed after the fact.
#[test]
fn the_privacy_page_describes_the_account_vault_and_no_switch() {
    let privacy = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
    assert!(!privacy.contains("No note bodies"), "the old bullet is still published");
    assert!(!privacy.contains("separate switch"), "the page still promises a switch nobody built");
    assert!(privacy.contains("encrypted at rest"), "and the new promise is spelled out");
    assert!(privacy.contains("Delete my data"), "with the way out named");
    // Task 11 review, M3: `encrypted at rest` and `Delete my data` both predate this page's new
    // content (the Security section and the Delete bullet), so a later edit that deleted the whole
    // (d) entry and the (g) bullet would still pass the four lines above. These two are new here.
    assert!(privacy.contains("<dt>Your tasks and notes</dt>"), "the new collection entry is on the page");
    assert!(privacy.contains("400 days"), "the new retention bullet names its own number");
}

/// Task 11 review amendment (R-C3'-exec-40), wording file §5: three page sentences the review found
/// false or incomplete, plus the disclosure §5(iii) adds — none of which §2's own edits touched.
#[test]
fn the_review_amendments_i2_i3_and_i4_are_on_the_page() {
    let privacy = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
    // I2: "Your corrections" no longer claims a title correction is not recorded ANYWHERE — only
    // that it is not kept in THIS set, because the journal under "Your tasks and notes" keeps every
    // title change so the student's computers agree.
    assert!(!privacy.contains("a correction to a <em>title</em> is not recorded at all"), "the old, page-wide claim is still published");
    assert!(privacy.contains("is not kept at all"), "the corrections set's own, narrower claim is on the page");
    assert!(privacy.contains("The journal described under <em>Your tasks and notes</em> is separate"), "and it points at the journal that does keep it");
    // I3: the six-companies paragraph no longer claims the text is simply let go — it is not kept by
    // either side, which is the promise (e) actually makes.
    assert!(!privacy.contains("We hold the text for the length of the call and then let it go"), "the old, stronger claim is still published");
    assert!(privacy.contains("kept by neither of them"), "the corrected claim is on the page");
    // I4 (M3's fourth assertion): every synced journal record carries the computer's name, and the
    // page now says so, in the same entry that discloses the journal at all.
    // Wording §6 (vi) (R-C3'-exec-41, N4): an issue note carries the computer's name too
    // (`opened_on`), and `issues/` syncs, so the disclosure names both.
    assert!(
        privacy.contains("Each change in the journal, and each issue you raise, also carries the name Windows gives the computer it was made on, so that Knowlu can tell your computers apart."),
        "the computer-name disclosure is on the page, issues included"
    );
    assert!(!privacy.contains("Each change in the journal also carries"), "the journal-only wording is still published");
    // §5(iv): the retention bullet's own "except…" clause is scoped to what `keep` actually is
    // (a human `set`/`create`), not to "the changes you made yourself" — Gmail-derived and other
    // system-made entries are pruned at 400 days same as anything else.
    assert!(
        !privacy.contains("except the changes you made yourself, which stay as long as the account does"),
        "the old, over-broad retention clause is still published"
    );
    // Wording §6 (vii) (R-C3'-exec-41, N5): `keep` exempts every `agent:` actor, imports as well as
    // judgments, so the clause names both.
    assert!(
        privacy.contains("except the entries that set a value or create a note other than by one of Knowlu's own automatic steps (its imports and its judgments), which stay as long as the account does"),
        "the corrected retention clause is on the page"
    );
    assert!(!privacy.contains("other than by Knowlu's automatic judgments"), "the judgments-only clause is still published");
}
