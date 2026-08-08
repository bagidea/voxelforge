#!/usr/bin/env python3
"""Is there a CAST SHADOW in this frame, and is its edge SOFT? (G4a)

`scripts/measure_penumbra.py` answers the second question without asking the
first: it takes the strongest luminance step in each floor row and measures its
20->80 ramp. On the 2026-08-07 plates that reported "penumbra 6-21 px" for
frames whose strongest floor steps were the seam between two block FACES and
the seam where grass meets terracotta. Neither is a shadow; both are steps in
the picture. So this file is built around the three ways that measurement lies:

  1. NOT EVERY DARK REGION IS A SHADOW. The grass block's own albedo is a
     light/dark checker, and once the key light raised contrast the checker
     resolves into two luminance humps all by itself. `--albedo-check` settles
     it against a plate of the same camera off the SAME binary with the sun's
     AZIMUTH MOVED: painted texture is bolted to the blocks and cannot follow
     the sun, a cast shadow must. Measured on `s1-vista`, one exe, only
     `VOXELFORGE_LOOK_SUN` moving: repeat shot r=0.994, azimuth +180 deg r=0.010.

     THE CONTROL THIS FLAG USED TO TAKE WAS INVALID AND IT SHIPPED A FALSE
     CLAIM. Until 2026-08-09 it compared against the PRE-LIGHT plate of the same
     camera -- which is the same azimuth 205 deg, only 17 deg instead of 22. The
     relight (8303db5) changed the key/fill RATIO, not whether a shadow is drawn,
     so the same walls threw the same bands onto the same grass in both frames
     and a real cast shadow that barely moved scored 89-94 % "already dark
     before" = PAINTED. That number is what put "THE GRASS STILL HAS NO CAST
     SHADOW ON IT" into docs/note-to-director-N6-regrade-2026-08-08.md; it is
     retracted there. So the control's provenance is now CHECKED, not trusted:
     the plate's sun comes off the shoot script's own `manifest.json` and a
     control within [`MIN_AZIMUTH_MOVE`] degrees of the plate's azimuth is
     REFUSED with no verdict printed. `scripts/tests/test_albedo_check_control.py`
     locks that refusal.
  2. NOT EVERY EDGE IS A SHADOW EDGE. A material boundary and a block-face
     seam both produce a clean luminance step. Neither is excluded by any
     numeric guard that survives contact with this renderer -- a hue-continuity
     test looked obvious and is WRONG here, measured: across a real cast-shadow
     edge on the terracotta wall of `gate3-boot` the hue swings 14.8 -> 36.4 deg,
     because the key is orange and the fill is sky-blue. So edges are not
     auto-classified. `--at X,Y` measures an edge a human has pointed at, and
     the frame-wide sweep is reported as what it is: every strong edge, shadow
     or not.
  3. A WIDTH MEANS NOTHING WITHOUT THE CONTROL. This renderer's edges are
     already 1-3 px from AA/TAA whatever the light does, and a DIAGONAL edge is
     wider than an axis-aligned one at identical softness, purely from the
     raster staircase. So every run also measures the sky silhouette -- pure
     geometry, no penumbra by definition -- and compares against the control
     edges in the SAME orientation bucket. Comparing a diagonal shadow edge to a
     vertical silhouette is how a hard shadow gets reported as soft.

Usage:
  cast_shadow_penumbra.py <frame>-nohud2.png [more...] [--at X,Y[;X,Y...]]
                          [--albedo-check <azimuth-moved>-nohud2.png]
                          [--sun ELEV,AZIM] [--control-sun ELEV,AZIM]

  The control is the same camera off the same exe, re-shot with
  `VOXELFORGE_LOOK_SUN=<elev>,<azim+180>,<lux>`. Both suns are read from the
  shoot script's `manifest.json`; `--sun` / `--control-sun` declare them by hand
  when a plate has no manifest beside it.
"""
import json
import os
import re
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from grass_bimodality import grass_select, hsv_planes, lum, shape  # noqa: E402
from nohud2_guard import require_nohud2  # noqa: E402

R = 14.0             # px — half-length of a sampled profile
DT = 0.25            # px — profile sample pitch
PLATEAU = 4.0        # px at each end that must be flat
PLATEAU_SD = 3.0     # L — above this the "plateau" is texture, not a level
MIN_STEP = 10.0      # L — dark->light range needed before a ramp is gradeable
MAX_EDGES = 600      # profiles per class; more is only slower
SKY_HUE = (195.0, 265.0)
ORIENT_BINS = [(0, 15), (15, 30), (30, 46)]  # deg of the normal off the nearest axis

# ---- the --albedo-check control, and the guard on it ------------------------
MIN_AZIMUTH_MOVE = 90.0   # deg — a control nearer than this to the plate's own
#                           azimuth is REFUSED. Not tuned: it is a floor with a
#                           measured gap under it. The invalid same-azimuth
#                           control scored r=0.761 (sep 0 deg) and the honest
#                           one r=0.010 (sep 180 deg); the only other azimuth
#                           shot, 115 deg (sep 90), kept just 35,802 grass px
#                           because most of that frame's grass falls out of the
#                           hue window once it is in shade. So 90 is where the
#                           control stops being able to answer, and 180 is what
#                           to actually shoot.
PAINTED_R = 0.80          # r over the shared grass mask above which the sharp
#                           structure is IN THE TEXTURE. Painted albedo cannot
#                           move: r~1 under any sun (repeat shot 0.994, contact
#                           shadows off 0.996). A real cast shadow decorrelates
#                           to 0.010. Nothing measured on this renderer lands
#                           between 0.2 and 0.7, so the bar sits in open space.
MIN_SHARED_GRASS = 0.5    # the control must still see at least this share of the
#                           PLATE's own grass, or it is not looking at the same
#                           subject. Measured, one exe, s1-vista: +180 deg keeps
#                           275,286 of 310,673 px (89 %), 90 deg keeps 35,802
#                           (12 %) — the mask is a hue/sat rule and that framing's
#                           grass leaves the green window once it is in shade. A
#                           verdict off 12 % of the subject is not a verdict.
SHIPPED_SUN = (22.0, 205.0)   # client/src/look.rs `Hour::GOLDEN` — elev, azim.
#                           What a plate carries when its manifest declares no
#                           VOXELFORGE_LOOK_SUN override.


def box3(a):
    p = np.pad(a, 1, mode="edge")
    return sum(p[y:y + a.shape[0], x:x + a.shape[1]]
               for y in range(3) for x in range(3)) / 9.0


def label(mask):
    """Two-pass 4-connected CCL; no scipy on this box. The python loop walks the
    mask's own pixels only, which is thousands, not the whole frame."""
    H, W = mask.shape
    lab = np.zeros((H, W), np.int32)
    parent = [0]

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    nxt = 1
    for y in range(H):
        for x in np.flatnonzero(mask[y]):
            up = lab[y - 1, x] if y else 0
            lf = lab[y, x - 1] if x else 0
            if up and lf:
                ru, rl = find(up), find(lf)
                if ru != rl:
                    parent[max(ru, rl)] = min(ru, rl)
                lab[y, x] = min(up, lf)
            elif up or lf:
                lab[y, x] = up or lf
            else:
                parent.append(nxt)
                lab[y, x] = nxt
                nxt += 1
    flat = np.array([find(i) for i in range(nxt)], np.int32)
    _, lab = np.unique(flat[lab], return_inverse=True)
    return lab.reshape(H, W)


def sample(a, xs, ys):
    H, W = a.shape
    xs, ys = np.clip(xs, 0, W - 1.001), np.clip(ys, 0, H - 1.001)
    x0, y0 = xs.astype(int), ys.astype(int)
    fx, fy = xs - x0, ys - y0
    return (a[y0, x0] * (1 - fx) * (1 - fy) + a[y0, x0 + 1] * fx * (1 - fy)
            + a[y0 + 1, x0] * (1 - fx) * fy + a[y0 + 1, x0 + 1] * fx * fy)


class Frame:
    def __init__(self, path):
        self.path = path
        self.arr = np.asarray(Image.open(path).convert("RGB"))
        self.H, self.W = self.arr.shape[:2]
        self.L = lum(self.arr)
        self.Ls = box3(self.L)                       # measured on
        big = box3(box3(box3(self.L)))               # normal direction only
        self.gy, self.gx = np.gradient(big)
        hue, sat, val = hsv_planes(self.arr)
        self.sky = ((hue >= SKY_HUE[0]) & (hue <= SKY_HUE[1]) & (sat > 20) & (val > 40))

    def profile(self, x, y, allow_sky=False):
        """(width20-80, orientation deg) for the edge at (x,y), or None."""
        g = np.hypot(self.gx[y, x], self.gy[y, x])
        if g < 1e-6:
            return None
        nx, ny = self.gx[y, x] / g, self.gy[y, x] / g       # points dark -> light
        t = np.arange(-R, R + DT, DT)
        xs, ys = x + nx * t, y + ny * t
        if not allow_sky:
            xi = np.clip(np.rint(xs), 0, self.W - 1).astype(int)
            yi = np.clip(np.rint(ys), 0, self.H - 1).astype(int)
            if self.sky[yi, xi].any():
                return None
        v = sample(self.Ls, xs, ys)
        n = int(PLATEAU / DT)
        dark, light = v[:n], v[-n:]
        if dark.std() > PLATEAU_SD or light.std() > PLATEAU_SD:
            return None
        d, l = dark.mean(), light.mean()
        if l - d < MIN_STEP:
            return None
        lo, hi = d + 0.2 * (l - d), d + 0.8 * (l - d)

        def cross(level):
            i = int(np.argmax(v >= level))
            if i == 0 or v[i] <= v[i - 1]:
                return None
            return t[i - 1] + (level - v[i - 1]) / (v[i] - v[i - 1]) * DT

        t20, t80 = cross(lo), cross(hi)
        if t20 is None or t80 is None or t80 < t20:
            return None
        ang = np.degrees(np.arctan2(abs(ny), abs(nx)))
        return t80 - t20, min(ang, 90.0 - ang)

    def ridge(self, mask=None):
        """Pixels that are a local maximum of |grad L| along the gradient."""
        g = np.hypot(self.gx, self.gy)
        keep = np.zeros_like(g, bool)
        i = g > 1.0
        ys, xs = np.nonzero(i)
        nx, ny = self.gx[i] / g[i], self.gy[i] / g[i]
        a = sample(g, xs + nx, ys + ny)
        b = sample(g, xs - nx, ys - ny)
        keep[ys, xs] = (g[i] >= a) & (g[i] >= b)
        return keep & mask if mask is not None else keep


def thin(mask, n):
    ys, xs = np.nonzero(mask)
    if ys.size <= n:
        return list(zip(ys, xs))
    step = max(1, ys.size // n)
    return list(zip(ys[::step][:n], xs[::step][:n]))


def collect(fr, pts, allow_sky):
    w, o = [], []
    for y, x in pts:
        r = fr.profile(int(x), int(y), allow_sky)
        if r:
            w.append(r[0])
            o.append(r[1])
    return np.array(w), np.array(o)


def by_orient(w, o):
    return {b: w[(o >= b[0]) & (o < b[1])] for b in ORIENT_BINS}


def fmt(a):
    if not a.size:
        return "     n=0"
    return (f"n={a.size:4d} median={np.median(a):5.2f}px "
            f"p25={np.percentile(a, 25):5.2f} p75={np.percentile(a, 75):5.2f}")


def sun_from_flags(s):
    """(elev, azim) out of a `KEY=V KEY=V` flags string, or None."""
    m = re.search(r"VOXELFORGE_LOOK_SUN=(-?[\d.]+),(-?[\d.]+)", s or "")
    return (float(m.group(1)), float(m.group(2))) if m else None


def plate_sun(path, _hop=0):
    """(elev, azim, where-that-came-from) for a plate, or None if unknowable.

    Read off the `manifest.json` the shoot script wrote beside the plate --
    provenance, not a promise from whoever typed the grading command. The shot
    dirs are `<probe>/{before,after}/<plate>.png` with `<probe>/manifest.json`
    carrying a top-level `extra_env` and a per-shot `env`; a manifest that names
    the plate as its `after` and declares no VOXELFORGE_LOOK_SUN means that shot
    took the shipped hour, which is a fact about it, not a gap.

    A `before` plate is NOT that shot's sun: it is a copy of an earlier run's
    frame (`before_src`), often off an older binary whose `Hour::GOLDEN` was a
    different elevation. So it is followed one hop to its own manifest, and
    reported unknown if that run left none -- refusing beats attributing this
    hour's numbers to last week's plate.
    """
    p = os.path.abspath(path)
    base = os.path.basename(p)
    d = os.path.dirname(p)
    for _ in range(3):
        man = os.path.join(d, "manifest.json")
        if os.path.isfile(man):
            try:                       # PowerShell writes it BOM-first
                with open(man, encoding="utf-8-sig") as f:
                    j = json.load(f)
            except (OSError, ValueError):
                return None
            try:
                where = os.path.relpath(man, os.getcwd())
            except ValueError:     # a plate on another drive letter: ntpath
                where = man        # refuses to relate C:\ to E:\, absolute is fine
            for shot in j.get("shots", []):
                if base == os.path.basename(str(shot.get("after", ""))):
                    sun = sun_from_flags(shot.get("env")) or sun_from_flags(j.get("extra_env"))
                    if sun:
                        return sun[0], sun[1], f"{where} VOXELFORGE_LOOK_SUN"
                    return SHIPPED_SUN[0], SHIPPED_SUN[1], f"{where}, no sun override => GOLDEN"
                if base == os.path.basename(str(shot.get("before", ""))):
                    src = shot.get("before_src")
                    if _hop or not src or not os.path.isfile(src):
                        return None
                    return plate_sun(src, _hop + 1)
            return None                # a manifest that does not know this plate
        d = os.path.dirname(d)
    return None


def azim_sep(a, b):
    """Angular separation of two azimuths in degrees, 0..180 (wraps)."""
    return abs((a - b + 180.0) % 360.0 - 180.0)


def albedo_check(after_path, control_path, sun=None, control_sun=None):
    """Is the frame's shade population CAST BY A LIGHT, or painted in the albedo?

    Answered against the same camera off the same exe with the sun's AZIMUTH
    MOVED -- the one control painted texture cannot follow (module docstring
    item 1 for why the pre-light plate could not, and what that cost). Refuses
    rather than guessing when the control's sun is unknown or too close.
    """
    sa = (sun + ("--sun",)) if sun else plate_sun(after_path)
    sb = (control_sun + ("--control-sun",)) if control_sun else plate_sun(control_path)
    for label_, s, path, flag in (("plate", sa, after_path, "--sun"),
                                  ("control", sb, control_path, "--control-sun")):
        if s is None:
            print(f"   albedo-check REFUSED: nothing on disk establishes the sun the {label_} "
                  f"({os.path.basename(path)}) was shot under -- no manifest.json beside it "
                  "names it as an `after`, so the control cannot be checked. Declare it with "
                  f"`{flag} ELEV,AZIM`.")
            return
    sep = azim_sep(sa[1], sb[1])
    print(f"   albedo-check control: plate sun {sa[0]:.0f}deg/{sa[1]:.0f}deg [{sa[2]}], "
          f"control {sb[0]:.0f}deg/{sb[1]:.0f}deg [{sb[2]}] -- azimuth separation {sep:.0f}deg")
    if sep < MIN_AZIMUTH_MOVE:
        print(f"                 REFUSED: the control is within {MIN_AZIMUTH_MOVE:.0f}deg of the "
              "plate's own azimuth, so the same walls throw the same bands onto the same grass "
              "in both frames and a real cast shadow that barely moved scores as paint. This is "
              "the control that put \"the grass has no cast shadow\" in docs/"
              "note-to-director-N6-regrade-2026-08-08.md (retracted). Re-shoot the control off "
              "the SAME exe with VOXELFORGE_LOOK_SUN=<elev>,<azim+180>,<lux>.")
        return

    from sun_locked_edges import corr_stats   # deferred: sun_locked_edges imports
    try:                                      # this module at ITS top, so a
        c = corr_stats(after_path, control_path)   # top-level import here is a cycle
    except ValueError:
        print("   albedo-check: frames differ in size -- not the same camera, skipped")
        return
    share = c["px"] / c["px_plate"] if c["px_plate"] else 0.0
    if not np.isfinite(c["r"]) or share < MIN_SHARED_GRASS:
        print(f"   albedo-check: UNRELIABLE -- the control shares only {c['px']} of the plate's "
              f"{c['px_plate']} grass px ({100 * share:.1f}%, floor {100 * MIN_SHARED_GRASS:.0f}%). "
              "Most of the subject left the mask under the moved sun; no verdict. Shoot the "
              "control at +180deg, which keeps the grass lit from the other side rather than "
              "dropping it into shade.")
        return
    painted = c["r"] > PAINTED_R
    print(f"   albedo-check: r = {c['r']:.3f} over {c['px']} shared grass px "
          f"({100 * share:.0f}% of the plate's own grass, high-pass box radius {c['radius']}px, "
          f"mean L {c['mean_b']:.1f} -> {c['mean_a']:.1f})")
    print("                 -> " + ("PAINTED IN THE ALBEDO -- the sharp structure did not move "
                                    f"with the sun (r > {PAINTED_R:.2f})"
                                    if painted else
                                    "CAST SHADOW -- the sharp structure is not in the texture, "
                                    "it moved with the sun"))

    # Corroboration, in the units the old (invalid-control) column was reported
    # in, so the two are comparable: the same blob-overlap question asked of a
    # control that can actually answer it.
    a = np.asarray(Image.open(after_path).convert("RGB"))
    b = np.asarray(Image.open(control_path).convert("RGB"))
    m, _, _ = grass_select(a)
    La, Lb = lum(a), lum(b)
    s = shape(La[m], 0.0, 100.0, 100, 2.5) if m.any() else {"peaks": None}
    if not s["peaks"]:
        print("                 corroboration: grass reads ONE hump -- no shade population")
        return
    shade = m & (La < s["valley"])
    lab = label(shade)
    areas = np.bincount(lab.ravel())[1:]
    big = np.isin(lab, np.flatnonzero(areas >= 500) + 1) & shade
    mb, _, _ = grass_select(b)
    dark_then = Lb < np.median(Lb[mb if mb.any() else m])
    ov, base = dark_then[big].mean(), dark_then[m].mean()
    print(f"                 corroboration: shade cut L={s['valley']:.1f}, {int(big.sum())}px in "
          f"blobs>=500px; {100 * ov:.1f}% of them are ALSO the dark half under the moved sun "
          f"(base rate {100 * base:.1f}%)")


def run(path, ats, control, sun=None, control_sun=None):
    print(f"== {os.path.splitext(os.path.basename(path))[0]}")
    fr = Frame(path)
    if control:
        albedo_check(path, control, sun, control_sun)

    ctl_w, ctl_o = collect(fr, thin(fr.ridge(fr.sky), MAX_EDGES), allow_sky=True)
    ctl = by_orient(ctl_w, ctl_o)
    print(f"   CONTROL sky silhouette (geometry, zero penumbra) : {fmt(ctl_w)}")
    for b in ORIENT_BINS:
        print(f"      normal {b[0]:2d}-{b[1]:2d}deg off-axis : {fmt(ctl[b])}")

    all_w, all_o = collect(fr, thin(fr.ridge(~fr.sky), MAX_EDGES), allow_sky=False)
    print(f"   ALL in-scene edges (shadow AND geometry mixed)   : {fmt(all_w)}")
    for b in ORIENT_BINS:
        print(f"      normal {b[0]:2d}-{b[1]:2d}deg off-axis : {fmt(by_orient(all_w, all_o)[b])}")

    named = []
    for (x, y) in ats:
        r = fr.profile(x, y)
        if not r:
            print(f"   --at ({x},{y}) : no gradeable ramp here "
                  "(flat, textured, under MIN_STEP, or touching sky)")
            continue
        w, ang = r
        named.append(w)
        pool = next((ctl[b] for b in ORIENT_BINS if b[0] <= ang < b[1]), np.array([]))
        if pool.size:
            ref = np.median(pool)
            verdict = "SOFT" if w >= 1.5 * ref else "HARD (at the AA/TAA floor)"
            print(f"   --at ({x},{y}) : width {w:5.2f}px  normal {ang:.0f}deg off-axis  "
                  f"vs control {ref:5.2f}px = {w / ref:.2f}x -> {verdict}")
        else:
            print(f"   --at ({x},{y}) : width {w:5.2f}px  normal {ang:.0f}deg off-axis  "
                  "-> no control edge at this orientation, UNCALIBRATED")
    if named:
        n = np.array(named)
        print(f"   --at pooled  : {fmt(n)}  vs CONTROL {fmt(ctl_w)}"
              + (f"  = {np.median(n) / np.median(ctl_w):.2f}x" if ctl_w.size else ""))
    print()


def main(argv=None):
    argv = list(argv if argv is not None else sys.argv[1:])
    ats, control, plates = [], None, []
    sun = control_sun = None
    while argv:
        a = argv.pop(0)
        if a == "--at":
            ats = [tuple(int(v) for v in p.split(",")) for p in argv.pop(0).split(";")]
        elif a == "--albedo-check":
            control = argv.pop(0)
        elif a == "--sun":
            sun = tuple(float(v) for v in argv.pop(0).split(","))[:2]
        elif a == "--control-sun":
            control_sun = tuple(float(v) for v in argv.pop(0).split(","))[:2]
        else:
            plates.append(a)
    # guarded here, not at import: the `[E]` prompt glyphs sit on the ground and
    # are a far stronger step than any shadow edge, so a raw capture would be
    # graded on text.
    require_nohud2(plates + ([control] if control else []), tool="cast_shadow_penumbra.py")
    for p in plates:
        run(p, ats, control, sun, control_sun)


if __name__ == "__main__":
    main()
