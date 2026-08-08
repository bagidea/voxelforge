#!/usr/bin/env python3
"""ADVISORY detail readout for a hero frame — G3/G5/G6 sampling detail + the
monochrome-collapse check.

NOT THE AUTHORITY. `scripts/grade_gate.py` is the canonical G3/G5/G6 grader and
is what docs/look-acceptance-rubric.md scores against. This script exists for the
extra detail it prints (the top-4 G6 candidate patches and the [MONO] material
identity block, neither of which grade_gate.py reports). Where the two disagree,
grade_gate.py wins.

Why it was demoted (2026-08-07): its sampling was hard-tuned to ONE frame — the
tight indoor golden-kitchen hero shot — and every Edhari plate in the current
shotset is outdoors. On those it silently graded the wrong things:

  * G5 searched a hardcoded box on the LEFT of frame ("the window is screen-left"),
    which on an outdoor plate points at open sky.
  * G6 had no `L >= 55` sunlit floor, so it was still running the pre-2026-08-04
    rule after the rubric closed clause (ข).

Both are fixed below, and the search boxes are gone — but a fixed search can still
land somewhere meaningless, so every verdict now carries the SITE it was read at,
and a gate whose site fails a sanity guard reports UNRELIABLE rather than a
number. A gate that cannot see its subject must say so, not guess: regrade.py
consumes the SUMMARY block, and UNRELIABLE parses as "no verdict" there, so an
unreadable plate contributes nothing instead of a confident wrong answer.

Two more of the same species, found 2026-08-08 by running the script over all
eight before plates and cropping every site it chose:

  * G6 had no flatness guard, so on `gate3-boot` it scored the player capsule's
    BLOOM HALO at (564,360) and returned a clean PASS — the twin of the torch
    that gave grade_gate.py a false PASS. Faces here are flat-shaded, so the
    patch's own luminance std-dev separates them cleanly: sd 24.1 there against
    <= 5.8 for every other candidate in the set. See PATCH_SD_MAX.
  * [MONO] tested `g >= r`, which grade_g7.py already documents as unusable in
    this look — it reported 0.00% vegetation on `s1-vista`, a plate whose bottom
    half is grass. Now a hue band, same as grade_g7.py.
  * [MONO] again, same day, found in review of that fix: the SHARE was corrected
    but the SITE printed beside it was still `argmax(G)` labelled "greenest" —
    an unguarded extremum, which is the one thing this file had just been
    rewritten to stop doing. It printed the bloom halo on `gate3-boot`
    (564,345), 15 px from the patch G6 now rejects, and the frame corner on
    `hero` (1587,810), the coordinate G6 flags `on-frame-edge`. Now the MEDIAN
    vegetation pixel, run through site_flags() like every other gate.

None was the fixed-eyedrop hypothesis: the boxes really are gone. All three are
the same underlying failure, that a search over the whole frame will happily
find the brightest wrong thing unless the site is sanity-checked — and the third
proves it survives a fix to the metric if the site is left alone.

Usage: python scripts/grade_hero.py <frame>-nohud2.png
"""
import colorsys
import os
import sys
import math
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

# HARD GUARD — same 6 gates as grade_gate.py on the same kind of frame, so the
# same rule: no numbers off a HUDded capture. (grade_ref2.py, whose sampling
# this mirrors, is exempt — it grades the golden ref, not a capture. See
# nohud2_guard.EXEMPT.) The `argv[1] or "hero-01.png"` default is gone.
require_nohud2(sys.argv[1:2], tool="grade_hero.py")

path = sys.argv[1]
img = Image.open(path).convert("RGB")
W, H = img.size
px = img.load()

# ---- sanity-guard constants -------------------------------------------------
# EDGE_FRAC: a patch centre this close to any border is reported but not trusted.
# The frame edge is where rim light, the de-HUD crop seam and the last voxel
# column all live, so a "brightest patch" that lands there is usually an artifact
# of the border rather than a surface the shot is actually about. Measured case:
# on the `hero` plate G6 pinned to (1590,809) of 1600x820 — 10 px from the right
# border, 11 px from the bottom — at every exposure tried.
EDGE_FRAC = 0.02
# SKY_BAND: above this the frame is sky/skyline on every wide plate in the set.
SKY_BAND = 0.30
# A pixel this blue-leaning is sky or haze, never sunlit wood.
SKY_RB = -20
# G5 locates glass by NEUTRAL hue, exactly as grade_gate.py does; a "brightest
# pixel" outside this is a fire, an emissive or a saturated wall, not a pane.
NEUTRAL_RB = 45
# PATCH_SD_MAX: a G6 patch whose own luminance varies more than this many L
# points is not a surface. Every lit face in this renderer is flat-shaded, so a
# genuine sunlit patch is near-uniform inside; a bloom halo is a continuous
# gradient and a silhouette straddles two depths. Measured on the before shotset
# (2026-08-08): the top-4 G6 candidates on all eight plates sit at sd <= 5.8 --
# except gate3-boot's winner @(564,360), sd 24.1, which a crop shows is the
# glowing rim of the player capsule, and which was scoring a confident PASS.
# That is the same defect Flamingo found in grade_gate.py (a torch reading as
# sunlight), so it gets the same treatment: flag the site, don't invent a number.
PATCH_SD_MAX = 10.0


def Lum(r, g, b): return (0.2126*r + 0.7152*g + 0.0722*b)/255*100


def patch(cx, cy, rad=5):
    rs = gs = bs = n = 0
    for y in range(max(0, cy-rad), min(H, cy+rad+1)):
        for x in range(max(0, cx-rad), min(W, cx+rad+1)):
            r, g, b = px[x, y]; rs += r; gs += g; bs += b; n += 1
    return rs/n, gs/n, bs/n


def patch_sd(cx, cy, rad=4):
    """Luminance std-dev INSIDE the patch — flat face vs glow gradient."""
    ls = [Lum(*px[x, y])
          for y in range(max(0, cy-rad), min(H, cy+rad+1))
          for x in range(max(0, cx-rad), min(W, cx+rad+1))]
    m = sum(ls)/len(ls)
    return math.sqrt(sum((v-m)**2 for v in ls)/len(ls))


def site_flags(x, y, r=None, g=None, b=None, sd=None):
    """Reasons this coordinate is a poor place to read a gate from."""
    f = []
    mx, my = EDGE_FRAC*W, EDGE_FRAC*H
    if x < mx or x > W-1-mx or y < my or y > H-1-my:
        f.append("on-frame-edge")
    if y < SKY_BAND*H:
        f.append("in-sky-band")
    if r is not None and (r-b) <= SKY_RB:
        f.append("sky-hued")
    if sd is not None and sd > PATCH_SD_MAX:
        f.append(f"not-a-flat-surface(sd={sd:.1f}L)")
    return f


print(f"== {path}  ({W}x{H}) ==")
print("== ADVISORY ONLY - grade_gate.py is the canonical G3/G5/G6 grader ==")

# ---- G6: brightest sunlit WOOD patch ----------------------------------------
# Whole frame below the horizon line, no left/right exclusion: the old version
# skipped x < 0.28W to dodge a window that only exists on one framing.
cands = []
for y in range(int(0.45*H), H-6, 6):
    for x in range(6, W-6, 6):
        r, g, b = patch(x, y, 4)
        if r > g > b and 40 <= (r-b) <= 210:
            cands.append((Lum(r, g, b), x, y, r, g, b))
cands.sort(reverse=True)
print("\n[G6] brightest golden patches (R>G>B, R-B 40..210), best first:")
g6_ok = False
g6_flags = ["no-golden-patch-in-frame"] if not cands else []
for lum, x, y, r, g, b in cands[:4]:
    fl = site_flags(x, y, r, g, b, patch_sd(x, y))
    # rubric §G6 clause (ข), added 2026-08-04: colour alone is not sunlight.
    ok = lum >= 55
    print(f"  @({x:4d},{y:4d}) RGB=({r:5.1f},{g:5.1f},{b:5.1f}) L={lum:4.1f}% "
          f"R-B={r-b:+6.1f} {'OK' if ok else 'x (L<55)'}"
          f"{'  [' + ','.join(fl) + ']' if fl else ''}")
if cands:
    g6_lum, g6x, g6y, g6r, g6g, g6b = cands[0]
    g6_ok = g6_lum >= 55
    g6_flags = site_flags(g6x, g6y, g6r, g6g, g6b, patch_sd(g6x, g6y))
    # A winner that is merely the corner of the frame is not a finding about the
    # lighting. Only the patch actually scored is guarded — the runners-up are
    # printed for context.
    print(f"  best @({g6x},{g6y}) L={g6_lum:.1f}% (floor 55)")
print(f"  G6 warm-golden + sunlit: {'PASS' if g6_ok else 'FAIL'}"
      f"{'   UNRELIABLE SITE: ' + ','.join(g6_flags) if g6_flags else ''}")

# ---- G5: window highlight ---------------------------------------------------
# Locate glass by hue over the WHOLE frame (grade_gate.py's method) instead of
# assuming a screen-left box. If nothing near-neutral and bright exists, the
# frame has no window and G5 has no subject — say that instead of grading the
# brightest saturated thing, which on `hero` is the campfire (255,34,0).
bwin = (-1, 0, 0)
for y in range(0, H, 3):
    for x in range(0, W, 3):
        r, g, b = px[x, y]
        mx = max(r, g, b)
        if mx > bwin[0] and mx >= 200 and abs(r-b) <= NEUTRAL_RB:
            bwin = (mx, x, y)
g5_flags = []
if bwin[0] < 0:
    g5_ok = False
    g5_flags = ["no-glassy-highlight-in-frame"]
    print("\n[G5] no bright near-neutral (glassy) region exists in this frame")
else:
    _, bx, by = bwin
    print(f"\n[G5] window located @({bx},{by}) by neutral hue (|R-B| <= {NEUTRAL_RB})")
    samples = []
    for dy in (-int(0.12*H), 0, int(0.12*H)):
        yy = min(H-4, max(4, by+dy))
        r, g, b = patch(bx, yy, 3)
        samples.append((yy, r, g, b, Lum(r, g, b)))
        print(f"  @({bx},{yy}) RGB=({r:5.1f},{g:5.1f},{b:5.1f}) L={Lum(r,g,b):5.1f}%")
    br, bg, bb = samples[[s[4] for s in samples].index(max(s[4] for s in samples))][1:4]
    gb_min = min(bg, bb)
    Ls = [s[4] for s in samples]
    grad = max(Ls) - min(Ls)
    g5_ok = (gb_min <= 245) and (grad >= 8)
    g5_flags = site_flags(bx, by, *px[bx, by])
    # Clamped samples are the crop seam, not a gradient: if the located pixel is
    # so near an edge that two of the three taps collapse onto the same row, the
    # spread is an artifact of the clamp.
    if len({s[0] for s in samples}) < 3:
        g5_flags.append("vertical-taps-clamped")
    print(f"  brightest G/B min={gb_min:.0f} (<=245?) gradient={grad:.1f}L (>=8?): "
          f"{'PASS' if g5_ok else 'FAIL'}")
print(f"  G5 verdict: {'PASS' if g5_ok else 'FAIL'}"
      f"{'   UNRELIABLE SITE: ' + ','.join(g5_flags) if g5_flags else ''}")

# ---- G3: interior luminance floor + warm/not-blue ---------------------------
m = int(0.08*W)
mh = int(0.08*H)
vals = []
for y in range(mh, H-mh, 3):
    for x in range(m, W-m, 3):
        r, g, b = px[x, y]; vals.append(Lum(r, g, b))
vals.sort()
p05 = vals[int(len(vals)*0.05)]
p10 = vals[int(len(vals)*0.10)]
best = None
for y in range(mh, H-mh, 8):
    for x in range(m, W-m, 8):
        r, g, b = patch(x, y, 5)
        l = Lum(r, g, b)
        if best is None or l < best[0]: best = (l, x, y, r, g, b)
dl, dx, dy, dr, dg, db = best
warm = dr >= dg >= db
notblue = db <= dr
g3_ok = (p05 >= 8) and warm and notblue
g3_flags = site_flags(dx, dy, dr, dg, db)
# The floor is 8, not 10: rubric §G3 and its calibration log replaced the old
# "darkest pixel L >= 10%" with "interior p05-L >= 8%" on 2026-07-25, because a
# single darkest pixel is always a crevice. grade_gate.py:55 uses 8 too. The
# printout said "job floor >=10" until 2026-08-08 while the code tested 8 —
# harmless on today's plates (p05 runs 14..29) but it read as a soft PASS.
print(f"\n[G3] interior p05-L={p05:.1f}% p10-L={p10:.1f}% (rubric floor: p05>=8)")
print(f"  darkest patch @({dx},{dy}) RGB=({dr:.1f},{dg:.1f},{db:.1f}) "
      f"warm(R>=G>=B)={warm} R-B={dr-db:+.1f}")
print(f"  G3 (p05>=8 & warm & not-blue): {'PASS' if g3_ok else 'FAIL'}")

# ---- monochrome-collapse: material identity survives ------------------------
# The one block here grade_gate.py does not cover.
#
# By HUE BAND, not by channel ordering. The old rule was `g >= r and g >= b`,
# which is the exact test scripts/grade_g7.py:is_veg documents as unusable:
# sunlit grass in this look is warm yellow-olive (R > G), so a green-dominant
# mask finds ZERO vegetation on a correctly lit frame. Measured 2026-08-08 on
# s1-vista-nohud2.png -- a village vista whose bottom half is solid grass -- the
# old rule reported 0.00%, i.e. "material identity collapsed", on a plate that
# had not collapsed at all. The hue band 40..150deg reads yellow-olive through
# green and sees 10.0% of that same frame.
VEG_HUE = (40.0, 150.0)
VEG_SAT_MIN, VEG_VAL_MIN = 15.0, 10.0


def is_veg(r, g, b):
    h, s, v = colorsys.rgb_to_hsv(r/255, g/255, b/255)
    return (VEG_HUE[0] <= h*360 <= VEG_HUE[1] and s*100 > VEG_SAT_MIN
            and v*100 > VEG_VAL_MIN)


print("\n[MONO] material identity:")


def veg_hue_lum(r, g, b):
    h, s, v = colorsys.rgb_to_hsv(r/255, g/255, b/255)
    return h*360, Lum(r, g, b)


# The printed site is the MEDIAN vegetation pixel (hue and luminance), not the
# brightest, and it is passed through site_flags() like every other gate here.
#
# `max(g)` was the third head of the same bug the rest of this file was fixed for
# (2026-08-08, reviewer-found): an unguarded extremum over the whole frame finds
# the brightest wrong thing and a label calls it "greenest". Measured: on
# gate3-boot it returned (564,345) RGB=(251.8,168.4,12.3) — 15 px from the bloom
# halo at (564,360) that G6 had just been taught to reject, i.e. the same object;
# on `hero` it returned (1587,810), the exact corner G6 flags `on-frame-edge`.
# Neither is green and neither is evidence. The share above was already fixed to
# a hue band; this is the site catching up with it.
#
# Median, not extremum, because the question is "does material identity survive",
# and a typical vegetation pixel answers that — an outlier never did.
sites = []
gn = 0; tot = 0
for y in range(0, H, 3):
    for x in range(0, W, 3):
        r, g, b = px[x, y]; tot += 1
        if is_veg(r, g, b):
            gn += 1
            h, l = veg_hue_lum(r, g, b)
            sites.append((x, y, r, g, b, h, l))
share = gn/tot*100
print(f"  vegetation share = {share:.2f}%  (accent visible if >0.2%; hue "
      f"{VEG_HUE[0]:.0f}-{VEG_HUE[1]:.0f}deg, was 'g>=r' until 2026-08-08)")
if sites:
    hs = sorted(s[5] for s in sites); ls = sorted(s[6] for s in sites)
    mh, ml = hs[len(hs)//2], ls[len(ls)//2]
    # Prefer a median-like site that already passes the cheap guards; if the whole
    # mask sits on an edge / in the sky there is nothing clean to show, so show the
    # median anyway and let the flags say why it should not be believed.
    clean = [s for s in sites if not site_flags(s[0], s[1], s[2], s[3], s[4])]
    pool = clean or sites
    x, y, r, g, b, h, l = min(pool, key=lambda s: ((s[5]-mh)/10.0)**2 + ((s[6]-ml)/10.0)**2)
    rr, gg, bb = patch(x, y, 6)
    fl = site_flags(x, y, r, g, b, patch_sd(x, y))
    print(f"    typical veg @({x},{y}) patch RGB=({rr:.1f},{gg:.1f},{bb:.1f}) "
          f"hue={h:.1f}deg L={l:.1f}%  (mask median hue={mh:.1f}deg L={ml:.1f}%; "
          f"{len(clean)}/{len(sites)} samples pass the site guards)")
    if fl:
        print(f"    UNRELIABLE SITE: {','.join(fl)}")
rs = gs = bs = 0; n = 0
for y in range(0, H, 4):
    for x in range(0, W, 4):
        r, g, b = px[x, y]; rs += r; gs += g; bs += b; n += 1
print(f"  frame mean RGB=({rs/n:.0f},{gs/n:.0f},{bs/n:.0f})  warm-lean R-B={(rs-bs)/n:+.0f}")


def verdict(ok, flags):
    """UNRELIABLE (not PASS/FAIL) when the site fails a guard.

    regrade.py's _parse_hero matches only PASS|FAIL here, so this string makes a
    guarded gate parse as ABSENT — which is the honest reading. The alternative,
    reporting the raw verdict anyway, is what let this script contribute a G5
    number for eight outdoor plates that have no window.
    """
    return f"{'PASS' if ok else 'FAIL'}" if not flags else f"UNRELIABLE ({','.join(flags)})"


print("\n== SUMMARY ==")
print(f"  G3 bounce   : {verdict(g3_ok, g3_flags)}")
print(f"  G5 no-clip  : {verdict(g5_ok, g5_flags)}")
print(f"  G6 warm     : {verdict(g6_ok, g6_flags)}")
print(f"  veg share   : {share:.2f}%")
print("  (advisory - grade_gate.py is canonical for G3/G5/G6)")
