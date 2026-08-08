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
# Axis            channel / definition                    target      REF (golden)
# warmth  R-B     midtone-band mean (R-B)                 >= +110     +120.9
# blue    B       midtone-band mean B                     <=  10.0      4.3
# clip    any     midtone % px with a channel at 0/255    <=  35.0     18.8  (co-gate)
# sat     HSV     midtone mean sat over NON-railed px     >=  90.0     95.2  (honest; N-A if clip>35)
# DOF fg:bg       hi-freq std ratio fg/bg                 >=   3.0      3.50
# micro-contrast  hi-pass std (GaussianBlur r3) on 1024L  >=   5.0      5.24
# p95             global luminance 95th pct (~170 band)   150..185    165.8
#
# midtone band = luminance L in [p35, p75] (the lit-wood body of the frame).
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
# and FAILs every known-bad N6 plate. This also retro-covers warmth/blue, which inflate
# or collapse on a B-clipped frame but are now moot -- the frame hard-FAILs on clip.
# (POST_SATURATION itself is Yamamoto's lane; this gate only detects the symptom.)
CLIP_THRESH = 35.0

TARGETS = [
    # key,   label,                        cmp,   bound(s),          ref
    ("warmth", "warmth R-B (mid)",         "ge",  110.0,            120.9),
    ("blue",   "blue B (mid)",             "le",   10.0,              4.3),
    ("clip",   "mid clip %",               "le",   CLIP_THRESH,      18.8),
    ("sat",    "saturation (mid, honest)", "ge",   90.0,             95.2),
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
    # midtone band L 35-75 pct
    tlo, thi = np.percentile(L, 35), np.percentile(L, 75)
    mm = (L >= tlo) & (L <= thi)
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
            # saturation grades on the HONEST value, and is meaningless (auto N-A) once
            # the midtone clip co-gate fires: the rail-excluded mean there runs over a
            # tiny saturated sliver (gate3 = 877 of 419k px). See probe d559e13.
            if key == "sat":
                v = m["sat_honest"]
                if m["clip"] > CLIP_THRESH:
                    vstr = "nan" if v != v else f"{v:.2f}"
                    print(f"  [N-A]  {label:<24} {vstr:>8}   midtone clipped {m['clip']:.1f}% -- honest sat over <{100.0-m['clip']:.1f}% survivors is noise")
                    continue
            else:
                v = m[key]
            ok = verdict(cmp, bound, v)
            frame_ok = frame_ok and ok
            tag = "PASS" if ok else "FAIL"
            print(f"  [{tag}] {label:<24} {v:8.2f}   target {fmt_target(cmp, bound):>10}   (REF {ref})")
        print(f"  => {'ALL AXES PASS' if frame_ok else 'FAIL (>=1 axis below target)'}")
        any_fail = any_fail or not frame_ok
        print()

    sys.exit(1 if any_fail else 0)

if __name__ == "__main__":
    main()
