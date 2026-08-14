#!/usr/bin/env python3
"""Grade the SKY DOME DIRECTLY. grade_gate.py/grade_g3.py's own caveat says it best
(docs/evidence/sky-verify-2026-08-14/README.md): "none of the three measurable gates
(G3/G5/G6) touch the sky dome. ... Do not read this table as 'sky fix verified'."
This script exists to close exactly that hole.

WHY IT READS THE RAW FRAME, NOT *-nohud2.png (and is EXEMPT in nohud2_guard.py,
not GUARDED). scripts/_flamingo_dehud2.py crops the top HUD_BAND=80 rows off
entirely -- that crop protects G3/G5/G6's ground-level/interior measurements, but
the sky lives in exactly the rows it deletes. Converting the hand-picked sky
windows in _poppy_sky_exposure_probe.py's FRAMES table into nohud2 coordinates
(subtract 80, clip) leaves edhari-load.png with a 10-20px sliver of survivable
sky -- grading nohud2 here would neuter the measurement, not protect it. The HUD
status line (FPS/HP/ST) is documented as confined to that same top-left
HUD_BAND=80 strip, and the crosshair patch sits at frame-centre (y ~= h/2, well
below this script's sky band) -- so a RAW frame is safe to read here as long as
sampling starts at SKY_TOP, a few px below HUD_BAND, which it does.

HOW SKY IS LOCATED -- structurally, not by brightness. A blown dome (the bug) and
a correctly-exposed dome (the fix) look nothing alike in absolute colour, so any
predicate keyed on "near white" only ever finds the BROKEN case (this is exactly
how _poppy_sky_exposure_probe.py's R>=250-and-B>=170 predicate is scoped -- it was
written to characterise the bug, not to survive the fix). What both cases share
structurally: the dome is a smooth vertical gradient, so at a FIXED elevation
(one row) it is nearly constant colour all the way across, whether that constant
is a blown 253 or a correctly-exposed 90. Terrain silhouette breaking into a row
(a rooftop, a tree line) instead mixes sky colour with building/foliage colour
and drives that row's horizontal spread up sharply. So: a "pure sky row" is one
whose horizontal R std-dev is low, and the elevation gradient is measured only
across those unoccluded rows.

3-STATE EXIT CODE (the pattern already shipped in this repo --
scripts/nohud2_guard.py's EXIT_REFUSED=2 is the same idea for a different
refusal): 0 = PASS (gradient present, not clipped) · 1 = FAIL (flat/blown,
matches the documented pre-fix signature) · 2 = UNMEASURABLE (could not find
enough unoccluded sky rows in this frame to say anything -- not a verdict on the
sky, a refusal to guess).

CALIBRATE BEFORE TRUSTING: run `--selftest` first. It builds a synthetic PASS
case (a real gradient), a synthetic FAIL case (the commit's own quoted blown
colour [252,226,191] duplicated near-flat), and a synthetic UNMEASURABLE case (no
uniform row anywhere), and asserts this script scores each one correctly. If
`--selftest` does not print CALIBRATION OK, the meter is broken -- do not trust
any verdict this script prints on a real frame until it does.

Usage:
    python scripts/grade_sky.py <frame.png>     # grade a real capture
    python scripts/grade_sky.py --selftest      # calibrate the meter itself
"""
import sys
from pathlib import Path

from PIL import Image

HUD_BAND = 80          # _flamingo_dehud2.py's crop -- the HUD status line's height
SKY_TOP = 90            # 10px safety margin below HUD_BAND
SKY_BOTTOM_FRAC = 0.45  # sky band = [SKY_TOP, SKY_BOTTOM_FRAC * H) -- same fraction
                        # grade_gate.py/grade_g3.py already trust for "upper portion"

# Per-pixel "is this sky, not terrain" predicates. TIER1 is the exact predicate
# scripts/_poppy_sky_exposure_probe.py already validated against these two frames
# (its numbers match docs/evidence/sky-verify-2026-08-14/burnout_measurements.txt) --
# reused verbatim rather than re-derived, because a first attempt at a brightness-
# free "uniform patch" locator here (std-dev over 40px blocks) turned out to match
# ordinary flat walls just as happily as sky, and produced a span of 180+ levels on
# a frame independently known to be flat-blown at ~1 level. TIER1 is scoped to the
# BROKEN (near-white, cool-leaning) case on purpose; TIER2 is this script's own
# fallback for a correctly-exposed dome, which TIER1 goes blind to by construction
# once the fix removes the near-white clipping. Which tier actually answered is
# always printed, so a PASS never hides behind an unstated predicate swap.
TIER1_NAME = "R>=250 & B>=170 (blown-sky signature)"
TIER2_NAME = "B>R+5 (cool-toned fallback for a correctly-exposed, non-blown sky)"


def _tier1(r, g, b):
    return r >= 250 and b >= 170


def _tier2(r, g, b):
    return b > r + 5


MIN_QUALIFY_PER_ROW = 30  # scripts/_poppy_sky_exposure_probe.py's own validated
                          # floor for "enough pixels in this row to be an open sky
                          # patch, not a stray edge pixel"
MIN_SKY_ROWS = 10       # fewer qualifying rows than this = can't trust an elevation
                        # span measured over them
R_SPAN_FAIL_MAX = 1.5   # documented pre-fix span was 0.7-1.0 (both frames,
                        # docs/evidence/sky-verify-2026-08-14/burnout_measurements.txt)
                        # -- 1.5 gives headroom above that range before calling PASS
PCT_R250_FAIL_MIN = 3.0  # documented pre-fix sky-band %R>=250 was 5.33-14.33% --
                         # 3.0 is roughly half the smallest observed blown reading

EXIT_PASS, EXIT_FAIL, EXIT_UNMEASURABLE = 0, 1, 2


def _rows_for(a, w, y0, y1, predicate):
    """{y: (meanR, meanG, meanB)} for rows where >=MIN_QUALIFY_PER_ROW pixels
    match `predicate`, using only the matching pixels for the row's mean."""
    out = {}
    for y in range(y0, y1):
        rs = gs = bs = n = 0
        for x in range(w):
            r, g, b = a[x, y]
            if predicate(r, g, b):
                rs += r; gs += g; bs += b; n += 1
        if n >= MIN_QUALIFY_PER_ROW:
            out[y] = (rs / n, gs / n, bs / n)
    return out


def measure(path):
    """Return the raw numbers -- no verdict yet. Caller decides pass/fail."""
    im = Image.open(path).convert("RGB")
    w, h = im.size
    a = im.load()

    y0 = SKY_TOP
    y1 = max(y0 + 1, int(h * SKY_BOTTOM_FRAC))

    tier1_rows = _rows_for(a, w, y0, y1, _tier1)
    if len(tier1_rows) >= MIN_SKY_ROWS:
        tier_name, rows = TIER1_NAME, tier1_rows
    else:
        rows = _rows_for(a, w, y0, y1, _tier2)
        tier_name = TIER2_NAME

    row_means = [(y, *v) for y, v in sorted(rows.items())]

    # whole-band %R>=250 -- a plain statistic over every pixel in the band,
    # independent of which tier located the elevation-gradient rows above.
    band_pixels_total = 0
    band_r250 = 0
    for y in range(y0, y1):
        for x in range(w):
            r = a[x, y][0]
            band_pixels_total += 1
            if r >= 250:
                band_r250 += 1
    pct_r250_sky = 100.0 * band_r250 / band_pixels_total if band_pixels_total else 0.0

    # whole-frame true burnout: every channel pinned >=250, same pixel
    frame_total = w * h
    frame_burnout = 0
    for y in range(0, h, 1):
        for x in range(0, w, 1):
            r, g, b = a[x, y]
            if r >= 250 and g >= 250 and b >= 250:
                frame_burnout += 1
    pct_burnout_frame = 100.0 * frame_burnout / frame_total if frame_total else 0.0

    r_span = g_span = b_span = 0.0
    if row_means:
        rs = [m[1] for m in row_means]
        gs = [m[2] for m in row_means]
        bs = [m[3] for m in row_means]
        r_span = max(rs) - min(rs)
        g_span = max(gs) - min(gs)
        b_span = max(bs) - min(bs)

    return {
        "path": str(path),
        "size": (w, h),
        "sky_band_rows": (y0, y1),
        "tier_used": tier_name,
        "unoccluded_rows": len(row_means),
        "pct_r250_sky_band": pct_r250_sky,
        "pct_burnout_frame": pct_burnout_frame,
        "r_span": r_span,
        "g_span": g_span,
        "b_span": b_span,
    }


def verdict(m):
    if m["unoccluded_rows"] < MIN_SKY_ROWS:
        return EXIT_UNMEASURABLE, (
            f"UNMEASURABLE -- only {m['unoccluded_rows']} qualifying sky row(s) found "
            f"under either tier (need >={MIN_SKY_ROWS}); cannot locate enough open sky "
            f"in this frame")
    if m["r_span"] <= R_SPAN_FAIL_MAX or m["pct_r250_sky_band"] >= PCT_R250_FAIL_MIN:
        return EXIT_FAIL, (
            f"FAIL -- R-span {m['r_span']:.2f} (need >{R_SPAN_FAIL_MAX}) / "
            f"sky-band %R>=250 {m['pct_r250_sky_band']:.2f}% (need <{PCT_R250_FAIL_MIN}%) "
            f"-- matches the documented pre-fix blown signature (span 0.7-1.0, "
            f"%R>=250 5.33-14.33%)")
    return EXIT_PASS, (
        f"PASS -- R-span {m['r_span']:.2f} > {R_SPAN_FAIL_MAX}, "
        f"sky-band %R>=250 {m['pct_r250_sky_band']:.2f}% < {PCT_R250_FAIL_MIN}%")


def report(path):
    m = measure(path)
    code, why = verdict(m)
    w, h = m["size"]
    y0, y1 = m["sky_band_rows"]
    print(f"# {m['path']} = {w}x{h}  sky band y[{y0}:{y1}]")
    print(f"  predicate used            : {m['tier_used']}")
    print(f"  qualifying sky rows found : {m['unoccluded_rows']} (need >={MIN_SKY_ROWS})")
    print(f"  sky-band %R>=250          : {m['pct_r250_sky_band']:.2f}%")
    print(f"  sky-band R/G/B span       : R {m['r_span']:.2f}  G {m['g_span']:.2f}  "
          f"B {m['b_span']:.2f}  (levels, top-to-bottom)")
    print(f"  whole-frame %true-burnout : {m['pct_burnout_frame']:.2f}%  (R,G,B all >=250)")
    print(f"  -> {why}")
    print(f"  EXIT {code}")
    return code, m


# ---------------------------------------------------------------------------
# --selftest: calibrate the meter against synthetic ground truth before any
# real-frame verdict is trusted. There is no pre-regression "known good sky"
# screenshot anywhere in this repo's history (Rose's dome-render commit and the
# exposure bug landed back to back -- nobody has ever seen a correctly-exposed
# dome), so the golden reference here is built from the maths, not a photo.
# ---------------------------------------------------------------------------
def _synthetic(w, h, sky_rows_fn, terrain_noise=False):
    """sky_rows_fn(y) -> (r,g,b) for y in [SKY_TOP, SKY_BOTTOM); everything else
    is mid-grey terrain (with per-pixel noise if terrain_noise, to make sure a
    noisy terrain row never gets mistaken for a low-std sky row)."""
    import random
    rnd = random.Random(1234)
    im = Image.new("RGB", (w, h))
    px = im.load()
    y1 = int(h * SKY_BOTTOM_FRAC)
    for y in range(h):
        if SKY_TOP <= y < y1:
            r, g, b = sky_rows_fn(y)
            for x in range(w):
                px[x, y] = (r, g, b)
        else:
            for x in range(w):
                if terrain_noise:
                    v = rnd.randint(40, 140)
                else:
                    v = 90
                px[x, y] = (v, v, v)
    return im


def _selftest():
    import tempfile

    print("=== grade_sky.py --selftest: calibrating against synthetic ground truth ===\n")
    W, H = 400, 300
    y1 = int(H * SKY_BOTTOM_FRAC)
    span_rows = max(1, y1 - SKY_TOP)

    def good_gradient(y):
        # authored deep-blue zenith -> warm haze horizon, well clear of clipping.
        t = (y - SKY_TOP) / span_rows
        r = int(60 + 110 * t)   # 60 .. 170  (span 110, >> R_SPAN_FAIL_MAX)
        g = int(90 + 60 * t)
        b = int(160 + 30 * t)
        return (r, g, b)

    def blown_flat(y):
        # the commit's own quoted flat near-clipped cream, +/-1 row-to-row noise
        t = (y - SKY_TOP) / span_rows
        return (253 - (1 if t > 0.5 else 0), 226, 191)

    def no_sky_visible(y):
        # flat mid-grey, same value the "terrain" rows already use -- fails BOTH
        # tiers cleanly (r<250 so not TIER1; r==g==b so b is never > r+5, not
        # TIER2 either), unlike per-pixel random noise, which turned out to pass
        # TIER2 by sheer volume (about half of independent-uniform pixel pairs
        # satisfy b>r+5, so a "noisy" sky band was actually being scored, not
        # correctly refused) -- a real frame with no open sky (fully indoors, or
        # a ceiling) looks like a flat surface, not noise, so this is the honest
        # ground truth for "nothing here answers either predicate."
        return (90, 90, 90)

    cases = [
        ("PASS case (synthetic healthy gradient)",
         _synthetic(W, H, good_gradient), EXIT_PASS),
        ("FAIL case (synthetic blown-flat, commit's [252,226,191])",
         _synthetic(W, H, blown_flat), EXIT_FAIL),
        ("UNMEASURABLE case (flat grey sky band -- no sky-tier pixel anywhere)",
         _synthetic(W, H, no_sky_visible), EXIT_UNMEASURABLE),
    ]

    all_ok = True
    with tempfile.TemporaryDirectory() as td:
        for label, im, want in cases:
            p = Path(td) / "case.png"
            im.save(p)
            m = measure(p)
            got, why = verdict(m)
            ok = got == want
            all_ok &= ok
            print(f"  {label}")
            print(f"    R-span={m['r_span']:.2f}  sky-band%R>=250={m['pct_r250_sky_band']:.2f}%  "
                  f"unoccluded_rows={m['unoccluded_rows']}")
            print(f"    got exit={got}  want exit={want}  -> {'OK' if ok else 'BROKEN'}")
            print()

    if all_ok:
        print("CALIBRATION OK -- the meter tells PASS/FAIL/UNMEASURABLE apart correctly "
              "on known ground truth. Verdicts on real frames below can be trusted.")
        return 0
    print("CALIBRATION FAILED -- the meter is broken, not the image. Do not trust any "
          "verdict this script prints until this passes.")
    return 1


def main(argv):
    if argv[:1] == ["--selftest"]:
        return _selftest()
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return EXIT_UNMEASURABLE
    path = Path(argv[0])
    if not path.is_file():
        print(f"UNMEASURABLE -- no such file: {path}", file=sys.stderr)
        return EXIT_UNMEASURABLE
    code, _ = report(path)
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
