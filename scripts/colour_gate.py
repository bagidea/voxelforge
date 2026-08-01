#!/usr/bin/env python3
"""Colour gate for gameplay frames — the checks the Gate 3 shoot could not see.

Implements the three framing-independent checks proposed in
`docs/gate3-colour-review-2026-08-01.md` §3, so "renders successfully" and
"renders correctly" stop being the same result:

  Gate A  magenta fraction   — lit pixels whose G sits below BOTH R and B.
                               Scene- and framing-independent. Measured 0.00%
                               on every approved frame and 16.4-73.7% on the
                               broken ones, so the 2% threshold has 8.2x
                               headroom below the mildest real failure and the
                               approved frames do not merely pass, they read
                               exactly zero.  [FATAL]
  Gate B  sky ordering       — the flat `ClearColor` sky must still leave the
                               post stack ordered B > G > R. This is the check
                               that caught the bug: a flat clear is not a lit
                               surface, so nothing in the scene can excuse it
                               changing rank. SKIPPED (not failed) when the
                               frame has too little sky to measure. Also reports
                               the per-channel LINEAR gain from the ClearColor
                               parsed out of client/src/main.rs to the measured
                               sky — that one line is the whole 2026-08-01
                               review.  [FATAL when it applies]
  Gate C  sunlit ordering    — the brightest lit (non-sky) surface should read
                               R > G > B (rubric G6, "warm golden"), the half a
                               magenta wash passes by accident because magenta
                               keeps R > B.  [WARNING ONLY — see below]

Gate C is a LOOK check, not a bug detector, and it is deliberately non-fatal.
Measured: it FAILS `docs/assets/thirdperson-walk.png` — an approved, clean,
*ungraded* outdoor frame — because that scene's brightest surface is grass
(121.3, 199.6, 112.8 -> G > R > B). "Warm golden" is a property of the amber
hero grade, not of every frame the engine can produce, so wiring it into the
exit code would fail legitimate green outdoor shots forever and the whole gate
would be switched off within a week. It prints WARN and does not fail the run.
The magenta wash it was meant to catch is already caught twice over by A and B.

Sky is found as the largest flat colour cluster in the top third of the frame
(the sky is a constant clear, so it is by far the biggest uniform area up
there) rather than by "is it blue", which would presuppose the answer.

Usage:
    python scripts/colour_gate.py frame.png [more.png ...]
    python scripts/colour_gate.py --tsv frame.png ...   # machine-readable table
Exit code 0 only if every frame passes every applicable FATAL gate.
"""
import os
import re
import sys

import numpy as np
from PIL import Image

MAGENTA_MAX_FRACTION = 0.02  # Gate A
LIT_MIN_MEAN = 25  # ignore near-black when judging hue
CHANNEL_MARGIN = 8  # how far G must sit below R and B to count as magenta
SKY_MIN_FRACTION = 0.02  # below this much sky, Gate B skips instead of failing
# A large flat region at the top of an INDOOR frame is a ceiling or a lit wall,
# not a sky, and grading its channel order against a sky rule is meaningless —
# both hero interiors trip it otherwise. Two discriminators, neither of which
# presupposes the sky is blue (that is the thing under test):
#   * bright — daylight sky sits near the top of the frame's range;
#   * it owns the top EDGE — sky runs off the top of frame, a wall does not.
# Measured on the approved set: top-2-row coverage is 33-66% on the three outdoor
# frames and 5-24% on the four interiors, so 30% separates them. Caveat stated
# rather than hidden: the luminance floor also skips a genuine night sky, and the
# combined rule is deliberately conservative — a frame that wrongly SKIPs Gate B
# is still caught by Gate A, which is the primary check.
SKY_MIN_LUM = 100.0
SKY_MIN_TOP_EDGE = 0.30

_MAIN_RS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "client", "src", "main.rs")
_CLEARCOLOR_RE = re.compile(r"ClearColor\s*\(\s*Color::srgb\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)")
AUTHORED_FALLBACK = (0.53, 0.72, 0.92)


def authored_clearcolor():
    """The sky as authored in source — parsed, never hand-copied."""
    try:
        with open(_MAIN_RS, "r", encoding="utf-8", errors="replace") as f:
            m = _CLEARCOLOR_RE.search(f.read())
        if m:
            return tuple(float(x) for x in m.groups())
    except OSError:
        pass
    return AUTHORED_FALLBACK


def srgb_to_linear(c):
    c = np.asarray(c, np.float64)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def sky_gain(sky_rgb):
    """Per-channel LINEAR gain applied to the authored ClearColor to land on `sky_rgb`."""
    return srgb_to_linear(np.asarray(sky_rgb, np.float64) / 255.0) / srgb_to_linear(
        np.asarray(authored_clearcolor(), np.float64))


def sky_patch(a):
    """Mean RGB of the flat sky region, or None if there isn't enough of one.

    Quantises the top third of the frame to a coarse 3D colour histogram and
    takes the largest bin. The sky is a flat clear, so it dominates that band;
    lit geometry is textured and scatters across many bins.
    """
    top = a[: a.shape[0] // 3].reshape(-1, 3)
    q = (top // 16).astype(np.int32)
    key = q[:, 0] * 4096 + q[:, 1] * 64 + q[:, 2]
    vals, counts = np.unique(key, return_counts=True)
    best = vals[counts.argmax()]
    frac = counts.max() / a.shape[0] / a.shape[1]
    if frac < SKY_MIN_FRACTION:
        return None, f"only {frac * 100:.1f}% flat top region"
    rgb = top[key == best].mean(0)
    if rgb @ np.array([0.2126, 0.7152, 0.0722]) < SKY_MIN_LUM:
        return None, "flat top region too dark to be sky"
    edge = (key[: a.shape[1] * 2] == best).mean()
    if edge < SKY_MIN_TOP_EDGE:
        return None, f"flat top region owns only {edge * 100:.0f}% of the top edge"
    return rgb, f"{frac * 100:.1f}% of frame"


def sunlit_patch(a, sky_rgb):
    """Mean RGB of the brightest lit surface that is NOT the sky."""
    lum = a @ np.array([0.2126, 0.7152, 0.0722])
    keep = np.ones(a.shape[:2], bool)
    if sky_rgb is not None:
        # Drop anything within a coarse bin of the sky colour.
        keep &= np.abs(a - sky_rgb).max(2) > 24
    if keep.sum() < 500:
        return None
    thr = np.percentile(lum[keep], 90)
    sel = keep & (lum >= thr)
    return a[sel].mean(0)


def order(rgb):
    return " > ".join(c for _, c in sorted(zip(rgb, "RGB"), reverse=True))


def measure(path):
    """Every number this gate knows about one frame, as a dict.

    Pre-processing, stated exactly because docs quote these figures: the PNG at
    NATIVE resolution, convert("RGB"), no resample, no crop, no HUD mask.
    `mag_frac` and `warm_frac` are over ALL pixels; the `_lit` variants are over
    lit pixels only. The all-pixel denominator is the conservative one for
    gating (a mostly-dark frame cannot trip Gate A on a small lit region); the
    share-of-lit one is the exposure-independent one for comparing frames whose
    lit fraction differs (an indoor hero shot vs a full-daylight gameplay one).
    """
    a = np.asarray(Image.open(path).convert("RGB")).astype(np.float32)
    n = float(a.shape[0] * a.shape[1])
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    lit = a.mean(2) > LIT_MIN_MEAN
    lit_n = max(float(lit.sum()), 1.0)
    magenta = lit & (G < R - CHANNEL_MARGIN) & (G < B - CHANNEL_MARGIN)
    warm = lit & (R > G) & (G > B)

    sky, sky_note = sky_patch(a)
    return {
        "path": path,
        "lit_frac": lit.sum() / n,
        "mag_frac": magenta.sum() / n,
        "mag_frac_lit": magenta.sum() / lit_n,
        "warm_frac": warm.sum() / n,
        "warm_frac_lit": warm.sum() / lit_n,
        "sky": sky,
        "sky_note": sky_note,
        "sky_gain": None if sky is None else sky_gain(sky),
        "sun": sunlit_patch(a, sky),
    }


def check(path):
    m = measure(path)
    mag_frac = m["mag_frac"]
    sky, sky_note, sun = m["sky"], m["sky_note"], m["sun"]

    rows = []
    ok_a = mag_frac <= MAGENTA_MAX_FRACTION
    rows.append(("A magenta fraction", f"{mag_frac * 100:6.2f}%", f"<= {MAGENTA_MAX_FRACTION * 100:g}%", ok_a))

    if sky is None:
        rows.append(("B sky order", sky_note, "SKIP", None))
        ok_b = True
    else:
        ok_b = sky[2] > sky[1] > sky[0]
        rows.append((
            "B sky order",
            f"{sky[0]:5.1f},{sky[1]:5.1f},{sky[2]:5.1f} -> {order(sky)}",
            "B > G > R",
            ok_b,
        ))

    # Gate C is WARNING-ONLY — it does not feed frame_ok. See the module docstring:
    # it fails clean green outdoor frames, because "warm golden" is a property of
    # the amber hero grade rather than of every frame the engine can render.
    warn_c = False
    if sun is None:
        rows.append(("C sunlit order", "no lit surface", "SKIP", None))
    else:
        warn_c = not (sun[0] > sun[1] > sun[2])
        rows.append((
            "C sunlit order",
            f"{sun[0]:5.1f},{sun[1]:5.1f},{sun[2]:5.1f} -> {order(sun)}",
            "R > G > B (advisory)",
            None if warn_c else True,
        ))

    print(f"\n{path}")
    for label, value, target, ok in rows:
        if label.startswith("C ") and warn_c:
            tag = "WARN"
        else:
            tag = "SKIP" if ok is None else ("PASS" if ok else "FAIL")
        print(f"  [{tag}] {label:<20} {value:<34} target {target}")
    if sky is not None:
        g = m["sky_gain"]
        ar, ag, ab = authored_clearcolor()
        print(f"         authored ClearColor({ar:g},{ag:g},{ab:g}) = "
              f"{ar * 255:.0f},{ag * 255:.0f},{ab * 255:.0f}"
              f"  ->  linear gain  R x{g[0]:.2f}  G x{g[1]:.2f}  B x{g[2]:.2f}")
    print(f"         warm-ordered R>G>B {m['warm_frac'] * 100:6.2f}% of frame "
          f"({m['warm_frac_lit'] * 100:.2f}% of lit)   [context, not gated]")
    frame_ok = ok_a and ok_b
    print(f"  => {'COLOUR GATE PASS' if frame_ok else 'COLOUR GATE FAIL'}"
          f"{'  (gate C advisory: not warm-ordered)' if warn_c else ''}")
    return frame_ok


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        print("usage: colour_gate.py [--tsv] <frame.png> [more.png ...]")
        sys.exit(2)
    if "--tsv" in sys.argv:
        print("frame\tmagenta%\tmagenta%(lit)\twarm%\twarm%(lit)\tlit%\tgateA")
        ok = True
        for p in args:
            m = measure(p)
            a_ok = m["mag_frac"] <= MAGENTA_MAX_FRACTION
            ok = ok and a_ok
            print(f"{p}\t{m['mag_frac'] * 100:.2f}\t{m['mag_frac_lit'] * 100:.2f}"
                  f"\t{m['warm_frac'] * 100:.2f}\t{m['warm_frac_lit'] * 100:.2f}"
                  f"\t{m['lit_frac'] * 100:.2f}\t{'PASS' if a_ok else 'FAIL'}")
        sys.exit(0 if ok else 1)
    ok = all([check(p) for p in args])
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
