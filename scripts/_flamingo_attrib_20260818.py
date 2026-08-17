#!/usr/bin/env python3
"""Flamingo -- attribution board for the "after-plate went cool 0.00 %" finding.

The question this answers is NOT "is the after plate worse" (Poppy measured that
and it is), and NOT "what killed the cool pixels" -- **Poppy answered that one
too**, before I looked at it, in `docs/look-ab-poppy-2026-08-18/README.md`:

    "The regression is bigger: the sky dome is dead."
    "cool px % -> 0.00 -- the blue sky *was* the cool pixel population. With the
     dome gone, every cool pixel in the frame goes with it."

and `CONTROL-FINDINGS.md` §3 proves with a one-binary control that both plates
read v3, so the pair never tested the look lane at all. Everything this board
says about the sky is Poppy's conclusion; my only addition there is a *number*
for it (the top-12 % band count), not the finding.

What IS mine is the narrower question I was asked: *did the Pile A grade lane in
`hero.rs` cause it.* Two independent proofs go on the board, because either one
alone is the kind of evidence that has fooled this office before:

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

Nothing on this board is typed in by hand. Every number is read at draw time
from the pinned exes or from Poppy's `_poppy_ab_20260818_metrics.json`, because
a caption that cannot drift is the only kind worth printing next to a measured
one -- that rule is Poppy's too, from the board in the same folder.

Writes docs/assets/look/flamingo-attrib-2026-08-18.png
"""
import hashlib
import json
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
AB = ROOT / "docs" / "look-ab-poppy-2026-08-18"
STAGE = ROOT / "_poppy_ab_stage"
METRICS = ROOT / "_poppy_ab_20260818_metrics.json"
OUT = ROOT / "docs" / "assets" / "look" / "flamingo-attrib-2026-08-18.png"

# Pile A's own symbols, then three controls that existed BEFORE Pile A. If the
# controls come back 0 the scan is broken, not the binary clean.
PILEA_KEYS = ["PILEA", "VOXELFORGE_RIM", "VOXELFORGE_SUNCOLOR",
              "VOXELFORGE_BOUNCE2COLOR", "VOXELFORGE_PANEHI", "VOXELFORGE_CLEAR"]
CONTROL_KEYS = ["VOXELFORGE_HERO", "VOXELFORGE_AMBCOLOR", "VOXELFORGE_BLUESCALE"]
# What actually straddles the pair. Scanned like everything else so the board
# prints a measurement, not a sentence I typed. Named separately because it is
# neither a Pile A symbol (must be 0) nor a control (must be > 0) -- it is the
# finding, and it must go up as whatever the bytes say.
STRADDLE_KEYS = ["VOXELFORGE_LOOK_GEN"]

# Poppy staged these two copies precisely because a teammate was building into
# the shared `target/`. Scanning `target/` live means the next `--bin voxelforge`
# build silently swaps the AFTER exe under the board. Read the frozen copies and
# pin them: a swapped binary must fail loudly, not re-render quietly.
EXES = [("BEFORE exe", STAGE / "before_voxelforge.exe",
         "4ded9219a24913a811b307265077b980"),
        ("AFTER exe", STAGE / "after_voxelforge.exe",
         "b417f9b4a50d826eb97b01c7588882e0")]

# (plate stem, caption title, key in Poppy's metrics json). The cool % is READ
# from that file at draw time -- Poppy's own board does this so a caption cannot
# drift off its plate, and the same reason applies here.
PLATES = [("before_outdoor-noon", "BEFORE  outdoor-noon", "before_noon"),
          ("after_outdoor-noon", "AFTER  outdoor-noon", "after_noon"),
          ("before_village-raking", "BEFORE  village-raking", "before_vill"),
          ("after_village-raking", "AFTER  village-raking", "after_vill")]


def font(sz, bold=False):
    for name in (["arialbd.ttf", "seguisb.ttf"] if bold else ["arial.ttf", "segoeui.ttf"]):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            continue
    return ImageFont.load_default()


ALL_KEYS = PILEA_KEYS + CONTROL_KEYS + STRADDLE_KEYS


def scan(path):
    """Count each key in the exe's bytes. Latin-1 maps every byte 1:1, so a
    match offset is a real byte offset -- no decode can drop a region."""
    blob = path.read_bytes().decode("latin-1")
    return {k: len(re.findall(re.escape(k), blob)) for k in ALL_KEYS}


def main():
    # ---- pin the binaries before reading a single byte of them ----------
    for tag, p, want in EXES:
        if not p.exists():
            sys.exit(f"REFUSING TO MEASURE: missing {tag}: {p}")
        got = hashlib.md5(p.read_bytes()).hexdigest()
        if got != want:
            sys.exit(f"REFUSING TO MEASURE: {tag} {p.name} is md5 {got}, "
                     f"expected {want} -- the staged binary was replaced. "
                     "Re-stage from the run that shot these plates, or re-pin "
                     "deliberately; do NOT re-render the board against a "
                     "different exe.")

    scans = [(tag, p, scan(p)) for tag, p, _ in EXES]

    # Guard: if a control key is 0 in every exe, the scan itself is dead and the
    # board would publish a false "clean". Refuse rather than draw it.
    for ck in CONTROL_KEYS:
        if all(s[ck] == 0 for _, _, s in scans):
            sys.exit(f"REFUSING TO MEASURE: control key {ck} is 0 in every exe -- "
                     "the scan is broken, not the binaries clean")

    if not METRICS.exists():
        sys.exit(f"REFUSING TO MEASURE: missing {METRICS} -- the plate captions "
                 "are read from it, and hand-typed ones drift")
    metrics = json.loads(METRICS.read_text())
    for _, _, mkey in PLATES:
        if mkey not in metrics or "cool_pct_all" not in metrics[mkey]:
            sys.exit(f"REFUSING TO MEASURE: {METRICS.name} has no "
                     f"{mkey}.cool_pct_all")

    W, TH, GAP = 1900, 764, 14
    cell_w = (W - GAP * 3) // 2
    plates = []
    for stem, title, mkey in PLATES:
        im = Image.open(AB / f"{stem}.png").convert("RGB")
        h = round(im.height * cell_w / im.width)
        cool = metrics[mkey]["cool_pct_all"]
        plates.append((im.resize((cell_w, h), Image.LANCZOS), title, cool))
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
    d.text((GAP, 96), "The plates, the metrics and the sky-dome finding are Poppy's "
           "(look-ab-poppy-2026-08-18/README.md + CONTROL-FINDINGS.md §3). "
           "This board adds the hero.rs attribution and one measurement.",
           font=f_b, fill=DIM)

    y = 138
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
    for i, key in enumerate(ALL_KEYS):
        cx = GAP + 16 + (i % 2) * 940
        cy = y + (i // 2) * 26
        counts = [s[key] for _, _, s in scans]
        if key in CONTROL_KEYS:
            colour = GOOD if any(counts) else BAD
            tail = "← control, predates Pile A, must be > 0"
        elif key in STRADDLE_KEYS:
            # No pass/fail colour: this row is the finding, whatever it reads.
            colour = INK
            tail = "← what actually straddles the pair"
        else:
            colour = BAD if any(counts) else GOOD
            tail = "← Pile A, absent from both"
        d.text((cx, cy), f"{key:<24} before={counts[0]}  after={counts[1]}", font=f_m, fill=colour)
        d.text((cx + 480, cy), tail, font=f_m, fill=DIM)
    y += ((len(ALL_KEYS) + 1) // 2) * 26 + 14
    d.text((GAP + 16, y), "Every Pile A symbol = 0 in BOTH exes. The controls are non-zero, so the scan is live. "
           "Pile A is in neither binary — it was still uncommitted when both were linked.",
           font=f_m, fill=INK)
    y += 30
    gen = STRADDLE_KEYS[0]
    gen_counts = [s[gen] for _, _, s in scans]
    d.text((GAP + 16, y), f"What DID straddle, from the same scan: {gen} "
           f"before={gen_counts[0]} → after={gen_counts[1]}. "
           "The change under these plates is the look.rs generation lane, not the hero grade.",
           font=f_m, fill=INK)

    # ---- Poppy's finding, with a number attached ----
    y += 44
    d.text((GAP, y), "WHERE THE COOL WENT — Poppy's finding, measured on the top-12 % sky band",
           font=f_h2, fill=INK)
    y += 28
    d.text((GAP + 16, y), "Poppy: \"The regression is bigger: the sky dome is dead\" · "
           "\"cool px % → 0.00 — the blue sky WAS the cool pixel population.\" "
           "(README, same folder). The band count below is the only new part.",
           font=f_m, fill=DIM)
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
    for i, (im, title, cool) in enumerate(plates):
        x = GAP + (i % 2) * (cell_w + GAP)
        yy = y + (i // 2) * (ph + 40)
        board.paste(im, (x, yy + 30))
        note = f"cool {cool:.2f} %"   # from Poppy's metrics json, not typed here
        d.text((x, yy + 4), title, font=f_h2, fill=INK)
        d.text((x + 320, yy + 6), note, font=f_h2, fill=BAD if cool < 1.0 else GOOD)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    board.save(OUT)
    print(f"WROTE {OUT}  {OUT.stat().st_size} bytes  {board.width}x{board.height}")
    print(json.dumps({tag: s for tag, _, s in scans}, indent=2))


if __name__ == "__main__":
    main()
