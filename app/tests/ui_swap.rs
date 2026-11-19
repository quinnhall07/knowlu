//! v0.4.0 T23 (note §8, plan §5 T23, §7): the swap's two pins. The new page moved from `app/static/next/`
//! into `app/static/`, the old page (`console.js`, `console.css`, its `index.html`, Instrument Sans and
//! JetBrains Mono) is gone, and the six tests `static_assets.rs` kept unchanged live in `bundle_config.rs`,
//! with `static_assets.rs` deleted.
#[path = "support/ui_common.rs"]
mod ui_common;

use std::path::Path;
use ui_common::*;

fn names_under(dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { names_under(&p, out); }
        out.push(p.strip_prefix(app_dir()).unwrap_or(&p).to_string_lossy().replace('\\', "/"));
    }
}

/// No file of the old page ships: no `console.js`, `console.css`, Instrument Sans or JetBrains Mono file
/// (font or licence) anywhere under `app/static`, and no `static/next/`. The one document is the new
/// page's, read from `ROOT`, which is `static`.
#[test]
fn no_old_page_file_ships() {
    let stat = app_dir().join("static");
    assert!(!stat.join("next").exists(), "app/static/next/ is still there: the swap moves it into app/static/");
    let mut names = Vec::new();
    names_under(&stat, &mut names);
    for n in &names {
        let low = n.to_ascii_lowercase();
        let file = low.rsplit('/').next().unwrap_or(&low);
        for old in ["console.js", "console.css"] {
            assert_ne!(file, old, "{n}: the old page's {old} ships");
        }
        for face in ["instrument", "jetbrains"] {
            assert!(!file.contains(face), "{n}: an old font family ships (D7)");
        }
    }
    assert_eq!(ROOT, "static", "ui_common's ROOT is the shipped root");
    let html = index_html();
    assert!(html.contains("<script src=\"js/boot.js\" defer></script>") && !html.contains("console.js") && !html.contains("console.css"), "app/static/index.html is the new page's");
    for f in ["js/kit.js", "js/router.js", "css/tokens.css", "fonts/SOURCES.md"] {
        assert!(try_read(f).is_some(), "app/static/{f} is in place");
    }
}

/// The six tests the swap keeps unchanged (plan §7, "Kept") live in `bundle_config.rs`, each once across
/// `app/tests`, and `static_assets.rs` is gone.
#[test]
fn bundle_config_tests_live_in_bundle_config_rs() {
    let tests = app_dir().join("tests");
    assert!(!tests.join("static_assets.rs").exists(), "app/tests/static_assets.rs is deleted at the swap");
    let kept = std::fs::read_to_string(tests.join("bundle_config.rs")).expect("app/tests/bundle_config.rs");
    let mut others = String::new();
    for e in std::fs::read_dir(&tests).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "bundle_config.rs" && n != "ui_swap.rs") {
            others.push_str(&std::fs::read_to_string(&p).unwrap());
        }
    }
    for name in [
        "the_bundle_config_is_the_one_the_installer_and_the_updater_need",
        "the_updater_endpoint_lives_in_the_app_config",
        "the_release_scripts_parse_under_powershell_5_1",
        "build_rs_still_stages_the_zero_byte_sidecar_placeholder",
        "the_privacy_version_constant_is_the_published_pages_date",
        "deleting_my_data_leaves_another_profiles_snapshots_alone",
    ] {
        let head = format!("fn {name}()");
        assert_eq!(kept.matches(&head).count(), 1, "bundle_config.rs holds {name} once");
        assert!(!others.contains(&head), "{name} is defined outside bundle_config.rs too");
    }
    assert_eq!(kept.matches("#[test]").count(), 6, "bundle_config.rs holds the six kept tests and no other");
}
