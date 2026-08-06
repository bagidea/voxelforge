#!/usr/bin/env python3
"""Before/after sheet for a _poppy_shotset run -- one row per plate, BEFORE | AFTER.

Reads the manifest the harness wrote, so the labels come from the same table the shots
were fired from and cannot drift from what the pixels are (the failure the first beauty
contact sheet had: a tile captioned "RAKING LIGHT" that had been shot at plain GOLDEN).

Each row also carries the mean per-channel |delta| between the two frames. That is the
cheap tell for the null-A/B trap -- a pair shot off the same binary reads ~0 and the row
is flagged NO CHANGE instead of quietly looking like a result.

Usage: python scripts/_poppy_shotset_sheet.py <manifest.json> <out.png>
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent

TW = 620          # tile width; each frame is scaled to this, aspect preserved
PAD = 12
HEAD_H = 40       # per-row caption band
CAP_GAP = 18      # gap under the caption band before the frames start
BG = (16, 16, 18)
FG = (232, 232, 236)
DIM = (150, 150, 158)
OK = (120, 220, 150)
BAD = (245, 130, 110)
WARN = (240, 200, 110)


def font(size: int) -> ImageFont.FreeTypeFont:
    for name in ("segoeui.ttf", "arial.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def load(rel):
    if not rel:
        return None
    p = ROOT / rel
    if not p.exists():
        return None
    return Image.open(p).convert("RGB")


def scaled(im, w):
    return im.resize((w, max(1, round(im.height * w / im.width))), Image.LANCZOS)


def mean_delta(a, b):
    """Mean per-channel |delta| over the overlapping top-left region.

    The two sides can differ in size (a 1280x640 gate3 baseline vs a re-shoot, a
    1600x820 beauty plate vs its 19:53 original). Cropping to the common region is
    honest for "did anything move at all"; it is NOT a registered diff, so the number
    is a tell, not a grade. The grade is regrade.py's job.
    """
    if a is None or b is None:
        return None
    w = min(a.width, b.width)
    h = min(a.height, b.height)
    aa = np.asarray(a.crop((0, 0, w, h)), dtype=np.int16)
    bb = np.asarray(b.crop((0, 0, w, h)), dtype=np.int16)
    d = np.abs(aa - bb)
    return (
        [float(d[:, :, c].mean()) for c in range(3)],
        float((d.max(axis=2) > 3).mean() * 100.0),
        (a.size == b.size),
    )


def main():
    if len(sys.argv) != 3:
        print(__doc__.strip())
        return 2
    manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8-sig"))
    out = Path(sys.argv[2])

    shots = manifest.get("shots", [])
    rows = []
    for s in shots:
        before, after = load(s.get("before")), load(s.get("after"))
        if before is None and after is None:
            continue
        rows.append((s, before, after))
    if not rows:
        print("SHEET: nothing to draw (no frames on disk)")
        return 1

    f_title = font(20)
    f_sub = font(14)
    f_small = font(13)
    f_hdr = font(26)

    tiles = []
    for s, before, after in rows:
        b = scaled(before, TW) if before else None
        a = scaled(after, TW) if after else None
        th = max((b.height if b else 0), (a.height if a else 0))
        tiles.append((s, b, a, th))

    # A baseline-only run (-BeforeSide) has no before column at all. Drawing eight
    # "BEFORE: no frame" boxes next to it reads as eight failures instead of one
    # complete set, so drop the column when nothing in the manifest fills it.
    sides = [(i, lab) for i, lab in ((0, "BEFORE"), (1, "AFTER"))
             if any((t[1] if i == 0 else t[2]) is not None for t in tiles)]

    top = 84
    W = PAD * (len(sides) + 1) + TW * len(sides)
    # Must match the per-row advance in the draw loop below exactly (HEAD_H + CAP_GAP
    # for the caption, th for the frames, PAD*2 between rows). It did not: the loop
    # added CAP_GAP and this sum did not, so the canvas came up 18px short per row and
    # the last row was cropped off the bottom of the sheet -- on the one file that goes
    # to the CEO. Both sides now read the same constants.
    H = top + sum(t[3] + HEAD_H + CAP_GAP + PAD * 2 for t in tiles) + PAD

    sheet = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(sheet)

    d.text((PAD, 16), "VOXELFORGE  -  BEFORE / AFTER  shotset", font=f_hdr, fill=FG)
    exe = manifest.get("exe", "?")
    d.text(
        (PAD, 52),
        f"after exe: {exe}  |  {manifest.get('exe_mtime','?')[:19]}  |  "
        f"sha {str(manifest.get('exe_sha256',''))[:12]}  |  commit {manifest.get('commit','?')}",
        font=f_small,
        fill=DIM,
    )

    y = top
    for s, b, a, th in tiles:
        # caption band
        d.text((PAD, y), s["key"], font=f_title, fill=FG)
        stat = mean_delta(
            load(s.get("before")), load(s.get("after"))
        )
        if stat is None:
            if len(sides) == 1:
                # Single-side sheet: there is nothing to diff and nothing missing.
                d.text((PAD + 190, y + 4), f"{sides[0][1]} only", font=f_sub, fill=DIM)
            else:
                side = "AFTER missing" if a is None else "BEFORE missing"
                d.text((PAD + 190, y + 4), side, font=f_sub, fill=BAD)
        else:
            (dr, dg, db), moved, same_size = stat
            colour = OK if moved >= 1.0 else BAD
            tag = "" if moved >= 1.0 else "  NO CHANGE - same binary?"
            geo = "" if same_size else "  (sizes differ - crop-compared)"
            d.text(
                (PAD + 190, y + 4),
                f"mean |d| R {dr:.1f} G {dg:.1f} B {db:.1f}   pixels moved {moved:.1f}%{tag}{geo}",
                font=f_sub,
                fill=colour if not tag else WARN,
            )
        # A block-count change between the two sides means maps/edhari.json was edited,
        # so the delta above is at least partly a world edit -- say so ON the row, not in
        # a log nobody re-reads. This is the exact trap that made the published 19:53
        # frames unusable as baselines (8513 blocks then, 8838 now).
        if s.get("map_drift"):
            d.text(
                (PAD, y + 22),
                f"MAP DRIFT: {s.get('before_map_blocks')} -> {s.get('map_blocks')} blocks "
                f"-- this delta includes a world edit, not just the binary",
                font=f_small,
                fill=BAD,
            )
        else:
            d.text((PAD, y + 22), s.get("note", ""), font=f_small, fill=DIM)
        env_bits = " ".join(x for x in [s.get("argv", ""), s.get("env", "")] if x)
        if s.get("cine"):
            env_bits += f"  CINE {s['cine']}"
        if s.get("map_blocks"):
            env_bits += f"  map {s['map_blocks']} blocks"
        d.text((PAD, y + 38), env_bits[:210], font=f_small, fill=(120, 140, 170))

        ty = y + HEAD_H + CAP_GAP
        for col, (i, label) in enumerate(sides):
            im = b if i == 0 else a
            x = PAD + col * (TW + PAD)
            if im is None:
                d.rectangle([x, ty, x + TW, ty + th], outline=(70, 50, 50))
                d.text((x + 12, ty + 12), f"{label}: no frame", font=f_sub, fill=BAD)
                continue
            sheet.paste(im, (x, ty))
            d.rectangle([x, ty, x + TW - 1, ty + im.height - 1], outline=(60, 60, 68))
            # Chip and path share one opaque strip: drawn as two boxes they overlapped the
            # render and the path was unreadable against sky.
            src = str(s.get("before") if i == 0 else s.get("after") or "")
            d.rectangle([x, ty, x + TW - 1, ty + 20], fill=(0, 0, 0))
            d.text((x + 6, ty + 3), label, font=f_small, fill=FG)
            d.text((x + 66, ty + 3), src[-70:], font=f_small, fill=DIM)

        y = ty + th + PAD * 2

    # Cheap tell that the canvas maths and the draw loop still agree. A cropped bottom
    # row is invisible in the exit code and in the printed size -- only in the pixels --
    # so fail loudly here instead of shipping a sheet that is missing a plate.
    if y + PAD != H:
        print(f"SHEET FAIL: layout needs {y + PAD}px, canvas is {H}px "
              f"({'cropped' if y + PAD > H else 'over-tall'} by {abs(y + PAD - H)}px)")
        return 1

    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"SHEET {out}  {sheet.width}x{sheet.height}  rows={len(tiles)}  layout-check OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
