//! knowlu.com (plan 4a, Task 10; spec §7; v0.4.0 note §9). The privacy page is plain HTML and CSS,
//! **no scripts**, carrying the privacy sentence; no page makes a claim the product cannot back; the
//! ported homepage (A5, A5') is checked apart, and every face the site ships carries its licence.
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

/// Pilot-privacy amendment (2026-11-21, r2) §3.1: the one-line promise gains "unless you choose This
/// computer only" (Q-P1), and "our servers keep them encrypted at rest" becomes "that copy is encrypted at
/// rest" (review r1 M3). `wizard.js`'s `PRIVACY` changes in the same commit, byte for byte (`ui_wizard.rs`).
const PRIVACY: &str = "Your tasks and notes live in a plain-text folder on this machine and, unless you choose This computer only, in your Knowlu account, so every computer you sign in on opens on the same day; that copy is encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.";

/// v0.4.0 (note §9, A5, A5'): the index and the privacy page are checked apart. The privacy page
/// keeps every pin it had; the index is the ported knowlu.com homepage, and four of its old pins and
/// one of its bans no longer describe it.
#[test]
fn the_privacy_page_is_plain_html_and_carries_the_privacy_sentence() {
    let privacy = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
    assert!(!privacy.contains("<script"), "privacy.html must carry no script (decision 11)");
    // Moved (pilot-privacy amendment §5, §8 item 3): the "uses the one stylesheet" `site.css` pin is now
    // `the_legal_pages_use_the_sites_own_stylesheets_and_not_site_css`, below.
    assert!(privacy.contains(PRIVACY), "the privacy page carries the exact sentence");
    // Amendment §3.1: the old promise, without the This computer only clause, is gone, and so is its
    // "our servers keep them" wording.
    assert!(
        !privacy.contains("plain-text folder on this machine and in your Knowlu account"),
        "the old one-line promise is still published"
    );
    assert!(!privacy.contains("our servers keep them encrypted at rest"), "the one-line promise still says \"our servers keep them\" (amendment §3.1)");
    // No claim the product cannot back (product plan §8: scraping ToS, minors and FERPA are
    // Quinn's external dependency and the page makes no claim about them).
    for banned in ["FERPA", "compliant", "certified", "guarantee"] {
        assert!(!privacy.contains(banned), "the privacy page must not claim {banned}");
    }
}

#[test]
fn the_index_makes_no_claim_the_product_cannot_back() {
    let index = fs::read_to_string(site("index.html")).expect("site/index.html");
    // Dropped (A5): the download-link pin. The homepage is early access with a mailto form and no
    // installer link, so `releases/Knowlu-setup.exe` is no longer on it (the M13 fix is reversed).
    // Dropped (A5): the no-script pin. The homepage runs its own scripts (`js/*.js`, all local).
    // Dropped (A5, r1): the `site.css` pin. The homepage uses its own stylesheets (`css/*.css`);
    // Since the pilot-privacy amendment (§5) the legal pages use the site's own stylesheets too, plus
    // `css/legal.css`; `site.css` stays only for `signed-in.html` and `subscribed.html`.
    // Dropped (A5): the privacy-sentence pin. The sentence lives on the privacy page, pinned above.
    // Dropped (A5'): the "guarantee" ban. Quinn's "30-day money-back guarantee" line is on the
    // homepage, in the pricing section and the FAQ; the privacy page still bans the word.
    for banned in ["FERPA", "compliant", "certified"] {
        assert!(!index.contains(banned), "the homepage must not claim {banned}");
    }
}

#[test]
fn the_release_drop_folder_stays_in_git() {
    assert!(site("releases/.gitkeep").is_file(), "the drop folder exists in git");
}

/// v0.4.0 (note §9, M11; plan P11): every face the site declares, and every font file under `site/`,
/// carries its OFL and a `SOURCES.md` row whose SHA-256 is the file's own; no file is named as a
/// subset. A face whose OFL reserves a font name ships whole: its row names a 40-hex upstream commit
/// and its file name carries no `-latin`, `_latin` or `subset`. The old homepage carried PT Serif
/// Caption's Google CSS2 `latin` subset under the reserved name "PT Serif"; a subset is a Modified
/// Version under the OFL, which may not keep a reserved name. A face with no reserved name may be such
/// a subset (Albert Sans, Zilla Slab). This test cannot prove a file is upstream's; it proves the
/// paper trail is on disk and true to the bytes, so a reviewer can re-fetch and compare.
#[test]
fn every_site_font_has_its_licence_and_a_full_file() {
    use std::path::{Path, PathBuf};
    const FONT_EXTS: [&str; 4] = ["ttf", "otf", "woff", "woff2"];
    fn is_font(p: &Path) -> bool {
        p.extension().and_then(|e| e.to_str()).map(|e| FONT_EXTS.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
    }
    fn is_hex(s: &str, len: usize) -> bool { s.len() == len && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
    fn slug(family: &str) -> String { family.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect() }
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().map(|n| n == "releases").unwrap_or(false) { continue; }
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let root = site("");
    let mut files = Vec::new();
    walk(&root, &mut files);

    // Every `@font-face` in every stylesheet under `site/`, as (family, resolved file).
    let mut faces: Vec<(String, PathBuf)> = Vec::new();
    for css in files.iter().filter(|p| p.extension().map(|e| e == "css").unwrap_or(false)) {
        let text = fs::read_to_string(css).unwrap();
        for block in text.split("@font-face").skip(1) {
            let block = block.split('}').next().unwrap_or("");
            let family = block.split("font-family:").nth(1).and_then(|r| r.split(';').next()).map(|f| f.trim().trim_matches('"').trim_matches('\'').to_string());
            let family = family.unwrap_or_else(|| panic!("{}: an @font-face without a font-family", css.display()));
            for part in block.split("url(").skip(1) {
                let url = part.split(')').next().unwrap().trim().trim_matches('"').trim_matches('\'');
                faces.push((family.clone(), css.parent().unwrap().join(url)));
            }
        }
    }
    assert!(!faces.is_empty(), "the site declares no @font-face (the ported homepage carries its own faces)");
    // A font file on disk that no stylesheet names still needs its paper trail.
    for f in files.iter().filter(|p| is_font(p)) {
        let canon = f.canonicalize().unwrap();
        if !faces.iter().any(|(_, p)| p.canonicalize().map(|c| c == canon).unwrap_or(false)) {
            faces.push((String::new(), f.clone()));
        }
    }

    for (family, path) in &faces {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(path.is_file(), "{family}: {} is not on disk", path.display());
        assert!(!name.to_ascii_lowercase().contains("subset"), "{name} is named as a subset; ship the whole file");
        let dir = path.parent().unwrap();
        let sources_path = dir.join("SOURCES.md");
        let sources = fs::read_to_string(&sources_path).unwrap_or_else(|e| panic!("{name}: {}: {e}", sources_path.display()));
        let row: Vec<String> = sources
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with('|'))
            .map(|l| l.trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect::<Vec<_>>())
            .find(|cells| cells.first().map(|c| c == &name).unwrap_or(false))
            .unwrap_or_else(|| panic!("{name}: no row in {}", sources_path.display()));
        let hash = row.iter().find(|c| is_hex(c, 64)).unwrap_or_else(|| panic!("{name}: no 64-hex SHA-256 in its row"));
        let bytes = fs::read(path).unwrap();
        let own: String = ring::digest::digest(&ring::digest::SHA256, &bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hash, &own, "{name}: the row's SHA-256 is not the file's");
        let row_family = row.get(1).cloned().unwrap_or_default();
        if !family.is_empty() {
            assert_eq!(&row_family, family, "{name}: the row's family is not the @font-face's");
        }
        let ofl = dir.join(format!("OFL-{}.txt", slug(&row_family)));
        let text = fs::read_to_string(&ofl).unwrap_or_else(|e| panic!("{row_family}'s licence {}: {e}", ofl.display()));
        assert!(text.contains("SIL Open Font License, Version 1.1"), "{} is not the OFL 1.1 text", ofl.display());
        // A reserved name (OFL condition 3) travels only on the unmodified file: the row names the
        // upstream commit, and the file is not named as a per-script subset. The name is reserved in
        // the copyright header, before the licence body (which defines the term for every font).
        let header = text.split("This Font Software is licensed under").next().unwrap_or("");
        if header.contains("Reserved Font Name") {
            let lower = name.to_ascii_lowercase();
            for subset in ["-latin", "_latin", "subset"] {
                assert!(!lower.contains(subset), "{name}: {row_family}'s licence reserves a name, so ship the whole upstream file, not a subset");
            }
            assert!(
                row.iter().any(|c| c.split(|ch: char| !ch.is_ascii_hexdigit()).any(|w| is_hex(w, 40))),
                "{name}: {row_family}'s licence reserves a name, so its row must name the 40-hex upstream commit"
            );
        }
    }
}

/// v0.4.0 review I1: site-dist carried review aids (the "REVIEW ONLY" comments, the "Review copy"
/// stage flow) and a made-up address on the joined page. `release.yml` deploys `site/` whole, so none
/// of it may ship.
#[test]
fn no_shipped_site_file_carries_a_review_aid_or_a_made_up_address() {
    use std::path::Path;
    fn walk(dir: &Path, hits: &mut Vec<String>, seen: &mut usize) {
        for e in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, hits, seen); continue; }
            let Ok(text) = fs::read_to_string(&p) else { continue };
            *seen += 1;
            for aid in ["REVIEW ONLY", "Review copy", "example.edu"] {
                if text.contains(aid) { hits.push(format!("{}: {aid}", p.display())); }
            }
        }
    }
    let (mut hits, mut seen) = (Vec::new(), 0);
    walk(&site(""), &mut hits, &mut seen);
    assert!(seen >= 10, "the scan read only {seen} text files under site/");
    assert!(hits.is_empty(), "review aids ship: {hits:?}");
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

// ---- L4 T9: the honest page's pins (draft §4.3 "L4.1"; red until T10's page lands) ----------------

/// The page as a reader sees it: tags dropped, the few entities it uses decoded, curly apostrophes
/// straightened and every run of whitespace one space, so a pin names a sentence, not its markup.
fn page_text() -> String {
    plain(&fs::read_to_string(site("privacy.html")).expect("site/privacy.html"))
}

/// `page_text`'s reading of any HTML fragment (the pilot-privacy amendment's pins reuse it on one table
/// row, one list item and one definition at a time).
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            '\u{2019}' => out.push('\''),
            c => out.push(c),
        }
    }
    let out = out.replace("&amp;", "&").replace("&nbsp;", " ").replace("&middot;", "\u{b7}");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every sentence in `wanted` that the page lacks, so one failure names all of them.
fn missing<'a>(text: &str, wanted: &[&'a str]) -> Vec<&'a str> {
    wanted.iter().copied().filter(|w| !text.contains(w)).collect()
}

#[test]
fn the_page_drops_the_sentences_that_stopped_being_true() {
    let text = page_text();
    let still: Vec<&str> = ["Gmail is not connected in this version", "to fetch two things", "Six companies", "These four things"]
        .into_iter().filter(|b| text.contains(b)).collect();
    assert!(still.is_empty(), "the page still publishes: {still:?}");
}

#[test]
fn the_page_names_the_kept_sign_ins_and_how_to_end_them() {
    let gone = missing(&page_text(), &[
        "Kept sign-ins at every school, for now",
        "Forget Blackboard sign-in",
        // Pilot-privacy amendment r2 §3.3: the new kept sign-ins bullet still names "Settings → Email"
        // (for a mailbox), so this pin stays; the whole bullet is pinned in
        // `the_kept_sign_ins_are_recommended_at_setup_and_can_be_turned_off`, below.
        "Settings → Email",
        "the pages and mail the site keeps in its own cache sit in the same folder without that encryption",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

#[test]
fn the_page_describes_forwarded_mail_as_it_is_read_and_kept() {
    let gone = missing(&page_text(), &[
        "Amazon Web Services",
        "mail that arrives is deleted unread",
        "2,000 characters",
        "the codes of your courses",
        "your school's tenant id",
        "180 days",
        "any mail from the last three days that did not reach Knowlu",
        "up to 30 days back after a lapse",
        "Knowlu does not yet bring in older mail, or mail that arrived while forwarding was off; only mail forwarded after setup reaches Knowlu",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

#[test]
fn the_page_describes_what_is_sent_to_a_model_and_what_survives_gmail() {
    let gone = missing(&page_text(), &[
        "up to the first 600 characters of the interests",
        "Google holds Calendar and Gmail as one permission",
        "who it was from",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

#[test]
fn the_page_carries_the_registrars_sentence_and_the_syllabus_parenthesis() {
    let gone = missing(&page_text(), &[
        "at schools Knowlu supports, your class schedule",
        "Knowlu does not read syllabi in this version; the app will say when it does",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

/// Draft Q3 (ii), the narrow form: reading a kind of material the page says Knowlu does not read is
/// never a "smaller change" that gets only a new date.
#[test]
fn the_change_rule_says_a_newly_read_kind_of_material_is_never_a_smaller_change() {
    let gone = missing(&page_text(), &["Reading a kind of material this page says Knowlu does not read, such as syllabi, is never a smaller change"]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

/// Draft Q3 (ii): a "Last changed" line apart from the version, a real date no earlier than the
/// version line's. T13 moves both together; until then both read the old date, which this allows.
#[test]
fn the_last_changed_line_is_a_real_date_no_earlier_than_the_version() {
    fn date_after(text: &str, marker: &str) -> Option<String> {
        let d: String = text.split(marker).nth(1)?.chars().take(10).collect();
        let b = d.as_bytes();
        let shaped = b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() });
        let (m, day) = (d.get(5..7)?.parse::<u32>().ok()?, d.get(8..10)?.parse::<u32>().ok()?);
        (shaped && (1..=12).contains(&m) && (1..=31).contains(&day)).then_some(d)
    }
    let text = page_text();
    let changed = date_after(&text, "Last changed ").expect("the page carries a `Last changed YYYY-MM-DD` line");
    let version = date_after(&text, "This page is version ").expect("the page carries its version line");
    assert!(changed >= version, "last changed {changed} is earlier than the version {version}");
}

/// Friends-testers spec Q7, as Quinn ruled it at Q-L8: both sentences, in the spec's words, except that
/// "AI calls" is now "model calls" (pilot-privacy amendment r2 §8 item 1, the rewrite's C18: no word "AI"
/// on the legal pages, pinned in `no_legal_page_says_ai_or_claims_what_the_product_cannot_back`).
#[test]
fn the_page_carries_the_two_tester_and_counter_sentences() {
    let gone = missing(&page_text(), &[
        "We keep daily counts of your model calls and cloud requests",
        "If Knowlu gives you free tester access",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

/// "What we collect" promises a closed list, so the tester-access record is a category inside it.
#[test]
fn the_tester_access_record_is_listed_under_what_we_collect() {
    let text = page_text();
    let at = text.find("If Knowlu gives you free tester access").expect("the tester sentence");
    let start = text.find("What we collect").expect("the heading");
    let end = text.find("What we do not collect").expect("the next heading");
    assert!(start < at && at < end, "the tester-access sentence is outside What we collect");
}

/// Security must not say sessions are never in a file: the kept sign-ins are browser profiles on disk.
#[test]
fn security_does_not_claim_every_session_is_outside_a_file() {
    let text = page_text();
    assert!(!text.contains("Sessions live in Windows Credential Manager, not in a file"));
    assert!(text.contains(
        "Your Knowlu sign-in lives in Windows Credential Manager, not in a file; the kept sign-ins described above are the exception, kept in a browser profile on this computer."
    ));
}

/// Preserve: the way out stays named in the page's own words (passes today and must keep passing).
#[test]
fn the_page_still_names_settings_delete_my_data() {
    assert!(page_text().contains("Settings → Delete my data"));
}

/// The page must not say more stays on the machine, or fewer sign-ins are kept, than the app does.
#[test]
fn the_page_does_not_overclaim_what_stays_local_or_which_sign_ins_are_kept() {
    let text = page_text();
    assert!(!text.contains("everything else in this list never leaves this machine"));
    assert!(text.contains("app data leaves only in an issue report you choose to send"));
    assert!(!text.contains("The one exception is the Blackboard sign-in"));
    assert!(text.contains("if you forward a school mailbox, its saved mail sign-in"));
}

/// The export bullet must say what the export file leaves out (judgments, the Google connection's
/// details, undelivered Gmail proposals), not claim "everything".
#[test]
fn the_export_bullet_names_what_the_file_does_not_yet_hold() {
    let text = page_text();
    assert!(!text.contains("everything our servers hold that is meaningful to you"));
    assert!(text.contains(
        "The record of each decision Knowlu made, your Google connection's details (never the permission itself) and Gmail proposals not yet on your computer are not in this file yet. Ask at support@knowlu.com and we will send the file, and those three with it if you want them."
    ));
    assert!(text.contains("your daily counts and any free tester access"));
    assert!(!text.contains("we will send them with it. Ask at"));
}

/// Grades spec §11: the two sentences the spec hands to privacy bump #1 (the kept Blackboard
/// sign-in on this computer, and your grades in the account), plus the retention line.
#[test]
fn the_page_carries_the_grades_sentences() {
    let text = page_text();
    let gone = missing(&text, &[
        // Replaced (pilot-privacy amendment r2 §3.3, the "two kept sign-ins" family): "If you connect
        // Blackboard for grades, Knowlu keeps your Blackboard sign-in on this computer, in a browser
        // profile Windows encrypts for your Windows account, … Forget Blackboard sign-in in Settings
        // deletes it." §3.3 replaces the whole kept sign-ins bullet; the Blackboard sign-in for grades
        // is now pinned in `the_kept_sign_ins_are_recommended_at_setup_and_can_be_turned_off`, which
        // also checks this sentence is gone.
        "if you connect Blackboard, your grades, kept with your tasks so that every computer you sign in on shows the same ones",
        "the current text of each and your grades, for as long as the account exists",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}

/// Gmail spec §8.5 test 3: "What we collect" has the Google-connection entry and "How long we keep
/// it" has the Gmail bullet. Each is pinned inside its own section, not just somewhere on the page.
#[test]
fn the_google_connection_entry_and_the_gmail_retention_bullet_sit_in_their_sections() {
    let text = page_text();
    let collect = text.find("What we collect").expect("the heading");
    let not_collect = text.find("What we do not collect").expect("the next heading");
    let entry = text.find("Your Google connection, if you make one").expect("the Google-connection entry");
    assert!(collect < entry && entry < not_collect, "the entry is outside What we collect");
    assert!(text.contains("The permission Google gave us, stored encrypted and never sent to your computer"));
    let keep = text.find("How long we keep it").expect("the heading");
    let bullet = text.find("Gmail — our list of the emails we have read, for 30 days").expect("the Gmail bullet");
    assert!(keep < bullet, "the Gmail bullet is outside How long we keep it");
}

/// Gmail spec §8.5 test 2, as the pilot-privacy amendment r2 §3.7 changes it (Q-P4, D6): the disconnect
/// sentence names what it deletes, and that now includes the record of each decision; the retention
/// bullet keeps the record only until Gmail is disconnected or the account is deleted, and says the
/// rules learned from your mail stay until the account is deleted. (Was
/// `the_decision_record_stays_until_the_account_is_deleted`, whose two sentences §3.7 replaces.)
#[test]
fn the_decision_record_goes_when_gmail_is_disconnected() {
    let text = page_text();
    let gone = missing(&text, &[
        "Disconnecting revokes our access at Google and deletes the stored connection, our list of which emails we have already read, any proposals from your mail that had not reached your computer yet, and the record of what Knowlu decided about each email.",
        "The record of what we decided about each email, including its message id and who it was from, is kept until you disconnect Gmail or delete your account. The rules Knowlu learned from your mail stay until you delete your account.",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
    // Amendment §8 item 1: the old sentences are gone.
    let still: Vec<&str> = [
        "The record of what Knowlu decided about each email, including its message id and who it was from, stays until you delete your account.",
        "The record of what we decided about each email, including its message id and who it was from, is kept until you delete your account.",
    ]
    .into_iter().filter(|s| text.contains(s)).collect();
    assert!(still.is_empty(), "the page still publishes: {still:?}");
}

// ---- Pilot-privacy amendment (docs/specs/2026-11-21-legal-pages-amendment-pilot-privacy.md) §8 ----
// r2, items 1-4. Red until the amended pages land; each pin names the note section it comes from.

/// Every value of `name="…"` (or `name='…'`) in `html`, in order.
fn attr_values<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    html.split(&format!("{name}="))
        .skip(1)
        .filter_map(|rest| match rest.chars().next()? {
            q @ ('"' | '\'') => rest[1..].split(q).next(),
            _ => rest.split(|c: char| c.is_whitespace() || c == '>').next(),
        })
        .collect()
}

/// The `href` of every `<link … stylesheet …>` in `html`, with any leading `./` or `/` dropped.
fn stylesheets(html: &str) -> Vec<String> {
    html.split("<link")
        .skip(1)
        .filter_map(|c| c.split('>').next())
        .filter(|tag| tag.contains("stylesheet"))
        .filter_map(|tag| attr_values(tag, "href").first().map(|h| h.trim_start_matches("./").trim_start_matches('/').to_string()))
        .collect()
}

/// The text of the first `<tag …>…</tag>` element whose text contains `needle`, cells of a table row
/// separated by " | ", read as `plain` reads a page.
fn element_text(html: &str, tag: &str, needle: &str) -> Option<String> {
    html.split(&format!("<{tag}"))
        .skip(1)
        // `<li` also begins `<link`: only a whole tag name counts.
        .filter(|c| c.starts_with('>') || c.starts_with(char::is_whitespace))
        .filter_map(|c| c.split(&format!("</{tag}>")).next()?.split_once('>').map(|(_, body)| body))
        .map(|body| plain(&body.replace("</td>", " | ")))
        .find(|t| t.contains(needle))
}

fn privacy_html() -> String { fs::read_to_string(site("privacy.html")).expect("site/privacy.html") }

/// The HTML of the section under the `<h2>` whose text is `heading`, up to the next `<h2`.
fn section_html<'a>(html: &'a str, heading: &str) -> &'a str {
    let body = html.split(&format!("{heading}</h2>")).nth(1).unwrap_or_else(|| panic!("no <h2>{heading}</h2> on the page"));
    body.split("<h2").next().unwrap_or(body)
}

/// `section_html`, read as `plain` reads a page.
fn section_text(html: &str, heading: &str) -> String { plain(section_html(html, heading)) }

/// Every sentence in `banned` that `text` still carries.
fn still<'a>(text: &str, banned: &[&'a str]) -> Vec<&'a str> {
    banned.iter().copied().filter(|b| text.contains(b)).collect()
}

/// §3.0 (Q-P8): the notice that the policy changes often before launch, its own `<p class="notice">`,
/// directly under the date lines and before the one-line promise (which stays a bare `<p>`).
#[test]
fn the_privacy_page_opens_with_the_notice_that_it_will_change_before_launch() {
    const NOTICE: &str = "This policy will change often until Knowlu launches in January 2027, with its pilot. Each version carries its date above. A change that matters, such as something new collected, a new company or a new use, is still shown to you in Knowlu and asked for before it applies, as Changes to this policy says.";
    let html = privacy_html();
    let at = html.find("<p class=\"notice\">").expect("a <p class=\"notice\"> (amendment §3.0)");
    let notice = plain(html[at..].split("</p>").next().unwrap_or(""));
    assert_eq!(notice, NOTICE, "the notice's text (amendment §3.0)");
    let version = html.find("This page is version").expect("the version line");
    let changed = html.find("Last changed ").expect("the Last changed line");
    let promise = html.find(&format!("<p>{PRIVACY}</p>")).expect("the one-line promise, as a bare <p>");
    assert!(version < at && changed < at, "the notice must sit under the date lines (amendment §3.0)");
    assert!(at < promise, "the notice must come before the one-line promise (amendment §3.0)");
}

/// §3.2 (Q-P1, review r1 I1, I2): *This computer only* under *What stays on your machine*, and every
/// sentence that said the vault is in the account now names the choice.
#[test]
fn the_page_offers_this_computer_only_and_every_vault_in_the_account_sentence_names_it() {
    const FIRST: &str = "Unless you chose This computer only, your vault also has a copy in your account";
    // Split around the note's [BACKUP WINDOW] placeholder, so filling it in changes no pin.
    const HEAD: &str = "This computer only. You can choose it at setup or in Settings → Data and privacy. Then your vault is not stored in your account at all: our servers refuse to store it from any of your computers, and choosing it deletes the copy already there (it leaves our database's backups within ";
    const TAIL: &str = "). Knowlu works the same on each computer, but your computers no longer share a plan, and a new one starts empty. Knowlu still sends our servers what it needs to work, all of it listed under What we collect: the excerpt each judgment needs, for one call; the judgment that comes back, with the first three words of a title, organizer or sender; the rules Knowlu learns for you; your calendar links; the replies from zyBooks and VHL, read for their assignment lists and not kept; your email, if you connect it, as its own sections describe; your corrections, what you click, your daily counts and your setup answers; and any issue report you choose to send.";
    let html = privacy_html();
    let stays = section_text(&html, "What stays on your machine");
    let first = stays.find(FIRST).expect("What stays on your machine's first paragraph names This computer only (amendment §3.2)");
    let head = stays.find(HEAD).expect("the This computer only paragraph, up to the backup window (amendment §3.2)");
    let tail = stays.find(TAIL).expect("the This computer only paragraph, after the backup window (amendment §3.2)");
    assert!(first < head && head < tail, "the This computer only paragraph must follow the first paragraph");
    let window = &stays[head + HEAD.len()..tail];
    assert!(window.chars().count() <= 40, "only the backup window may sit inside the parenthesis: {window:?}");

    let text = page_text();
    let gone = missing(&text, &[
        // *Export*.
        "the plain files on your computer are the same text your account holds (with This computer only, they are the only copy)",
        // After the grades sentence (the note's "site.rs:405").
        "if you connect Blackboard, your grades, kept with your tasks so that every computer you sign in on shows the same ones. With This computer only, they stay in the vault on this computer.",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
    let old = still(&text, &[
        "Your vault also has a copy in your account",
        // r1's paragraph, which r2 replaces.
        "your vault is not copied to your account at all",
    ]);
    assert!(old.is_empty(), "the page still publishes: {old:?}");

    let entry = html.split("<dt>Your tasks and notes</dt>").nth(1).expect("the Your tasks and notes entry");
    let dd = plain(entry.split("</dd>").next().unwrap_or(""));
    assert!(
        dd.starts_with("Unless you chose This computer only, a copy of your vault:"),
        "Your tasks and notes must open with \"Unless you chose This computer only, a copy of your vault:\" (amendment §3.2); it opens: {:?}",
        dd.chars().take(80).collect::<String>()
    );
    let row = element_text(&html, "tr", "your vault, which your account holds").expect("the forwarded-mail table's vault row");
    assert!(row.contains("your vault, which your account holds unless you chose This computer only"), "the vault row (amendment §3.2): {row}");
}

/// §3.3 (Q-P2, review r1 M1, K3): the whole kept sign-ins bullet, worded by kind and not by count, then
/// the cache sentence unchanged. This replaces the "two kept sign-ins" family of pins inside the bullet
/// (the old Blackboard grades sentence and r1's "either one" sentence); "Settings → Email" stays.
#[test]
fn the_kept_sign_ins_are_recommended_at_setup_and_can_be_turned_off() {
    const BULLET: &str = "Kept sign-ins. Some sites sign you out often. So that Knowlu can keep reading them without asking you to sign in again each time, it can keep your sign-in to a site on this computer, in a browser profile encrypted for your account on this computer: your Blackboard sign-in, for grades, and, if you set up email forwarding, a saved sign-in for each mailbox, so Knowlu can check that forwarding is still on, turn it back on and, for Gmail, bring in mail it missed. Knowlu asks at setup, with each kept sign-in already on and marked Recommended. You can turn any of them off at setup or later (Forget Blackboard sign-in in Settings, or Settings → Email for a mailbox); each is deleted when you turn it off, disconnect it, sign out of Knowlu or delete your account. None keeps a password, and none ever leaves this computer.";
    let html = privacy_html();
    let li = element_text(section_html(&html, "What stays on your machine"), "li", "Kept sign-ins. Some sites sign you out often.")
        .expect("the kept sign-ins bullet under What stays on your machine (amendment §3.3)");
    assert!(
        li.starts_with(&format!("{BULLET} A kept sign-in is full access to that account on this computer:")),
        "the kept sign-ins bullet must be §3.3's text, then the unchanged cache sentence; it reads: {li}"
    );
    let old = still(&page_text(), &[
        "Kept sign-ins, if you use them.",
        "in a browser profile Windows encrypts for your Windows account, so it can read your grades without asking you to sign in every time",
        "is on unless you turn it off",
        "You can turn either one off",
        "Neither keeps a password",
    ]);
    assert!(old.is_empty(), "the page still publishes: {old:?}");
}

/// §3.4 (Q-P3, Q-P4, D5, review r1 M2): forwarded mail is deleted the moment Knowlu is done with it or
/// gives up on it; the expiry is only the backstop for a message Knowlu never reached; disconnecting a
/// mailbox takes its decision records, and the learned rules stay until the account is deleted.
#[test]
fn forwarded_mail_is_deleted_once_read_and_its_decision_records_go_with_the_mailbox() {
    let html = privacy_html();
    let text = page_text();
    let gone = missing(&text, &[
        "It waits in Amazon's storage only until Knowlu has read it, and is deleted the moment Knowlu is done with it, whether or not it was about school, and also when Knowlu gives up on a message it could not read. A message Knowlu never reached is removed by Amazon's one-day expiry, which in practice means up to about two days, plus Amazon's own lag.",
        "holds it until Knowlu is done with it, and a message Knowlu never reached at most about two days.",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
    let old = still(&text, &[
        "It waits in Amazon's storage until Knowlu has read it and is deleted then",
        "anything left over is removed by Amazon's one-day expiry",
        "holds it until Knowlu has read it",
        "except the decision records",
        // r1's wording, which r2 replaces.
        "A message Knowlu could not read is removed",
        "deleted as soon as Knowlu has read it",
    ]);
    assert!(old.is_empty(), "the page still publishes: {old:?}");

    // The forwarded-mail table: two rows' How long cells.
    let whole = element_text(&html, "tr", "The whole message").expect("the table's The whole message row");
    assert!(
        whole.contains("deleted as soon as Knowlu is done with it; a message Knowlu never reached, up to about two days in Amazon's storage, plus Amazon's lag"),
        "The whole message row's How long (amendment §3.4): {whole}"
    );
    assert!(!whole.contains("deleted when read; otherwise"), "the old How long is still in the row: {whole}");
    let decision = element_text(&html, "tr", "The record of each decision").expect("the table's decision-record row");
    assert!(decision.contains("until you disconnect that mailbox or delete your account"), "the decision-record row's How long (amendment §3.4): {decision}");

    // *How long we keep it*, forwarded email.
    let keep = section_html(&html, "How long we keep it");
    let item = element_text(keep, "li", "Email you forward to Knowlu").expect("How long we keep it's forwarded-email bullet");
    for want in [
        "until Knowlu is done with it, and a message Knowlu never reached at most about two days",
        "Disconnecting a mailbox deletes what our servers hold about it, the decision records included; the fingerprints and anything your computer has not yet taken keep their own times above. The rules Knowlu learned from your mail stay until you delete your account.",
    ] {
        assert!(item.contains(want), "the forwarded-email retention bullet lacks {want:?} (amendment §3.4): {item}");
    }
}

/// §3.5 (Q-P5, D7, D8, review r1 I6, M4, M5): the new section sits right after *Who else touches it*,
/// names roles, not people, and says each of its seven answers in the note's words.
#[test]
fn the_page_says_who_can_access_your_data() {
    let html = privacy_html();
    let who_else = html.find("Who else touches it</h2>").expect("the Who else touches it heading");
    let access = html.find("Who can access your data</h2>").expect("a Who can access your data heading (amendment §3.5)");
    let next = html.find("We do not sell or share your personal information</h2>").expect("the next heading");
    assert!(who_else < access && access < next, "Who can access your data is not the section after Who else touches it");
    let gone = missing(&section_text(&html, "Who can access your data"), &[
        "You. Your plan is in the plain files on your computer, and Your rights below says how to get a copy of what our servers hold.",
        "People at Knowlu. The people who run Knowlu can reach our database through its administration tools, because someone has to keep it running. No one at Knowlu reads your tasks, notes, grades, your email or anything found in it without your explicit consent, unless a valid legal demand requires it. Administrative accounts use two-step sign-in and are given only to the people who need them.",
        "The companies that run parts of Knowlu, listed above, each for its one job and nothing else.",
        "Your school does not see what Knowlu holds about you.",
        "Courts and the police, only with a valid legal demand that names your account. We tell you before we hand anything over, unless the law forbids it, and hand over no more than the demand requires. With This computer only, your tasks and notes are not on our servers to hand over.",
        "If Knowlu is sold, the buyer takes on this policy, and we tell you first (see Who else touches it).",
        "Nobody else. No advertiser, data broker or analytics company.",
    ]);
    assert!(gone.is_empty(), "Who can access your data lacks: {gone:?}");
    // D8: no "to stop an attack" exception; r1's overpromises (I6) and absolute school line (M4) are gone.
    let old = still(&page_text(), &[
        "to stop an attack",
        "never on a student's computer",
        "Everything Knowlu holds about you is in your plain files",
        "Your school, never.",
    ]);
    assert!(old.is_empty(), "the page still publishes: {old:?}");
}

/// §3.6 (D8, review r1 K2): the two consent promises each gain the one legal-demand exception.
#[test]
fn the_two_consent_promises_carry_the_legal_demand_exception() {
    let html = privacy_html();
    let entry = html.split("<dt>Your tasks and notes</dt>").nth(1).expect("the Your tasks and notes entry");
    let dd = plain(entry.split("</dd>").next().unwrap_or(""));
    assert!(
        dd.contains("No human at Knowlu reads it without your explicit consent, unless a valid legal demand requires it (see Who can access your data)"),
        "Your tasks and notes' consent promise lacks the legal-demand clause (amendment §3.6): {dd}"
    );
    let gmail = section_text(&html, "Gmail");
    assert!(
        gmail.contains("No human at Knowlu reads your mail without your explicit consent, unless a valid legal demand requires it"),
        "the Gmail section's consent promise lacks the legal-demand clause (amendment §3.6)"
    );
    let old = still(&page_text(), &[
        "No human at Knowlu reads it without your explicit consent, it is never sold",
        "No human at Knowlu reads your mail without your explicit consent.",
    ]);
    assert!(old.is_empty(), "the page still publishes: {old:?}");
}

/// §3.7's third change: the Gmail list's kept-items sentence keeps them only until Gmail is disconnected.
#[test]
fn what_gmail_keeps_of_a_message_lasts_until_gmail_is_disconnected() {
    let html = privacy_html();
    let item = element_text(&html, "li", "Message text is discarded").expect("the Gmail list's Message text is discarded item");
    assert!(item.contains("until you disconnect Gmail"), "the kept-items sentence lacks \"until you disconnect Gmail\" (amendment §3.7): {item}");
}

/// §3.7's fourth change (D6): the rewrite's judgments-and-rules entry, in *What we collect* and in *How
/// long we keep it*, excepts the judgments about your email, which go with a disconnect.
#[test]
fn the_judgments_about_your_email_go_when_you_disconnect() {
    const CLAUSE: &str = "except the judgments about your email, which go when you disconnect that mailbox or Gmail";
    let html = privacy_html();
    let n = page_text().matches(CLAUSE).count();
    assert!(n >= 2, "the clause {CLAUSE:?} is on the page {n} time(s); the rewrite's two judgments entries both carry it (amendment §3.7)");
    assert!(section_text(&html, "How long we keep it").contains(CLAUSE), "How long we keep it's judgments bullet lacks the clause (amendment §3.7)");
}

/// §3.8 (review r1 I7): *Mail you send us* names the pilot application form.
#[test]
fn mail_you_send_us_names_the_pilot_form() {
    let html = privacy_html();
    let entry = html.split("<dt>Mail you send us</dt>").nth(1).expect("the Mail you send us entry");
    let dd = plain(entry.split("</dd>").next().unwrap_or(""));
    let want = "If you apply to the pilot on our website, your answers go to a Google Form and are kept in Knowlu's Google account, used only to choose pilot students and to contact you about the pilot, and deleted when the pilot ends, or sooner if you ask.";
    assert!(dd.contains(want), "Mail you send us lacks the pilot form sentence (amendment §3.8): {dd}");
}

/// §5 and §8 item 3: the three legal pages use the site's own system (tokens.css, base.css) plus
/// legal.css, and no longer the old `site.css` (which stays for signed-in.html and subscribed.html).
#[test]
fn the_legal_pages_use_the_sites_own_stylesheets_and_not_site_css() {
    for page in ["privacy.html", "terms.html", "pilot-privacy.html"] {
        let html = fs::read_to_string(site(page)).unwrap_or_else(|e| panic!("site/{page}: {e}"));
        let sheets = stylesheets(&html);
        for want in ["css/tokens.css", "css/base.css", "css/legal.css"] {
            assert!(sheets.iter().any(|s| s == want), "{page} does not link {want}; it links {sheets:?}");
        }
        assert!(!sheets.iter().any(|s| s.ends_with("site.css")), "{page} still links site.css: {sheets:?}");
    }
}

/// §5 (privacy's decision-11 pin, extended) and §8 items 2-3: no legal page carries a script.
#[test]
fn no_legal_page_carries_a_script() {
    for page in ["privacy.html", "terms.html", "pilot-privacy.html"] {
        let html = fs::read_to_string(site(page)).unwrap_or_else(|e| panic!("site/{page}: {e}"));
        assert!(!html.contains("<script"), "{page} must carry no script (amendment §5)");
    }
}

/// Whether `html` carries an `href` to `page` (or an anchor in it), with any leading `./` or `/` dropped.
fn links_to(html: &str, page: &str) -> bool {
    attr_values(html, "href").iter().any(|h| {
        let h = h.trim_start_matches("./").trim_start_matches('/');
        h == page || h.starts_with(&format!("{page}#"))
    })
}

/// How many times `html` says "AI" as a word: word-boundary tokens over the whole file (attributes and
/// title included), so "email" passes.
fn says_ai(html: &str) -> usize { html.split(|c: char| !c.is_alphanumeric()).filter(|w| *w == "AI").count() }

/// §4 and §8 item 2 (Q-P6, review r1 I5): the Pilot Privacy Statement exists, never says "AI", points
/// to the full policy, says nothing the policy does not (no Credential Manager for a campus password, no
/// model names, no platform words) and names the pilot form. r2 drops r1's seven-company check: the
/// statement says the policy names every company, and the link to it is what this pins.
#[test]
fn the_pilot_privacy_statement_is_plain_and_points_to_the_policy() {
    let html = fs::read_to_string(site("pilot-privacy.html")).expect("site/pilot-privacy.html (amendment §4)");
    let ai = says_ai(&html);
    assert_eq!(ai, 0, "pilot-privacy.html says \"AI\" {ai} time(s); the site brief bans the word");
    assert!(links_to(&html, "privacy.html"), "pilot-privacy.html must link the full policy, privacy.html");
    for word in ["Credential Manager", "Granite", "Qwen"] {
        assert!(!html.contains(word), "pilot-privacy.html must not say {word:?} (amendment §4)");
    }
    // Platform words are checked in the statement itself: the shared site footer (§5) carries the
    // site's own "Windows and Mac" line.
    let main = html.split("<main").nth(1).and_then(|m| m.split("</main>").next()).and_then(|m| m.split_once('>')).map(|(_, body)| body)
        .expect("pilot-privacy.html carries its statement in a <main>");
    let text = plain(main);
    assert!(!text.contains("Windows"), "the statement must carry no platform words (amendment §4)");
    assert!(text.contains("Applying to the pilot"), "the statement names the pilot form (amendment §4, review r1 I7)");
}

/// §7 and §8 item 3: no legal page says "AI", and the banned-word list (this file's first test) runs on
/// all three. terms.html keeps "guarantee": it carries the signed 30-day money-back guarantee, as the
/// homepage does (A5'), so its list drops that one word.
#[test]
fn no_legal_page_says_ai_or_claims_what_the_product_cannot_back() {
    for page in ["privacy.html", "terms.html", "pilot-privacy.html"] {
        let html = fs::read_to_string(site(page)).unwrap_or_else(|e| panic!("site/{page}: {e}"));
        let ai = says_ai(&html);
        assert_eq!(ai, 0, "{page} says \"AI\" {ai} time(s); no legal page may (amendment §7)");
        let banned: &[&str] = if page == "terms.html" { &["FERPA", "compliant", "certified"] } else { &["FERPA", "compliant", "certified", "guarantee"] };
        for word in banned {
            assert!(!html.contains(word), "{page} must not claim {word}");
        }
    }
}

/// C35 and §8 item 4: the statement is linked from the homepage's pilot band and footer, the joined page,
/// and the top of the privacy page (before its first section).
#[test]
fn the_pilot_band_the_footers_and_the_privacy_page_link_the_statement() {
    let index = fs::read_to_string(site("index.html")).expect("site/index.html");
    let band = index.split("id=\"pilot\"").nth(1).and_then(|b| b.split("</section>").next()).expect("the homepage's pilot band");
    assert!(links_to(band, "pilot-privacy.html"), "the pilot band must link pilot-privacy.html (C35)");
    let footer = index.split("<footer").nth(1).and_then(|f| f.split("</footer>").next()).expect("the homepage's footer");
    assert!(links_to(footer, "pilot-privacy.html"), "the homepage footer must link pilot-privacy.html (C35)");
    let joined = fs::read_to_string(site("joined.html")).expect("site/joined.html");
    assert!(links_to(&joined, "pilot-privacy.html"), "joined.html must link pilot-privacy.html (C35)");
    let privacy = privacy_html();
    let top = privacy.split("<h2").next().unwrap_or("");
    assert!(links_to(top, "pilot-privacy.html"), "privacy.html must link pilot-privacy.html at its top, before its first section (C35)");
}

/// The v0.4.1 pricing design (`docs/specs/2026-11-19-v041-pricing-design.md` §2): the terms leave out *A
/// year for someone else* until the gift lane ships, because terms must not promise a checkout that does
/// not exist. The restyled terms (amendment §5) must not bring it back.
#[test]
fn the_terms_promise_no_gift_year_yet() {
    let terms = fs::read_to_string(site("terms.html")).expect("site/terms.html");
    assert!(!terms.contains("A year for someone else"), "terms.html promises a gift year the app cannot sell yet");
}
