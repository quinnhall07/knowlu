//! v0.4.0: the app icon and the tray icon are the Knowlu brand mark (a red `#c33137` tile with the
//! cream `#fffefb` letters "lu"). `icons/icon.ico` is the 2026-11-17 high-def brand icon with its
//! directory reordered, 32 px first, because Tauri builds the window icon from the first entry
//! alone; `scripts/brand-icons.py` does the reorder and derives the two tray images from that entry.
//! These pins read the generated files and `src/tray.rs` as bytes and text; no window or tray is
//! started here. The tray keeps its one contract: a failing step turns it amber (`TrayState::Warn`,
//! which `scheduler.rs` sets) — now an amber dot in the mark's bottom-right corner.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const SIDE: usize = 32;
const RED: [u8; 4] = [0xC3, 0x31, 0x37, 0xFF];
const AMBER: [u8; 4] = [0xD9, 0xA4, 0x41, 0xFF];

fn app_path(rel: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join(rel) }

fn read(rel: &str) -> Vec<u8> {
    let p = app_path(rel);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn u16le(b: &[u8], at: usize) -> u16 { u16::from_le_bytes([b[at], b[at + 1]]) }

/// The pixel at `(x, y)` of a 32x32 row-major RGBA buffer.
fn px(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIDE + x) * 4;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

/// Each entry of the 2026-11-17 high-def icon: its size and the SHA-256 of its image data (all
/// PNG). The committed file holds exactly these blobs, reordered and never re-encoded.
const HIGH_DEF_ENTRIES: [(u32, &str); 10] = [
    (16, "476792d22f655b060b24cc5cff4fe8a6eebbc112c1d26016293cafbf23653e4d"),
    (20, "6733f14ce8ee3094be0d1fca84b6b9df9c910ccf53028f7d60a6c85b8dd22941"),
    (24, "fefb1ae2a331f00e5107d460cd3647a04ceee57ca6b1a34c435173038fa7dc6c"),
    (32, "60a113f17aab66179bff1d8ddf06d043362fcafe78f992cc22ff90bfd3ec52df"),
    (40, "c16d1dfa12e8ff3cb71dbfe98a904f5b57679867720633aad3086aaadf823968"),
    (48, "f6d5d3de845450c8f26b3e8a199517a9e6ae55c4c3b806de84fd54de68b775b2"),
    (64, "c1c488a0846983b2725f1271362df94b99961cec84c1f1a8c62e0ee561615623"),
    (96, "9700214cbe6245aac2d2968e84d3d312507642a901d58faae08c6ff0f079388f"),
    (128, "ee6f24f74acd7ecff2e651187d9459fc43a15729daa5a59bbb0005393f9cdbb7"),
    (256, "8c37decd36beff311e991a76af847f37529fa27f76db0de022a57a79ea2653c9"),
];

fn sha256_hex(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect() }

/// `s` with every whitespace character removed, so a pin does not depend on line breaks.
fn squash(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

#[test]
fn the_app_icon_is_an_ico_holding_every_size_windows_asks_for() {
    let ico = read("icons/icon.ico");
    assert!(ico.len() >= 6, "icon.ico is {} bytes", ico.len());
    assert_eq!(u16le(&ico, 0), 0, "ICONDIR reserved field");
    assert_eq!(u16le(&ico, 2), 1, "ICONDIR type (1 = icon)");
    let count = u16le(&ico, 4) as usize;
    assert!(ico.len() >= 6 + count * 16, "icon.ico is too short for {count} directory entries");
    let mut sizes = Vec::new();
    let mut entries = Vec::new();
    let mut png_256 = false;
    for i in 0..count {
        let e = 6 + i * 16;
        // A width or height byte of 0 means 256.
        let w = if ico[e] == 0 { 256 } else { ico[e] as u32 };
        let h = if ico[e + 1] == 0 { 256 } else { ico[e + 1] as u32 };
        assert_eq!(w, h, "entry {i} is not square");
        let len = u32::from_le_bytes([ico[e + 8], ico[e + 9], ico[e + 10], ico[e + 11]]) as usize;
        let off = u32::from_le_bytes([ico[e + 12], ico[e + 13], ico[e + 14], ico[e + 15]]) as usize;
        assert!(len > 0 && off + len <= ico.len(), "entry {i} ({w}px) points outside the file");
        if w == 256 && ico[off..].starts_with(b"\x89PNG") { png_256 = true }
        sizes.push(w);
        entries.push((w, sha256_hex(&ico[off..off + len])));
    }
    // tauri-codegen decodes only the first entry into the window icon (ICON_SMALL), so it is the
    // 32 px one: a 16 px first entry is stretched on a 150% or 200% display.
    assert_eq!(sizes.first(), Some(&32), "icon.ico's first entry is not 32 px; the order is {sizes:?}");
    entries.sort();
    let mut want: Vec<(u32, String)> = HIGH_DEF_ENTRIES.iter().map(|(w, h)| (*w, h.to_string())).collect();
    want.sort();
    assert_eq!(entries, want, "icon.ico's images are not the 2026-11-17 high-def icon's, byte for byte");
    for want in [16, 20, 24, 32, 40, 48, 64, 96, 128, 256] {
        assert!(sizes.contains(&want), "icon.ico has no {want}px entry; it has {sizes:?}");
    }
    assert!(png_256, "icon.ico's 256px entry is not PNG-compressed");
}

#[test]
fn the_tray_images_are_the_mark_and_the_mark_with_an_amber_dot() {
    let ok = read("icons/tray-ok.rgba");
    let warn = read("icons/tray-warn.rgba");
    assert_eq!(ok.len(), SIDE * SIDE * 4, "tray-ok.rgba is not 32x32 RGBA");
    assert_eq!(warn.len(), SIDE * SIDE * 4, "tray-warn.rgba is not 32x32 RGBA");

    // (16, 3): the top middle of the tile, above both letters (they start at y = 10.94 of the
    // 64-unit viewBox, row 5 at 32 px) and away from the rounded corners. Solid brand red.
    assert_eq!(px(&ok, 16, 3), RED, "tray-ok (16,3) is not opaque #c33137");

    // Outside the bottom-right 16x16 quadrant the two are the same image, byte for byte.
    for y in 0..SIDE {
        for x in 0..SIDE {
            if x >= 16 && y >= 16 { continue }
            assert_eq!(px(&warn, x, y), px(&ok, x, y), "tray-warn differs from tray-ok at ({x},{y}), outside the dot's quadrant");
        }
    }
    // Inside it, the dot: opaque amber pixels, and the quadrant is not just the ok image.
    let mut amber = 0;
    let mut differs = 0;
    for y in 16..SIDE {
        for x in 16..SIDE {
            if px(&warn, x, y) == AMBER { amber += 1 }
            if px(&warn, x, y) != px(&ok, x, y) { differs += 1 }
        }
    }
    assert!(amber >= 20, "tray-warn has only {amber} opaque #d9a441 pixels in its bottom-right quadrant");
    assert!(differs > amber, "tray-warn's dot has no ring cut around it");
    assert!(!ok.chunks(4).any(|p| p == AMBER), "tray-ok carries the warning amber");
}

#[test]
fn the_tray_uses_the_generated_images_and_warn_maps_to_the_dot() {
    let p = app_path("src/tray.rs");
    let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let flat = squash(&src);
    assert!(!flat.contains("fnsquare("), "tray.rs still draws its own square");
    assert!(flat.contains("include_bytes!(\"../icons/tray-ok.rgba\")"), "tray.rs does not include tray-ok.rgba");
    assert!(flat.contains("include_bytes!(\"../icons/tray-warn.rgba\")"), "tray.rs does not include tray-warn.rgba");
    let set_state = &flat[flat.find("pubfnset_state(").expect("set_state")..];
    // set_state runs up to the next doc comment (`redact`'s).
    let set_state = &set_state[..set_state.find("///").unwrap_or(set_state.len())];
    assert!(set_state.contains("TrayState::Ok=>tray_icon(TRAY_OK)"), "set_state does not map Ok to the plain mark");
    assert!(set_state.contains("TrayState::Warn=>tray_icon(TRAY_WARN)"), "set_state does not map Warn to the mark with the amber dot");
    assert!(flat.contains(".icon(tray_icon(TRAY_OK))"), "the tray does not start on the plain mark");
}
