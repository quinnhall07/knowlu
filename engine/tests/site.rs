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

const PRIVACY: &str = "Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on opens on the same day; our servers keep them encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.";

/// v0.4.0 (note §9, A5, A5'): the index and the privacy page are checked apart. The privacy page
/// keeps every pin it had; the index is the ported knowlu.com homepage, and four of its old pins and
/// one of its bans no longer describe it.
#[test]
fn the_privacy_page_is_plain_html_and_carries_the_privacy_sentence() {
    let privacy = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
    assert!(!privacy.contains("<script"), "privacy.html must carry no script (decision 11)");
    assert!(privacy.contains("site.css"), "privacy.html uses the one stylesheet");
    assert!(privacy.contains(PRIVACY), "the privacy page carries the exact sentence");
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
    // `site.css` stays the legal pages' one stylesheet, pinned on the privacy page above.
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
    let html = fs::read_to_string(site("privacy.html")).expect("site/privacy.html");
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

/// Friends-testers spec Q7, as Quinn ruled it at Q-L8: both sentences, in the spec's words.
#[test]
fn the_page_carries_the_two_tester_and_counter_sentences() {
    let gone = missing(&page_text(), &[
        "We keep daily counts of your AI calls and cloud requests",
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
        "Your Knowlu sign-in lives in Windows Credential Manager, not in a file; the two kept sign-ins described above are the exception, kept in a browser profile on this computer."
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
        "If you connect Blackboard for grades, Knowlu keeps your Blackboard sign-in on this computer, in a browser profile Windows encrypts for your Windows account, so it can read your grades without asking you to sign in every time. Forget Blackboard sign-in in Settings deletes it.",
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

/// Gmail spec §8.5 test 2: the disconnect sentence names what it deletes, and the record of each
/// decision, message id included, stays until the account is deleted (in both places it is said).
#[test]
fn the_decision_record_stays_until_the_account_is_deleted() {
    let text = page_text();
    let gone = missing(&text, &[
        "Disconnecting revokes our access at Google and deletes the stored connection, our list of which emails we have already read, and any proposals from your mail that had not reached your computer yet. The record of what Knowlu decided about each email, including its message id and who it was from, stays until you delete your account.",
        "The record of what we decided about each email, including its message id and who it was from, is kept until you delete your account.",
    ]);
    assert!(gone.is_empty(), "the page lacks: {gone:?}");
}
