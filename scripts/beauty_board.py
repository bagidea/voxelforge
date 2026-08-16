#!/usr/bin/env python3
"""Assemble the per-lane before/after contact board — ONE png the CEO opens once.

Scans docs/assets/ fresh on every run and lays the current art-order evidence out
as rows grouped by lane (look / enemies / vfx / ai / beauty-shot).  Every row is
one before->after pair: BEFORE on the left, AFTER on the right, with the lane
name + source filename + file mtime under each panel.  Nothing here draws game
content — every panel is a resize of a PNG that actually exists on disk, and a
pair whose AFTER has no matching BEFORE renders an explicit "no before" cell
rather than being silently dropped.

Frame class is CALLER-DECLARED, never guessed from pixel dimensions — the exact
lesson from 1bc06f1.  --frame-class is the same two values the art-order grader
uses (scripts/art_order_grade.py: FRAME_CLASSES = "environment" | "portrait"):
it decides the cell aspect, not the image content, and the board refuses to run
if you don't pick one of the two known values.

    python scripts/beauty_board.py                          # environment, out docs/assets/beauty-board.png
    python scripts/beauty_board.py --frame-class portrait
    python scripts/beauty_board.py --out docs/assets/pairs/beauty-board.png

Pairing is discovered, not hard-coded, so re-running after each lane submits
picks the new work up automatically:

    look        docs/assets/look/*_after.png            <- *_before.png
    enemies     docs/assets/enemies/*_after.png         <- _before_husk.png (one shared before)
    vfx         docs/assets/pairs/*-b-after.png         <- *-a-before.png
    ai          docs/assets/ai/*-b-after*.png           <- matching *-a-before*.png
    beauty-shot docs/assets/look/beauty/ladder/*-win-atmosfog.png
                                                         <- look/beauty/probe/<pose>.png
"""
import argparse
import sys
import time
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT_DEFAULT = ROOT / "docs" / "assets" / "beauty-board.png"

# Same vocabulary as scripts/art_order_grade.py — the board never infers this
# from image width/height; the caller says it out loud.
FRAME_CLASSES = ("environment", "portrait")

BG = (17, 18, 22)
PANEL_BG = (24, 26, 32)
INK = (233, 237, 245)
DIM = (150, 156, 168)
RULE = (58, 61, 72)
WARN = (255, 176, 120)

# Cell geometry per frame class.  environment -> 16:9 wide cell, portrait ->
# 9:16 tall cell.  All panels in one board share the SAME declared class.
CELL = {"environment": (640, 360), "portrait": (360, 640)}

PAD = 14
CAP_H = 104           # three caption lines under each panel
LANE_H = 40           # lane section band
HEAD_H = 74           # board title block
FOOT_H = 30

LANE_COLORS = {
    "look": (255, 200, 118),
    "enemies": (255, 130, 130),
    "vfx": (140, 214, 255),
    "ai": (158, 255, 170),
    "beauty-shot": (216, 158, 255),
}


def font(sz, bold=False):
    for p in (r"C:\Windows\Fonts\segoeuib.ttf" if bold else r"C:\Windows\Fonts\segoeui.ttf",
              r"C:\Windows\Fonts\arialbd.ttf" if bold else r"C:\Windows\Fonts\arial.ttf",
              r"C:\Windows\Fonts\consolab.ttf" if bold else r"C:\Windows\Fonts\consola.ttf"):
        try:
            return ImageFont.truetype(p, sz)
        except OSError:
            continue
    return ImageFont.load_default()


class Pair:
    """One before->after row.  before/after are Paths or None (None renders the
    explicit "no before" / "no after" cell — never a silent skip)."""

    def __init__(self, lane, stem, before, after, before_note=""):
        self.lane = lane
        self.stem = stem
        self.before = before
        self.after = after
        self.before_note = before_note


# ------------------------------------------------------------- discovery ---

def _pngs(*dirs):
    out = []
    for d in dirs:
        p = d if isinstance(d, Path) else ROOT / d
        if p.is_dir():
            out.extend(sorted(q for q in p.glob("*.png") if q.is_file()))
    return out


def discover_look():
    pairs = []
    for after in _pngs("docs/assets/look"):
        if after.stem.endswith("_after"):
            bname = after.stem[: -len("_after")] + "_before.png"
        elif after.stem.endswith("-after"):
            bname = after.stem[: -len("-after")] + "-before.png"
        else:
            continue
        before = after.with_name(bname)
        pairs.append(Pair("look", after.stem.rsplit("_", 1)[0],
                          before if before.is_file() else None, after))
    return pairs


def discover_enemies():
    before = ROOT / "docs" / "assets" / "enemies" / "_before_husk.png"
    before = before if before.is_file() else None
    pairs = []
    for after in _pngs("docs/assets/enemies"):
        if not after.stem.endswith("_after"):
            continue  # skips *_silhouette.png
        pairs.append(Pair("enemies", after.stem[: -len("_after")], before, after))
    return pairs


def discover_vfx():
    pairs = []
    for after in _pngs("docs/assets/pairs"):
        if not after.stem.endswith("-b-after"):
            continue  # skips *-sheet.png and *-control.png
        before = after.with_name(after.stem[: -len("-b-after")] + "-a-before.png")
        pairs.append(Pair("vfx", after.stem[: -len("-b-after")],
                          before if before.is_file() else None, after))
    return pairs


def discover_ai():
    pairs = []
    for after in _pngs("docs/assets/ai"):
        if "-b-after" not in after.stem:
            continue  # skips _paircheck_*.png, runlog, mp4
        left, _, right = after.stem.partition("-b-after")
        before = after.with_name(left + "-a-before" + right + ".png")
        stem = right.lstrip("-") if right else left.rsplit("-", 1)[-1]
        pairs.append(Pair("ai", stem, before if before.is_file() else None, after))
    return pairs


def discover_beauty_shot():
    ladder = ROOT / "docs" / "assets" / "look" / "beauty" / "ladder"
    probe = ROOT / "docs" / "assets" / "look" / "beauty" / "probe"
    pairs = []
    for after in _pngs(ladder):
        if not after.stem.endswith("-win-atmosfog"):
            continue
        stem = after.stem[: -len("-win-atmosfog")]
        before = probe / (stem + ".png")
        pairs.append(Pair("beauty-shot", stem,
                          before if before.is_file() else None, after))
    return pairs


LANES = [
    ("look", "LOOK — post-stack relights", discover_look),
    ("enemies", "ENEMIES — new enemy visuals", discover_enemies),
    ("vfx", "VFX — per-beat emitter pass", discover_vfx),
    ("ai", "AI — archetype behaviours", discover_ai),
    ("beauty-shot", "BEAUTY-SHOT — graded win frames", discover_beauty_shot),
]


# -------------------------------------------------------------- painting ---

def _stamp(path):
    try:
        st = path.stat()
        with Image.open(path) as im:
            w, h = im.size
        return ("%dx%d   %s" % (w, h,
                time.strftime("%Y-%m-%d %H:%M", time.localtime(st.st_mtime))))
    except OSError:
        return "(missing)"


def _fit(im, cw, ch):
    """Contain-fit: whole frame visible, letterboxed on the panel bg.  Nothing is
    ever cropped away on a comparison board — the point of a shot must not be
    hidden.  For today's 1280x720 sources in a 640x360 environment cell this is
    an exact scale, no bars."""
    im = im.convert("RGB")
    scale = min(cw / im.width, ch / im.height)
    nw, nh = max(1, round(im.width * scale)), max(1, round(im.height * scale))
    im = im.resize((nw, nh), Image.LANCZOS)
    tile = Image.new("RGB", (cw, ch), PANEL_BG)
    tile.paste(im, ((cw - nw) // 2, (ch - nh) // 2))
    return tile


def _draw_panel(sheet, d, x, y, cw, ch, path, lane, tag, note=""):
    d.rectangle([x, y, x + cw - 1, y + ch - 1], outline=RULE)
    if path is None:
        f = font(30, True)
        # the requirement: a missing BEFORE is a labelled empty cell, not a gap
        label = "no before" if tag == "BEFORE" else "no after"
        tw = d.textlength(label, font=f)
        d.text((x + (cw - tw) / 2, y + ch / 2 - 24), label, font=f, fill=WARN)
        if note:
            sn = font(14)
            tw = d.textlength(note, font=sn)
            d.text((x + (cw - tw) / 2, y + ch / 2 + 14), note, font=sn, fill=DIM)
    else:
        sheet.paste(_fit(Image.open(path), cw, ch), (x, y))

    cy = y + ch + 8
    d.text((x, cy), f"{lane}  ·  {tag}", font=font(19, True),
           fill=LANE_COLORS.get(lane, INK))
    d.text((x, cy + 28), path.name if path else "(no file)", font=font(16), fill=INK)
    d.text((x, cy + 52), _stamp(path) if path else note, font=font(14), fill=DIM)


def render(pairs, out, frame_class):
    cw, ch = CELL[frame_class]
    W = PAD + 2 * (cw + PAD)
    H = HEAD_H + sum(LANE_H + CAP_H + ch + PAD for _ in pairs) + FOOT_H

    sheet = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(sheet)

    d.text((PAD, 14), "VOXELFORGE — per-lane before/after board", font=font(26, True), fill=INK)
    d.text((PAD, 46),
           f"frame-class: {frame_class}  (caller-declared, never inferred from pixels)  ·  "
           f"generated {time.strftime('%Y-%m-%d %H:%M')}  ·  "
           f"{sum(1 for p in pairs if p.before is None)} row(s) missing a before frame",
           font=font(13), fill=DIM)

    y = HEAD_H
    no_before = 0
    for p in pairs:
        lane_col = LANE_COLORS.get(p.lane, INK)
        d.rectangle([PAD, y, W - PAD - 1, y + LANE_H - 1], fill=(30, 32, 40))
        d.rectangle([PAD, y, PAD + 6, y + LANE_H - 1], fill=lane_col)
        d.text((PAD + 16, y + 9), f"{p.lane.upper()}  —  {p.stem}", font=font(20, True),
               fill=lane_col)
        y += LANE_H

        if p.before is None:
            no_before += 1
        _draw_panel(sheet, d, PAD, y, cw, ch, p.before, p.lane, "BEFORE",
                    note="before frame not yet captured — shown, not skipped")
        _draw_panel(sheet, d, PAD + cw + PAD, y, cw, ch, p.after, p.lane, "AFTER")
        y += ch + CAP_H + PAD

    d.line([PAD, H - FOOT_H - 8, W - PAD, H - FOOT_H - 8], fill=RULE)
    d.text((PAD, H - FOOT_H + 6),
           "scripts/beauty_board.py  ·  every panel a resize of a PNG on disk  ·  "
           f"{len(pairs)} rows, {no_before} missing before(s) kept visible",
           font=font(12), fill=DIM)

    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return sheet.size, no_before


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=str(OUT_DEFAULT))
    ap.add_argument("--frame-class", default="environment", choices=FRAME_CLASSES,
                    help="environment: 16:9 cells (default) · portrait: 9:16 cells "
                         "— declared, never guessed from the image (1bc06f1)")
    a = ap.parse_args(argv[1:])

    pairs, notes = [], []
    for lane, _label, fn in LANES:
        got = fn()
        pairs.extend(got)
        missing = sum(1 for p in got if p.before is None)
        print(f"{lane:<12} {len(got):2d} pair(s)  {missing} missing before")
        notes.append(f"{lane}: {len(got)} pair(s)")

    if not pairs:
        print("REFUSED: no before/after pairs discovered under docs/assets/", file=sys.stderr)
        return 1

    size, no_before = render(pairs, Path(a.out), a.frame_class)
    print(f"WROTE {a.out}  ({size[0]}x{size[1]})  "
          f"frame-class={a.frame_class}  {no_before} missing before(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
