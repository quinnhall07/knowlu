"""Order the app icon's directory and derive the tray icons from it.

Source of truth: app/icons/icon.ico, the Knowlu brand tile (red #c33137, cream "lu"). It is the
high-def knowlu.ico made on 2026-11-17 (16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 px, each a
PNG-compressed entry) with its directory reordered, 32 px first: Tauri builds the window icon from
the first entry alone (tauri-codegen), and a 16 px one is stretched on a 150% or 200% display. The
order is the one `tauri icon` uses: 32, 16, 24, 48, 64, 256, then the rest by size. Only the
16-byte directory records and the offsets change; every image is kept byte for byte. Nothing here
draws the mark or re-encodes an image.

First, it rewrites icon.ico in that order (from --from PATH when given, else from icon.ico itself;
running it twice changes nothing). Then it writes, next to it, two raw 32x32 RGBA images (4096 bytes each, row-major, straight alpha) that
app/src/tray.rs compiles in with include_bytes!:

  app/icons/tray-ok.rgba    the ICO's own 32 px entry, so the tray is the same art as the taskbar
  app/icons/tray-warn.rgba  the same, with an amber #d9a441 dot about 12 px across in the
                            bottom-right corner and a 2 px transparent ring cut around it, so it
                            reads against the red (a failing step turns the tray amber)

Raw RGBA keeps tray.rs off any new tauri feature or crate.

Run from the repo root, only when the mark changes:

  python scripts/brand-icons.py [--from NEW.ico] [--preview DIR]

--preview DIR also writes icon-256.png (the ICO's 256 px entry) and tray-preview.png (ok and warn
at 32 px and at 4x, on light and dark grounds) into DIR. Needs Pillow; nothing else.
"""

import argparse
import pathlib
import struct
import sys

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent.parent
ICONS = ROOT / "app" / "icons"
ICO = ICONS / "icon.ico"

ORDER = [32, 16, 24, 48, 64, 256]  # first; every other size follows, smallest first
SIDE = 32
AMBER = (0xD9, 0xA4, 0x41)
# The dot: 12 px across, its box flush with the bottom-right corner ([20, 32) on both axes).
DOT_CX = DOT_CY = 26.0
DOT_R = 6.0
RING = 2.0  # transparent cut around the dot
SAMPLES = 8  # per axis, for anti-aliased circle coverage


def ordered_ico(data):
    """`data` (an ICO file) with its directory in ORDER and its images, unchanged, laid out after it
    in the same order. Idempotent: ordered input comes back byte for byte."""
    reserved, kind, count = struct.unpack_from("<HHH", data, 0)
    if reserved != 0 or kind != 1 or count == 0:
        sys.exit("not an icon file (reserved %d, type %d, %d entries)" % (reserved, kind, count))
    entries = []
    for i in range(count):
        rec = data[6 + 16 * i:22 + 16 * i]
        length, offset = struct.unpack_from("<II", rec, 8)
        if offset + length > len(data):
            sys.exit("entry %d points outside the file" % i)
        size = rec[0] or 256
        entries.append((size, rec[:8], data[offset:offset + length]))
    sizes = [e[0] for e in entries]
    if len(set(sizes)) != len(sizes):
        sys.exit("two entries share a size: %s" % sizes)
    rank = {s: i for i, s in enumerate(ORDER)}
    entries.sort(key=lambda e: (rank.get(e[0], len(ORDER)), e[0]))
    head = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count
    directory, blobs = b"", b""
    for _, prefix, blob in entries:
        directory += prefix + struct.pack("<II", len(blob), offset)
        blobs += blob
        offset += len(blob)
    return head + directory + blobs


def ico_entry(size):
    """The ICO's own entry at `size` px, as RGBA. Never a resample: a missing size is an error."""
    with Image.open(ICO) as im:
        have = sorted(im.info.get("sizes", set()))
        if (size, size) not in have:
            sys.exit("%s has no %d px entry (it has %s)" % (ICO, size, have))
        im.size = (size, size)  # selects that entry
        im.load()
        out = im.convert("RGBA")
    if out.size != (size, size):
        sys.exit("%s: the %d px entry decoded as %s" % (ICO, size, out.size))
    return out


def coverage(x, y, r):
    """The fraction of pixel (x, y) inside the circle of radius r about the dot's centre."""
    inside = 0
    for j in range(SAMPLES):
        for i in range(SAMPLES):
            sx = x + (i + 0.5) / SAMPLES - DOT_CX
            sy = y + (j + 0.5) / SAMPLES - DOT_CY
            if sx * sx + sy * sy <= r * r:
                inside += 1
    return inside / (SAMPLES * SAMPLES)


def with_dot(ok):
    """`ok` (32x32 straight-alpha RGBA bytes) with the ring cut and the amber dot laid over it."""
    out = bytearray(ok)
    for y in range(SIDE):
        for x in range(SIDE):
            cut = coverage(x, y, DOT_R + RING)
            if cut == 0:
                continue
            dot = coverage(x, y, DOT_R)
            k = (y * SIDE + x) * 4
            r, g, b, a = out[k:k + 4]
            # Cut the ring (and the dot's own area) out of the mark, then lay the dot over it.
            base_a = a / 255 * (1 - cut)
            out_a = dot + base_a * (1 - dot)
            if out_a == 0:
                out[k:k + 4] = bytes(4)
                continue
            mix = [(AMBER[c] * dot + (r, g, b)[c] * base_a * (1 - dot)) / out_a for c in range(3)]
            out[k:k + 4] = bytes([round(v) for v in mix] + [round(out_a * 255)])
    return bytes(out)


def preview(ok, warn, dest):
    dest.mkdir(parents=True, exist_ok=True)
    ico_entry(256).save(dest / "icon-256.png")
    tiles = [Image.frombytes("RGBA", (SIDE, SIDE), b) for b in (ok, warn)]
    pad, big = 16, SIDE * 4
    w = pad + (SIDE + pad) * 2 + (big + pad) * 2
    h = (big + pad * 2) * 2
    sheet = Image.new("RGBA", (w, h))
    for row, ground in enumerate([(0xF3, 0xF3, 0xF3, 255), (0x20, 0x1C, 0x21, 255)]):
        top = row * (big + pad * 2)
        sheet.paste(ground, (0, top, w, top + big + pad * 2))
        x = pad
        for t in tiles:
            sheet.alpha_composite(t, (x, top + pad + (big - SIDE) // 2))
            x += SIDE + pad
        for t in tiles:
            sheet.alpha_composite(t.resize((big, big), Image.Resampling.NEAREST), (x, top + pad))
            x += big + pad
    sheet.save(dest / "tray-preview.png")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--from", dest="source", type=pathlib.Path,
                    help="the ICO to order into app/icons/icon.ico (default: icon.ico itself)")
    ap.add_argument("--preview", type=pathlib.Path, help="also write preview PNGs into this folder")
    args = ap.parse_args()
    ICO.write_bytes(ordered_ico((args.source or ICO).read_bytes()))
    ok = ico_entry(SIDE).tobytes()
    assert len(ok) == SIDE * SIDE * 4
    warn = with_dot(ok)
    (ICONS / "tray-ok.rgba").write_bytes(ok)
    (ICONS / "tray-warn.rgba").write_bytes(warn)
    print("wrote %s and %s from %s" % (ICONS / "tray-ok.rgba", ICONS / "tray-warn.rgba", ICO))
    if args.preview:
        preview(ok, warn, args.preview)
        print("previews in %s" % args.preview)


if __name__ == "__main__":
    main()
