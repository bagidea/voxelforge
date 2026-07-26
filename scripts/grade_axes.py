import sys, numpy as np
from PIL import Image, ImageFilter

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
# sat             midtone-band mean saturation %          >=  90.0     96.1
# DOF fg:bg       hi-freq std ratio fg/bg                 >=   3.0      3.50
# micro-contrast  hi-pass std (GaussianBlur r3) on 1024L  >=   5.0      5.24
# p95             global luminance 95th pct (~170 band)   150..185    165.8
#
# midtone band = luminance L in [p35, p75] (the lit-wood body of the frame).
# ============================================================================

TARGETS = [
    # key,   label,             cmp,   bound(s),          ref
    ("warmth", "warmth R-B (mid)", "ge",  110.0,            120.9),
    ("blue",   "blue B (mid)",     "le",   10.0,              4.3),
    ("sat",    "saturation (mid)", "ge",   90.0,             96.1),
    ("dof",    "DOF fg:bg ratio",  "ge",    3.0,             3.50),
    ("micro",  "micro-contrast",   "ge",    5.0,             5.24),
    ("p95",    "highlight p95",    "band", (150.0, 185.0),  165.8),
]

def verdict(cmp, bound, v):
    if cmp == "ge":   return v >= bound
    if cmp == "le":   return v <= bound
    if cmp == "band": return bound[0] <= v <= bound[1]
    raise ValueError(cmp)

def fmt_target(cmp, bound):
    if cmp == "ge":   return f">= {bound:g}"
    if cmp == "le":   return f"<= {bound:g}"
    if cmp == "band": return f"{bound[0]:g}..{bound[1]:g}"

def measure(path):
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
    return {
        "warmth": float((R[mm]-B[mm]).mean()),
        "blue":   float(B[mm].mean()),
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
    if len(sys.argv) < 2:
        print("usage: grade_axes.py <frame.png> [more.png ...]")
        sys.exit(2)
    any_fail = False
    for path in sys.argv[1:]:
        m = measure(path)
        print(f"\n{path}")
        print(f"  context: warmth(global) {m['_warmth_global']:+6.1f}  sat(global) {m['_sat_global']:4.1f}%"
              f"  DOF fg {m['_dof_fg']:5.2f} bg {m['_dof_bg']:5.2f}")
        frame_ok = True
        for key, label, cmp, bound, ref in TARGETS:
            v = m[key]
            ok = verdict(cmp, bound, v)
            frame_ok = frame_ok and ok
            tag = "PASS" if ok else "FAIL"
            print(f"  [{tag}] {label:<18} {v:8.2f}   target {fmt_target(cmp, bound):>10}   (REF {ref})")
        print(f"  => {'ALL AXES PASS' if frame_ok else 'FAIL (>=1 axis below target)'}")
        any_fail = any_fail or not frame_ok
    sys.exit(1 if any_fail else 0)

if __name__ == "__main__":
    main()
