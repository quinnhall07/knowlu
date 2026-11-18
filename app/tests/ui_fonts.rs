//! v0.4.0 T1 (note D7, plan P7): the new page's three faces ship as unmodified upstream files, each
//! with its licence and a `SOURCES.md` row whose SHA-256 is the file's own.
//!
//! The old page's Instrument Sans files were per-script subsets taken from the Google Fonts CSS2
//! endpoint (`static/fonts/SOURCES.md`). A subset is a Modified Version under the OFL, and PT Serif
//! Caption's licence reserves the name "PT Serif", so the new faces come whole from their
//! repositories at a recorded commit. This test cannot prove a file is upstream's; it proves the
//! paper trail is there and true to the bytes on disk, so a reviewer can re-fetch and compare.
//! Zilla Slab (only the dropped Ask surface used it), Instrument Sans and JetBrains Mono do not ship
//! here.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn fonts_dir() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("static").join("next").join("fonts") }

const FONT_EXTS: [&str; 4] = ["ttf", "otf", "woff", "woff2"];

fn is_font(p: &Path) -> bool { p.extension().and_then(|e| e.to_str()).map(|e| FONT_EXTS.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false) }

fn sha256_hex(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect() }

fn is_hex(s: &str, len: usize) -> bool { s.len() == len && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }

/// The family's file-name slug: lowercase letters and digits only ("PT Serif Caption" → "ptserifcaption").
fn slug(family: &str) -> String { family.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect() }

/// Every `SOURCES.md` table row as its trimmed cells, backticks dropped.
fn rows(sources: &str) -> Vec<Vec<String>> {
    sources
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|') && !l.starts_with("|---") && !l.starts_with("| ---"))
        .map(|l| l.trim_matches('|').split('|').map(|c| c.trim().trim_matches('`').to_string()).collect())
        .collect()
}

#[test]
fn each_next_font_family_has_files_an_ofl_and_a_hashed_sources_row() {
    let dir = fonts_dir();
    let sources = std::fs::read_to_string(dir.join("SOURCES.md")).unwrap_or_else(|e| panic!("{}: {e}", dir.join("SOURCES.md").display()));
    let table = rows(&sources);
    let files: Vec<PathBuf> = std::fs::read_dir(&dir).expect("static/next/fonts").flatten().map(|e| e.path()).filter(|p| is_font(p)).collect();
    assert!(!files.is_empty(), "no font files under {}", dir.display());

    // Every font file on disk has exactly one row: its name first, a 40-hex source commit somewhere in
    // the row, and a 64-hex SHA-256 that is the file's own (lowercase, as `Get-FileHash` prints it
    // once lowered).
    let mut families_seen = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        for banned in ["zillaslab", "zilla-slab", "instrument", "jetbrains"] {
            assert!(!lower.contains(banned), "{name}: this face does not ship with the new page (D7)");
        }
        for subset in ["subset", "-latin", "_latin"] {
            assert!(!lower.contains(subset), "{name} is named as a per-script subset; ship the whole upstream file (P7)");
        }
        let bytes = std::fs::read(path).unwrap();
        let magic = &bytes[..4.min(bytes.len())];
        assert!(
            [&b"\x00\x01\x00\x00"[..], &b"OTTO"[..], &b"wOFF"[..], &b"wOF2"[..], &b"true"[..]].contains(&magic),
            "{name} does not start with a font signature"
        );
        let mine: Vec<&Vec<String>> = table.iter().filter(|r| r.first().map(|c| c == &name).unwrap_or(false)).collect();
        assert_eq!(mine.len(), 1, "{name} needs exactly one SOURCES.md row, found {}", mine.len());
        let row = mine[0];
        let hash = row.iter().find(|c| is_hex(c, 64)).unwrap_or_else(|| panic!("{name}: no 64-hex SHA-256 in its row"));
        assert_eq!(hash, &sha256_hex(&bytes), "{name}: the row's SHA-256 is not the file's");
        assert!(
            row.iter().any(|c| c.split(|ch: char| !ch.is_ascii_hexdigit()).any(|w| is_hex(w, 40))),
            "{name}: its row names no 40-hex source commit"
        );
        assert!(row.iter().any(|c| c.contains("OFL") || c.contains("Open Font License")), "{name}: its row names no licence");
        let family = row.get(1).cloned().unwrap_or_default();
        families_seen.push(family);
    }

    // Every row that names a font file names one that is on disk (no stale rows).
    for row in &table {
        if let Some(first) = row.first() {
            if is_font(Path::new(first)) {
                assert!(dir.join(first).is_file(), "SOURCES.md names {first}, which is not in {}", dir.display());
            }
        }
    }

    // The three families, each with at least one file and its own OFL beside it.
    for family in ["PT Serif Caption", "Albert Sans", "OpenDyslexic"] {
        assert!(families_seen.iter().any(|f| f == family), "no {family} file has a SOURCES.md row");
        let ofl = dir.join(format!("OFL-{}.txt", slug(family)));
        let text = std::fs::read_to_string(&ofl).unwrap_or_else(|e| panic!("{family}'s licence {}: {e}", ofl.display()));
        assert!(text.contains("SIL Open Font License, Version 1.1"), "{} is not the OFL 1.1 text", ofl.display());
    }
    for family in &families_seen {
        assert!(["PT Serif Caption", "Albert Sans", "OpenDyslexic"].contains(&family.as_str()), "{family} is not one of the new page's three faces");
    }
    // The RFN note: the two licences that reserve a name say so in SOURCES.md.
    assert!(sources.contains("Reserved Font Name"), "SOURCES.md carries no Reserved Font Name note");
}
