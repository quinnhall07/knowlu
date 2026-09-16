//! Event-discovery configuration and the shared event type — port of `engine/events.py`.
//!
//! Wave 5 of the Rust port. This is the leaf of the events group: `eventfeed`, `eventfilter`,
//! `eventroster` and `eventemit` all build on [`DiscoveredEvent`], [`EventsConfig`] and
//! [`Interests`].
//!
//! **Nothing here may fail a run.** A missing `config/events.yaml` means the feature is off, not
//! broken; a malformed one degrades to defaults plus warnings. `cli.run` wraps the whole events
//! pass in `except Exception` because a crash there costs the morning's ranking — Rust has no
//! equivalent of that catch, so no function in this module panics, indexes, or unwraps.

use std::path::Path;

use jiff::civil::{Date, DateTime};
use serde_yaml_ng::Value;

use crate::models::split_frontmatter;
use crate::pystr;
use crate::yaml;

/// `DEFAULTS`, in Python's insertion order — the order `load_events_config` reads them, and
/// therefore the order their warnings appear in.
pub const DEFAULTS: [(&str, i64); 7] = [
    ("roster_window_days", 90),
    ("audit_window_days", 14),
    ("judge_per_run_cap", 150),
    ("propose_horizon_days", 14),
    ("urgency_window_days", 7),
    ("daily_proposal_target", 5),
    ("daily_proposal_ceiling", 15),
];

pub const VALID_TYPES: [&str; 4] = ["ics", "localist", "engage", "html"];

fn default_for(key: &str) -> i64 {
    DEFAULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .unwrap_or(0)
}

/// One event instance, normalized to naive America/Chicago wall time.
///
/// Construct through [`DiscoveredEvent::new`] or finish with [`DiscoveredEvent::normalized`]:
/// Python's `__post_init__` defaults `series_uid` to `uid`, and several call sites rely on it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DiscoveredEvent {
    pub uid: String,
    pub title: String,
    pub start: Option<DateTime>,
    pub end: Option<DateTime>,
    pub source: String,
    pub organizer: String,
    pub location: String,
    pub url: String,
    pub description: String,
    pub categories: Vec<String>,
    pub benefits: Vec<String>,
    pub audiences: Vec<String>,
    pub registration: bool,
    pub registration_deadline: Option<Date>,
    pub series_uid: String,
}

impl DiscoveredEvent {
    pub fn new(uid: &str, title: &str, start: DateTime, end: DateTime, source: &str) -> Self {
        DiscoveredEvent {
            uid: uid.to_string(),
            title: title.to_string(),
            start: Some(start),
            end: Some(end),
            source: source.to_string(),
            ..Default::default()
        }
        .normalized()
    }

    /// Python's `__post_init__`: an empty `series_uid` becomes the `uid`.
    pub fn normalized(mut self) -> Self {
        if self.series_uid.is_empty() {
            self.series_uid = self.uid.clone();
        }
        self
    }

    /// Every consumer treats `start` as present; the Option exists only so `Default` works.
    pub fn start(&self) -> DateTime {
        self.start.unwrap_or_else(|| Date::constant(1970, 1, 1).to_datetime(jiff::civil::Time::midnight()))
    }

    pub fn end(&self) -> DateTime {
        self.end.unwrap_or_else(|| self.start())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventsSource {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub enabled: bool,
}

/// Python's `EventsConfig` is the one mutable dataclass in the group — `load_events_config`
/// assigns each `DEFAULTS` key onto it in turn, and the urgency/horizon guard rewrites two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventsConfig {
    pub sources: Vec<EventsSource>,
    pub roster_window_days: i64,
    pub audit_window_days: i64,
    pub judge_per_run_cap: i64,
    pub propose_horizon_days: i64,
    pub urgency_window_days: i64,
    pub daily_proposal_target: i64,
    pub daily_proposal_ceiling: i64,
    pub timezone: String,
}

impl Default for EventsConfig {
    fn default() -> Self {
        EventsConfig {
            sources: Vec::new(),
            roster_window_days: default_for("roster_window_days"),
            audit_window_days: default_for("audit_window_days"),
            judge_per_run_cap: default_for("judge_per_run_cap"),
            propose_horizon_days: default_for("propose_horizon_days"),
            urgency_window_days: default_for("urgency_window_days"),
            daily_proposal_target: default_for("daily_proposal_target"),
            daily_proposal_ceiling: default_for("daily_proposal_ceiling"),
            timezone: "America/Chicago".to_string(),
        }
    }
}

impl EventsConfig {
    fn set(&mut self, key: &str, value: i64) {
        match key {
            "roster_window_days" => self.roster_window_days = value,
            "audit_window_days" => self.audit_window_days = value,
            "judge_per_run_cap" => self.judge_per_run_cap = value,
            "propose_horizon_days" => self.propose_horizon_days = value,
            "urgency_window_days" => self.urgency_window_days = value,
            "daily_proposal_target" => self.daily_proposal_target = value,
            "daily_proposal_ceiling" => self.daily_proposal_ceiling = value,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Interests {
    pub strong: Vec<String>,
    pub mild: Vec<String>,
    pub never: Vec<String>,
    pub clubs: Vec<String>,
}

/// Python's `int(value)` over a YAML scalar: `True` is 1, a float truncates toward zero, a
/// numeric string parses, everything else raises and becomes a warning.
fn python_int_of(value: &Value) -> Option<i64> {
    match value {
        Value::Bool(b) => Some(i64::from(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(i)
            } else {
                n.as_f64().map(|f| f as i64)
            }
        }
        Value::String(s) => pystr::strip(s).parse::<i64>().ok(),
        _ => None,
    }
}

/// `{value!r}` for the warning text. Only strings differ from `str()`, and nothing asserts this.
fn python_repr(value: Option<&Value>) -> String {
    match value {
        None => "None".to_string(),
        Some(Value::String(s)) => format!("'{s}'"),
        Some(Value::Null) => "None".to_string(),
        Some(Value::Bool(b)) => if *b { "True".into() } else { "False".into() },
        Some(v) => yaml::text(v).unwrap_or_else(|| format!("{v:?}")),
    }
}

fn int_or_default(raw: &serde_yaml_ng::Mapping, key: &str, warnings: &mut Vec<String>) -> i64 {
    let fallback = default_for(key);
    let Some(value) = yaml::get(raw, key) else {
        return fallback;
    };
    match python_int_of(value) {
        Some(v) => v,
        None => {
            warnings.push(format!(
                "config: bad {key} ({}); using {fallback}",
                python_repr(Some(value))
            ));
            fallback
        }
    }
}

/// Load `config/events.yaml`. A missing file means the feature is simply off.
pub fn load_events_config(path: &Path) -> (EventsConfig, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();
    if !path.exists() {
        return (EventsConfig::default(), warnings);
    }
    let text = match pystr::read_text(path) {
        Ok(text) => text,
        Err(err) => {
            return (EventsConfig::default(), vec![format!("config unreadable: {err}")]);
        }
    };
    let parsed: Value = match serde_yaml_ng::from_str(&text) {
        Ok(v) => v,
        Err(err) => {
            return (EventsConfig::default(), vec![format!("config unreadable: {err}")]);
        }
    };
    let raw = match parsed {
        Value::Null => serde_yaml_ng::Mapping::new(),
        Value::Mapping(m) => m,
        _ => {
            return (
                EventsConfig::default(),
                vec!["config unreadable: not a mapping".to_string()],
            );
        }
    };

    let mut config = EventsConfig {
        timezone: yaml::opt_text(yaml::get(&raw, "timezone"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "America/Chicago".to_string()),
        ..Default::default()
    };
    for (key, _) in DEFAULTS {
        let value = int_or_default(&raw, key, &mut warnings);
        config.set(key, value);
    }

    // A urgency window at least as wide as the propose horizon would make every proposal urgent,
    // so both fields snap back to their defaults together rather than one being clamped.
    if config.urgency_window_days >= config.propose_horizon_days {
        warnings.push(
            "config: urgency_window_days must be less than propose_horizon_days; using defaults"
                .to_string(),
        );
        config.propose_horizon_days = default_for("propose_horizon_days");
        config.urgency_window_days = default_for("urgency_window_days");
    }

    let mut sources: Vec<EventsSource> = Vec::new();
    match yaml::get(&raw, "sources") {
        None => {}
        Some(Value::Sequence(entries)) => {
            for entry in entries {
                let Some(map) = entry.as_mapping() else {
                    warnings.push(format!("bad source entry: {}", python_repr(Some(entry))));
                    continue;
                };
                let name = yaml::opt_text(yaml::get(map, "name")).unwrap_or_default();
                let name = pystr::strip(&name).to_string();
                let kind = yaml::opt_text(yaml::get(map, "type")).unwrap_or_default();
                let kind = pystr::strip(&kind).to_string();
                let url = yaml::opt_text(yaml::get(map, "url")).unwrap_or_default();
                let url = pystr::strip(&url).to_string();
                if name.is_empty() {
                    warnings.push("source with no name skipped".to_string());
                    continue;
                }
                if !VALID_TYPES.contains(&kind.as_str()) {
                    warnings.push(format!("{name}: unknown type '{kind}'"));
                    continue;
                }
                if url.is_empty() {
                    warnings.push(format!("{name}: no url"));
                    continue;
                }
                // Python: `bool(entry.get("enabled"))` — truthiness, not a boolean. So a quoted
                // `"false"`, a `1` or a non-empty list all turn a source ON; preserved defect 19's
                // second site (the first is coursework's `enabled:`). Until the Task 17 sweep
                // (2026-09-02) this port accepted only a bare `true`, which would have switched a
                // source spelled `enabled: 1` off at cutover with nothing on the page to say so.
                let enabled = yaml::get(map, "enabled").is_some_and(pystr::yaml_truthy);
                sources.push(EventsSource { name, kind, url, enabled });
            }
        }
        Some(_) => warnings.push("sources must be a list".to_string()),
    }
    config.sources = sources;
    (config, warnings)
}

fn tuple_of(meta: &serde_yaml_ng::Mapping, key: &str) -> Vec<String> {
    let Some(Value::Sequence(values)) = yaml::get(meta, key) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|v| {
            // Python's `str(v).strip()`, then drop the empties — so a null item is the term
            // `None` and a bare `true` is `True`, Python's spellings, not `Null`/`true`.
            let text = pystr::yaml_str(v);
            let trimmed = pystr::strip(&text).to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .collect()
}

/// Read `profile/interests.md` frontmatter. Absent or broken means empty.
pub fn load_interests(path: &Path) -> (Interests, Vec<String>) {
    if !path.exists() {
        return (Interests::default(), Vec::new());
    }
    let text = match pystr::read_text(path) {
        Ok(text) => text,
        Err(err) => {
            return (Interests::default(), vec![format!("interests unreadable: {err}")]);
        }
    };
    let meta = match split_frontmatter(&text) {
        Ok((meta, _)) => meta,
        Err(crate::models::NoteError::NotAMapping) => {
            return (
                Interests::default(),
                vec!["interests unreadable: not a mapping".to_string()],
            );
        }
        Err(err) => {
            return (
                Interests::default(),
                vec![format!("interests unreadable: {err:?}")],
            );
        }
    };
    (
        Interests {
            strong: tuple_of(&meta, "strong"),
            mild: tuple_of(&meta, "mild"),
            never: tuple_of(&meta, "never"),
            clubs: tuple_of(&meta, "clubs"),
        },
        Vec::new(),
    )
}

/// The agent actor for event verdicts. `agent:` prefix, because `provenance::is_agent` is a plain
/// `starts_with("agent:")` test and nothing else.
pub const ACTOR: &str = "agent:knowlu.events";

/// Judge every event the feeds carry that has no verdict yet, and record what comes back.
///
/// **One verdict per uid, forever** (events spec §7). The ledger is append-only and a uid that
/// already has a verdict is never re-asked, because a drop the student has lived with for a week
/// must not silently become an opportunity.
///
/// **Judged from the FEEDS, not from `state/events.md`.** `eventroster::read_roster` reconstructs
/// six fields and hard-codes `source: "roster"`; the description, the categories, the audiences and
/// the series uid — most of what the prompt is written around, and two of the four promotion
/// features — are not in it. So this calls `eventfeed::load_discovered_events`, the same function
/// `rank`'s events pass calls, through the same `Fetchers.events` seam and therefore through the
/// same server-side proxy (hand-off H4). One extra fetch per slot buys a judgment that can see the
/// event.
///
/// **Writes only the ledger.** Nothing here writes `state/events.md` — `rank` regenerates it a few
/// seconds later, and by then the verdicts are in the ledger it reads.
///
/// Never panics and never fails a run: every failure is a line.
///
/// **`budget`** (fix 1, R-C2-E22 #2): a wall-clock bound on this call, checked BEFORE each item
/// starts — the same discipline `enrich_with`'s own `BATCH_BUDGET` loop uses, and for the same
/// reason. This pass runs after the enrichment batch has already spent part of `opts.budget`, and
/// with no bound of its own it could push up to `cap` sequential `/judge-event` calls (150 by
/// default, 120 seconds each) well past `scheduler::CHILD_TIMEOUT`. Items cut by the budget are
/// folded into the same "left for the next slot" line as items cut by `cap` — nothing is lost,
/// and no slot is held for hours.
pub fn judge_roster(
    vault: &Path,
    model: &dyn crate::judge::EventModel,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
    today: Date,
    cap: usize,
    budget: std::time::Duration,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let (config, _) = load_events_config(&vault.join("config").join("events.yaml"));
    if !config.sources.iter().any(|s| s.enabled) {
        return lines;
    }
    let (discovered, warnings) = crate::eventfeed::load_discovered_events(vault, fetch);
    for warning in warnings {
        lines.push(format!("events: {warning}"));
    }
    if discovered.is_empty() {
        return lines;
    }

    let ledger = crate::eventledger::load_ledger(vault, None);
    let interests = crate::pystr::read_text(&vault.join("profile").join("interests.md"))
        .map(|t| crate::judge::clip(t.trim(), crate::judge::MAX_PREFS_CHARS))
        .unwrap_or_default();

    let pending: Vec<&DiscoveredEvent> = discovered
        .iter()
        .filter(|e| ledger.get(&e.uid).and_then(|entry| entry.verdict.as_ref()).is_none())
        .collect();
    let left_for_cap = pending.len().saturating_sub(cap);
    let batch: Vec<&DiscoveredEvent> = pending.into_iter().take(cap).collect();
    let started = std::time::Instant::now();
    let mut judged = 0usize;
    let mut processed = 0usize;

    for event in &batch {
        // Checked BEFORE starting each item, not after — the worst case this bounds is the budget
        // plus one call already in flight, not the budget plus a whole extra item.
        if started.elapsed() >= budget {
            break;
        }
        processed += 1;
        let event = *event;
        let item = crate::judge::EventItem {
            uid: event.uid.clone(),
            title: crate::judge::one_line(&event.title, 200),
            start: event.start().strftime("%Y-%m-%dT%H:%M").to_string(),
            end: event.end().strftime("%Y-%m-%dT%H:%M").to_string(),
            source: event.source.clone(),
            organizer: crate::judge::one_line(&event.organizer, 120),
            location: crate::judge::one_line(&event.location, 120),
            url: event.url.clone(),
            description: crate::judge::clip(event.description.trim(), crate::judge::MAX_BODY_CHARS),
            categories: event.categories.clone(),
            audiences: event.audiences.clone(),
            series_uid: event.series_uid.clone(),
            interests: interests.clone(),
        };
        let verdict = match model.judge_event(&item) {
            Ok(v) => v,
            Err(e) => {
                lines.push(format!("events {}: not judged ({e})", event.uid));
                continue;
            }
        };
        // Checked again here, not only on the server: `record_verdict` refuses an unknown word and
        // a `why` carrying a quote, a newline or the field separator, and a refusal at that depth
        // would lose the verdict with no line to explain it.
        if !crate::eventledger::VALID_VERDICTS.contains(&verdict.verdict.as_str()) {
            lines.push(format!("events {}: refused ({:?} is not a verdict)", event.uid, verdict.verdict));
            continue;
        }
        let why = crate::judge::one_line(
            &verdict.why.replace('"', "'").replace(" \u{b7} ", " - "),
            140,
        );
        match crate::eventledger::record_verdict(
            vault, &event.uid, &event.title, today, &verdict.verdict, "", &why, "",
        ) {
            Ok(()) => {
                judged += 1;
                lines.push(format!("events {}: {} ({:.2})", event.uid, verdict.verdict, verdict.confidence));
            }
            Err(e) => lines.push(format!("events {}: not recorded ({e:?})", event.uid)),
        }
    }
    // `left_for_cap` already counts what `cap` excluded from the batch; fold in whatever the
    // budget cut off the batch itself, so one line always names everything not reached this run.
    let left = left_for_cap + (batch.len() - processed);
    let mut summary = format!("events: {judged} judged");
    if left > 0 {
        summary.push_str(&format!(", {left} left for the next slot"));
    }
    lines.push(summary);
    lines
}

#[cfg(test)]
mod tests {
    //! Direct port of `tests/test_events_config.py` — all 14 tests, same names.

    use super::*;
    use jiff::civil::date;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qo-events-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        pystr::write_text(&path, text).unwrap();
        path
    }

    #[test]
    fn missing_config_is_off_not_an_error() {
        let dir = tmp_dir("missingcfg");
        let (config, warnings) = load_events_config(&dir.join("nope.yaml"));
        assert!(config.sources.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn loads_sources_and_defaults() {
        let dir = tmp_dir("defaults");
        let path = write(
            &dir,
            "events.yaml",
            "sources:\n  - name: blount\n    type: ics\n    url: https://blount.as.ua.edu/events/?ical=1\n    enabled: true\n  - name: campus\n    type: localist\n    url: https://calendar.ua.edu/api/2/events\n",
        );
        let (config, warnings) = load_events_config(&path);
        assert!(warnings.is_empty());
        let names: Vec<&str> = config.sources.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["blount", "campus"]);
        assert!(config.sources[0].enabled);
        assert!(!config.sources[1].enabled, "absent means off");
        assert_eq!(config.propose_horizon_days, 14);
        assert_eq!(config.urgency_window_days, 7);
        assert_eq!(config.daily_proposal_target, 5);
        assert_eq!(config.daily_proposal_ceiling, 15);
        assert_eq!(config.roster_window_days, 90);
        assert_eq!(config.audit_window_days, 14);
        assert_eq!(config.judge_per_run_cap, 150);
    }

    #[test]
    fn urgency_must_be_narrower_than_horizon() {
        let dir = tmp_dir("urgency");
        let path = write(
            &dir,
            "events.yaml",
            "propose_horizon_days: 7\nurgency_window_days: 7\nsources: []\n",
        );
        let (config, warnings) = load_events_config(&path);
        assert!(warnings.iter().any(|w| w.contains("urgency_window_days")));
        assert_eq!(config.propose_horizon_days, 14, "fell back to defaults");
        assert_eq!(config.urgency_window_days, 7);
    }

    #[test]
    fn unreadable_config_warns_and_stays_off() {
        let dir = tmp_dir("unreadable");
        let path = write(&dir, "events.yaml", "sources: [oops\n");
        let (config, warnings) = load_events_config(&path);
        assert!(config.sources.is_empty());
        assert!(warnings.iter().any(|w| w.contains("unreadable")));
    }

    #[test]
    fn source_missing_url_is_skipped_with_warning() {
        let dir = tmp_dir("nourl");
        let path = write(&dir, "events.yaml", "sources:\n  - name: broken\n    type: ics\n");
        let (config, warnings) = load_events_config(&path);
        assert!(config.sources.is_empty());
        assert!(warnings.iter().any(|w| w.contains("broken")));
    }

    #[test]
    fn load_interests_reads_the_four_lists() {
        let dir = tmp_dir("interests");
        let path = write(
            &dir,
            "interests.md",
            "---\nstrong:\n  - research opportunities\n  - hackathons\nmild:\n  - free food\nnever:\n  - greek life\nclubs:\n  - Artificial Intelligence Club\n---\n\nNotes here.\n",
        );
        let (interests, warnings) = load_interests(&path);
        assert!(warnings.is_empty());
        assert_eq!(interests.strong, vec!["research opportunities", "hackathons"]);
        assert_eq!(interests.mild, vec!["free food"]);
        assert_eq!(interests.never, vec!["greek life"]);
        assert_eq!(interests.clubs, vec!["Artificial Intelligence Club"]);
    }

    #[test]
    fn missing_interests_is_empty() {
        let dir = tmp_dir("nointerests");
        let (interests, warnings) = load_interests(&dir.join("nope.md"));
        assert!(warnings.is_empty());
        assert!(interests.strong.is_empty());
        assert!(interests.never.is_empty());
    }

    #[test]
    fn discovered_event_is_hashable_and_defaults_series_to_uid() {
        let event = DiscoveredEvent::new(
            "engage:1",
            "Kickoff",
            date(2026, 9, 3).at(18, 0, 0, 0),
            date(2026, 9, 3).at(19, 30, 0, 0),
            "clubs",
        );
        let set: HashSet<DiscoveredEvent> = [event.clone(), event.clone()].into_iter().collect();
        assert_eq!(set.len(), 1);
        assert_eq!(event.series_uid, "engage:1");
    }

    /// PORT DIVERGENCE, not a Rust bug in this module. PyYAML raises `ConstructorError` for an
    /// unknown `!!tag`; `serde_yaml_ng` silently resolves it away (`strong: !!invalid` becomes
    /// `String("")`), so the frontmatter parses and no warning is produced. Recorded as
    /// cross-cutting trap 4 in the preserved-defects report, where the load_tasks consequence is
    /// spelled out — it can put a task on the page that Python skips as unreadable.
    ///
    /// Left as a visible red rather than rewritten to assert the Rust behaviour: encoding the
    /// divergence as correct is how it would get forgotten. `cargo test -- --ignored` shows it.
    #[test]
    #[ignore = "trap 4: serde_yaml_ng accepts unknown !!tags that PyYAML rejects — ACCEPTED by decision 2026-09-02 (cutover plan C8); scripts/lint-yaml-11.py measures reachability every dual run (0 tags at acceptance); the assertion stays Python's"]
    fn malformed_interests_warns_and_stays_empty() {
        let dir = tmp_dir("badinterests");
        let path = write(&dir, "interests.md", "---\nstrong: !!invalid\n---\n");
        let (interests, warnings) = load_interests(&path);
        assert!(interests.strong.is_empty());
        assert!(warnings.iter().any(|w| w.contains("unreadable")), "{warnings:?}");
    }

    /// Python monkeypatches `Path.read_text` to raise `PermissionError`; the Rust equivalent is a
    /// path that exists as a DIRECTORY, so the read fails for a real reason rather than a mocked
    /// one. Same contract: an unreadable file is a warning, never a crash.
    #[test]
    fn load_interests_read_error_is_a_warning_not_a_crash() {
        let dir = tmp_dir("lockedinterests");
        let path = dir.join("interests.md");
        fs::create_dir_all(&path).unwrap();
        let (interests, warnings) = load_interests(&path);
        assert!(interests.strong.is_empty());
        assert!(warnings.iter().any(|w| w.contains("unreadable")), "{warnings:?}");
    }

    #[test]
    fn source_with_invalid_type_is_skipped_with_warning() {
        let dir = tmp_dir("badtype");
        let path = write(
            &dir,
            "events.yaml",
            "sources:\n  - name: bad-type\n    type: unknown\n    url: https://example.com\n",
        );
        let (config, warnings) = load_events_config(&path);
        assert!(config.sources.is_empty());
        assert!(warnings.iter().any(|w| w.contains("unknown type")));
    }

    #[test]
    fn source_with_no_name_is_skipped_with_warning() {
        let dir = tmp_dir("noname");
        let path = write(
            &dir,
            "events.yaml",
            "sources:\n  - type: ics\n    url: https://example.com\n",
        );
        let (config, warnings) = load_events_config(&path);
        assert!(config.sources.is_empty());
        assert!(warnings.iter().any(|w| w.contains("no name")));
    }

    #[test]
    fn source_entry_not_a_mapping_is_skipped_with_warning() {
        let dir = tmp_dir("notamapping");
        let path = write(&dir, "events.yaml", "sources:\n  - just a string\n");
        let (config, warnings) = load_events_config(&path);
        assert!(config.sources.is_empty());
        assert!(warnings.iter().any(|w| w.contains("bad source entry")));
    }

    #[test]
    fn sources_as_scalar_is_skipped_with_warning() {
        let dir = tmp_dir("scalarsources");
        let path = write(&dir, "events.yaml", "sources: 5\n");
        let (config, warnings) = load_events_config(&path);
        assert!(config.sources.is_empty());
        assert!(warnings.iter().any(|w| w.contains("sources must be a list")));
    }

    /// Not in the Python suite. `int(True)` is 1 and `int(3.7)` is 3, and this module's whole job
    /// is to coerce YAML scalars the way Python does — so the coercion gets its own test rather
    /// than being asserted only through the six fields that happen to use it.
    #[test]
    fn int_coercion_matches_python() {
        assert_eq!(python_int_of(&Value::Bool(true)), Some(1));
        assert_eq!(python_int_of(&Value::Bool(false)), Some(0));
        assert_eq!(python_int_of(&Value::from(3.7)), Some(3));
        assert_eq!(python_int_of(&Value::from(-3.7)), Some(-3));
        assert_eq!(python_int_of(&Value::from("  5  ")), Some(5));
        assert_eq!(python_int_of(&Value::from("abc")), None);
        assert_eq!(python_int_of(&Value::Null), None);
    }

    // --- the Task 17 defect sweep (2026-09-02); every expectation measured against Python ---

    /// `bool(entry.get("enabled"))`: the quoted `"false"` that turns a coursework source ON
    /// (preserved defect 19) turns an event source on too, and so do `1` and `[1]`; `0`, `""`,
    /// `null` and `[]` are off. This port accepted only a bare `true` until the sweep found it —
    /// the one direction that would have silenced a source at cutover. (`yes`/`no` are trap 5,
    /// below.)
    #[test]
    fn preserved_defect_19_applies_to_events_yaml_too() {
        let dir = tmp_dir("enabledtruthy");
        for (spelling, expected) in [
            ("\"false\"", true),
            ("1", true),
            ("[1]", true),
            ("true", true),
            ("0", false),
            ("\"\"", false),
            ("null", false),
            ("[]", false),
            ("false", false),
        ] {
            let path = write(
                &dir,
                "events.yaml",
                &format!("sources:\n  - name: s\n    type: ics\n    url: http://x\n    enabled: {spelling}\n"),
            );
            let (config, warnings) = load_events_config(&path);
            assert!(warnings.is_empty(), "{spelling}: {warnings:?}");
            assert_eq!(config.sources[0].enabled, expected, "enabled: {spelling}");
        }
    }

    /// Cross-cutting trap 5 (found by the Task 17 sweep, 2026-09-02). PyYAML resolves YAML
    /// **1.1** scalars: `yes`/`no`/`on`/`off` are booleans, `1_000` is 1000, `0x10` is 16, `1:30`
    /// is 90, `.inf` is a float. `serde_yaml_ng` follows YAML 1.2 and reads every one of those
    /// as a string — so `enabled: no` is OFF in Python and, through truthiness, ON here. Same
    /// family as the timestamp trap, and the same treatment as trap 4: measured unreachable in
    /// the live vault (236 YAML documents scanned, zero 1.1-only scalars outside the 111 known
    /// timestamps), recorded, and settled before cutover rather than in code. This test says
    /// what "settled" would mean for the one spelling a hand-written config is likely to use.
    #[test]
    #[ignore = "trap 5: PyYAML resolves YAML 1.1 booleans (yes/no/on/off) that serde_yaml_ng reads as strings — ACCEPTED by decision 2026-09-02 (cutover plan C8); scripts/lint-yaml-11.py measures reachability every dual run (0 scalars at acceptance); the assertion stays Python's"]
    fn trap_5_a_yaml_1_1_boolean_spelling_is_read_as_python_reads_it() {
        let dir = tmp_dir("yaml11bool");
        for (spelling, expected) in [("yes", true), ("no", false), ("on", true), ("off", false), ("NO", false)] {
            let path = write(
                &dir,
                "events.yaml",
                &format!("sources:\n  - name: s\n    type: ics\n    url: http://x\n    enabled: {spelling}\n"),
            );
            let (config, _) = load_events_config(&path);
            assert_eq!(config.sources[0].enabled, expected, "enabled: {spelling}");
        }
    }

    /// `str(v).strip()` over an interests list: a null item is the term `None`, a bare `true`
    /// is `True`, numbers print as Python prints them, and the empties are dropped.
    #[test]
    fn interest_terms_are_pythons_str_of_each_item() {
        let dir = tmp_dir("interestsstr");
        let path = write(
            &dir,
            "interests.md",
            "---\nnever: [null, true, 5, 2.5, '  spaced  ', '']\nstrong: [Robotics]\n---\n",
        );
        let (interests, warnings) = load_interests(&path);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(interests.never, vec!["None", "True", "5", "2.5", "spaced"]);
        assert_eq!(interests.strong, vec!["Robotics"]);
    }

    /// A scripted event model, so the roster pass is exercised with no socket at all.
    struct Scripted(std::cell::RefCell<Vec<Result<crate::judge::EventVerdict, crate::judge::ModelError>>>);

    impl crate::judge::EventModel for Scripted {
        fn judge_event(
            &self,
            _item: &crate::judge::EventItem,
        ) -> Result<crate::judge::EventVerdict, crate::judge::ModelError> {
            self.0
                .borrow_mut()
                .pop()
                .unwrap_or_else(|| Err(crate::judge::ModelError::Failed("nothing scripted".into())))
        }
    }

    fn verdict(word: &str, why: &str) -> crate::judge::EventVerdict {
        crate::judge::EventVerdict { verdict: word.into(), why: why.into(), confidence: 0.9, tier: 3 }
    }

    /// A vault with two events on one enabled ICS source, and a verdict already recorded for the
    /// first. Never points at the worktree (ruling R-3a-12).
    fn scratch_vault_with_feed(name: &str) -> (std::path::PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("knowlu-c2-events-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
        std::fs::create_dir_all(dir.join("state")).expect("scratch vault");
        std::fs::create_dir_all(dir.join("profile")).expect("scratch vault");
        std::fs::write(
            dir.join("config").join("events.yaml"),
            "sources:\n  - name: engage\n    type: ics\n    url: https://example.invalid/e.ics\n    enabled: true\n",
        )
        .expect("write events.yaml");
        std::fs::write(dir.join("profile").join("interests.md"), "Machine learning, German.\n")
            .expect("write interests.md");
        // Seeded under `ics:engage:1`, not the raw `engage:1` the ICS UID line carries:
        // `eventfeed::event_from_block` prefixes every ICS-sourced uid with `ics:` before it ever
        // reaches the ledger, and a seed at the unprefixed key would never be recognised as
        // already-judged.
        crate::eventledger::record_verdict(
            &dir, "ics:engage:1", "AI Club Kickoff", jiff::civil::date(2026, 8, 20), "drop", "", "not this term", "",
        )
        .expect("seed a verdict");
        let feed = "BEGIN:VCALENDAR\r\n\
             BEGIN:VEVENT\r\nUID:engage:1\r\nSUMMARY:AI Club Kickoff\r\n\
             DTSTART:20260829T230000Z\r\nDTEND:20260830T000000Z\r\n\
             DESCRIPTION:An evening for anyone curious about machine learning.\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:engage:2\r\nSUMMARY:Career Fair\r\n\
             DTSTART:20260830T150000Z\r\nDTEND:20260830T190000Z\r\n\
             DESCRIPTION:Employers across engineering and computing.\r\nEND:VEVENT\r\n\
             END:VCALENDAR\r\n"
            .to_string();
        (dir, feed)
    }

    #[test]
    fn only_unjudged_uids_are_sent_and_one_verdict_per_uid_forever() {
        // §7's one-verdict-per-uid-forever rule is what makes a drop permanent, and re-asking
        // would quietly overwrite a decision the student already lives with.
        let (vault, feed) = scratch_vault_with_feed("unjudged");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![Ok(verdict("opportunity", "matches the stated interests"))]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150, std::time::Duration::from_secs(60));
        assert!(lines.iter().any(|l| l.contains("engage:2") && l.contains("opportunity")), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("engage:1")), "{lines:?}");
        let ledger = crate::eventledger::load_ledger(&vault, None);
        assert_eq!(ledger["ics:engage:2"].verdict.as_deref(), Some("opportunity"));
        assert_eq!(ledger["ics:engage:1"].verdict.as_deref(), Some("drop"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_judged_item_carries_the_description_and_the_real_source() {
        // The defect this pass was rewritten to avoid: judging from `read_roster` would send an
        // empty description and the literal source "roster" for every event on every account,
        // which is most of the prompt and two of the four promotion features.
        let (vault, feed) = scratch_vault_with_feed("carries");
        let fetch = |_: &str| Ok(feed.clone());
        let seen: std::cell::RefCell<Vec<crate::judge::EventItem>> = std::cell::RefCell::new(Vec::new());
        struct Recorder<'a>(&'a std::cell::RefCell<Vec<crate::judge::EventItem>>);
        impl crate::judge::EventModel for Recorder<'_> {
            fn judge_event(
                &self,
                item: &crate::judge::EventItem,
            ) -> Result<crate::judge::EventVerdict, crate::judge::ModelError> {
                self.0.borrow_mut().push(item.clone());
                Err(crate::judge::ModelError::Failed("recorded only".into()))
            }
        }
        let _ = judge_roster(&vault, &Recorder(&seen), Some(&fetch), jiff::civil::date(2026, 8, 28), 150, std::time::Duration::from_secs(60));
        let items = seen.borrow();
        assert_eq!(items.len(), 1);
        assert!(items[0].description.contains("Employers"), "{:?}", items[0].description);
        assert_eq!(items[0].source, "engage");
        assert!(items[0].interests.contains("Machine learning"));
        drop(items);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_verdict_word_the_ledger_refuses_is_dropped_and_named() {
        let (vault, feed) = scratch_vault_with_feed("badword");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![Ok(verdict("maybe", "unsure"))]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150, std::time::Duration::from_secs(60));
        assert!(lines.iter().any(|l| l.contains("refused")), "{lines:?}");
        assert!(crate::eventledger::load_ledger(&vault, None).get("ics:engage:2").is_none());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_why_carrying_the_field_separator_is_repaired_rather_than_lost() {
        // `eventledger::why_problem` refuses a double quote, a newline and the field separator
        // outright, and a refused why throws the whole verdict away. The service one-lines it; the
        // device checks again, because a service that changed and a device that did not is the
        // case this guards.
        let (vault, feed) = scratch_vault_with_feed("why");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![Ok(verdict("drop", "she said \"no\" \u{b7} twice"))]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150, std::time::Duration::from_secs(60));
        assert!(lines.iter().any(|l| l.contains("engage:2")), "{lines:?}");
        let ledger = crate::eventledger::load_ledger(&vault, None);
        assert_eq!(ledger["ics:engage:2"].verdict.as_deref(), Some("drop"));
        assert!(!ledger["ics:engage:2"].why.contains('"'));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_per_run_cap_bounds_the_batch_and_says_what_is_left() {
        let (vault, feed) = scratch_vault_with_feed("cap");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 0, std::time::Duration::from_secs(60));
        assert!(lines.iter().any(|l| l.contains("1 left for the next slot")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_feed_that_will_not_fetch_is_a_line_and_never_a_panic() {
        let (vault, _) = scratch_vault_with_feed("deadfeed");
        let fetch = |_: &str| Err("connection refused".to_string());
        let model = Scripted(std::cell::RefCell::new(vec![]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150, std::time::Duration::from_secs(60));
        assert!(lines.iter().any(|l| l.contains("fetch failed")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// C2 Task 9 fix 1 (R-C2-E22 #2): the events pass has its own wall-clock budget, checked
    /// BEFORE each item starts — the same discipline `enrich_with`'s own `BATCH_BUDGET` loop
    /// uses. A model that sleeps past the budget on its first call must never be handed a second
    /// event: the pass stops there and folds what it did not reach into the existing "left for the
    /// next slot" line. This is what stands between `judge_per_run_cap` (150 sequential calls at
    /// 120 seconds each) and `scheduler::CHILD_TIMEOUT` killing the slot.
    #[test]
    fn the_events_pass_stops_at_its_wall_clock_budget_and_reports_the_remainder() {
        let dir = std::env::temp_dir()
            .join(format!("knowlu-c2-events-budget-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
        std::fs::create_dir_all(dir.join("state")).expect("scratch vault");
        std::fs::write(
            dir.join("config").join("events.yaml"),
            "sources:\n  - name: engage\n    type: ics\n    url: https://example.invalid/e.ics\n    enabled: true\n",
        )
        .expect("write events.yaml");
        let feed = "BEGIN:VCALENDAR\r\n\
             BEGIN:VEVENT\r\nUID:engage:1\r\nSUMMARY:First\r\n\
             DTSTART:20260829T230000Z\r\nDTEND:20260830T000000Z\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:engage:2\r\nSUMMARY:Second\r\n\
             DTSTART:20260830T150000Z\r\nDTEND:20260830T190000Z\r\nEND:VEVENT\r\n\
             END:VCALENDAR\r\n"
            .to_string();
        let fetch = |_: &str| Ok(feed.clone());

        struct Sleepy(std::time::Duration);
        impl crate::judge::EventModel for Sleepy {
            fn judge_event(
                &self,
                _item: &crate::judge::EventItem,
            ) -> Result<crate::judge::EventVerdict, crate::judge::ModelError> {
                std::thread::sleep(self.0);
                Ok(verdict("drop", "sleepy"))
            }
        }
        let model = Sleepy(std::time::Duration::from_millis(300));
        let lines = judge_roster(
            &dir,
            &model,
            Some(&fetch),
            jiff::civil::date(2026, 8, 28),
            150,
            std::time::Duration::from_millis(20),
        );
        let summary = lines.last().expect("a summary line");
        assert!(summary.starts_with("events: 1 judged"), "only the first item should have started: {summary}");
        assert!(summary.contains("1 left"), "the other one must be folded into the existing line: {summary}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
