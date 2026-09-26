use knowlu::commands::state_inner;
use knowlu::scaffold::{campus_yaml, create_vault, ingest_yaml, runners_yaml, VaultPlan, CAMPUSES};
use knowlu::state::ConsoleState;
use std::path::{Path, PathBuf};

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-scaffold-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The plan every scaffold test starts from: no feeds, no mappings, no courses — each test sets the
/// one or two fields it is about. `dest` decides the profile id, exactly as `create_vault_in` does.
fn plan_for(dest: &Path) -> VaultPlan {
    VaultPlan {
        profile_id: knowlu::profiles::id_for(dest),
        ics_url: None,
        personal_calendar: None,
        google_calendar: false,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into(), "18:00".into()],
        device: "M".into(),
        campus: "none".into(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        // The real `api_base()`, not a fixture host: `create_vault_writes_cloud_yaml_beside_the_other_config_files`
        // reads the file back through `cloud_config`, whose R-C1-59 I1 host check now refuses
        // anything else — and this is exactly what `create_vault_in` itself passes in production.
        api_base: knowlu::account::api_base(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    }
}

/// Every `.knowlu-new-*` sitting in `parent` right now. Empty is the only right answer after
/// `create_vault` returns, whichever way it returned.
fn strays(parent: &Path) -> Vec<String> {
    std::fs::read_dir(parent).unwrap().flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with(".knowlu-new-"))
        .collect()
}

/// Every journal line in the vault, as JSON, oldest file first.
fn journal_text(vault: &Path) -> String {
    let mut files: Vec<PathBuf> = std::fs::read_dir(vault.join("state").join("journal")).unwrap()
        .flatten().map(|e| e.path()).collect();
    files.sort();
    files.iter().map(|p| std::fs::read_to_string(p).unwrap().replace("\r\n", "\n")).collect::<Vec<_>>().join("")
}

fn journal_records(vault: &Path) -> Vec<serde_json::Value> {
    journal_text(vault).lines().filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()).collect()
}

/// The same records with the three fields that are *supposed* to differ between two runs removed:
/// the minted note `id` (random), `ts` (the clock) and `seq` (a process counter). What is left is
/// everything a vault's location could possibly have influenced.
fn journal_shape(vault: &Path) -> Vec<serde_json::Value> {
    journal_records(vault).into_iter().map(|mut r| {
        let obj = r.as_object_mut().unwrap();
        for k in ["id", "ts", "seq"] { obj.remove(k); }
        if let Some(n) = obj.get_mut("new").and_then(|n| n.as_object_mut()) { n.remove("id"); }
        r
    }).collect()
}

/// The whole point of the scaffold: a vault that did not exist a second ago RANKS — no crash, and
/// **no `journal has no migration records`**, which is what a friend would otherwise meet on their
/// first slot (decision 5). No network: `run_with`'s two fetcher seams both refuse.
///
/// The warning set is pinned, not merely searched: the only warnings a fresh vault may produce are
/// one per campus source, from the stub. Anything else — a config the engine cannot read, a pass
/// that complains — would be a scaffold defect arriving on a friend's very first run.
#[test]
fn a_scaffolded_vault_ranks_without_the_unmigrated_warning() {
    let root = temp("ranks");
    let v = root.join("Vault");
    let mut p = plan_for(&v);
    p.campus = "university-of-alabama".into();
    // …carrying a seeded course (R-OB-2). A `courses/` note is note-folder content every pass
    // walks, so it has to be as quiet on a fresh vault's first run as the first task is.
    p.courses = vec![knowlu::scaffold::CourseSeed {
        code: "UACS100Fall2026".into(),
        name: "CS 100 Intro to Computer Science".into(),
        slug: "cs-100".into(),
        label: "CS 100".into(),
    }];
    create_vault(&v, &p).unwrap();

    let no_net = |_: &str| -> Result<String, String> { Err("no network in tests".to_string()) };
    let fetchers = knowlu_engine::cli::Fetchers {
        calendar: Some(&no_net as &dyn Fn(&str) -> Result<String, String>),
        events: Some(&no_net as &dyn Fn(&str) -> Result<String, String>),
        ..Default::default()
    };
    let out = knowlu_engine::cli::run_with(&v, Some("2026-09-07"), "local", None, fetchers).expect("rank runs");
    assert!(out.output.is_file(), "state/today.md was written");
    let all = format!("{} {}", out.summary, out.steps.iter().map(|s| s.message.clone()).collect::<Vec<_>>().join(" "));
    assert!(!all.contains("journal has no migration records"), "the unmigrated guard is satisfied from day one: {all}");
    let tasks = out.steps.iter().find(|s| s.name == "tasks").unwrap();
    assert!(tasks.counts.iter().any(|(_, n)| *n >= 1), "the first task is there to order: {:?}", tasks.counts);

    // One warning per preset source, and no other warning anywhere.
    let (cfg, _) = knowlu_engine::events::load_events_config(&v.join("config").join("events.yaml"));
    let events = out.steps.iter().find(|s| s.name == "events").unwrap();
    let lines: Vec<&str> = events.message.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), cfg.sources.len(), "one stubbed fetch failure per source: {:?}", lines);
    for line in &lines {
        assert!(line.contains("fetch failed") && line.contains("no network in tests"), "unexpected event warning: {line}");
    }
    for step in out.steps.iter().filter(|s| s.name != "events") {
        assert!(step.message.trim().is_empty(), "{} warned on a fresh vault: {}", step.name, step.message);
    }
}

/// Decision 3 + spec §3.1: the scaffold is materialised beside the target and renamed in, so a
/// crash never leaves a half-vault. A refused create leaves NOTHING — not the destination, not a
/// staging folder.
#[test]
fn the_scaffold_is_all_or_nothing() {
    let root = temp("atomic");
    let v = root.join("Vault");
    create_vault(&v, &plan_for(&v)).unwrap();
    assert!(v.join("config").join("planning.yaml").is_file());
    // …and the seed is part of the same atomic step, not a second call the caller could skip or
    // fail at (review round 1, Important 2).
    assert!(v.join("archive").join("_migrated.md").is_file());
    assert!(v.join("tasks").join("get-to-know-knowlu.md").is_file());
    assert!(strays(&root).is_empty(), "the staging folder is renamed, never left behind");

    let err = create_vault(&v, &plan_for(&v)).unwrap_err();
    assert!(err.contains("already"), "{err}");
    assert!(strays(&root).is_empty(), "a refused create cleans its staging folder up");
}

/// The refusal above returns before staging exists, so it proves nothing about cleanup. THIS is
/// the real case: `build_into` gets three files onto disk, then a wizard value is rejected. The
/// destination must not appear, the staging folder must not survive, and the error must name what
/// went wrong (review round 1, Important 2).
#[test]
fn a_failure_part_way_through_leaves_nothing() {
    let root = temp("partway");
    let v = root.join("Vault");

    // A control character in a value the engine would read back: caught at `config/ingest.yaml`,
    // by which point planning.yaml, week_template.yaml, .gitignore and events.yaml exist.
    let mut p = plan_for(&v);
    p.timezone = "America/Chi\ncago".into();
    let err = create_vault(&v, &p).unwrap_err();
    assert!(err.contains("timezone") && err.contains("control character"), "{err}");
    assert!(!v.exists(), "no destination is created from a failed materialisation");
    assert!(strays(&root).is_empty(), "the part-built staging folder is removed: {:?}", strays(&root));

    // Same again from the other fallible half of build_into.
    let mut p = plan_for(&v);
    p.campus = "not-a-campus".into();
    let err = create_vault(&v, &p).unwrap_err();
    assert!(err.contains("not-a-campus"), "{err}");
    assert!(!v.exists());
    assert!(strays(&root).is_empty());

    // A real filesystem error, and it carries the path it happened at.
    let blocked = root.join("a-file");
    std::fs::write(&blocked, "not a folder").unwrap();
    let err = create_vault(&blocked.join("Vault"), &plan_for(&blocked.join("Vault"))).unwrap_err();
    assert!(err.contains("a-file"), "an fs error names the path it happened at: {err}");
}

/// Decision 4: a vault born in the wizard is `scheduler: app` on the machine that made it, from
/// birth — and `device:` still gates a second install (F3).
#[test]
fn a_fresh_vault_is_scheduler_app_on_the_machine_that_made_it() {
    let v = temp("runners").join("Vault");
    let mut p = plan_for(&v);
    p.device = "TEST-MACHINE".into();
    create_vault(&v, &p).unwrap();
    let cfg = v.join("config").join("runners.yaml");
    let s = knowlu_engine::runs::runner_settings(&cfg, "local");
    assert_eq!(s.scheduler, knowlu_engine::schedule::SchedulerMode::App);
    assert_eq!(s.device.as_deref(), Some("TEST-MACHINE"));
    let runners = knowlu_engine::runs::load_runners_config(&cfg).unwrap();
    let local = runners.iter().find(|r| r.name == "local").unwrap();
    assert_eq!(local.times, vec!["12:00".to_string(), "18:00".to_string()]);
    assert_eq!(local.tz, "America/Chicago");
    assert_eq!(local.grace_minutes, 20);
}

/// Important 1: a wizard value is DATA, never structure. Every field a friend can type is loaded
/// with YAML metacharacters and every reader still reads exactly what was typed.
///
/// The stake is `config/runners.yaml`: `load_runners_config` turns an unparsable file into
/// `Ok(vec![])` and `runner_settings` into `scheduler: Script`, so a value that broke the file
/// would switch a friend's only runner off in total silence.
#[test]
fn a_value_full_of_yaml_metacharacters_is_data_and_never_structure() {
    let root = temp("quoting");
    let v = root.join("Vault");
    // Not a real timezone, deliberately: the scaffold's job is to write faithfully, and the engine
    // is the one that judges an unknown zone — a value that vanished or reshaped the file could
    // never even be diagnosed.
    let nasty = "Odd/Zone: x #c 'q' \"d\" {e} [f], g";
    let url = "https://lms.example.invalid/f?a=1&b={x}#frag: 'q' \"d\"";
    // m2 (review round 1): the six new value sites go through the same `yaml_scalar`, so they get
    // the same nasty treatment as every other wizard-typed field — a course-map fragment as a KEY
    // (`course_map` writes `fragment: slug`) and a course/label as VALUES.
    let nasty_fragment = "CS 100: x #c 'q'";
    let p = VaultPlan {
        profile_id: "profile_8888888888".into(),
        ics_url: Some(url.to_string()),
        personal_calendar: None,
        google_calendar: false,
        timezone: nasty.to_string(),
        slots: vec!["12:00: x #c 'q'".into(), "18:00 {b} \"d\"".into()],
        device: "DESK: TOP #1 'q' \"d\" {z}".into(),
        campus: "university-of-alabama".into(),
        campus_choice: Default::default(),
        zybooks: true,
        vhl: true,
        zybooks_courses: vec![knowlu::scaffold::BookMapping {
            code: "UACS100Fall2026".into(),
            course: "cs-100: x #c 'q'".into(),
            label: "CS 100: x #c 'q' \"d\" {e} [f], g".into(),
        }],
        vhl_sections: vec![knowlu::scaffold::SectionMapping {
            section: "2102121".into(),
            course: "gn-103: x #c 'q'".into(),
            label: "GN 103 Hausaufgaben: x #c 'q' \"d\" {e} [f], g".into(),
        }],
        zybooks_ignore: vec!["HowToUseZyBooks2".into(), "Odd: Book #c 'q'".into()],
        course_map: vec![(nasty_fragment.into(), "cs-100: x #c 'q'".into())],
        courses: Vec::new(),
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        // C3' Task 11: no account, so `ics_url` still round-trips literally below — this test is
        // about YAML quoting fidelity, not about the account-vs-no-account routing
        // `a_vault_with_an_account_carries_no_capability_url` covers.
        account_id: String::new(),
    };
    create_vault(&v, &p).unwrap();
    let cfg = v.join("config").join("runners.yaml");

    let runners = knowlu_engine::runs::load_runners_config(&cfg).unwrap();
    let local = runners.iter().find(|r| r.name == "local").expect("runners.yaml still parses");
    assert_eq!(local.tz, nasty);
    assert_eq!(local.times, p.slots, "each slot survives whole, commas and colons included");
    assert_eq!(local.grace_minutes, 20);
    let s = knowlu_engine::runs::runner_settings(&cfg, "local");
    assert_eq!(s.scheduler, knowlu_engine::schedule::SchedulerMode::App, "the runner is NOT silently switched off");
    assert_eq!(s.device.as_deref(), Some(p.device.as_str()));

    // The campus preset is untouched by any of it.
    let (ev, warnings) = knowlu_engine::events::load_events_config(&v.join("config").join("events.yaml"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(ev.sources.len(), 6);

    // `ics_url` round-trips byte for byte — the engine fetches exactly what was pasted.
    let text = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("ingest.yaml still parses");
    assert_eq!(parsed.get("ics_url").and_then(|u| u.as_str()), Some(url));
    assert_eq!(parsed.get("timezone").and_then(|t| t.as_str()), Some(nasty));
    assert_eq!(
        knowlu::scheduler::ics_state(&v),
        knowlu::scheduler::IcsState::Feed,
        "the slot still knows there is a feed"
    );
    let cw = parsed.get("coursework").expect("the coursework block survives");
    assert_eq!(cw.get("zybooks").and_then(|z| z.get("credential_target")).and_then(|t| t.as_str()), Some("knowlu/profile_8888888888/zybooks"));
    assert_eq!(cw.get("vhl").and_then(|z| z.get("credential_target")).and_then(|t| t.as_str()), Some("knowlu/profile_8888888888/vhl"));

    // m2: a student-typed label carrying YAML metacharacters round-trips through the parsed-back
    // mapping unchanged — key AND value, for all six new value sites.
    let zy_course = cw.get("zybooks").and_then(|z| z.get("courses")).and_then(|c| c.get("UACS100Fall2026")).expect("the nasty zybook entry parses");
    assert_eq!(zy_course.get("course").and_then(|c| c.as_str()), Some("cs-100: x #c 'q'"));
    assert_eq!(zy_course.get("label").and_then(|c| c.as_str()), Some(p.zybooks_courses[0].label.as_str()));
    let vhl_section = cw.get("vhl").and_then(|z| z.get("sections")).and_then(|s| s.get("2102121")).expect("the nasty section parses");
    assert_eq!(vhl_section.get("course").and_then(|c| c.as_str()), Some("gn-103: x #c 'q'"));
    assert_eq!(vhl_section.get("label").and_then(|c| c.as_str()), Some(p.vhl_sections[0].label.as_str()));
    let ignore: Vec<&str> = cw.get("zybooks").and_then(|z| z.get("ignore")).and_then(|i| i.as_sequence()).expect("ignore parses")
        .iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(ignore, vec!["HowToUseZyBooks2", "Odd: Book #c 'q'"]);
    let course_map = parsed.get("course_map").and_then(|m| m.get(nasty_fragment)).and_then(|s| s.as_str());
    assert_eq!(course_map, Some("cs-100: x #c 'q'"));

    // A control character cannot be quoted onto one line, so it is refused by field name rather
    // than written out and silently breaking the file.
    for (field, mut bad) in [
        ("timezone", plan_for(&root.join("V-timezone"))),
        ("device name", plan_for(&root.join("V-device name"))),
        ("slot 2", plan_for(&root.join("V-slot 2"))),
        ("LMS feed URL", plan_for(&root.join("V-LMS feed URL"))),
    ] {
        match field {
            "timezone" => bad.timezone = "America/\u{7}Chicago".into(),
            "device name" => bad.device = "DESK\u{1}TOP".into(),
            "slot 2" => bad.slots = vec!["12:00".into(), "18:\u{9}00".into()],
            // C3' Task 11: an account vault never writes `ics_url` at all (H11b), so there is
            // nothing here for `yaml_scalar` to validate unless the plan has no account — the
            // no-account fallback is where a bad LMS feed URL is still caught.
            _ => { bad.account_id = String::new(); bad.ics_url = Some("https://x.invalid/\u{b}a.ics".into()); }
        }
        let err = create_vault(&root.join(format!("V-{field}")), &bad).unwrap_err();
        assert!(err.contains(field) && err.contains("control character"), "{field}: {err}");
    }
    // The two generators say the same thing on their own, so a future caller cannot route round it.
    assert!(ingest_yaml(&{ let mut b = plan_for(&root.join("V-ingest-err")); b.timezone = "a\rb".into(); b }).is_err());
    assert!(runners_yaml(&{ let mut b = plan_for(&root.join("V-runners-err")); b.device = "a\rb".into(); b }).is_err());
}

/// The campus preset is a file, and the file is the shape `load_events_config` already reads —
/// so adding a campus is adding a file, and never a code change (spec §3, panel 6).
#[test]
fn the_campus_preset_is_the_shape_the_engine_already_reads() {
    let v = temp("campus").join("Vault");
    let mut p = plan_for(&v);
    p.campus = "university-of-alabama".into();
    create_vault(&v, &p).unwrap();
    let (cfg, warnings) = knowlu_engine::events::load_events_config(&v.join("config").join("events.yaml"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(cfg.sources.len(), 6, "the six UA sources");
    assert_eq!(CAMPUSES.len(), 2);
    assert!(campus_yaml("none").unwrap().contains("sources: []"));
    assert!(campus_yaml("not-a-campus").is_none());
    // Every curated school resolves, through `events_preset_for`, to a preset file — `none` for
    // `university-of-kentucky` until someone writes its preset, and a preset with no file behind it
    // would be a wizard that fails at Finish.
    for c in CAMPUSES {
        let preset = knowlu::scaffold::events_preset_for(c.unitid);
        assert!(campus_yaml(preset).is_some(), "{} ({}) resolves to preset {preset:?} with no file", c.key, c.label);
    }
    assert!(campus_yaml("university-of-alabama").unwrap().contains("timezone:"), "the preset tells a copier to set its own timezone");

    let none = temp("campus-none").join("Vault");
    let mut p = plan_for(&none); p.campus = "none".into();
    create_vault(&none, &p).unwrap();
    let (cfg, warnings) = knowlu_engine::events::load_events_config(&none.join("config").join("events.yaml"));
    assert!(warnings.is_empty() && cfg.sources.is_empty());
}

/// I2 (final review): **the read model over a vault the wizard has just made.** The console's first
/// poll happens seconds after *Finish*, on a vault where nothing has ever run: no `state/today.md`,
/// no `state/runs/`, no `state/runner-log.md`, not even a git repository. Every view the console
/// serves has to answer on it — a view that refused would be a friend's first screen showing an
/// error toast over an empty column, on a vault with nothing wrong with it.
///
/// Every name `surface::View::parse` accepts, so a view added there without a thought for an empty
/// vault fails here rather than in front of someone.
#[test]
fn every_view_answers_over_a_vault_the_wizard_has_just_made() {
    let root = temp("readmodel");
    let v = root.join("Vault");
    create_vault(&v, &plan_for(&v)).unwrap();
    // The things a rank, a slot and a sync leave behind — none of them exist yet.
    assert!(!v.join("state").join("today.md").exists(), "nothing has ranked this vault");
    assert!(!v.join("state").join("runs").exists(), "no run records");
    assert!(!v.join("state").join("runner-log.md").exists(), "no runner log");
    assert!(!v.join(".git").exists(), "and it is not a repository");

    let cs = ConsoleState::open(v.clone(), root.join("appdata"));
    for view in ["today", "overdue", "week", "later", "all", "decisions", "good-to-know", "issues", "runs"] {
        let env = state_inner(&cs, view).unwrap_or_else(|e| panic!("{view}: {e}"));
        assert_eq!(env["ok"], true, "{view}: {env}");
        assert_eq!(env["state"]["schema"], 1, "{view}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// R2 + decision 5: exactly ONE `system:migration` record, and every other write the wizard makes
/// is an ordinary `quinn`/`dashboard` one. An adopted vault gets neither.
#[test]
fn one_migration_record_and_the_rest_are_dashboard_writes() {
    let v = temp("journal").join("Vault");
    let mut p = plan_for(&v);
    p.zybooks = true;
    create_vault(&v, &p).unwrap();
    let records = journal_records(&v);
    let migration: Vec<_> = records.iter().filter(|r| r["actor"] == "system:migration").collect();
    assert_eq!(migration.len(), 1, "exactly one, and it is a create");
    assert_eq!(migration[0]["op"], "create");
    assert_eq!(migration[0]["via"], "cli", "the S1 migration's own shape — the via vocabulary does not grow");
    assert!(migration[0]["path"].as_str().unwrap().contains("archive/_migrated.md"));
    let others: Vec<_> = records.iter().filter(|r| r["actor"] != "system:migration").collect();
    assert_eq!(others.len(), 1, "the first task, and nothing else");
    assert_eq!(others[0]["actor"], "quinn");
    assert_eq!(others[0]["via"], "dashboard");
    // The credential target the engine will read is named, and holds no secret.
    let ingest = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    let target = knowlu::credentials::target_for(&knowlu::profiles::id_for(&v), "zybooks");
    assert!(ingest.contains(&format!("credential_target: '{target}'")), "{ingest}");
    assert!(!ingest.contains("vhl:"), "a friend with no VHL course gets no VHL block");
}

/// Important 2: seeding happens in the staging folder, so the journal the rename carries into
/// `dest` must be the journal seeding `dest` directly would have written. It is, because a record
/// names its note by a VAULT-RELATIVE path (`write::create` → `ids::rel`) and nothing else in a
/// record depends on where the vault sits.
///
/// Proved two ways: two vaults built at different destinations, through differently-named staging
/// folders, produce identical records once the three fields that are meant to differ (the minted
/// `id`, `ts`, `seq`) are removed; and neither journal contains an absolute path or the staging
/// folder's name at all.
#[test]
fn the_move_does_not_leak_the_staging_path_into_the_journal() {
    let root = temp("relocate");
    let a = root.join("One");
    let b = root.join("deeper").join("Two");
    create_vault(&a, &plan_for(&a)).unwrap();
    create_vault(&b, &plan_for(&b)).unwrap();
    assert_eq!(journal_shape(&a), journal_shape(&b), "a record does not depend on where the vault sits");

    for v in [&a, &b] {
        let text = journal_text(v);
        assert!(!text.contains("knowlu-new"), "the staging folder's name reached the journal: {text}");
        assert!(!text.contains(":\\\\"), "an absolute path reached the journal: {text}");
        let paths: Vec<String> = journal_records(v).iter().map(|r| r["path"].as_str().unwrap().to_string()).collect();
        assert_eq!(paths, vec!["archive/_migrated.md".to_string(), "tasks/get-to-know-knowlu.md".to_string()]);
        // Written through `ledger::dumps_value`, and the move did not rewrite a byte of it.
        assert!(text.contains("\", \"") && text.contains("\": \""), "Python's json.dumps separators: {text}");
    }
}

/// Nothing shipped inside the binary may carry a live feed, a token or a personal calendar. The
/// campus preset is public campus URLs and nothing else; `config/ingest.yaml`'s own `ics_url:` and
/// `calendars:` lines must never be copied into an asset (spec §5).
#[test]
fn no_embedded_asset_carries_a_live_feed_or_a_secret() {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out) } else { out.push(p) }
        }
    }
    let mut files = Vec::new();
    walk(Path::new("assets"), &mut files);
    assert!(files.len() >= 5, "the scaffold's assets are there to check: {files:?}");
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        for needle in ["ualearn", "calendarFeed", "calendar.google.com/calendar/ical", "webcal:", "password", "token="] {
            assert!(!text.contains(needle), "{} carries {needle:?}", f.display());
        }
    }
}

#[test]
fn a_new_vault_carries_the_four_cloud_keys_and_no_secret() {
    use knowlu::scaffold::{cloud_yaml, VaultPlan};
    let p = VaultPlan {
        profile_id: "profile_0123456789".into(),
        ics_url: None,
        personal_calendar: None,
        google_calendar: false,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into(), "18:00".into()],
        device: "MACHINE".into(),
        campus: "none".into(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "a-public-anon-key".into(),
        account_id: "11111111-2222-3333-4444-555555555555".into(),
    };
    let text = cloud_yaml(&p).expect("cloud.yaml");
    // The C2 contract, in its order, single-line single-quoted scalars.
    assert_eq!(
        text,
        "api_base: 'https://example.supabase.co/functions/v1'\n\
         anon_key: 'a-public-anon-key'\n\
         session_credential_target: 'knowlu/profile_0123456789/session'\n\
         account_id: '11111111-2222-3333-4444-555555555555'\n"
    );
    // The session is NAMED, never carried — the same promise `ingest.yaml` makes about a password.
    assert!(!text.contains("access_token") && !text.contains("refresh_token"));
    // …and a value that would change the file's SHAPE is refused by field name, as everywhere else.
    let mut bad = p.clone();
    bad.account_id = "acc\nid".into();
    assert!(cloud_yaml(&bad).unwrap_err().contains("account_id"));
}

#[test]
fn a_personal_calendar_becomes_the_engines_calendars_list() {
    use knowlu::scaffold::ingest_yaml;
    // m7: a real temp path, not a bare relative one — this test never reads the derived id, but a
    // throwaway path under `temp_dir()` reads as deliberate rather than a stray.
    let mut base = plan_for(&std::env::temp_dir().join("knowlu-personal-calendar-vault"));
    // C3' Task 11: this test is about the calendar-entry shape, not about accounts — an account vault
    // routes the same address through `cloud:personal` instead, and that half is
    // `a_vault_with_an_account_carries_no_capability_url`'s job below.
    base.account_id = String::new();
    // No calendar: the list the engine has always read, empty.
    assert!(ingest_yaml(&base).unwrap().contains("calendars: []\n"));
    // One: the shape `calfeed::load_calendar_events` parses — a list of {name, ics_url} mappings.
    let mut with = base.clone();
    with.personal_calendar = Some("https://calendar.google.com/calendar/ical/x/private-def/basic.ics".into());
    let text = ingest_yaml(&with).unwrap();
    assert!(
        text.contains("calendars:\n  - name: personal\n    ics_url: 'https://calendar.google.com/calendar/ical/x/private-def/basic.ics'\n"),
        "{text}"
    );
    // …and a value that would change the file's shape is refused by field name, as everywhere else.
    let mut bad = base.clone();
    bad.personal_calendar = Some("https://a\nb".into());
    assert!(ingest_yaml(&bad).unwrap_err().contains("personal calendar address"));
}

/// H9 (a3), §11a: the Google grant is a flag, not a URL, and the marker it writes is `cloud:google`
/// — the `cloud:` prefix hand-off H4's fetcher matches and routes to `/ingest-calendar?name=google`.
/// Personal comes first when both are present, matching the fixed order `ingest_yaml` writes them in.
#[test]
fn a_google_grant_becomes_the_engines_calendars_list() {
    use knowlu::scaffold::ingest_yaml;
    let mut base = plan_for(&std::env::temp_dir().join("knowlu-google-calendar-vault"));
    // C3' Task 11: the google marker is `cloud:google` with or without an account (§11a's own
    // decision, unconditional), so only the personal-address half of this test needs the no-account
    // plan to keep asserting the literal URL it always has.
    base.account_id = String::new();
    // Google alone, no personal address: one entry, the marker feed.
    let mut google_only = base.clone();
    google_only.google_calendar = true;
    assert!(
        ingest_yaml(&google_only).unwrap().contains("calendars:\n  - name: google\n    ics_url: 'cloud:google'\n"),
        "{}", ingest_yaml(&google_only).unwrap()
    );
    // Both, WITH the account a Google grant always implies (Task 11 review, M1: production cannot
    // reach a grant with no account, so a no-account "both" case would pass a reorder of the two
    // `entries.push` calls that a real vault would never survive). Personal first, google second —
    // the fixed order the writer promises, both routed through `cloud:` because the account holds
    // both.
    let mut both = base.clone();
    both.account_id = "acc-1".into();
    both.personal_calendar = Some("https://calendar.google.com/calendar/ical/x/private-def/basic.ics".into());
    both.google_calendar = true;
    let text = ingest_yaml(&both).unwrap();
    assert!(
        text.contains("calendars:\n  - name: personal\n    ics_url: 'cloud:personal'\n  - name: google\n    ics_url: 'cloud:google'\n"),
        "{text}"
    );
}

#[test]
fn a_vault_with_an_account_carries_no_capability_url() {
    // C3', and §9's Alabama SPII line: a capability URL is a credential in all but name, and the one
    // place it belongs is the account, encrypted, where `PUT /account/sources` already puts it.
    let dest = temp("no-capability-url");
    let mut plan = plan_for(&dest);
    plan.account_id = "acc-1".into();   // `plan_for`'s own default; named here because it is the point
    plan.ics_url = Some("https://lms.example.invalid/feed/secret-capability.ics".into());
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(!yaml.contains("secret-capability"), "{yaml}");
    assert!(!yaml.contains("private-abc"), "{yaml}");
    assert!(yaml.contains("ics_url: ''"), "the key stays, empty, so `ingest` still parses it: {yaml}");
    assert!(yaml.contains("- name: personal\n    ics_url: 'cloud:personal'"), "{yaml}");
}

#[test]
fn a_vault_with_no_account_still_carries_its_own_urls() {
    // A friend who has not signed in still has nowhere else to keep the feed. Nothing about that
    // path changes, and the fallback in `ingest.rs` is what reads it.
    let dest = temp("keeps-its-urls");
    let mut plan = plan_for(&dest);
    plan.account_id = String::new();
    plan.ics_url = Some("https://lms.example.invalid/feed/secret-capability.ics".into());
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(yaml.contains("secret-capability"), "{yaml}");
    assert!(yaml.contains("private-abc"), "{yaml}");
}

#[test]
fn create_vault_writes_cloud_yaml_beside_the_other_config_files() {
    use knowlu::scaffold::create_vault;
    let root = std::env::temp_dir().join(format!("knowlu-scaffold-cloud-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("Fall 2026");
    let p = plan_for(&dest);
    create_vault(&dest, &p).expect("create");
    let text = knowlu_engine::pystr::read_text(&dest.join("config").join("cloud.yaml")).expect("read");
    assert!(text.contains("account_id: 'acc-1'"), "{text}");
    // …and the account is readable through the same door C2 will use, not by string matching.
    let cfg = knowlu::account::cloud_config(&dest).expect("cloud_config");
    assert_eq!(cfg.account_id, "acc-1");
    assert_eq!(cfg.session_credential_target, format!("knowlu/{}/session", p.profile_id));
    // A second write is refused rather than silently repointing the vault at another account.
    assert!(knowlu::scaffold::write_cloud_yaml_if_absent(&dest, &p).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

/// Fix round 1, item 5: Task 18 adopts a vault that may never have had a `config/` folder at all
/// (an old, pre-`cloud.yaml` layout is still just a folder with `tasks/` and `planning.yaml` — the
/// console's own predicate never required `config/` to hold anything else). `write_cloud_yaml_if_absent`
/// must make the folder itself rather than fail with a raw "the system cannot find the path".
#[test]
fn write_cloud_yaml_if_absent_creates_the_config_folder_first() {
    use knowlu::scaffold::write_cloud_yaml_if_absent;
    let vault = std::env::temp_dir().join(format!("knowlu-cloud-yaml-noconfig-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&vault);
    std::fs::create_dir_all(&vault).unwrap();
    assert!(!vault.join("config").exists(), "the vault has no config/ folder yet");
    let p = plan_for(&vault);
    write_cloud_yaml_if_absent(&vault, &p).expect("creates config/ and the file");
    assert!(vault.join("config").join("cloud.yaml").is_file());
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn a_zybook_code_suggests_the_course_it_obviously_is() {
    use knowlu::scaffold::suggest_course;
    use knowlu_engine::ingest::slugify;
    // The real one, from Quinn's own account: an institution prefix, a code, a term.
    assert_eq!(suggest_course("UACS100Fall2026").as_deref(), Some("CS 100"));
    assert_eq!(suggest_course("CS200Spring2027").as_deref(), Some("CS 200"));
    assert_eq!(suggest_course("UAMATH125Fall2026").as_deref(), Some("MATH 125"));
    // No code in it at all: the panel shows the raw name and the student types the course.
    assert_eq!(suggest_course("HowToUseZyBooks2"), None);
    assert_eq!(suggest_course(""), None);
    // The engine's, not a twin: this is the function that decides the note's stem and the key
    // `judge::Heuristics::knows_course` matches, and a second one would diverge in silence.
    assert_eq!(slugify("CS 100"), "cs-100");
    assert_eq!(slugify("GN 103 Hausaufgaben"), "gn-103-hausaufgaben");
    // …including the two behaviours a naive twin gets wrong: the 60-character cap, and a fallback
    // that is never the empty string (which would write `courses/.md`).
    assert_eq!(slugify("!!!"), "item");
    assert_eq!(slugify(&"x".repeat(80)).len(), 60);
}

/// D4: one course code, one slug. Blackboard names a course by term, department, number and
/// section (`202640-BUI-100-101`), which `suggest_course` refuses because of the dash — so the
/// vault used to hold `courses/202640-bui-100-101.md` beside a typed `BUI 100` that could never
/// meet it. The name's own code is read, with no institution-prefix peel: the letters immediately
/// before the number are the subject, and there is nothing glued in front of them to peel.
#[test]
fn a_course_name_gives_up_the_code_its_school_wrote_into_it() {
    use knowlu::scaffold::course_code_in_name;
    use knowlu_engine::ingest::slugify;
    assert_eq!(course_code_in_name("202640-BUI-100-101").as_deref(), Some("BUI 100"));
    assert_eq!(course_code_in_name("MATH-125-001").as_deref(), Some("MATH 125"));
    assert_eq!(course_code_in_name("CS 100 Intro to Computer Science").as_deref(), Some("CS 100"));
    assert_eq!(course_code_in_name("PSYC 101H Honors").as_deref(), Some("PSYC 101H"));
    // No code in it at all, and never a guess: an invented fragment matches somebody else's course.
    assert_eq!(course_code_in_name("Biology"), None);
    assert_eq!(course_code_in_name(""), None);
    // A number that is not three digits is not a course number.
    assert_eq!(course_code_in_name("MATH-1250-001"), None);
    // The glued form is `suggest_course`'s, not this one's — reading it here would peel nothing
    // and answer `UACS 100`, which is nobody's course.
    assert_eq!(course_code_in_name("UACS100Fall2026"), None);
    assert_eq!(slugify("BUI 100"), "bui-100");
}

/// …and the note that course gets is titled by the code, keeps the school's own name, and keeps the
/// LMS's key: the title is what a student reads, `name:` is what their LMS calls it, and `code:` is
/// the fragment `ingest::match_course` matches a UID against.
#[test]
fn a_seeded_course_note_is_titled_by_its_code_and_keeps_the_lms_name() {
    let root = temp("coursenote");
    let v = root.join("Vault");
    let mut p = plan_for(&v);
    p.courses = vec![
        knowlu::scaffold::CourseSeed {
            code: "_404752_1".into(),
            name: "202640-BUI-100-101".into(),
            slug: "bui-100".into(),
            label: "BUI 100".into(),
        },
        // No readable code anywhere: the label is empty and the note keeps today's behaviour.
        knowlu::scaffold::CourseSeed {
            code: "Independent Study".into(),
            name: "Independent Study".into(),
            slug: "independent-study".into(),
            label: String::new(),
        },
    ];
    create_vault(&v, &p).unwrap();
    let note = std::fs::read_to_string(v.join("courses").join("bui-100.md")).unwrap().replace("\r\n", "\n");
    assert!(note.contains("title: BUI 100\n"), "{note}");
    assert!(note.contains("name: 202640-BUI-100-101\n"), "{note}");
    assert!(note.contains("code: _404752_1\n"), "{note}");
    assert!(note.contains("slug: bui-100\n"), "{note}");
    // A course whose id and name both carry no readable code keeps today's behaviour — the name.
    let plain = std::fs::read_to_string(v.join("courses").join("independent-study.md")).unwrap().replace("\r\n", "\n");
    assert!(plain.contains("title: Independent Study\n"), "{plain}");
    assert!(plain.contains("name: Independent Study\n"), "{plain}");
}

#[test]
fn a_confirmed_mapping_becomes_the_config_the_engine_reads() {
    use knowlu::scaffold::{ingest_yaml, BookMapping, SectionMapping, VaultPlan};
    let mut p = VaultPlan {
        profile_id: "profile_0123456789".into(),
        ics_url: None,
        personal_calendar: None,
        google_calendar: false,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into()],
        device: "M".into(),
        campus: "none".into(),
        campus_choice: Default::default(),
        zybooks: true,
        vhl: true,
        zybooks_courses: vec![BookMapping { code: "UACS100Fall2026".into(), course: "cs-100".into(), label: "CS 100".into() }],
        vhl_sections: vec![SectionMapping { section: "2102121".into(), course: "gn-103".into(), label: "GN 103 Hausaufgaben".into() }],
        zybooks_ignore: vec!["HowToUseZyBooks2".into()],
        course_map: vec![("CS 100".into(), "cs-100".into()), ("GN 103".into(), "gn-103".into())],
        courses: Vec::new(),
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    };
    let text = ingest_yaml(&p).expect("ingest.yaml");

    // The three things the engine actually reads, in the shape `route_zybook` and
    // `vhl::parse_dashboard` expect — a non-empty mapping under the code, with `course` and `label`.
    assert!(text.contains("    courses:\n      'UACS100Fall2026':\n        course: 'cs-100'\n        label: 'CS 100'\n"), "{text}");
    assert!(text.contains("    sections:\n      '2102121':\n        course: 'gn-103'\n        label: 'GN 103 Hausaufgaben'\n"), "{text}");
    // zyBooks' own onboarding book has zero assignments and is never coursework. In `ignore:` it is
    // skipped silently; out of it, it is a WARN on every healthy run forever.
    assert!(text.contains("    ignore:\n      - 'HowToUseZyBooks2'\n"), "{text}");
    // …and the blocks `parse_assignments` needs, or every zyBooks item is uncategorised.
    for needed in ["    categories:\n      HW: hw\n", "      minutes_per_section: 6\n", "    importance:\n      hw: 2\n"] {
        assert!(text.contains(needed), "missing {needed:?} in {text}");
    }
    // The course map the ICS ingest and tier-1 judgment both read.
    assert!(text.contains("course_map:\n  'CS 100': 'cs-100'\n  'GN 103': 'gn-103'\n"), "{text}");

    // An empty mapping is an EMPTY block, not `courses: {}` with nothing under it — `route_zybook`
    // treats a falsy mapping as unmapped either way, but a config that lies about what it maps is
    // what produced the first slot this task exists because of.
    p.zybooks_courses.clear();
    p.vhl_sections.clear();
    p.course_map.clear();
    let bare = ingest_yaml(&p).expect("ingest.yaml");
    assert!(bare.contains("    courses: {}\n") && bare.contains("    sections: {}\n"), "{bare}");
    assert!(bare.contains("course_map: {}\n"), "{bare}");
}

/// Review round 1, I1 (part 2): the emitter is unable to produce an unreadable file. This builds a
/// `VaultPlan` directly with two mappings for the SAME zyBook code — bypassing `create_vault_in`'s
/// de-duplication on purpose, to prove the OTHER half of the fix: `build_into` now parses its own
/// `config/ingest.yaml` back through the engine's own loader before the wizard is allowed to
/// finish, so a duplicate key (which `serde_yaml_ng`'s `Mapping` deserializer refuses) is a loud
/// refusal here rather than a vault that silently never syncs.
#[test]
fn a_duplicate_zybook_key_is_refused_rather_than_written_unparsable() {
    let v = temp("dup-key").join("Vault");
    let mut p = plan_for(&v);
    p.zybooks = true;
    p.zybooks_ignore = vec!["HowToUseZyBooks2".into()];
    p.zybooks_courses = vec![
        knowlu::scaffold::BookMapping { code: "UACS100Fall2026".into(), course: "cs-100".into(), label: "CS 100".into() },
        knowlu::scaffold::BookMapping { code: "UACS100Fall2026".into(), course: "cs-100-again".into(), label: "CS 100, again".into() },
    ];
    let err = create_vault(&v, &p).unwrap_err();
    assert!(err.contains("would not parse"), "{err}");
    assert!(!v.exists(), "no half-made vault is left behind");
}

/// R-OB-2: one note per enrolled course, in the shape `judge::Heuristics::load` reads — the stem is
/// the slug, the frontmatter carries `title` and `slug`, and the `## Grade weights` heading is there
/// and empty, because the weights are the student's to write and the model's to read.
/// D4: the note is titled by the human code the capture read and `name:` carries the LMS's own name.
#[test]
fn every_enrolled_course_becomes_a_note_the_engine_can_find() {
    use knowlu::scaffold::{create_vault, CourseSeed};
    let root = std::env::temp_dir().join(format!("knowlu-courses-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("Fall 2026");
    let mut p = plan_for(&dest);          // the helper the other scaffold tests already use
    p.courses = vec![
        CourseSeed { code: "CS 100".into(), name: "CS 100 Intro to Computer Science".into(), slug: "cs-100".into(), label: "CS 100".into() },
        CourseSeed { code: "GN 103".into(), name: "GN 103 German".into(), slug: "gn-103".into(), label: "GN 103".into() },
    ];
    p.course_map = vec![("CS 100".into(), "cs-100".into()), ("GN 103".into(), "gn-103".into())];
    create_vault(&dest, &p).expect("create");

    // D4: the note is titled by the human code, and `name:` carries the school's own name.
    for (slug, title, name) in [
        ("cs-100", "CS 100", "CS 100 Intro to Computer Science"),
        ("gn-103", "GN 103", "GN 103 German"),
    ] {
        let note = dest.join("courses").join(format!("{slug}.md"));
        let text = knowlu_engine::pystr::read_text(&note).unwrap_or_else(|e| panic!("{}: {e}", note.display()));
        assert!(text.contains(&format!("title: {title}")), "{text}");
        assert!(text.contains(&format!("name: {name}")), "{text}");
        assert!(text.contains(&format!("slug: {slug}")), "{text}");
        assert!(text.contains("## Grade weights"), "{text}");
        // Every note has an opaque id, like every other note this app writes.
        assert!(text.contains("id: course_"), "{text}");
    }
    // …and the engine agrees it knows them: this is the predicate tier-1 judgment uses.
    let h = knowlu_engine::judge::Heuristics::load(&dest);
    assert!(h.knows_course("cs-100") && h.knows_course("gn-103"));
    let _ = std::fs::remove_dir_all(&root);
}

/// **R-C1-48: two fragments per captured course, pointing at the same slug.** `course_map` is matched
/// literally — a case-sensitive substring of the UID, then a case-insensitively bounded run in the
/// SUMMARY (`ingest::contains_bounded`) — and the UA feed's summaries do not lead with a `CS 100`
/// code the way its UIDs carry the LMS's own course id. So a captured course contributes **both**
/// spellings: the id the feed pins by, and the human code a summary might say. A fragment the feed
/// never carries simply never matches; a fragment nobody wrote is 28 tasks with no course.
#[test]
fn a_captured_course_maps_by_its_lms_id_and_by_the_code_a_summary_spells() {
    use knowlu::scaffold::CourseSeed;
    use knowlu_engine::ingest::match_course_fields;
    let dest = temp("captured-courses").join("Fall 2026");
    let mut p = plan_for(&dest);
    p.courses = vec![
        CourseSeed { code: "UACS100Fall2026".into(), name: "CS 100 Intro to Computer Science".into(), slug: "cs-100".into(), label: "CS 100".into() },
        // The lab section of the same course: a second LMS id under the SAME slug. Both ids become
        // fragments, and the note is written once — a student enrolled in a lecture and its lab must
        // not be a wizard that refuses to make a vault.
        CourseSeed { code: "UACS100LFall2026".into(), name: "CS 100 Lab".into(), slug: "cs-100".into(), label: "CS 100".into() },
        // …and a course whose id carries no readable code at all: the human half comes off the name.
        CourseSeed { code: "202610-GN-103-001".into(), name: "GN 103 German".into(), slug: "gn-103".into(), label: "GN 103".into() },
    ];
    let text = ingest_yaml(&p).expect("ingest.yaml");
    for line in [
        "  'UACS100Fall2026': 'cs-100'\n",
        "  'UACS100LFall2026': 'cs-100'\n",
        "  'CS 100': 'cs-100'\n",
        "  '202610-GN-103-001': 'gn-103'\n",
        "  'GN 103': 'gn-103'\n",
    ] {
        assert!(text.contains(line), "missing {line:?} in {text}");
    }
    // Two courses whose human code is the same spelling write ONE key: a duplicate key is a config
    // `serde_yaml_ng` refuses whole, which is every book and section "not in config; skipped".
    assert_eq!(text.matches("'CS 100':").count(), 1, "{text}");

    // …and a mapping the student confirmed on the panel is the same key, still written once — first
    // wins, so what they typed decides the slug and the capture never overwrites it.
    p.course_map = vec![("CS 100".into(), "cs-100".into())];
    let text = ingest_yaml(&p).expect("ingest.yaml");
    assert_eq!(text.matches("'CS 100':").count(), 1, "{text}");

    create_vault(&dest, &p).expect("create");
    assert!(dest.join("courses").join("cs-100.md").is_file());
    assert!(dest.join("courses").join("gn-103.md").is_file());
    assert_eq!(std::fs::read_dir(dest.join("courses")).unwrap().count(), 2, "one note per slug");

    // The proof is the engine's own matcher, over the two things a feed actually carries.
    let h = knowlu_engine::judge::Heuristics::load(&dest);
    assert_eq!(match_course_fields("UACS100Fall2026-abc-123", "Homework 4", &h.course_map).as_deref(), Some("cs-100"));
    assert_eq!(match_course_fields("UACS100LFall2026-lab-1", "Lab 2", &h.course_map).as_deref(), Some("cs-100"));
    assert_eq!(match_course_fields("nothing-in-here", "CS 100 Homework 4 is due", &h.course_map).as_deref(), Some("cs-100"));
    assert_eq!(match_course_fields("nothing-in-here", "GN 103 Hausaufgaben", &h.course_map).as_deref(), Some("gn-103"));
    // …and nothing it does not name: `STATISTICS-100` is not `CS 100`.
    assert_eq!(match_course_fields("nothing-in-here", "STATISTICS 100 reading", &h.course_map), None);
    let _ = std::fs::remove_dir_all(dest.parent().expect("the scratch folder"));
}

/// **Step 4a: every `course:` a coursework mapping names must be a slug the vault knows** — a seeded
/// course note, or a `course_map` target. Two panels fill these, and a slug that matches nothing is a
/// task filed under a course that does not exist.
#[test]
fn every_mapped_course_is_a_slug_the_vault_knows() {
    use knowlu::scaffold::{create_vault, BookMapping, CourseSeed, SectionMapping};
    let root = std::env::temp_dir().join(format!("knowlu-mapped-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("Fall 2026");
    let mut p = plan_for(&dest);
    p.zybooks = true;
    p.vhl = true;
    p.zybooks_courses = vec![BookMapping { code: "UACS100Fall2026".into(), course: "cs-100".into(), label: "CS 100".into() }];
    p.vhl_sections = vec![SectionMapping { section: "2102121".into(), course: "gn-103".into(), label: "GN 103".into() }];
    p.courses = vec![CourseSeed { code: "CS 100".into(), name: "CS 100 Intro".into(), slug: "cs-100".into(), label: "CS 100".into() }];
    p.course_map = vec![("CS 100".into(), "cs-100".into()), ("GN 103".into(), "gn-103".into())];
    create_vault(&dest, &p).expect("create");

    let h = knowlu_engine::judge::Heuristics::load(&dest);
    for slug in p.zybooks_courses.iter().map(|b| &b.course).chain(p.vhl_sections.iter().map(|v| &v.course)) {
        assert!(h.knows_course(slug), "{slug} is mapped and the vault does not know it");
    }
    // …and `gn-103` is known by the map alone, with no note behind it — which is the whole reason
    // `knows_course` tests both. A student who has a VHL section and no Blackboard course for it is
    // not a broken vault.
    assert!(!dest.join("courses").join("gn-103.md").exists());
    assert!(h.knows_course("gn-103"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_chosen_school_becomes_campus_yaml_and_a_timezone_suggestion() {
    use knowlu::scaffold::{campus_config_yaml, curated, state_timezone, CampusChoice};
    let ua = CampusChoice { unitid: "100751".into(), name: "The University of Alabama".into(), state: "AL".into(), lms: "blackboard".into() };
    let text = campus_config_yaml(&ua).expect("campus.yaml");
    assert_eq!(
        text,
        "unitid: '100751'\nname: 'The University of Alabama'\nstate: 'AL'\nlms: 'blackboard'\ncurated: true\n"
    );
    // A school nobody has curated is still a school: it gets a file, no event feeds, and an LMS the
    // sign-in window (or the student) names.
    let other = CampusChoice { unitid: "999999".into(), name: "Somewhere Community College".into(), state: "OR".into(), lms: String::new() };
    let text = campus_config_yaml(&other).expect("campus.yaml");
    assert!(text.contains("curated: false\n") && text.contains("lms: ''\n"), "{text}");
    // …and a name with an apostrophe does not break the file, like every other wizard value.
    let odd = CampusChoice { unitid: "1".into(), name: "St. Mary's College".into(), state: "MD".into(), lms: String::new() };
    assert!(campus_config_yaml(&odd).expect("campus.yaml").contains("name: 'St. Mary''s College'\n"));

    assert_eq!(curated("100751").map(|c| c.key), Some("university-of-alabama"));
    assert_eq!(curated("157085").map(|c| c.key), Some("university-of-kentucky"));
    assert!(curated("999999").is_none());

    // The timezone the wizard suggests, from the state — the OS zone stays the default and the
    // student can always type over it.
    assert_eq!(state_timezone("AL"), Some("America/Chicago"));
    assert_eq!(state_timezone("KY"), Some("America/New_York"));
    assert_eq!(state_timezone("AZ"), Some("America/Phoenix"));
    assert_eq!(state_timezone("HI"), Some("Pacific/Honolulu"));
    assert_eq!(state_timezone("zz"), None);
    // Fifty states, DC and the five inhabited territories — IPEDS keeps Puerto Rico's hundred-odd
    // institutions, and a student in Mayagüez is not a special case any more than one in Wyoming.
    assert_eq!(knowlu::scaffold::STATE_TZ.len(), 56);
    assert_eq!(state_timezone("PR"), Some("America/Puerto_Rico"));
    assert_eq!(state_timezone("GU"), Some("Pacific/Guam"));
    for (st, tz) in knowlu::scaffold::STATE_TZ {
        assert!(jiff::tz::TimeZone::get(tz).is_ok(), "{st} maps to {tz}, which the tz database does not have");
    }
}
