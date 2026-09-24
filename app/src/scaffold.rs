//! The vault a wizard makes from nothing (plan 4a, Task 4; spec §3.1).
//!
//! **`create_vault` is the whole of it.** Files and the two seed notes are materialised into a
//! sibling staging folder and moved into place by ONE rename, so the destination either does not
//! exist or is a complete, already-seeded, already-migrated vault — never a folder that
//! `dest_for` refuses to reuse and adoption would greet with `journal has no migration records`
//! (review round 1, Important 2). `seed_writes` is deliberately private for that reason: a second
//! caller running it against a vault `create_vault` already seeded is exactly the stranded-vault
//! failure the staging folder exists to prevent.
//!
//! Adopting or restoring a vault calls NEITHER: writing a seed note into an existing vault would
//! be the app inventing history.
use std::path::{Path, PathBuf};
use knowlu_engine::journal::Journal;
use knowlu_engine::write::{self, WriteContext};
use knowlu_engine::yamlemit::{safe_dump_block, Node};

/// What a **curated** school gets on top of being in the list: event feeds, a known LMS, and a
/// sign-in URL the window can be pointed at. Keyed by IPEDS `UNITID`, which is the one identifier
/// that is stable across years and unambiguous across the four "University of ——" in a state.
///
/// **Adding a campus is adding a row here** — plus, if it is to have event feeds, one preset file
/// under `app/assets/campus/`. A school that is not in this table is still perfectly usable: it has
/// no event feeds, and its LMS comes from the sign-in window or from the student.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Curated {
    pub unitid: &'static str,
    /// The preset key `campus_yaml` maps to an `app/assets/campus/*.yaml` file.
    pub key: &'static str,
    pub label: &'static str,
    /// **This school's own LMS host**, no scheme and no path — `ualearn.blackboard.com`. Every URL the
    /// sign-in window is driven to is built from it, so a third curated Blackboard school does not send
    /// its student to Alabama's LMS. The paths are the same across every tenant of a kind; only the
    /// host differs, which is exactly what this field is.
    pub lms_host: &'static str,
    /// `blackboard` or `canvas`. With `lms_host`, it is enough to build every endpoint either LMS has.
    pub lms_kind: &'static str,
}

pub const CAMPUSES: [Curated; 2] = [
    Curated {
        unitid: "100751",
        key: "university-of-alabama",
        label: "The University of Alabama",
        lms_host: "ualearn.blackboard.com",
        lms_kind: "blackboard",
    },
    Curated {
        unitid: "157085",
        key: "university-of-kentucky",
        label: "University of Kentucky",
        lms_host: "uk.instructure.com",
        lms_kind: "canvas",
    },
];

/// The curated row for a unitid, if there is one.
pub fn curated(unitid: &str) -> Option<&'static Curated> {
    CAMPUSES.iter().find(|c| c.unitid == unitid)
}

/// The preset key whose `app/assets/campus/*.yaml` becomes this vault's `config/events.yaml`.
/// **`none` for anything uncurated, and for `university-of-kentucky` until someone writes its preset**
/// — `app/assets/campus/` is not this stream's to add a file to, and a preset key with no file behind
/// it is a vault that will not scaffold. Adding UK's feeds is one asset file and one line here.
pub fn events_preset_for(unitid: &str) -> &'static str {
    match curated(unitid).map(|c| c.key) {
        Some("university-of-alabama") => "university-of-alabama",
        _ => "none",
    }
}

const CAMPUS_NONE: &str = include_str!("../assets/campus/none.yaml");
const CAMPUS_UA: &str = include_str!("../assets/campus/university-of-alabama.yaml");
const PLANNING: &str = include_str!("../assets/scaffold/planning.yaml");
const WEEK_TEMPLATE: &str = include_str!("../assets/scaffold/week_template.yaml");
const GITIGNORE: &str = include_str!("../assets/scaffold/gitignore.txt");

pub fn campus_yaml(key: &str) -> Option<&'static str> {
    match key { "none" => Some(CAMPUS_NONE), "university-of-alabama" => Some(CAMPUS_UA), _ => None }
}

/// The school the student picked. `lms` is empty for a school whose kind nothing has established yet;
/// Task 14's sign-in window fills it from where it lands, or the panel asks.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct CampusChoice {
    pub unitid: String,
    pub name: String,
    pub state: String,
    pub lms: String,
}

/// `config/campus.yaml`. Five single-line scalars, through the same `yaml_scalar` every other wizard
/// value goes through — a school name is free text and `St. Mary's College` is a real one.
pub fn campus_config_yaml(c: &CampusChoice) -> Result<String, String> {
    Ok(format!(
        "unitid: {}\nname: {}\nstate: {}\nlms: {}\ncurated: {}\n",
        yaml_scalar("school id", &c.unitid)?,
        yaml_scalar("school name", &c.name)?,
        yaml_scalar("school state", &c.state)?,
        yaml_scalar("school LMS", &c.lms)?,
        curated(&c.unitid).is_some(),
    ))
}

/// A US state's IANA zone, for the wizard's timezone **suggestion** — the OS zone stays the default
/// and the field stays editable, because a student in El Paso is in Texas and on Mountain time.
///
/// **Split states take their majority zone**, which is the honest simplification: Florida (Eastern,
/// bar the western panhandle), Idaho (Mountain, bar the north), Indiana (Eastern, bar the corners),
/// Kansas, Kentucky (Eastern, bar the west), Michigan (Eastern, bar four counties), Nebraska, North
/// Dakota, Oregon (Pacific, bar Malheur), South Dakota, Tennessee (Central, bar the east) and Texas.
/// The panel says the suggestion came from the state, so a student who is in the minority half can see
/// why it is wrong and change it.
pub const STATE_TZ: [(&str, &str); 56] = [
    ("AL", "America/Chicago"), ("AK", "America/Anchorage"), ("AZ", "America/Phoenix"),
    ("AR", "America/Chicago"), ("CA", "America/Los_Angeles"), ("CO", "America/Denver"),
    ("CT", "America/New_York"), ("DC", "America/New_York"), ("DE", "America/New_York"),
    ("FL", "America/New_York"), ("GA", "America/New_York"), ("HI", "Pacific/Honolulu"),
    ("IA", "America/Chicago"), ("ID", "America/Boise"), ("IL", "America/Chicago"),
    ("IN", "America/Indiana/Indianapolis"), ("KS", "America/Chicago"), ("KY", "America/New_York"),
    ("LA", "America/Chicago"), ("MA", "America/New_York"), ("MD", "America/New_York"),
    ("ME", "America/New_York"), ("MI", "America/Detroit"), ("MN", "America/Chicago"),
    ("MO", "America/Chicago"), ("MS", "America/Chicago"), ("MT", "America/Denver"),
    ("NC", "America/New_York"), ("ND", "America/Chicago"), ("NE", "America/Chicago"),
    ("NH", "America/New_York"), ("NJ", "America/New_York"), ("NM", "America/Denver"),
    ("NV", "America/Los_Angeles"), ("NY", "America/New_York"), ("OH", "America/New_York"),
    ("OK", "America/Chicago"), ("OR", "America/Los_Angeles"), ("PA", "America/New_York"),
    ("RI", "America/New_York"), ("SC", "America/New_York"), ("SD", "America/Chicago"),
    ("TN", "America/Chicago"), ("TX", "America/Chicago"), ("UT", "America/Denver"),
    ("VA", "America/New_York"), ("VT", "America/New_York"), ("WA", "America/Los_Angeles"),
    ("WI", "America/Chicago"), ("WV", "America/New_York"), ("WY", "America/Denver"),
    // The territories, because `CYACTIVE = 1` and `ICLEVEL ∈ {1,2}` keep them: Puerto Rico alone has
    // about a hundred institutions, the UPR system among them, and a student there getting no
    // suggestion at all would be a degradation nobody chose.
    ("PR", "America/Puerto_Rico"), ("VI", "America/Puerto_Rico"), ("GU", "Pacific/Guam"),
    ("MP", "Pacific/Guam"), ("AS", "Pacific/Pago_Pago"),
];

pub fn state_timezone(state: &str) -> Option<&'static str> {
    let up = state.trim().to_ascii_uppercase();
    STATE_TZ.iter().find(|(s, _)| *s == up).map(|(_, tz)| *tz)
}

#[derive(Debug, Clone)]
pub struct VaultPlan {
    pub profile_id: String,
    pub ics_url: Option<String>,
    /// The student's own busy-time calendar, by its secret iCal address (spec §11a). Written into
    /// `config/ingest.yaml`'s `calendars:` list, which is what makes today's page know the day is
    /// already half full — the reason this ruling exists at all.
    pub personal_calendar: Option<String>,
    /// C2 (§11a): the account has a Google calendar grant, so the vault carries the marker feed
    /// `cloud:google`. Not a URL — hand-off H4's fetcher matches the `cloud:` prefix and routes it
    /// to `/ingest-calendar?name=google`. Deliberately unfetchable: a vault whose grant is gone
    /// gets one "using snapshot" warning and still ranks the day.
    pub google_calendar: bool,
    pub timezone: String,
    pub slots: Vec<String>,
    pub device: String,
    pub campus: String,
    /// R-OB-4: the school itself, out of the bundled list. `campus` above is only which preset of
    /// event feeds it gets.
    pub campus_choice: CampusChoice,
    pub zybooks: bool,
    pub vhl: bool,
    /// R-OB-1: what the student confirmed on the logins panel. Empty is honest — a wizard run with no
    /// coursework logins has nothing to map — and it is what `ingest_yaml` writes as `{}`.
    pub zybooks_courses: Vec<BookMapping>,
    pub vhl_sections: Vec<SectionMapping>,
    /// R-OB-1, review round 1 I2: every zyBook code the student declined to map, plus zyBooks' own
    /// `HowToUseZyBooks2`. Out of `ignore:`, an unmapped book is `zybook <code> not in config;
    /// skipped` on **every healthy run, forever** — `route_zybook`'s `Unmapped` arm — so a declined
    /// book has to land somewhere, and this is where.
    pub zybooks_ignore: Vec<String>,
    /// R-OB-1 and R-OB-2: `<code fragment> -> <slug>`, read by `ingest::match_course_fields` and by
    /// `judge::Heuristics`. Every confirmed mapping contributes one, and so does every course the
    /// sign-in window found (Task 14b).
    pub course_map: Vec<(String, String)>,
    /// R-OB-2: the enrolled courses, seeded as `courses/<slug>.md` notes (Task 14b).
    pub courses: Vec<CourseSeed>,
    /// The three cloud values (C1). `api_base` and `anon_key` are Rust's — the page never sees a URL
    /// (`static_assets.rs` forbids one) — and `account_id` comes from the session the wizard signed
    /// in with, read from Credential Manager, never from the page.
    pub api_base: String,
    pub anon_key: String,
    pub account_id: String,
}

/// One discovered zyBook, as the student confirmed it. `code` is the vendor's own
/// (`UACS100Fall2026`). **`course` is the course code the student confirmed on the panel — it is
/// the ONLY thing the vault's slug is ever derived from** (review round 1, I3): `suggest_course`
/// proposes it, the student can edit it, and by the time this struct is inside a `VaultPlan`
/// (`create_vault_in`'s work) `course` already holds the slug itself, since that is the one place
/// the engine's `slugify` runs. `label` is a separate string — what a human reads in the task
/// title — and is never slugged; a book's course text and its display title can differ (`GN 103` /
/// `GN 103 Hausaufgaben`) and only the first one may ever decide a vault identifier. The engine's
/// `route_zybook` needs a **non-empty** mapping under the code, and `parse_assignments` reads
/// exactly `course` and `label` out of it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct BookMapping {
    pub code: String,
    pub course: String,
    #[serde(default)]
    pub label: String,
}

/// One VHL section, likewise. `section` is the id out of the dashboard's `detail_url`; `course` is
/// the course code the student confirmed (never `label` — see `BookMapping`'s doc, I3) and `label`
/// is display-only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct SectionMapping {
    pub section: String,
    pub course: String,
    #[serde(default)]
    pub label: String,
}

// **There is no `slugify` here, and there must not be.** `knowlu_engine::ingest::slugify` is `pub`,
// this crate already depends on the engine, and that function is the one that decides the
// `courses/<slug>.md` stem `judge::Heuristics::load` reads and the key `knows_course` matches — and
// the one `zybooks::parse_assignments` uses for `tasks/<slug>.md`. A second implementation would
// diverge silently: the note would be written, the task would be written, and the match would simply
// never happen. It also caps at 60 characters and falls back to `item`, where a naive twin returns an
// empty string and writes `courses/.md`. Call the engine's.

/// `UACS100Fall2026` → `CS 100`. A **suggestion**, not a decision: the student confirms or edits it,
/// and `None` means the panel shows the raw name and asks.
///
/// The rule is the smallest one that fits every code this product has met: find the first run of two
/// to four capitals followed by exactly three digits, and read that as `<LETTERS> <DIGITS>`. It reads
/// past an institution prefix (`UA`) because the letters immediately before the digits are the
/// subject, and it declines `HowToUseZyBooks2` because there is no three-digit number in it — which
/// is the case that matters, since that book is zyBooks' own and is never a course.
pub fn suggest_course(code: &str) -> Option<String> {
    let bytes: Vec<char> = code.chars().collect();
    for start in 0..bytes.len() {
        let letters: String = bytes[start..].iter().take_while(|c| c.is_ascii_uppercase()).collect();
        if letters.len() < 2 || letters.len() > 4 {
            continue;
        }
        let after = start + letters.len();
        let digits: String = bytes[after..].iter().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() != 3 {
            continue;
        }
        // `letters` is already 2..=4 chars long (the guard above refuses anything else), so it IS
        // the subject — no further trimming needed. The loop's forward order (trying the earliest
        // `start` first, then advancing) is what makes `UACS100` read `CS 100` and not `UACS 100`:
        // `trim_prefix` peels the two-letter institution prefix off a four-letter run (m1, review
        // round 1 — the review's own trace confirmed the behaviour; this just says the true thing).
        let subject = letters;
        return Some(format!("{} {}", trim_prefix(&subject), digits));
    }
    None
}

/// `UACS` → `CS` when a two-letter institution prefix is glued to a two-letter subject. Only the
/// prefixes this product has actually met, and never a guess: a four-letter subject like `MATH` is
/// left alone because it is in the list of things that are subjects.
fn trim_prefix(subject: &str) -> String {
    const SUBJECTS: [&str; 12] = ["MATH", "CHEM", "PHYS", "BIOL", "ECON", "HIST", "ENGL", "SPAN", "STAT", "PSYC", "ANTH", "GEOG"];
    if subject.len() == 4 && !SUBJECTS.contains(&subject) && subject.starts_with("UA") {
        return subject[2..].to_string();
    }
    subject.to_string()
}

/// One course to seed. `code` is what a task's title says (`CS 100`), `slug` what its `course:` field
/// carries, `name` what the student sees. Filled by Task 14b's capture; empty until then.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CourseSeed {
    pub code: String,
    // m3: `create_vault_in` explicitly invents both when they are blank
    // (`c.code.clone()` / `slug(&c.code)`), so a page that omits either is not a refusal.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slug: String,
}

/// `CS 100 Intro to Computer Science` → `CS 100`: the course code a **display name** leads with, when
/// it leads with one. This is the half of R-C1-48's pair that a feed's `SUMMARY` can carry, and the
/// shape is the only one this product has met — two to four capitals, then three digits, which is
/// exactly how `lms_link::summarise` counts the courses in a feed. `None` for a name without one, and
/// never a guess: an invented fragment matches somebody else's course.
fn code_in_name(name: &str) -> Option<String> {
    let mut words = name.split_whitespace();
    let (a, b) = (words.next()?, words.next()?);
    let alpha = a.len() >= 2 && a.len() <= 4 && a.chars().all(|c| c.is_ascii_uppercase());
    let digits = b.len() == 3 && b.chars().all(|c| c.is_ascii_digit());
    (alpha && digits).then(|| format!("{a} {b}"))
}

/// **R-C1-48: the two `course_map` fragments one enrolled course contributes**, both pointing at its
/// own slug.
///
/// `ingest::match_course_fields` matches a fragment literally — a case-sensitive substring of the
/// event's UID first, then a case-insensitive run in its SUMMARY bounded by non-alphanumerics — and
/// the two carry different spellings: the UA feed's UIDs hold the LMS's own course id
/// (`UACS100Fall2026`) while its summaries, when they name the course at all, say `CS 100`. So both
/// are written. A fragment the feed never carries simply never matches; a fragment nobody wrote is
/// 28 tasks with no course, which is the failure R-OB-2 exists to remove.
///
/// Never invented: a course whose id and whose name both carry no readable code contributes its id
/// alone.
fn course_fragments(c: &CourseSeed) -> Vec<String> {
    let code = c.code.trim();
    let mut out = vec![code.to_string()];
    if let Some(human) = suggest_course(code).or_else(|| code_in_name(&c.name)) {
        out.push(human);
    }
    out.retain(|f| !f.is_empty());
    out.dedup();
    out
}

/// The enrolled courses a vault is seeded a note for: **one per slug, first wins**. Two enrolments
/// can share a course — a lecture and its lab section are both `CS 100` — and `courses/cs-100.md` is
/// one file, so a second `write::create` for it would be `already exists` and the whole vault would
/// refuse to be made over something that is not a student's mistake. Their fragments are kept
/// regardless (`course_map_lines` reads `plan.courses` whole): the lab section's own LMS id should
/// still point at the course.
fn seeded_courses(plan: &VaultPlan) -> Vec<&CourseSeed> {
    let mut seen = std::collections::HashSet::new();
    plan.courses
        .iter()
        .filter(|c| !c.slug.trim().is_empty())
        .filter(|c| seen.insert(c.slug.clone()))
        .collect()
}

/// Every `course_map` line the vault will carry: the mappings the student confirmed on the coursework
/// panel, then the fragments each enrolled course contributes (R-C1-48) — **first wins by key**.
///
/// First wins twice over. A duplicate key is a `config/ingest.yaml` that `serde_yaml_ng` refuses
/// whole, which is every book and section "not in config; skipped" (review round 1, I1). And where
/// the two panels name the same course, what the student typed outranks what the window guessed.
fn course_map_lines(plan: &VaultPlan) -> Vec<(String, String)> {
    let derived = plan
        .courses
        .iter()
        .filter(|c| !c.slug.trim().is_empty())
        .flat_map(|c| course_fragments(c).into_iter().map(|f| (f, c.slug.clone())));
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<(String, String)> = Vec::new();
    for (fragment, slug) in plan.course_map.iter().cloned().chain(derived) {
        if fragment.trim().is_empty() || slug.trim().is_empty() {
            continue;
        }
        if seen.insert(fragment.clone()) {
            out.push((fragment, slug));
        }
    }
    out
}

/// **Every** wizard-supplied value goes through this before it reaches a YAML file, so a typed
/// value can never change the file's SHAPE (review round 1, Important 1).
///
/// PyYAML's single-quoted style, and it is the right one precisely because inside `'…'` YAML does
/// no escape processing at all: `: `, ` #`, `{`, `[`, `"`, `\`, a leading `%` or `!` are ordinary
/// characters, and the only special case is `'` itself, escaped by doubling. What a single-quoted
/// scalar cannot carry on one line is a control character — so those are REFUSED, by field name,
/// rather than written out and silently breaking the file.
///
/// Silently is the operative word. `runs::load_runners_config` degrades an unparsable
/// `config/runners.yaml` to `Ok(vec![])` and `runs::runner_settings` to `scheduler: Script`, so an
/// unquoted timezone like `America/Chicago: x` would leave a friend's only runner switched off
/// with nothing on any page saying so — the vault would simply never refresh.
fn yaml_scalar(field: &str, value: &str) -> Result<String, String> {
    if let Some(c) = value.chars().find(|c| c.is_control()) {
        return Err(format!("{field}: control character U+{:04X} is not allowed", c as u32));
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

/// The vault's own settings. **No secret is ever written here** — only the `credential_target`
/// names the engine's `wincred::read_credential` looks up (decision 6).
///
/// `ics_url` is written verbatim (quoted, never rewritten): the engine fetches exactly the string
/// the friend pasted, so any mangling here would be a feed that 404s for a reason nothing explains.
pub fn ingest_yaml(p: &VaultPlan) -> Result<String, String> {
    // m8: a plan carrying confirmed mappings with the source's own flag off would drop them in
    // silence below (they are only emitted inside the `if p.zybooks`/`if p.vhl` arms) — unreachable
    // through today's wizard, since the flag comes off the same login panel that fills the
    // mappings, but an early, loud refusal keeps it unreachable on purpose rather than by accident.
    // `zybooks_ignore` is deliberately NOT part of this guard: `create_vault_in` seeds it with
    // `HowToUseZyBooks2` unconditionally (I2), so it is routinely non-empty even with zyBooks off,
    // and nothing is lost when it is — the whole `if p.zybooks` arm that would write it is skipped.
    if !p.zybooks && !p.zybooks_courses.is_empty() {
        return Err("zybooks_courses is set but zybooks is not enabled — a plan bug, not a student's".to_string());
    }
    if !p.vhl && !p.vhl_sections.is_empty() {
        return Err("vhl_sections is set but vhl is not enabled — a plan bug, not a student's".to_string());
    }
    let mut s = String::new();
    // C3′, Task 11 (cloud design §9, Alabama SPII): a capability URL is a credential in all but
    // name, and on a vault that has an account the one place it belongs is the account, encrypted,
    // where `PUT /account/sources` already put it. The KEY stays, empty, so `ingest` still parses
    // the file and so a reader can see the feed is elsewhere rather than missing.
    if !p.account_id.is_empty() {
        s.push_str("ics_url: ''\n");
    } else if let Some(u) = &p.ics_url {
        s.push_str(&format!("ics_url: {}\n", yaml_scalar("LMS feed URL", u)?));
    }
    s.push_str(&format!("timezone: {}\n", yaml_scalar("timezone", &p.timezone)?));
    // R-OB-2 and R-C1-48: what the student confirmed, plus two fragments per enrolled course.
    let course_map = course_map_lines(p);
    if course_map.is_empty() {
        s.push_str("course_map: {}\n");
    } else {
        s.push_str("course_map:\n");
        for (fragment, slug) in &course_map {
            s.push_str(&format!("  {}: {}\n", yaml_scalar("course code", fragment)?, yaml_scalar("course slug", slug)?));
        }
    }
    // The engine reads `calendars:` as a list of `{name, ics_url}` mappings (`calfeed.rs`), and an
    // empty list is why the first page of a fresh install used to show a day with no busy time in
    // it at all. Two possible entries, in a fixed order, through the same `yaml_scalar` every other
    // wizard value goes through: `personal` (the secret iCal address, C1's) and `google` (the
    // marker for the account's grant, C2's — §11a).
    let mut entries: Vec<String> = Vec::new();
    if p.personal_calendar.is_some() {
        if p.account_id.is_empty() {
            let u = p.personal_calendar.as_deref().unwrap_or_default();
            entries.push(format!("  - name: personal\n    ics_url: {}\n", yaml_scalar("personal calendar address", u)?));
        } else {
            // The same removal and the same argument; `cloud:personal` is C2's own routing
            // (`cli.rs:241-250`) and resolves to `/ingest-calendar?name=personal`.
            entries.push("  - name: personal\n    ics_url: 'cloud:personal'\n".to_string());
        }
    }
    if p.google_calendar {
        entries.push("  - name: google\n    ics_url: 'cloud:google'\n".to_string());
    }
    if entries.is_empty() {
        s.push_str("calendars: []\n");
    } else {
        s.push_str("calendars:\n");
        for entry in &entries {
            s.push_str(entry);
        }
    }
    if p.zybooks || p.vhl {
        s.push_str("\n# Passwords are NOT here. They live in Windows Credential Manager under the\n");
        s.push_str("# credential_target names below.\ncoursework:\n");
        if p.zybooks {
            let target = yaml_scalar("zybooks credential target", &crate::credentials::target_for(&p.profile_id, "zybooks"))?;
            s.push_str(&format!("  zybooks:\n    enabled: true\n    credential_target: {target}\n"));
            // Review round 1, I2: every code the student declined to map, plus zyBooks' own
            // `HowToUseZyBooks2` (zero assignments, never coursework) — `create_vault_in` is the one
            // that seeds both; this just writes whatever `p.zybooks_ignore` holds. Out of `ignore:`,
            // an unmapped book is one WARN per healthy run forever (`route_zybook`'s own doc).
            if p.zybooks_ignore.is_empty() {
                s.push_str("    ignore: []\n");
            } else {
                s.push_str("    ignore:\n");
                for code in &p.zybooks_ignore {
                    s.push_str(&format!("      - {}\n", yaml_scalar("zybook ignore", code)?));
                }
            }
            // What `parse_assignments` reads. Without these three blocks every item is uncategorised
            // and takes the default effort, which is the second half of the first-slot failure.
            s.push_str("    categories:\n      HW: hw\n      Lab: lab\n      Project: project\n");
            s.push_str("    effort:\n      minutes_per_section: 6\n      floors:\n        hw: 0.25\n        lab: 0.5\n        project: 1.0\n");
            s.push_str("    importance:\n      hw: 2\n      lab: 2\n      project: 2\n");
            if p.zybooks_courses.is_empty() {
                s.push_str("    courses: {}\n");
            } else {
                s.push_str("    courses:\n");
                for b in &p.zybooks_courses {
                    s.push_str(&format!(
                        "      {}:\n        course: {}\n        label: {}\n",
                        yaml_scalar("zybook code", &b.code)?,
                        yaml_scalar("zybook course", &b.course)?,
                        yaml_scalar("zybook label", &b.label)?
                    ));
                }
            }
        }
        if p.vhl {
            let target = yaml_scalar("vhl credential target", &crate::credentials::target_for(&p.profile_id, "vhl"))?;
            s.push_str(&format!("  vhl:\n    enabled: true\n    credential_target: {target}\n"));
            s.push_str("    importance: 3\n");
            if p.vhl_sections.is_empty() {
                s.push_str("    sections: {}\n");
            } else {
                s.push_str("    sections:\n");
                for v in &p.vhl_sections {
                    s.push_str(&format!(
                        "      {}:\n        course: {}\n        label: {}\n",
                        yaml_scalar("vhl section", &v.section)?,
                        yaml_scalar("vhl course", &v.course)?,
                        yaml_scalar("vhl label", &v.label)?
                    ));
                }
            }
        }
    }
    Ok(s)
}

/// Decision 4: `scheduler: app` and `device:` from birth. `grace_minutes: 20` is the local
/// runner's own number, unchanged; `cloud` is deliberately absent — a friend has no cloud runner,
/// and an expected-but-never-seen runner would paint every Runs view amber forever.
pub fn runners_yaml(p: &VaultPlan) -> Result<String, String> {
    let mut times = Vec::with_capacity(p.slots.len());
    for (i, t) in p.slots.iter().enumerate() {
        // A quoted scalar inside a flow sequence keeps its own commas and colons, so `[…]` stays
        // one item per slot however the slot is spelled.
        times.push(yaml_scalar(&format!("slot {}", i + 1), t)?);
    }
    Ok(format!(
        "runners:\n  - name: local\n    times: [{}]\n    tz: {}\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n",
        times.join(", "),
        yaml_scalar("timezone", &p.timezone)?,
        yaml_scalar("device name", &p.device)?,
    ))
}

/// `config/cloud.yaml` — the four keys the C2 contract fixes, in that order, through the same
/// `yaml_scalar` every other wizard value goes through, so a typed value can never change the file's
/// shape.
///
/// **No secret is here.** `anon_key` is published in every Supabase client and grants nothing on its
/// own; the session JWT is *named*, not carried — `session_credential_target` is a Credential Manager
/// target, exactly as `credential_target` is for the coursework logins.
pub fn cloud_yaml(p: &VaultPlan) -> Result<String, String> {
    Ok(format!(
        "api_base: {}\nanon_key: {}\nsession_credential_target: {}\naccount_id: {}\n",
        yaml_scalar("api_base", &p.api_base)?,
        yaml_scalar("anon_key", &p.anon_key)?,
        yaml_scalar("session_credential_target", &crate::credentials::target_for(&p.profile_id, "session"))?,
        yaml_scalar("account_id", &p.account_id)?,
    ))
}

/// Task 18's half: a vault that already exists gains `config/cloud.yaml` and nothing else — no
/// scaffold, no seed note, no rewrite of anything. **Refuses to overwrite one**: an install adopted
/// twice must not silently repoint at a second account, and a user who signed in with the wrong
/// address needs to hear that rather than to lose the first one.
pub fn write_cloud_yaml_if_absent(vault: &Path, plan: &VaultPlan) -> Result<(), String> {
    let path = vault.join("config").join("cloud.yaml");
    if path.exists() {
        return Err(format!("{}: this vault already has an account", path.display()));
    }
    // An adopted vault's `config/` already exists, but Task 18 does not promise that — this is the
    // only writer of `config/cloud.yaml` outside `build_into`, which makes the folder itself.
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write_file(vault, "config/cloud.yaml", &cloud_yaml(plan)?)
}

/// Materialise **everything** — files and the two seed notes — into a sibling staging folder, then
/// **one rename** into place: a crash, a full disk, a rejected value or a failed seed leaves the
/// destination untouched and the staging folder removed. Never a half-vault, and never a whole
/// vault missing its migration record (spec §3.1; review round 1, Important 2).
///
/// Seeding the staging folder is safe because a journal record's `path` is vault-relative
/// (`write::create` → `ids::rel`) and nothing it writes names the vault's own location, so the
/// records the rename carries into `dest` are the records seeding `dest` directly would have
/// written. `the_move_does_not_leak_the_staging_path_into_the_journal` pins that.
pub fn create_vault(dest: &Path, plan: &VaultPlan) -> Result<(), String> {
    if dest.exists() { return Err(format!("{}: already exists", dest.display())); }
    let parent = dest.parent().ok_or_else(|| format!("{}: no parent folder", dest.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let staging: PathBuf = parent.join(format!(".knowlu-new-{}", knowlu_engine::ids::new_id("stage")));
    let built = build_into(&staging, plan).and_then(|()| seed_writes(&staging, plan));
    match built.and_then(|()| std::fs::rename(&staging, dest).map_err(|e| format!("{}: {e}", dest.display()))) {
        Ok(()) => Ok(()),
        Err(e) => { let _ = std::fs::remove_dir_all(&staging); Err(e) }
    }
}

fn write_file(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    // `pystr::write_text`: the repo's files are CRLF and the engine reads them back through the
    // same translation (CLAUDE.md's line-ending rule).
    let path = root.join(rel);
    knowlu_engine::pystr::write_text(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn build_into(root: &Path, plan: &VaultPlan) -> Result<(), String> {
    for folder in ["tasks", "approvals", "archive", "courses", "issues", "info", "state", "state/journal", "config"] {
        let path = root.join(folder);
        std::fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    // The three files that cannot fail first, the three built from wizard values after — so a
    // rejected value or an unknown campus fails PART-WAY through materialisation, which is the
    // case the staging folder exists for and the case `a_failure_part_way_through_leaves_nothing`
    // drives.
    write_file(root, "config/planning.yaml", PLANNING)?;
    write_file(root, "config/week_template.yaml", WEEK_TEMPLATE)?;
    write_file(root, ".gitignore", GITIGNORE)?;
    let campus = campus_yaml(&plan.campus).ok_or_else(|| format!("unknown campus preset {:?}", plan.campus))?;
    write_file(root, "config/events.yaml", campus)?;
    write_file(root, "config/campus.yaml", &campus_config_yaml(&plan.campus_choice)?)?;
    write_file(root, "config/ingest.yaml", &ingest_yaml(plan)?)?;
    // Review round 1, I1: make the emitter unable to produce an unreadable file at all — a
    // duplicate `course_map`/`courses`/`sections` key (`ingest_yaml` walks a `Vec`; two rows
    // mapping the same book would otherwise write the same key twice) makes `serde_yaml_ng`'s
    // `Mapping` deserializer refuse the whole file, which is `config unreadable: …` and every book
    // and section "not in config; skipped" — the first-slot failure this task exists to remove,
    // arrived at from the write side instead of the read side. The engine's OWN loader decides —
    // not a second, hand-rolled parse call — so this fails exactly where `coursework-discover`/
    // `coursework`/`judge` would fail, and `create_vault` is already all-or-nothing, so a plan bug
    // refuses the wizard loudly instead of handing a student a vault that never syncs.
    let (_, ingest_warnings) = knowlu_engine::coursework::load_coursework_config(root)
        .map_err(|e| format!("config/ingest.yaml would not parse: {e}"))?;
    if !ingest_warnings.is_empty() {
        return Err(format!("config/ingest.yaml would not parse: {}", ingest_warnings.join("; ")));
    }
    write_file(root, "config/runners.yaml", &runners_yaml(plan)?)?;
    write_file(root, "config/cloud.yaml", &cloud_yaml(plan)?)?;
    Ok(())
}

const SEED_BODY: &str = "This vault was created by Knowlu's onboarding. The note exists so the \
journal has a migration record from the vault's first day; nothing reads its body.";

const FIRST_TASK_BODY: &str = "Open the drawer on this row to see every field Knowlu keeps. Tick \
it off when you have had a look — Today will fill up as your courses do.";

/// The two engine calls, in this order (spec §3.1). Both go through `knowlu_engine::write`, and only
/// the first uses a non-`console_ctx()` context — **the one exception in the whole app** (R2).
///
/// **Private, and it stays private** (review round 1, Important 2): `create_vault` runs it against
/// the staging folder before the rename, so a second call — against a vault that already holds
/// `archive/_migrated.md` — could only ever be an error arriving after the vault is already on
/// disk, which is the stranded vault this design removes.
fn seed_writes(vault: &Path, plan: &VaultPlan) -> Result<(), String> {
    let mut journal = Journal::new(vault);

    // Decision 5: `passes::detect_external` refuses to attribute edits unless the journal holds a
    // `create` record whose actor starts with `system:migration` — the shape
    // `scripts/migrate_s1.py` left in Quinn's journal. One archived note satisfies it from the
    // vault's first day, so a friend never meets "journal has no migration records".
    // `via: "cli"` — the exact context `scripts/migrate_s1.py` and `seed_migrated` used. The
    // journal's `via` vocabulary does not grow for this (R-P4a-6): the record is recognised by its
    // ACTOR, which is what `passes.rs` keys on.
    let seed = format!("---\ntitle: Vault created by Knowlu\nstatus: archived\n---\n\n{SEED_BODY}\n");
    write::create(vault, "archive/_migrated.md", &seed, &WriteContext::new("system:migration", "cli"), &mut journal, None)
        .map_err(|e| e.to_string())?;

    // …and one real task, so Today is not empty and the first `rank` has something to order.
    let front = Node::map(vec![
        ("title", Node::text("Get to know Knowlu")),
        ("course", Node::Null),
        ("domain", Node::text("school")),
        ("due", Node::Null),
        ("effort_hours", Node::Float(0.5)),
        ("effort_source", Node::text("quinn")),
        ("importance", Node::Int(2)),
        ("status", Node::text("active")),
        ("progress", Node::Int(0)),
        ("created_by", Node::text("quinn")),
    ]);
    let text = format!("---\n{}---\n\n{FIRST_TASK_BODY}\n", safe_dump_block(&front));
    write::create(vault, "tasks/get-to-know-knowlu.md", &text, &crate::commands::console_ctx(), &mut journal, None)
        .map_err(|e| e.to_string())?;

    // R-OB-2: one note per enrolled course, so tier-1 judgment can place a task and the model has a
    // slug it is allowed to use (`judge::Heuristics::knows_course` tests exactly this).
    // `## Grade weights` is present and empty on purpose: the weights are the student's to write and
    // the judgment's to read, and an invented weight would be a number nobody chose.
    for c in seeded_courses(plan) {
        let front = Node::map(vec![
            ("title", Node::text(&c.name)),
            ("slug", Node::text(&c.slug)),
            ("code", Node::text(&c.code)),
            ("status", Node::text("active")),
        ]);
        let body = format!(
            "---\n{}---\n\n## Grade weights\n\nFill this in from your syllabus — Knowlu uses it to decide what matters.\n",
            safe_dump_block(&front)
        );
        write::create(vault, &format!("courses/{}.md", c.slug), &body, &crate::commands::console_ctx(), &mut journal, None)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
