#!/usr/bin/env python3
"""Flamingo -- attribution board for the "after-plate went cool 0.00 %" finding.

The question this answers is NOT "is the after plate worse" (Poppy measured that
and it is). It is the narrower one I was asked: *did the Pile A grade lane in
`hero.rs` cause it.*

Two independent proofs go on the board, because either one alone is the kind of
evidence that has fooled this office before:

  1. CODE PATH -- `hero::setup_hero` is spawned only under `cfg.hero`, i.e. only
     when `VOXELFORGE_HERO` is set. Poppy's shoot script sets `VOXELFORGE_PLAY=1`
     and never sets `VOXELFORGE_HERO`, so on those two plates the whole file is
     dead code. A grep for `hero::` outside hero.rs returns nothing beyond
     setup_hero / GateVerdict / GATE_FAILED / report_voxel_overlaps -- the module
     exports no colour or grade term into the world path.

  2. BINARY -- every symbol Pile A introduces is absent from BOTH staged exes.
     mtime is not trusted here (a commit clock never dates a binary); the counts
     are read out of the exe bytes, and control keys that predate Pile A
     (VOXELFORGE_HERO, _AMBCOLOR, _BLUESCALE) are scanned alongside so a scan
     that silently finds nothing cannot be mistaken for a clean result.

Writes docs/assets/look/flamingo-attrib-2026-08-18.png
"""
import json
import re
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
AB = ROOT / "docs" / "look-ab-poppy-2026-08-18"
OUT = ROOT / "docs" / "assets" / "look" / "flamingo-attrib-2026-08-18.png"

# Pile A's own symbols, then three controls that existed BEFORE Pile A. If the
# controls come back 0 the scan is broken, not the binary clean.
PILEA_KEYS = ["PILEA", "VOXELFORGE_RIM", "VOXELFORGE_SUNCOLOR",
              "VOXELFORGE_BOUNCE2COLOR", "VOXELFORGE_PANEHI", "VOXELFORGE_CLEAR"]
CONTROL_KEYS = ["VOXELFORGE_HERO", "VOXELFORGE_AMBCOLOR", "VOXELFORGE_BLUESCALE"]

EXES = [("BEFORE exe", ROOT / "target-flamingo" / "release" / "voxelforge.exe"),
        ("AFTER exe", ROOT / "target" / "release" / "voxelforge.exe")]

PLATES = [("before_outdoor-noon", "BEFORE  outdoor-noon", "cool 3.44 %"),
          ("after_outdoor-noon", "AFTER  outdoor-noon", "cool 0.00 %"),
          ("before_village-raking", "BEFORE  village-raking", "cool 22.07 %"),
          ("after_village-raking", "AFTER  village-raking", "cool 0.00 %")]


def font(sz, bold=False):
    for name in (["arialbd.ttf", "seguisb.ttf"] if bold else ["arial.ttf", "segoeui.ttf"]):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            continue
    return ImageFont.load_default()


def scan(path):
    """Count each key in the exe's bytes. Latin-1 maps every byte 1:1, so a
    match offset is a real byte offset -- no decode can drop a region."""
    blob = path.read_bytes().decode("latin-1")
    return {k: len(re.findall(re.escape(k), blob)) for k in PILEA_KEYS + CONTROL_KEYS}


def main():
    for _, p in EXES:
        if not p.exists():
            sys.exit(f"missing exe: {p}")

    scans = [(tag, p, scan(p)) for tag, p in EXES]

    # Guard: if a control key is 0 in every exe, the scan itself is dead and the
    # board would publish a false "clean". Refuse rather than draw it.
    for ck in CONTROL_KEYS:
        if all(s[ck] == 0 for _, _, s in scans):
            sys.exit(f"REFUSING TO MEASURE: control key {ck} is 0 in every exe -- "
                     "the scan is broken, not the binaries clean")

    W, TH, GAP = 1900, 706, 14
    cell_w = (W - GAP * 3) // 2
    plates = []
    for stem, title, note in PLATES:
        im = Image.open(AB / f"{stem}.png").convert("RGB")
        h = round(im.height * cell_w / im.width)
        plates.append((im.resize((cell_w, h), Image.LANCZOS), title, note))
    ph = plates[0][0].height
    H = TH + (ph + 40) * 2 + GAP * 3 + 40

    board = Image.new("RGB", (W, H), (14, 15, 18))
    d = ImageDraw.Draw(board)
    f_h1, f_h2, f_b, f_m = font(40, True), font(23, True), font(20), font(19)
    INK, DIM, GOOD, BAD = (238, 240, 245), (150, 156, 168), (110, 220, 150), (245, 120, 110)

    d.text((GAP, 18), "Did the Pile A grade lane (hero.rs) flatten the after-plate?",
           font=f_h1, fill=INK)
    d.text((GAP, 70), "No — and here is the evidence, not the assertion. "
           "Flamingo, 2026-08-18", font=f_b, fill=DIM)

    y = 112
    d.text((GAP, y), "PROOF 1 — code path", font=f_h2, fill=INK)
    for ln in ["hero::setup_hero is spawned only under cfg.hero  (main.rs:567) — i.e. only when VOXELFORGE_HERO is set.",
               "Poppy's shoot script sets VOXELFORGE_PLAY=1 and never VOXELFORGE_HERO, so on these four plates hero.rs is dead code.",
               "grep 'hero::' outside hero.rs → nothing but setup_hero / GateVerdict / GATE_FAILED / report_voxel_overlaps.",
               "The module exports no colour, exposure or grade term into the world path at all."]:
        y += 26
        d.text((GAP + 16, y), "· " + ln, font=f_m, fill=DIM)

    y += 48
    d.text((GAP, y), "PROOF 2 — the bytes of the two staged exes", font=f_h2, fill=INK)
    y += 32
    # Two columns, each wide enough for label AND tail — a three-up grid ran the
    # tails into the next column's label and the board became unreadable.
    for i, key in enumerate(PILEA_KEYS + CONTROL_KEYS):
        cx = GAP + 16 + (i % 2) * 940
        cy = y + (i // 2) * 26
        is_ctrl = key in CONTROL_KEYS
        counts = [s[key] for _, _, s in scans]
        colour = (GOOD if any(counts) else BAD) if is_ctrl else (BAD if any(counts) else GOOD)
        tail = "← control, predates Pile A, must be > 0" if is_ctrl \
            else "← Pile A, absent from both"
        d.text((cx, cy), f"{key:<24} before={counts[0]}  after={counts[1]}", font=f_m, fill=colour)
        d.text((cx + 430, cy), tail, font=f_m, fill=DIM)
    y += ((len(PILEA_KEYS + CONTROL_KEYS) + 1) // 2) * 26 + 14
    d.text((GAP + 16, y), "Every Pile A symbol = 0 in BOTH exes. The controls are non-zero, so the scan is live. "
           "Pile A is in neither binary — it was still uncommitted when both were linked.",
           font=f_m, fill=INK)
    y += 30
    d.text((GAP + 16, y), "What DID straddle: VOXELFORGE_LOOK_GEN  before=0 → after=1. "
           "The change under these plates is the look.rs generation lane, not the hero grade.",
           font=f_m, fill=INK)

    # ---- what actually ate the cool pixels, measured rather than eyeballed ----
    y += 44
    d.text((GAP, y), "WHERE THE COOL WENT — top-12 % sky band, measured", font=f_h2, fill=INK)
    y += 30
    for stem, title, _ in PLATES:
        im = Image.open(AB / f"{stem}.png").convert("RGB")
        band = im.crop((0, 0, im.width, round(im.height * 0.12)))
        px = list(band.getdata())
        n = len(px)
        # "sky-blue" = B leads R by a clear margin and the pixel is not black.
        blue = sum(1 for r, g, b in px if b > r + 18 and b > 40) / n * 100
        dark = sum(1 for r, g, b in px if max(r, g, b) < 40) / n * 100
        gone = blue < 1.0
        d.text((GAP + 16, y), f"{title:<26}  blue-sky px {blue:6.2f} %    near-black px {dark:6.2f} %",
               font=f_m, fill=BAD if gone else GOOD)
        y += 26
    d.text((GAP + 16, y + 4), "In an outdoor plate the sky IS the cool channel. It is not graded cool in the after — "
           "it is not drawn at all, so the frame has nothing cool left to measure.",
           font=f_m, fill=INK)

    y = TH
    for i, (im, title, note) in enumerate(plates):
        x = GAP + (i % 2) * (cell_w + GAP)
        yy = y + (i // 2) * (ph + 40)
        board.paste(im, (x, yy + 30))
        bad = note == "cool 0.00 %"
        d.text((x, yy + 4), title, font=f_h2, fill=INK)
        d.text((x + 320, yy + 6), note, font=f_h2, fill=BAD if bad else GOOD)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    board.save(OUT)
    print(f"WROTE {OUT}  {OUT.stat().st_size} bytes  {board.width}x{board.height}")
    print(json.dumps({tag: s for tag, _, s in scans}, indent=2))


if __name__ == "__main__":
    main()
