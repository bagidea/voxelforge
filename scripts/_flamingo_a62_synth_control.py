#!/usr/bin/env python3
"""Instrument control for the A6.2 accent scanner — prove a 0-px reading means
the ART is missing, not that the SCANNER is blind.

The negative control (`_flamingo_a62_accent_presence.py` on every pre-fix frame)
reads 0 px, 3/3. On its own that is ambiguous: a scanner that returned 0 on
everything would print exactly the same table. So before that 0 is allowed to
carry a verdict, plant a KNOWN accent into the very same frames, at the very same
screen coordinate the clasp is predicted to land on, and require the scanner to
recover it.

The coordinate is not chosen for convenience: **(1337, 842)** on the 2560x1360
de-HUDded boot frame is where `docs/VERDICT-a6-teal-accent-2026-08-11.md` projected
the clasp's torso-local point through `grade_character.py`'s own camera math, and
then cropped at 5x to find flat, featureless cloak colour. Planting there is what
makes this a control on the 08-11 finding rather than a generic self-test.

That site is kept, but it is NOT stored as a pixel pair any more. A pixel pair is
bound to one plate size: run this script bare on the current 1280x640 plate class
and (1337, 842) lands outside the frame's hero entirely, the on-hero rungs recover
0 px, and the whole control reports FAIL — loud and correct, but for a reason that
has nothing to do with the render. So the site is stored as the fraction it occupies
of the hero's own bbox (`PRED_FX/PRED_FY`, measured from that same 08-11 plate:
bbox x1057-1501 y598-1213) and re-projected onto whatever plate is handed in.
Round-trip proof: on the 08-11 plate the fraction resolves back to (1337, 842)
exactly; on the 1280x640 08-14 plate it resolves to (668, 380), 39 px clear of the
silhouette edge. The patch size scales with the plate the same way (frame_w/160 ->
16 px at 2560, 8 px at 1280), and the off-hero rung is placed in whichever frame
corner is furthest from the hero, not at a fixed (60, 60).

A hand-passed `--x/--y` is still honoured, and is checked against the hero body
before anything is planted: landing off the hero says so in one line instead of
silently producing a 0-px "FAIL" that reads like a render defect.

Three rungs, because a lit gem is not a flat swatch and because "on the hero" is
a load-bearing clause that has to be falsifiable:
  * full value  #4FC9D6  — the nominal accent, on the hero. Gate must PASS.
  * 45 % value           — a gem sitting in the cloak's shade. This is the rung
                           that matters for the floor: the L>=20 degeneracy guard
                           must not eat a legitimately shaded gem. #4FC9D6 only
                           falls under L 20 below ~11.4 % of full value, so 45 %
                           keeps >4x headroom and MUST survive.
  * off-hero             — the SAME gem at full value planted in a frame corner,
                           far from the character's bbox. The scanner must SEE it
                           (whole-frame px == planted px) but the gate must
                           **FAIL** it. Without this rung the hero-mask clause has
                           no control at all: the mask is recovered as
                           frame-minus-overlay, so before 2026-08-14 a planted
                           pixel joined the mask by construction and a gem in the
                           far corner PASSed (mask grew 33,505 -> 33,569,
                           exactly the 64 px of the patch). See `hero_mask()`.

This script writes the charmask beside every planted plate itself (copied from the
source frame's), so the gate command in `docs/look-acceptance-rubric.md` runs on
these outputs as written — no manual copying, no renaming. Without a charmask the
gate reports UNRELIABLE, which is not a positive control.

PASS for this control = the two on-hero rungs recover the planted pixel count
exactly, with a bbox centred on the plant site and a minor axis that clears the
read floor, AND every rung's gate verdict matches what it was planted to prove
(PASS / PASS / FAIL).

Usage:
    python scripts/_flamingo_a62_synth_control.py FRAME [--x X --y Y] [--size N]
                                                  [--off-x X --off-y Y]
    (bare is the intended form — every default is derived from FRAME's own hero mask)

EXIT CODE (added 2026-08-14): the run printed `INSTRUMENT CONTROL: PASS|FAIL` and
exited 0 either way, so a caller reading `$?` was told the instrument was sound by
a run whose own last line said it was not.

    0 = the control passed
    1 = the control failed, or the inputs cannot carry it (no charmask, no hero
        body, a hand-passed site off the hero)
    2 = the stored torso fraction no longer round-trips onto the plate it was
        measured on — the instrument is stale and nothing was planted

Round-trip, re-run 2026-08-14 on both live plate classes:
    2560x1360 (zfix 08-11): site -> (1337,842), patch 16x16, off-hero (2543,16)
    1280x640  (teal 08-14): site -> ( 668,380), patch  8x8,  off-hero (8,8)
both PASS 3/3 rungs. `--x 1337 --y 842` on the 1280 plate is REFUSED (exit 1),
which is the check that the old pixel pair can no longer be smuggled in.
"""
import argparse
import shutil
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _flamingo_a62_accent_presence import (GEM_SRGB, L_MIN, MIN_AXIS_PX, hero_mask,
                                           measure, verdict_of)

# See module docstring — the 08-11 pixel-projection site (1337, 842), expressed as
# its share of that plate's hero bbox (x1057-1501, y598-1213) so it survives a
# change of plate size. NOT an arbitrary spot, and NOT a pixel pair.
PRED_FX, PRED_FY = 0.629213, 0.396104
PRED_SITE = "(1337,842) on the 2560x1360 08-11 plate"
# The plate those fractions were measured on, kept so the claim above can be
# CHECKED rather than believed. `hero_mask()` has already been rewritten once
# (83d8630); if it is rewritten again the 08-11 bbox moves, these fractions
# quietly resolve somewhere else, and the control would still print PASS — at a
# site that is no longer the projected clasp. `assert_roundtrip()` runs on every
# invocation and is pure arithmetic, so it costs nothing.
REF_BBOX = (1057, 1501, 598, 1213)      # x0, x1, y0, y1 on the 08-11 plate
REF_SITE = (1337, 842)
# 16 px at 2560 wide — the size the 08-11 control ran at — scaled per plate.
SIZE_PER_PX = 1.0 / 160.0
CHARMASK_SUFFIX = "-charmask.png"


def charmask_of(frame):
    return Path(str(Path(frame).with_suffix("")) + CHARMASK_SUFFIX)


def bbox_of(mask):
    ys, xs = np.nonzero(mask)
    return int(xs.min()), int(xs.max()), int(ys.min()), int(ys.max())


def project(bbox):
    """The stored torso fraction, resolved onto one hero bbox. One implementation,
    so the round-trip check below tests the same arithmetic the run uses."""
    x0, x1, y0, y1 = bbox
    bw, bh = x1 - x0 + 1, y1 - y0 + 1
    return int(round(x0 + PRED_FX * bw)), int(round(y0 + PRED_FY * bh))


def assert_roundtrip():
    """PRED_FX/PRED_FY must still resolve to REF_SITE on REF_BBOX. Returns an
    error string, or None when the stored site is self-consistent."""
    got = project(REF_BBOX)
    if got != REF_SITE:
        return (f"stored site is stale: PRED_FX/PRED_FY resolve to {got} on the "
                f"08-11 hero bbox x{REF_BBOX[0]}-{REF_BBOX[1]} y{REF_BBOX[2]}-"
                f"{REF_BBOX[3]}, but that plate's projected clasp is {REF_SITE}. "
                f"Re-measure the fractions before planting anything.")
    return None


def patch_box(x, y, size, shape):
    """The exact pixels `plant()` will paint — kept in one place so the fit checks
    below test the patch that actually lands, not an approximation of it."""
    h, w = shape[:2]
    half = size // 2
    return (max(0, x - half), min(w, x - half + size),
            max(0, y - half), min(h, y - half + size))


def fits_on_hero(mask, x, y, size):
    x0, x1, y0, y1 = patch_box(x, y, size, mask.shape)
    if (x1 - x0) * (y1 - y0) != size * size:
        return False           # clipped by the frame edge
    return bool(mask[y0:y1, x0:x1].all())


def default_size(mask):
    """Scale the gem with the plate, but never below the read floor it must clear."""
    w = mask.shape[1]
    return max(int(MIN_AXIS_PX) * 2, int(round(w * SIZE_PER_PX)))


def resolve_site(mask, size):
    """Project the 08-11 torso site onto THIS plate's hero bbox.

    Returns (x, y, note). If the projected point cannot hold the whole patch on the
    hero (a thin limb, an odd pose), snap to the nearest centre that can rather than
    planting half a gem into the background — a half-off patch would make the
    on-hero rungs recover fewer px than planted and the control would fail for a
    reason that is about geometry, not about the gate.
    """
    x0, x1, y0, y1 = bbox_of(mask)
    ax, ay = project((x0, x1, y0, y1))
    if fits_on_hero(mask, ax, ay, size):
        return ax, ay, f"projected from {PRED_SITE} onto hero bbox x{x0}-{x1} y{y0}-{y1}"
    room = ndimage.binary_erosion(mask, structure=np.ones((size + 2, size + 2), bool))
    cy, cx = np.nonzero(room)
    if len(cx):
        for i in np.argsort((cx - ax) ** 2 + (cy - ay) ** 2)[:512]:
            px, py = int(cx[i]), int(cy[i])
            if fits_on_hero(mask, px, py, size):
                d = int(round(float(np.hypot(px - ax, py - ay))))
                return px, py, (f"projected site ({ax},{ay}) could not hold a "
                                f"{size}x{size} patch — snapped {d} px to ({px},{py})")
    return None, None, (f"no {size}x{size} spot on the hero body of this plate "
                        f"(hero bbox x{x0}-{x1} y{y0}-{y1})")


def resolve_offhero(mask, size):
    """The frame corner furthest from the hero that the patch clears entirely."""
    h, w = mask.shape[:2]
    ys, xs = np.nonzero(mask)
    hx, hy = float(xs.mean()), float(ys.mean())
    inset = size
    corners = [(inset, inset), (w - 1 - inset, inset),
               (inset, h - 1 - inset), (w - 1 - inset, h - 1 - inset)]
    corners.sort(key=lambda p: -((p[0] - hx) ** 2 + (p[1] - hy) ** 2))
    for x, y in corners:
        cx0, cx1, cy0, cy1 = patch_box(x, y, size, mask.shape)
        if (cx1 - cx0) * (cy1 - cy0) == size * size and not mask[cy0:cy1, cx0:cx1].any():
            return int(x), int(y)
    return None, None


def plant(src, dst, x, y, size, value):
    """Paint a `size`x`size` #4FC9D6 patch at `value` scale centred on (x, y).

    The source frame's charmask is copied beside `dst` as well, so the planted
    plate is a complete gate input: `_flamingo_a62_accent_presence.py <dst> --gate`
    runs on it exactly as the rubric writes the command. (The charmask is copied
    UNMODIFIED on purpose — the planted pixels must not be able to redraw the hero
    mask underneath themselves; that is the whole point of the off-hero rung.)
    """
    im = Image.open(src).convert("RGB")
    a = np.asarray(im, dtype=np.uint8).copy()
    h, w = a.shape[:2]
    rgb = np.array([round(c * 255.0 * value) for c in GEM_SRGB], dtype=np.uint8)
    half = size // 2
    x0, x1 = max(0, x - half), min(w, x - half + size)
    y0, y1 = max(0, y - half), min(h, y - half + size)
    a[y0:y1, x0:x1] = rgb
    Image.fromarray(a).save(dst)
    shutil.copyfile(charmask_of(src), charmask_of(dst))
    L = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
    return (x1 - x0) * (y1 - y0), tuple(int(v) for v in rgb), float(L)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("frame")
    ap.add_argument("--x", type=int, default=None,
                    help="override the plant site (default: the 08-11 torso site "
                         "re-projected onto this plate's own hero bbox)")
    ap.add_argument("--y", type=int, default=None)
    ap.add_argument("--size", type=int, default=None,
                    help="patch size in px (default: scaled from frame width)")
    ap.add_argument("--off-x", type=int, default=None,
                    help="x of the off-hero rung (default: the frame corner "
                         "furthest from the hero)")
    ap.add_argument("--off-y", type=int, default=None)
    ap.add_argument("--outdir", default="_fl_a62_control")
    args = ap.parse_args()

    stale = assert_roundtrip()
    if stale:
        print(f"!! {stale}")
        return 2

    src_cm = charmask_of(args.frame)
    if not src_cm.exists():
        print(f"!! no charmask beside the frame ({src_cm}) — run grade_character.py "
              f"first.\n   Without it every rung reports UNRELIABLE, which proves "
              f"nothing in either direction.")
        return 1

    # Every default below is read off THIS plate's hero body, so the control travels
    # between plate classes instead of silently pointing at last month's resolution.
    mask, _, _ = hero_mask(args.frame, CHARMASK_SUFFIX)
    if mask is None or not mask.any():
        print(f"!! the charmask beside {args.frame} does not recover a hero body "
              f"(empty, or a different size than the frame) — re-run "
              f"grade_character.py on this exact frame.")
        return 1
    fh, fw = mask.shape[:2]
    hx0, hx1, hy0, hy1 = bbox_of(mask)

    size = args.size if args.size is not None else default_size(mask)
    if size < MIN_AXIS_PX:
        print(f"!! --size {size} is under the scanner's own read floor "
              f"({MIN_AXIS_PX:.0f} px) — the on-hero rungs would be planted to fail.")
        return 1

    if (args.x is None) != (args.y is None):
        print("!! pass --x and --y together, or neither.")
        return 1
    if args.x is not None:
        x, y, site_note = args.x, args.y, "hand-passed --x/--y"
        if not (0 <= x < fw and 0 <= y < fh) or not fits_on_hero(mask, x, y, size):
            on = 0 <= x < fw and 0 <= y < fh and bool(mask[y, x])
            print(f"!! --x/--y ({x},{y}) does not carry a {size}x{size} patch on the "
                  f"hero body of this plate.\n"
                  f"   frame {fw}x{fh}, hero bbox x{hx0}-{hx1} y{hy0}-{hy1} — the point "
                  f"is {'on the hero but too close to the silhouette edge' if on else 'off the hero'}.\n"
                  f"   Planting there makes the on-hero rungs read short and the control "
                  f"FAIL for geometry, not for the gate. Pass a point inside the hero, "
                  f"or drop --x/--y and let the site be derived.")
            return 1
    else:
        x, y, site_note = resolve_site(mask, size)
        if x is None:
            print(f"!! {site_note}.\n"
                  f"   The derived site does not fit this plate class — pass --x/--y "
                  f"explicitly (a flat spot on the hero's torso).")
            return 1

    if (args.off_x is None) != (args.off_y is None):
        print("!! pass --off-x and --off-y together, or neither.")
        return 1
    if args.off_x is not None:
        ox, oy, off_note = args.off_x, args.off_y, "hand-passed --off-x/--off-y"
        ox0, ox1, oy0, oy1 = patch_box(ox, oy, size, mask.shape)
        if mask[oy0:oy1, ox0:ox1].any():
            print(f"!! --off-x/--off-y ({ox},{oy}) overlaps the hero body — that rung "
                  f"exists to prove the mask clause FAILs an accent that is NOT on the "
                  f"hero, so it must be planted clear of it. Drop the flags to use the "
                  f"furthest free corner.")
            return 1
    else:
        ox, oy = resolve_offhero(mask, size)
        off_note = "furthest frame corner clear of the hero"
        if ox is None:
            print("!! no frame corner is clear of the hero on this plate — pass "
                  "--off-x/--off-y for the off-hero rung.")
            return 1

    out = Path(args.outdir)
    out.mkdir(parents=True, exist_ok=True)

    print(f"instrument control — plant #4FC9D6 at ({x},{y}) size {size}x{size}")
    print(f"source frame: {args.frame}  ({fw}x{fh}, hero bbox x{hx0}-{hx1} y{hy0}-{hy1})")
    print(f"charmask    : {src_cm}  (copied beside every planted plate)")
    print(f"on-hero site: {site_note}")
    print(f"off-hero    : ({ox},{oy}) — {off_note}")
    print("-" * 104)

    # rung 0: the untouched frame. Must read 0 — otherwise the plant proves nothing.
    base = measure(args.frame, CHARMASK_SUFFIX)
    print(f"{'rung':>10}  {'planted':>8}  {'recovered':>9}  {'bbox':>9}  "
          f"{'centre':>13}  {'L':>6}  {'gate':>6}  verdict")
    print(f"{'unplanted':>10}  {'-':>8}  {base['px']:9d}  {'-':>9}  {'-':>13}  "
          f"{'-':>6}  {verdict_of(base)[0][:4]:>6}  "
          + ("clean baseline" if base["px"] == 0
             else "!! frame already has accent — control void"))

    ok = base["px"] == 0
    rungs = (("full", 1.00, x, y, "PASS"),
             ("45value", 0.45, x, y, "PASS"),
             ("offhero", 1.00, ox, oy, "FAIL"))
    for label, value, px, py, want in rungs:
        # NOTE: plant into a *-nohud2.png name so the graders' de-HUD guard and the
        # scanner's charmask lookup behave exactly as they do on a real plate.
        dst = out / f"synth-{label}-nohud2.png"
        n, rgb, L = plant(args.frame, dst, px, py, size, value)
        r = measure(str(dst), CHARMASK_SUFFIX)
        got, why = verdict_of(r)
        top = r["top"][0] if r["top"] else None
        exact = top is not None and r["px"] == n
        centred = top is not None and abs(top["cx"] - px) <= 1 and abs(top["cy"] - py) <= 1
        reads = top is not None and top["minor"] >= MIN_AXIS_PX
        # Every rung must be SEEN whole-frame (else the scanner, not the clause, is
        # doing the work) and must land on the verdict it was planted to produce.
        good = exact and centred and reads and L >= L_MIN and got == want
        ok = ok and good
        bbox = f"{top['w']}x{top['h']}" if top else "-"
        ctr = f"({top['cx']},{top['cy']})" if top else "-"
        note = ("RECOVERED" if exact and centred and reads else "LOST — scanner is blind here")
        if got != want:
            note = f"!! gate said {got}, this rung is planted to prove {want}"
        elif label == "offhero":
            note = f"RECOVERED whole-frame, gate FAILs it — {why}"
        print(f"{label:>10}  {n:8d}  {r['px']:9d}  {bbox:>9}  {ctr:>13}  {L:6.1f}  "
              f"{got[:4]:>6}  {note}")

    print("-" * 104)
    print("INSTRUMENT CONTROL: " + ("PASS — the scanner sees a planted accent, the gate "
                                    "passes it only ON the hero, so a 0-px reading means "
                                    "the art is missing"
                                    if ok else "FAIL — do not trust any reading from this gate"))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
