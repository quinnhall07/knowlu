//! Phase 3 of the commitment model (spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`
//! §2, D1, D4): `registrar.rs`'s pure half, its engine run, and the window promises it inherits
//! from `lms_link.rs`, read from the source where a live WebView2 would be needed. The engine run
//! spawns the real sibling engine: build it first with `cargo build -p knowlu-engine -j 2`, because
//! `app/build.rs` leaves a zero-byte placeholder at `target/debug/knowlu-engine.exe`.
use std::path::{Path, PathBuf};
use knowlu::registrar::{call_urls, registrar_argv, run_file, school_of, start_url};
use knowlu::state::ConsoleState;

const FIXTURE: &str = include_str!("../../engine/tests/fixtures/registrar/banner-ua-registration.json");

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qo-registrar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "courses"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    std::fs::write(dir.join("config/ingest.yaml"), "timezone: America/Chicago\n").unwrap();
    std::fs::write(dir.join("config/campus.yaml"), "unitid: '100751'\nname: 'Invented'\nstate: 'AL'\nlms: 'blackboard'\ncurated: true\n").unwrap();
    std::fs::write(dir.join("courses/cs-100.md"), "---\ntitle: \"CS 100 Invented Computing\"\ncode: \"CS 100\"\n---\n").unwrap();
    dir
}

fn open(v: &Path, name: &str) -> ConsoleState {
    ConsoleState::open(v.to_path_buf(), std::env::temp_dir().join(format!("qo-registrar-data-{name}-{}", std::process::id())))
}

/// Removes the scratch vault and its data directory however the test ends (R4 review M4).
struct Gone(Vec<PathBuf>);
impl Drop for Gone {
    fn drop(&mut self) {
        for p in &self.0 { let _ = std::fs::remove_dir_all(p); }
    }
}

/// `KNOWLU_ENGINE_EXE` is process-wide; this file's own lock, `week.rs`'s shape.
static ENGINE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn real_engine() -> std::sync::MutexGuard<'static, ()> {
    let exe = Path::new("../target/debug/knowlu-engine.exe");
    let len = std::fs::metadata(exe).map(|m| m.len()).unwrap_or(0);
    assert!(len > 0, "{}: run `cargo build -p knowlu-engine -j 2` first (build.rs leaves a zero-byte placeholder)", exe.display());
    let guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("KNOWLU_ENGINE_EXE", exe.canonicalize().unwrap()) };
    guard
}

#[test]
fn ua_has_a_registrar_and_every_call_is_on_its_own_registrar_host() {
    let v = scratch("host");
    let reg = school_of(&v).expect("UA's row has a registrar entry");
    assert_eq!((reg.school, reg.label), ("ua", "myBama"));
    assert_eq!(start_url(reg), "https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/registration");
    let calls = call_urls(reg, "202640");
    assert!(!calls.is_empty() && calls.len() <= 3, "{calls:?}");
    for (method, url, form) in &calls {
        assert!(url.starts_with("https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/"), "{url}");
        assert!(!url.contains("{term}") && !form.as_deref().unwrap_or("").contains("{term}"), "{url} {form:?}");
        assert!(method == "GET" || method == "POST", "{method}");
    }
    assert_eq!(calls.last().unwrap().0, "GET", "the rows call is last and a GET");
    for c in knowlu::scaffold::CAMPUSES {
        if c.unitid != "100751" {
            assert!(c.registrar.is_none(), "{} has no registrar before the pilot", c.label);
        }
    }
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_school_is_read_from_campus_yaml_as_text_or_number_and_absent_is_none() {
    let v = scratch("school");
    std::fs::write(v.join("config/campus.yaml"), "unitid: 100751\n").unwrap();
    assert!(school_of(&v).is_some(), "a number is read too");
    std::fs::write(v.join("config/campus.yaml"), "unitid: '157085'\n").unwrap();
    assert!(school_of(&v).is_none(), "a curated school without a registrar");
    std::fs::remove_file(v.join("config/campus.yaml")).unwrap();
    assert!(school_of(&v).is_none(), "no campus.yaml, no button");
    let _ = std::fs::remove_dir_all(&v);
}

/// D1: the cookies go only to the registrar host. On Okta, on Okta Verify's page or anywhere else
/// the window's own URL is not used; the call's URL is.
#[test]
fn the_cookies_are_read_for_the_registrar_host_and_never_for_the_sign_in_provider() {
    use knowlu::lms_link::cookie_url;
    let rows = "https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/classRegistration/x";
    let here = "https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/registration/registration";
    assert_eq!(cookie_url(Some(here), rows), here);
    for elsewhere in ["https://login.example.invalid/oauth2/v1/authorize", "https://bannerssb.ua.edu.example.invalid/x", "about:blank"] {
        assert_eq!(cookie_url(Some(elsewhere), rows), rows, "{elsewhere}");
    }
}

#[test]
fn the_argv_carries_the_vault_the_file_the_school_today_and_the_consoles_via() {
    assert_eq!(
        registrar_argv(Path::new(r"C:\v"), Path::new(r"C:\d\tmp\r.json"), "ua", "2026-09-26".parse().unwrap()),
        ["commitments", "--vault", r"C:\v", "--today", "2026-09-26", "--registrar", r"C:\d\tmp\r.json", "--school", "ua", "--via", "dashboard"]
    );
}

/// D4 + R4-e: the bytes go to a temp file under the profile's `tmp\`, the engine writes the
/// notes as `quinn` via `dashboard`, and the file is gone afterwards.
#[test]
fn a_fetched_schedule_runs_through_the_engine_and_the_temp_file_is_gone() {
    let _engine = real_engine();
    let v = scratch("run");
    let cs = open(&v, "run");
    let _gone = Gone(vec![v.clone(), cs.data_dir.clone()]);
    cs.set_test_today(Some("2026-09-01".parse().unwrap()));
    let env = run_file(&cs, "today", "ua", FIXTURE.as_bytes(), "2026-09-01".parse().unwrap());
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["closed"], true);
    assert_eq!((env["result"]["confirmed"].as_u64(), env["result"]["term"].as_str()), (Some(2), Some("202640")));
    assert_eq!(env["state"]["schema"], 1);
    assert!(v.join("commitments/cs-100-lab.md").is_file());
    let tmp = cs.data_dir.join("tmp");
    assert_eq!(std::fs::read_dir(&tmp).map(|d| d.count()).unwrap_or(0), 0, "the temp file is deleted");
}

#[test]
fn a_parse_failure_is_named_writes_nothing_and_the_temp_file_is_gone_too() {
    let _engine = real_engine();
    let v = scratch("fail");
    let cs = open(&v, "fail");
    let _gone = Gone(vec![v.clone(), cs.data_dir.clone()]);
    let env = run_file(&cs, "today", "ua", b"{\"data\": []}", "2026-09-01".parse().unwrap());
    assert_eq!(env["ok"], false, "{env}");
    assert!(env["error"].as_str().unwrap().contains("no class with meeting times"), "{env}");
    assert!(!v.join("commitments").exists() && !v.join("state/calendar-series.json").exists());
    assert_eq!(std::fs::read_dir(cs.data_dir.join("tmp")).map(|d| d.count()).unwrap_or(0), 0);
}

/// R4 review M2: a crash mid-run leaves a schedule file in `tmp\`; opening the window again sweeps
/// the stale ones, and only `registrar-*.json`.
#[test]
fn a_stale_schedule_file_is_swept_and_nothing_else_is() {
    use std::time::{Duration, SystemTime};
    let dir = std::env::temp_dir().join(format!("qo-registrar-sweep-{}", std::process::id()));
    let _gone = Gone(vec![dir.clone()]);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["registrar-1-20260901T000000Z.json", "confirm-1.json", "registrar-notes.txt"] {
        std::fs::write(dir.join(name), "{}").unwrap();
    }
    let hour = Duration::from_secs(3600);
    assert_eq!(knowlu::registrar::sweep_stale_files_in(&dir, SystemTime::now(), hour), 0, "a fresh file is a run in progress");
    assert_eq!(knowlu::registrar::sweep_stale_files_in(&dir, SystemTime::now() + 2 * hour, hour), 1);
    assert!(!dir.join("registrar-1-20260901T000000Z.json").exists());
    assert!(dir.join("confirm-1.json").exists() && dir.join("registrar-notes.txt").exists());
    assert_eq!(knowlu::registrar::sweep_stale_files_in(&dir.join("absent"), SystemTime::now(), hour), 0);
}

/// R4 review M1: a failed request is named by its kind in fixed words; ureq's own text (which can
/// carry the URL) never reaches the page.
#[test]
fn a_failed_request_is_named_in_fixed_words() {
    use knowlu::registrar::fetch_error;
    assert_eq!(fetch_error("myBama", &ureq::Error::StatusCode(500)), "myBama answered with an error");
    assert_eq!(fetch_error("myBama", &ureq::Error::HostNotFound), "couldn't reach myBama");
    assert_eq!(fetch_error("myBama", &ureq::Error::ConnectionFailed), "couldn't reach myBama");
    assert_eq!(fetch_error("myBama", &ureq::Error::BodyStalled), "myBama took too long");
    assert_eq!(fetch_error("myBama", &ureq::Error::BadUri("https://x.invalid/?a=secret".into())), "couldn't reach myBama");
    let src = std::fs::read_to_string("src/registrar.rs").unwrap();
    assert!(!src.contains("({e})"), "no error's own text is formatted into an envelope");
    let open = src.split("pub fn open_registrar_window").nth(1).unwrap();
    assert!(open.contains("sweep_stale_files_in("), "the window's opening sweeps the stale schedule files");
}

/// D1 and constraint 9, read from the source: the registrar opens `lms_link`'s window and no
/// other, keeps no credential, and closes and wipes after the engine run.
#[test]
fn the_registrar_uses_the_sign_in_window_and_keeps_nothing() {
    let src = std::fs::read_to_string("src/registrar.rs").expect("src/registrar.rs");
    for needle in ["lms_link::open_window_at", "lms_link::session_dir()", "lms_link::close_and_wipe", "lms_link::cookie_url", "looks_signed_out", "remove_file(&file)"] {
        assert!(src.contains(needle), "registrar.rs must use {needle}");
    }
    for banned in ["WebviewWindowBuilder", "password", "credentials::", "app_data_root", "set_password", "eprintln!", "log::"] {
        assert!(!src.contains(banned), "registrar.rs must not contain {banned}");
    }
    assert!(!src.contains("\"bytes\""), "the fetched bytes never go back to the page");
}

/// R4-f: the console's builder wipes a sign-in session as the wizard's does, and hides only its
/// own window on close, so the sign-in window can be closed.
#[test]
fn the_console_builder_wipes_the_sign_in_session_and_hides_only_main() {
    let src = std::fs::read_to_string("src/main.rs").expect("src/main.rs");
    let console = src.split("fn run_console").nth(1).expect("run_console");
    assert!(console.contains("app.manage(lms_link::LmsSession::default())"), "the console manages LmsSession");
    assert!(console.contains("lms_link::wipe_session(w.app_handle())"), "…wipes on Destroyed");
    assert!(console.contains("lms_link::wipe_session_on_exit(app)"), "…and at exit");
    assert!(console.contains("w.label() == \"main\""), "only main hides on close");
    for cmd in ["registrar::open_registrar_window", "registrar::capture_registrar", "registrar::close_registrar_window"] {
        assert!(console.contains(cmd), "{cmd} is registered on the console window");
    }
    let shell = src.split("fn run_shell").nth(1).and_then(|s| s.split("fn run_console").next()).expect("run_shell");
    assert!(!shell.contains("registrar::"), "the wizard has no registrar command (spec D6)");
}
