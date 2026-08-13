#!/usr/bin/env python3
"""A6.2 accent-presence check — does the reserved cool accent actually RENDER,
and does it render big enough to read?

This is the honest half of A6.2. The other half (`hero vs bg hue split >= 25`)
was proven to be a phantom target — see `_flamingo_a62_huesep_control.py` and
`_flamingo_a62_metric_probe.py`: every CEO-approved concept sheet scores 1.4-5.8
deg on it, and two of the three acknowledged-failure gate3 frames score inside or
above that same band, so the statistic does not even rank approved art above the
art it was written to reject.

What IS a real, closable defect is that the clasp drew **0 pixels** in 3/3 frames
(`docs/VERDICT-a6-teal-accent-2026-08-11.md`) because the torso body mesh occluded
it. That is a binary fact about the render, it has a clean negative control (every
pre-fix frame = 0 px), and it is what this script measures.

DEGENERACY GUARD — why there is a luminance floor:
Scanning hue+saturation alone finds 1,021 "cool" pixels in the approved Auren
concept sheet, which looks like the accent is already there. It is not: those
pixels average RGB (3.2, 4.1, 6.0) — near-black shadow, where sat = (max-min)/max
blows up on quantisation noise, in 642 disconnected 8-14 px specks. With `L >= 20`
only 7 survive. Counting a metric's population before checking it isn't degenerate
is how a gate ends up passing on nothing (see the office scar
"gamut clip fakes colour gates"). So every pixel counted here must clear a real
luminance floor, and the report prints the surviving component structure, not just
a total.

WHY `--gate` NEEDS THE CHARACTER MASK:
Without a mask this scans the WHOLE FRAME, and the world itself is allowed to
contain the accent — `look-acceptance-rubric.md` Pass 9b budgets "teal accent
<= 15% of the frame" for scenery blocks. A whole-frame scan would therefore hand
out a PASS for a teal block sitting three metres behind the hero while the clasp
is still missing. So the gate form requires the charmask `grade_character.py`
writes and counts only components ON the hero; with no mask it reports
UNRELIABLE rather than PASS (same rule the rubric already applies to
`grade_hero.py`). Informational runs (no `--gate`) stay whole-frame.

That clause needs its own two-sided control, and the first version of it did not
have one: the mask is recovered as frame-minus-overlay, so anything painted into
the frame joined the mask by definition and a gem planted in a far frame corner
still PASSed. `hero_mask()` now keeps only the hero BODY (largest component + what its
silhouette encloses), and `_flamingo_a62_synth_control.py` plants a third rung
off-hero that must FAIL. Do not weaken either without re-running that rung.

Usage:
    python scripts/_flamingo_a62_accent_presence.py FRAME [FRAME ...]
                        [--charmask-suffix -charmask.png] [--json OUT.json]
    python scripts/_flamingo_a62_accent_presence.py FRAME --gate   # verdict + exit code
"""
import argparse
import colorsys
import json
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

# anim.rs: Color::srgb(0.310, 0.788, 0.839) — look-bible.md:132 "Accent 2 (cool)"
GEM_SRGB = (0.310, 0.788, 0.839)
GEM_HUE = colorsys.rgb_to_hsv(*GEM_SRGB)[0] * 360.0
HUE_TOL = 25.0     # the lit gem shifts hue under a golden key; this is the band
SAT_MIN = 0.25
L_MIN = 20.0       # the degeneracy guard — see the module docstring
MIN_AXIS_PX = 4.0  # under ~3 px an accent is destroyed by AA; 4 is the read floor


def load(path):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im, dtype=np.float32)
    L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    mx, mn = a.max(2), a.min(2)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    # hue via the same max/min formulation colorsys uses, vectorised
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    d = np.maximum(mx - mn, 1e-6)
    h = np.where(mx == r, ((g - b) / d) % 6,
                 np.where(mx == g, (b - r) / d + 2, (r - g) / d + 4)) * 60.0
    return a, L, sat, h % 360.0


def hero_mask(frame, suffix):
    """The character mask grade_character.py already wrote next to the frame.

    `grade_character.py` stores the mask as a tinted OVERLAY of the frame, so the
    mask is recovered as "where the overlay differs from the frame". That recovery
    is frame-relative, and on its own it makes the "must be on the hero" clause
    unfalsifiable: ANY pixel that differs from the overlay — including one painted
    into the frame by an instrument control, or a scenery block the mask never
    covered — is labelled hero by construction. Proof from the 08-14 plate: an 8x8
    gem planted at (60,60) — the frame's top-left corner, nowhere near the
    character's own bbox (x528-750, y258-566) — grew the recovered mask by exactly
    +64 px (33,505 -> 33,569) and the gate PASSed on it.

    So the recovered diff is not the mask; it is the mask PLUS islands. The hero is
    one body: keep the largest connected component and everything enclosed by its
    silhouette (interior holes where the tint diff fell under the threshold), and
    drop every free-floating island. On the 08-14 boot plate that is 33,505 ->
    33,433 px (4 islands, 72 px dropped), and the (60,60) plant is now excluded:
    the mask does not move at all and the gate FAILs. Returns (mask, path, stats).
    """
    cm = Path(str(Path(frame).with_suffix("")) + suffix)
    if not cm.exists():
        return None, None, None
    a = np.asarray(Image.open(frame).convert("RGB"), dtype=np.int16)
    b = np.asarray(Image.open(cm).convert("RGB"), dtype=np.int16)
    if a.shape != b.shape:
        return None, None, None
    raw = np.abs(a - b).sum(axis=2) > 20
    lab, n = ndimage.label(raw)
    if not n:
        return raw, str(cm), dict(raw_px=0, body_px=0, islands_dropped_px=0, islands=0)
    sizes = ndimage.sum(raw, lab, range(1, n + 1))
    body = ndimage.binary_fill_holes(lab == (int(np.argmax(sizes)) + 1))
    kept = raw & body
    return kept, str(cm), dict(raw_px=int(raw.sum()), body_px=int(kept.sum()),
                               islands_dropped_px=int(raw.sum() - kept.sum()),
                               islands=int(n - 1))


def components_of(binary, rgb, top_n=5):
    """Connected components of `binary`, largest first, with bbox + mean colour."""
    lab, n = ndimage.label(binary)
    comps = []
    if n:
        sizes = ndimage.sum(binary, lab, range(1, n + 1))
        for i in np.argsort(sizes)[::-1][:top_n]:
            m = lab == (i + 1)
            ys, xs = np.nonzero(m)
            w, h = xs.max() - xs.min() + 1, ys.max() - ys.min() + 1
            comps.append(dict(px=int(sizes[i]), cx=int(xs.mean()), cy=int(ys.mean()),
                              w=int(w), h=int(h), minor=int(min(w, h)),
                              rgb=[round(float(v), 1) for v in rgb[m].mean(0)]))
    return n, comps


def measure(frame, suffix):
    rgb, L, sat, hue = load(frame)
    d = np.abs(hue - GEM_HUE) % 360.0
    d = np.minimum(d, 360.0 - d)
    raw = (d <= HUE_TOL) & (sat >= SAT_MIN)
    kept = raw & (L >= L_MIN)

    n, comps = components_of(kept, rgb)

    mask, cmpath, mstats = hero_mask(frame, suffix)
    out = dict(frame=str(frame), size=list(rgb.shape[:2][::-1]),
               raw_px=int(raw.sum()), px=int(kept.sum()),
               dropped_by_L_floor=int(raw.sum() - kept.sum()),
               components=n, top=comps, charmask=cmpath)
    if mask is not None:
        out["hero_mask"] = mstats
        out["hero_px"] = int(mask.sum())
        out["pct_of_hero"] = round(100.0 * float((kept & mask).sum()) / max(1, mask.sum()), 4)
        out["px_on_hero"] = int((kept & mask).sum())
        # The gate counts ONLY accent that lands on the character — see the
        # module docstring on why a whole-frame scan cannot carry a PASS.
        _, hero_comps = components_of(kept & mask, rgb)
        out["hero_components"] = hero_comps
    return out


def verdict_of(r):
    """The A6.2' hard check: PASS / FAIL / UNRELIABLE (+ the reason)."""
    if r.get("charmask") is None or "hero_components" not in r:
        return "UNRELIABLE", ("no charmask beside the frame — run grade_character.py "
                              "first; a whole-frame scan may be reading scenery teal "
                              "(rubric Pass 9b budgets teal blocks), so it cannot PASS")
    best = max((c["minor"] for c in r["hero_components"]), default=0)
    if not r["hero_components"]:
        if r["px"]:
            return "FAIL", (f"{r['px']} accent px in the frame but 0 on the hero body — "
                            "this is scenery/plant teal, not the character's accent")
        return "FAIL", "0 accent px on the hero mask — the accent is not rendering"
    if best < MIN_AXIS_PX:
        return "FAIL", (f"largest accent blob on the hero has minor axis {best} px "
                        f"(< {MIN_AXIS_PX:.0f}) — present but too thin to read")
    return "PASS", (f"accent renders on the hero: {r['px_on_hero']} px, "
                    f"largest blob minor axis {best} px")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("frames", nargs="+")
    ap.add_argument("--charmask-suffix", default="-charmask.png")
    ap.add_argument("--json")
    ap.add_argument("--gate", action="store_true",
                    help="carry a verdict: exit 1 unless every frame PASSes "
                         "(requires a charmask — see the module docstring)")
    args = ap.parse_args()

    print(f"accent #4FC9D6  hue {GEM_HUE:.1f} +-{HUE_TOL:.0f} deg   "
          f"sat >= {SAT_MIN}   L >= {L_MIN}  (degeneracy guard)")
    print("-" * 96)
    rows = []
    for fr in args.frames:
        r = measure(fr, args.charmask_suffix)
        rows.append(r)
        verdict = "RENDERS" if r["px"] else "0 PX — NOT RENDERING"
        print(f"{Path(fr).name:44s} {r['px']:6d} px  {verdict}")
        if r["dropped_by_L_floor"]:
            print(f"{'':44s} ({r['dropped_by_L_floor']} px rejected by the L>={L_MIN:.0f} "
                  f"floor as shadow hue-noise)")
        if "pct_of_hero" in r:
            print(f"{'':44s} hero mask {r['hero_px']:,} px -> accent is "
                  f"{r['pct_of_hero']:.4f}% of the character")
            ms = r["hero_mask"]
            if ms["islands_dropped_px"]:
                print(f"{'':44s} (mask recovered {ms['raw_px']:,} px; dropped "
                      f"{ms['islands_dropped_px']} px in {ms['islands']} island(s) not "
                      f"enclosed by the hero body — see hero_mask() docstring)")
        for c in r["top"][:3]:
            reads = "reads" if c["minor"] >= MIN_AXIS_PX else f"TOO THIN (<{MIN_AXIS_PX:.0f} px)"
            print(f"{'':44s} blob {c['px']:5d} px  {c['w']}x{c['h']} at "
                  f"({c['cx']},{c['cy']})  rgb {c['rgb']}  {reads}")

    if args.gate:
        print("-" * 96)
        print(f"A6.2' HARD CHECK — accent must form a component with minor axis "
              f">= {MIN_AXIS_PX:.0f} px ON THE HERO MASK")
        bad = 0
        for r in rows:
            v, why = verdict_of(r)
            bad += v != "PASS"
            print(f"  {Path(r['frame']).name:44s} {v:10s} {why}")
        if bad:
            print(f"\nA6.2 ACCENT GATE: FAIL ({bad}/{len(rows)} frame(s) not PASS)")
            return 1
        print(f"\nA6.2 ACCENT GATE: PASS ({len(rows)}/{len(rows)})")

    if args.json:
        Path(args.json).write_text(json.dumps(rows, indent=1))
        print(f"\nwrote {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
