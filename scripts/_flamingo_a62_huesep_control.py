#!/usr/bin/env python3
"""A6.2 control harness — run the `hero vs bg hue split` gate on the APPROVED
concept sheets before believing any verdict it hands down on a rendered frame.

Office rule that produced this file: *run the grader on the golden reference
first as a control; if the control cannot score full marks, the instrument is
broken, not the picture.*  The same rule already retired one phantom target on
this project (`docs/look-acceptance-rubric.md` §P0-axes calibration log).

The gate under test is `art_order_grade.py` A6 `hero vs bg hue split >= 25 deg`,
implemented in `silhouette()` as

    hue_sep = | mean(hue[hero_blob]) - mean(hue[bg_ring]) |

i.e. a **whole-blob mean over a linear hue axis**.  Two things follow that the
gate's own line never accounted for, and this script measures both:

  * **reachability** — how large a fraction of the hero has to carry the cool
    accent (`#4FC9D6`, hue 185.8 deg) before that mean can cross the bar at all.
    A brooch-sized part cannot move a whole-body mean; the arithmetic is closed
    form, so this is provable without another render.
  * **mask honesty** — the gate's blob is a percentile split inside a caller box,
    not a matte.  On the concept sheets `grade_character.mask_from_ref` gives a
    ground-truth matte, so the same statistic is reported on BOTH masks; if they
    disagree the blob heuristic is the problem, if they agree the threshold is.

Usage:
    python scripts/_flamingo_a62_huesep_control.py IMAGE --matte
    python scripts/_flamingo_a62_huesep_control.py IMAGE --box x0,y0,x1,y1
    python scripts/_flamingo_a62_huesep_control.py IMAGE --charmask PATH

EXIT CODE (added 2026-08-14): this harness printed `PASS`/`FAIL` per mask and
then exited 0 either way, so a caller reading `$?` was told the control passed
by a run whose own line said FAIL. The verdict is now carried:

    0 = every mask reported cleared the bar
    1 = at least one mask reported FAIL
    2 = nothing could be measured (no mask / no blob) -- unknown is not a pass

Callers that want this number as ADVISORY (it is a retired target -- see
`_flamingo_a62_chain.sh` step 5) must say so explicitly with `|| true`, which is
what that chain already does. Silence is no longer read as agreement.
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
import art_order_grade as A  # noqa: E402
import grade_character as C  # noqa: E402

ACCENT_HUE = 185.8  # #4FC9D6 = Color::srgb(0.310, 0.788, 0.839) in anim.rs
BAR = A.T["hero_hue_sep"]


def ring_of(blob):
    """The gate's own background ring: dilate 24, minus dilate 10."""
    return (ndimage.binary_dilation(blob, iterations=24)
            & ~ndimage.binary_dilation(blob, iterations=10))


def gate_blob(f, box):
    """Reproduce `art_order_grade.silhouette`'s blob exactly."""
    x0, y0, x1, y1 = box
    sub = f["L"][y0:y1, x0:x1]
    thr = float(np.percentile(sub, 55))
    best, bestscore = None, -1.0
    for c in (sub > thr, sub <= thr):
        lab, n = ndimage.label(c)
        if n == 0:
            continue
        sz = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
        b = lab == int(np.argmax(sz)) + 1
        border = (b[0].mean() + b[-1].mean() + b[:, 0].mean() + b[:, -1].mean()) / 4.0
        score = b.mean() * (1.0 - border)
        if score > bestscore:
            best, bestscore = b, score
    if best is None:
        return None
    blob = np.zeros(f["L"].shape, dtype=bool)
    blob[y0:y1, x0:x1] = best
    return blob


def box_from_mask(mask, pad=0.06):
    ys, xs = np.nonzero(mask)
    h, w = mask.shape
    dy, dx = int((ys.max() - ys.min()) * pad), int((xs.max() - xs.min()) * pad)
    return (max(0, int(xs.min()) - dx), max(0, int(ys.min()) - dy),
            min(w, int(xs.max()) + dx), min(h, int(ys.max()) + dy))


def box_from_charmask(frame_path, mask_path, pad=0.06):
    """Recover the hero box from a grade_character.py overlay (drawn-on mask)."""
    a = np.asarray(Image.open(frame_path).convert("RGB"), dtype=np.int16)
    b = np.asarray(Image.open(mask_path).convert("RGB").resize(
        (a.shape[1], a.shape[0]), Image.NEAREST), dtype=np.int16)
    diff = np.abs(a - b).sum(axis=2) > 20
    if not diff.any():
        # exit 2, not 1: nothing was measured, so this is "unknown", not "FAIL".
        print(f"{mask_path}: overlay identical to frame, no mask to read",
              file=sys.stderr)
        raise SystemExit(2)
    return box_from_mask(diff, pad)


def reachability(hero_hue, bg_hue):
    """Fraction of hero pixels that must become ACCENT_HUE to reach the bar.

    The gate averages hue linearly, so painting fraction f of the hero at the
    accent hue moves the statistic as  hue' = (1-f)*hero_hue + f*ACCENT_HUE,
    and the smallest f clearing |hue' - bg_hue| >= BAR is closed form.
    """
    lever = ACCENT_HUE - hero_hue
    if abs(lever) < 1e-6:
        return None
    want = bg_hue + BAR if lever > 0 else bg_hue - BAR
    f = (want - hero_hue) / lever
    return f if f > 0 else None


def report(kind, f, blob, total_px):
    hue = f["hue"]
    ring = ring_of(blob)
    hero_hue = float(hue[blob].mean())
    bg_hue = float(hue[ring].mean())
    dh = abs(hero_hue - bg_hue) % 360.0
    sep = min(dh, 360.0 - dh)
    px = int(blob.sum())
    ok = "PASS" if sep >= BAR else "FAIL"
    need = reachability(hero_hue, bg_hue)
    print(f"  [{kind}] hero {px:>9,} px ({100.0 * px / total_px:5.2f}% of frame)"
          f"  hue {hero_hue:6.2f}  bg-ring {bg_hue:6.2f}"
          f"  ->  hue_sep {sep:6.2f} deg  (want >= {BAR:.0f})  {ok}")
    if need is None:
        print(f"        accent unreachable in this direction")
    else:
        print(f"        to clear the bar: {100 * need:5.2f}% of the hero "
              f"({int(need * px):,} px) must sit at hue {ACCENT_HUE:.1f}")
    return sep, sep >= BAR


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("--box", help="x0,y0,x1,y1")
    ap.add_argument("--charmask")
    ap.add_argument("--matte", action="store_true",
                    help="studio-sheet matte via grade_character.mask_from_ref "
                         "(concept sheets only); also drives the box")
    ap.add_argument("--label", default=None)
    args = ap.parse_args()

    f = A.load(args.image)
    total = f["L"].size
    print(args.label or Path(args.image).name)

    matte = None
    if args.matte:
        rgb = np.asarray(Image.open(args.image).convert("RGB"), dtype=np.float32)
        matte = C.mask_from_ref(rgb)

    if args.box:
        box = tuple(int(v) for v in args.box.split(","))
    elif args.charmask:
        box = box_from_charmask(args.image, args.charmask)
    elif matte is not None:
        box = box_from_mask(matte)
    else:
        print("need --box, --charmask or --matte (A6 refuses to guess a mask)",
              file=sys.stderr)
        return 2
    print(f"  box {box}")

    blob = gate_blob(f, box)
    if blob is None:
        print(f"{args.image}: no blob for box {box}", file=sys.stderr)
        return 2

    oks = [report("gate blob ", f, blob, total)[1]]
    if matte is not None:
        oks.append(report("true matte", f, matte, total)[1])

    bad = oks.count(False)
    if bad:
        print(f"  -> CONTROL FAIL: {bad}/{len(oks)} mask(s) below the "
              f"{BAR:.0f} deg bar (exit 1)")
        return 1
    print(f"  -> CONTROL PASS: {len(oks)}/{len(oks)} mask(s) clear the "
          f"{BAR:.0f} deg bar (exit 0)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
