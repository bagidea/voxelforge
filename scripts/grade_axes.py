import os, sys, argparse, numpy as np
from PIL import Image, ImageFilter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, see main())

# ============================================================================
# P0-AXES — SINGLE SOURCE OF TRUTH for the look-acceptance numeric axes.
# ----------------------------------------------------------------------------
# One frame in → 6 axes measured → per-axis PASS/FAIL → overall verdict.
# All axes measured on a 1024x1024 LANCZOS resample so numbers are comparable
# across renders (same geometry as scripts/grade_beauty.py).
#
# TARGETS below are the ONLY authority. docs/look-acceptance-rubric.md mirrors
# them for humans, but this table is what grades. Do NOT re-derive targets from
# a different metric — every target here is "close the gap toward REF", where
# REF = docs/assets/golden-beauty-shot-ref.png measured by THIS script.
# (History: the old ">=+175 / <=18 / >=88" targets were a phantom — a different
# metric that REF itself could never hit. Retired 2026-07-26.)
#
# Axis            channel / definition                       role      REF (golden)
# --- HARD GATES (contribute to PASS/FAIL) ---
# clip    any     terrain-midtone % px with a chan at 0/255  GATE <=35  18.8   (chromatic damage)
# DOF fg:bg       hi-freq std ratio fg/bg                    GATE >=3.0  3.50  (hero profile only)
# micro-contrast  hi-pass std (GaussianBlur r3) on 1024L     GATE >=5.0  5.24
# p95             global luminance 95th pct (~170 band)      GATE 150..185 165.8
# --- ADVISORY (measured + printed for tuning, NEVER hard-gated) ---
# warmth  R-B     terrain-midtone mean (R-B)                 ADV ~REF  +120.9
# blue    B       terrain-midtone mean B                     ADV ~REF    4.3
# sat     HSV     terrain-midtone mean sat over NON-railed   ADV ~REF   95.2  (honest; N-A if clip>35)
#
# midtone band = terrain luminance L in [p35, p75] -- SKY IS DROPPED FIRST (see
# measure()). REF is indoor with 0% sky, so its band is bit-identical to the old
# whole-frame band and every REF-relative number below is unchanged.
#
# RE-DERIVE 2026-08-09 (Rose; probes _flamingo_band_probe.py / _rose_blue_ramp_probe.py):
# warmth/blue/sat_honest used to be HARD gates (>=110 / <=10 / >=90), all calibrated
# on the INDOOR golden ref (warm-wood midtone, near-zero blue, 95% sat). Two proven
# defects made them dishonest as gates on outdoor content:
#   (1) CLAMP-INVERTED. All three ride on B. When POST_SATURATION over-drives and the
#       swapchain clamps B to 0, warmth R-B INFLATES (N6 wrecked plates read 132-175,
#       PASSing >=110) and blue COLLAPSES (reads ~0.04, PASSing <=10) -- the damage
#       makes the frame score BETTER, not worse. Only sat is honest, and only because
#       the clip co-gate forces it N-A. So a wrecked frame passed warmth+blue.
#   (2) SCENE-CLASS SPREAD. Even un-clamped, good outdoor plates span warmth 71-121,
#       blue 4-141, sat 42-95 (grass/limestone/sky-lit ground vs indoor wood). No
#       single threshold passes all good outdoor plates AND REF, and none can fail N6
#       by value (N6's low blue / high warmth are the clamp artefact). Proven by sweep.
# The ONE statistic that cleanly separates good from damaged is CLIP: shipped-good
# @1.02 plates clip <=22% midtone; every N6 (@1.90) plate clips 55-99.8%. So clip is
# the sole chromatic-damage gate, and warmth/blue/sat become ADVISORY (printed with
# their REF so Yamamoto can still tune toward the indoor target, but they no longer
# false-FAIL a healthy outdoor frame nor false-PASS a B-clamped one). A future
# known-bad COLD/desaturated frame (none exists today) can re-earn a hard chromatic
# gate -- but only with its own calibrated threshold, never the indoor ref's.
#
# PROFILE MODE (--profile hero|gameplay):
#   hero     — all 6 axes (default; for beauty/hero-shot 1024² calibration frames)
#   gameplay — drops DOF fg:bg (framing-dependent; keeps P0 chromatic + micro + p95)
#
#   DOF fg:bg is a KNOWN TRAP for non-hero framing. Its measurement zones
#   (fg: 72-95% height, bg: 10-40% height) were designed for the indoor 1024²
#   beauty shot where a lit-wood body fills the foreground zone. On wide-hero
#   and gameplay frames the zones capture different scene content, so DOF
#   intrinsically fails even on CEO-approved baselines:
#     golden-beauty-shot-ref (hero 1024²)  → DOF 3.50 (PASS)
#     wide-hero-final (CEO baseline, 1024²) → DOF 0.17 (FAIL by design)
#     gate3 gameplay frames (1280×720)      → DOF 0.60–1.34 (all FAIL)
#   Grading DOF on gameplay frames produces a gate that everyone learns to ignore,
#   which is more dangerous than a gate that always passes. — Sun, 2026-08-01
# ============================================================================

# --- midtone clip co-gate + honest saturation (Rose, 2026-08-09; probe d559e13) -----
# saturation USED to be a clip-signature liar: per-pixel HSV-V sat = (max-min)/max
# returns 1.0 for ANY pixel with a channel at 0, so a midtone whose B is gamut-clipped
# to 0 (POST_SATURATION push + clamp) reads as "100% saturated" and PASSes the >=90
# target on a wrecked frame -- it scored the DAMAGE as quality. The N6 shotset sat at
# 99.5-99.8% midtone clip with legacy sat 100.0 on every plate, before AND after.
# Fix = co-gate: hard-FAIL if the midtone band is mostly railed (any channel 0/255),
# and grade an HONEST sat = mean over NON-railed pixels only. Honest sat is itself
# meaningless once the band is mostly railed (mean over a tiny saturated sliver -- e.g.
# gate3 honSat 82.6 over 877 of 419k px), so it auto-reports N-A when clip fires.
# Calibrated on golden REF: clip 18.8%, honest sat 95.2. 35% leaves REF a ~2x margin
# and FAILs every known-bad N6 plate (55-99.8%), passing every shipped-good @1.02 plate
# (<=22%). (POST_SATURATION itself is Yamamoto's lane; this gate only detects the symptom.)
CLIP_THRESH = 35.0

# ADVISORY axes: measured + printed for tuning context, but they NEVER contribute to
# PASS/FAIL. warmth/blue/sat_honest are the B-dependent midtone statistics proven
# clamp-inverted AND scene-class-ungateable (see the RE-DERIVE block in the header).
# clip is the one honest chromatic-damage gate. Their `bound` in TARGETS is kept only
# as the indoor-ref-calibrated reference point for display + sat_status; the verdict
# loop (main) and axes_pass (make_gate3_verdict_card) skip anything in this set.
ADVISORY = {"warmth", "blue", "sat"}

TARGETS = [
    # key,   label,                        cmp,   bound(s),          ref
    ("warmth", "warmth R-B (mid)",         "ge",  110.0,            120.9),   # ADVISORY
    ("blue",   "blue B (mid)",             "le",   10.0,              4.3),   # ADVISORY
    ("clip",   "mid clip %",               "le",   CLIP_THRESH,      18.8),
    ("sat",    "saturation (mid, honest)", "ge",   90.0,             95.2),   # ADVISORY
    ("dof",    "DOF fg:bg ratio",          "ge",    3.0,             3.50),
    ("micro",  "micro-contrast",           "ge",    5.0,             5.24),
    ("p95",    "highlight p95",            "band", (150.0, 185.0),  165.8),
]

# Which axes are graded per profile.  DOF is excluded from gameplay because
# its measurement zones assume the hero/beauty-shot composition (see header).
PROFILES = {
    "hero":     ["warmth", "blue", "clip", "sat", "dof", "micro", "p95"],
    "gameplay": ["warmth", "blue", "clip", "sat", "micro", "p95"],
}

DOF_EXCLUSION_NOTE = (
    "DOF fg:bg is framing-dependent -- its fg/bg measurement zones were designed\n"
    "for the indoor 1024^2 hero shot. On wide-hero the CEO baseline itself scores\n"
    "0.17 (vs target >= 3.0); on gameplay (1280x720) every frame fails (0.60-1.34).\n"
    "Grading it on non-hero frames creates a gate everyone learns to ignore.\n"
    "See scripts/grade_axes.py header and docs/gate3-colour-review-2026-08-01.md S2."
)

def verdict(cmp, bound, v):
    if cmp == "ge":   return v >= bound
    if cmp == "le":   return v <= bound
    if cmp == "band": return bound[0] <= v <= bound[1]
    raise ValueError(cmp)

def fmt_target(cmp, bound):
    if cmp == "ge":   return f">= {bound:g}"
    if cmp == "le":   return f"<= {bound:g}"
    if cmp == "band": return f"{bound[0]:g}..{bound[1]:g}"

def sat_status(m):
    """Honest-saturation verdict for a measured frame, with the clip co-gate.

    Downstream callers (make_gate3_verdict_card, _flamingo_sat_ladder_sheet,
    _poppy_sweep_report) used to read m["sat"] -- the LEGACY clip-signature
    value -- with a hard >=90 and graded a wrecked B-clipped frame as
    "100% saturated = PASS". This is the honest replacement. It mirrors the
    sat branch of main() exactly: judge m["sat_honest"] against the sat target
    pulled from TARGETS, but auto-report N-A once m["clip"] > CLIP_THRESH -- over
    a mostly-railed band the rail-excluded mean runs on too few survivors to mean
    anything, and the frame is already carried FAIL by the clip axis. Keeping
    this as the one shared function means the three callers cannot drift from
    the gate's own N-A rule.

    Returns (ok, text):
      ok   = True/False when judged, or None when N-A (callers must NOT FAIL on
             sat when ok is None -- the clip axis carries that FAIL, not sat).
      text = '95.2' (judged) or 'N-A' (co-gate fired), for display.
    """
    _, _, cmp, bound, _ = next(t for t in TARGETS if t[0] == "sat")  # target from the table
    if m["clip"] > CLIP_THRESH:
        return None, "N-A"
    v = m["sat_honest"]
    return verdict(cmp, bound, v), f"{v:.1f}"

def measure(path):
    # HARD GUARD, AT THE MEASUREMENT — not only in main(). This module is
    # imported: `make_gate3_verdict_card.py` called measure() directly and put
    # the result on a verdict card a human reads, walking straight past the
    # guard sitting in main(). A gate on the CLI door only is not a gate while
    # `import grade_axes` is a door too. Calling it again from main() is
    # harmless (the check is pure).
    require_nohud2([path], tool="grade_axes.measure()")
    im = Image.open(path).convert("RGB").resize((1024, 1024), Image.LANCZOS)
    a = np.asarray(im).astype(np.float32)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    L = 0.2126*R + 0.7152*G + 0.0722*B
    mx = a.max(2); mn = a.min(2)
    sat = np.where(mx > 0, (mx-mn)/np.maximum(mx, 1e-6), 0)
    # global highlight
    p95 = float(np.percentile(L, 95))
    # midtone band L[p35,p75] on TERRAIN only -- sky is dropped BEFORE the percentiles.
    # Sky (B>R and brighter than the frame median) is the one in-frame thing whose high
    # blue is *supposed* to be there; letting it into the chromatic band made sky-bearing
    # plates read as false FAILs (s1-vista: 37.9% sky inside the old band -> blue 101 /
    # warmth 26 on a fine frame). The percentiles run over terrain luminance, so the band
    # is the lit-wood/grass BODY of the frame on every plate. REF is indoor, 0% sky, so its
    # terrain band == the old whole-frame band bit-for-bit (REF-relative targets unchanged).
    # Proven invariant + per-plate numbers: _flamingo_band_probe.py / _rose_blue_ramp_probe.py.
    sky = (B > R) & (L > np.median(L))
    ter = ~sky
    Lt = L[ter] if int(ter.sum()) > 1000 else L           # fall back to whole frame if ~no terrain
    tlo, thi = np.percentile(Lt, 35), np.percentile(Lt, 75)
    mm = ter & (L >= tlo) & (L <= thi)
    # DOF zones (identical to grade_beauty.region_sharpness)
    g = np.asarray(im.convert("L")).astype(np.float32)
    H, Wd = g.shape
    def hf(sub):
        b = np.asarray(Image.fromarray(sub.astype(np.uint8)).filter(ImageFilter.GaussianBlur(2))).astype(np.float32)
        return float((sub-b).std())
    fg = g[int(H*0.72):int(H*0.95), int(Wd*0.30):int(Wd*0.70)]
    bg = g[int(H*0.10):int(H*0.40), int(Wd*0.35):int(Wd*0.85)]
    fgv, bgv = hf(fg), hf(bg)
    # micro-contrast: hi-pass std, GaussianBlur r3 on the full 1024 luminance
    blur3 = np.asarray(Image.fromarray(g.astype(np.uint8)).filter(ImageFilter.GaussianBlur(3))).astype(np.float32)
    micro = float((g-blur3).std())
    # midtone clip-fraction + honest saturation (co-gate; see CLIP_THRESH above).
    # Same 1024^2 resample, same L[p35..p75] band as every other axis.
    Rm, Gm, Bm = R[mm], G[mm], B[mm]
    railed = (Rm == 0)|(Gm == 0)|(Bm == 0)|(Rm == 255)|(Gm == 255)|(Bm == 255)
    nmid = int(mm.sum())
    clip = float(railed.sum()) / nmid * 100.0
    mxm = np.maximum(np.maximum(Rm, Gm), Bm)
    mnm = np.minimum(np.minimum(Rm, Gm), Bm)
    hon = ~railed
    sat_honest = (float(np.where(mxm[hon] > 0,
                    (mxm[hon]-mnm[hon])/np.maximum(mxm[hon], 1e-6), 0).mean()*100)
                  if int(hon.sum()) > 0 else float("nan"))
    return {
        "warmth": float((R[mm]-B[mm]).mean()),
        "blue":   float(B[mm].mean()),
        "clip":   clip,
        "sat_honest": sat_honest,
        # LEGACY saturation (max-min)/max mean over ALL midtone pixels -- a clip
        # SIGNATURE (1.0 per railed pixel), NOT saturation. Kept so downstream
        # consumers that still read m["sat"] (make_gate3_verdict_card,
        # _flamingo_sat_ladder_sheet, _poppy_sweep_report) are not silently broken.
        # The gate grades on sat_honest; do NOT judge on this value.
        "sat":    float(sat[mm].mean()*100),
        "dof":    fgv/max(bgv, 1e-3),
        "micro":  micro,
        "p95":    p95,
        # extra context (not gated)
        "_warmth_global": float((R-B).mean()),
        "_sat_global": float(sat.mean()*100),
        "_dof_fg": fgv, "_dof_bg": bgv,
    }

def main():
    parser = argparse.ArgumentParser(
        description="Grade P0 colour axes (single source of truth for look-acceptance)"
    )
    parser.add_argument(
        "--profile", choices=["hero", "gameplay"], default="hero",
        help="hero = all 6 axes (default); gameplay = skip DOF fg:bg (framing-dependent)"
    )
    parser.add_argument(
        "images", nargs="+",
        help="de-HUDded PNG frame(s) to grade - must be named *-nohud2.png"
    )
    args = parser.parse_args()

    # HARD GUARD — first thing after arg parsing, before any measurement or any
    # printed number. Every axis here is only meaningful on a de-HUDded frame:
    # HUD glyphs sit at ~250 and drag the p95 axis up to the UI. Placed after
    # parse_args only so `--help` still works; nothing is measured before it.
    require_nohud2(args.images, tool="grade_axes.py")

    profile = args.profile
    active_axes = PROFILES[profile]
    skipped_axes = [ax for ax, _, _, _, _ in TARGETS if ax not in active_axes]

    # --- header: always print the active profile so nobody can quietly relax criteria ---
    ax_labels = [label for key, label, _, _, _ in TARGETS if key in active_axes]
    print(f"profile: {profile}")
    print(f"  axes graded: {', '.join(active_axes)}")
    adv_active = [k for k in active_axes if k in ADVISORY]
    if adv_active:
        print(f"  advisory (printed, not gated): {', '.join(adv_active)}  -- clip is the sole chromatic gate")
    if skipped_axes:
        print(f"  axes skipped: {', '.join(skipped_axes)}")
        # print the framing-dependent rationale (compact)
        for line in DOF_EXCLUSION_NOTE.strip().split("\n"):
            print(f"  NOTE: {line.strip()}")
    print()

    any_fail = False
    for path in args.images:
        m = measure(path)
        print(f"{path}")
        print(f"  profile: {profile}  context: warmth(global) {m['_warmth_global']:+6.1f}  "
              f"sat(mid,legacy/clip-sig) {m['sat']:5.1f}% [do not judge]  DOF fg {m['_dof_fg']:5.2f} bg {m['_dof_bg']:5.2f}")
        frame_ok = True
        for key, label, cmp, bound, ref in TARGETS:
            if key not in active_axes:
                v = m[key]
                print(f"  [SKIP] {label:<24} {v:8.2f}   target {fmt_target(cmp, bound):>10}   (REF {ref})")
                continue
            # ADVISORY axes (warmth/blue/sat): printed for tuning context, NEVER gated.
            # They are B-dependent midtone stats -- clamp-inverted (a B-clamped wreck
            # scores BETTER) and scene-class-spread (indoor wood vs outdoor grass), so no
            # honest single threshold exists. clip is the sole chromatic-damage gate.
            # See the RE-DERIVE block in the header.
            if key in ADVISORY:
                if key == "sat":
                    v = m["sat_honest"]
                    if m["clip"] > CLIP_THRESH:
                        vstr = "nan" if v != v else f"{v:.2f}"
                        print(f"  [N-A]  {label:<24} {vstr:>8}   midtone clipped {m['clip']:.1f}% -- honest sat over <{100.0-m['clip']:.1f}% survivors is noise")
                        continue
                else:
                    v = m[key]
                print(f"  [ADV]  {label:<24} {v:8.2f}   ref {ref:>6}   (advisory -- tune toward REF, not a gate)")
                continue
            # HARD GATE
            v = m[key]
            ok = verdict(cmp, bound, v)
            frame_ok = frame_ok and ok
            tag = "PASS" if ok else "FAIL"
            print(f"  [{tag}] {label:<24} {v:8.2f}   target {fmt_target(cmp, bound):>10}   (REF {ref})")
        print(f"  => {'ALL AXES PASS' if frame_ok else 'FAIL (>=1 gate below target)'}")
        any_fail = any_fail or not frame_ok
        print()

    sys.exit(1 if any_fail else 0)

if __name__ == "__main__":
    main()
