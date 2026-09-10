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

const PRIVACY: &str = "Your vault stays on this machine. Knowlu's servers hold your account, the judgments they make for you, and what you correct; they never hold the text of your notes, and nothing here is ever sold or shared.";

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
