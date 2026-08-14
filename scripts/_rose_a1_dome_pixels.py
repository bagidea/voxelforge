#!/usr/bin/env python3
"""
A1 sky-dome green-vs-gold pixel probe (Rose).

WHAT IT DOES
  Decides, from a rendered gate3 frame, whether the sky DOME actually reached
  the fragment shader (frame goes GREEN) or never rendered (frame stays GOLD —
  we see the flat ClearColor / background, not the dome material).

WHY IT EXISTS
  The A1 dome was proven dead at the fragment level (see
  docs/_rose_a1_dome_dead_2026-08-11.md): an unlit HDR green(0,8,0) emissive on
  the dome changed NOTHING — every frame stayed gold. Root cause (fixed in
  client/src/look.rs @ 7299237): the dome spawned without a `Visibility`
  component, so Bevy 0.19 never computed `ViewVisibility` and never extracted
  it. This script is the FALSIFICATION GATE for that fix: run the binary with
  VOXELFORGE_LOOK_SKYPROBE=1 (nuclear-green dome material), capture the frame to
  docs/assets/gate3/_probe_green.png, and this script must report GREEN (exit 0).
  A gold frame (exit 1) means the fix is wrong or the binary did not relink —
  it is never a silent pass.

WHAT IT MEASURES  (top 40% sky band of the frame, 8-bit RGB)
  sky_green_dom : mean G / (mean R + mean B)
                 green sky >> 1.5 ; gold/warm sky ~0.5-0.6
  sky_frac_gmax : fraction of sky pixels where G is the STRICT max channel
                 green sky ~1.0 ; gold sky ~0.0

VERDICT  (exit-coded — a FAIL is never silent, per office grader standard)
  PASS / exit 0 : sky_frac_gmax >= 0.50 AND sky_green_dom >= 1.5  -> dome rendered
  FAIL / exit 1 : otherwise, OR the primary probe image is missing
                  (missing proof = not proven = FAIL, never a free pass)

USAGE
  python scripts/_rose_a1_dome_pixels.py
      Diagnostic table of every gate3 frame + PRIMARY verdict on _probe_green.png.
  python scripts/_rose_a1_dome_pixels.py IMG.png
      Grade a single frame (absolute path or name under docs/assets/gate3/);
      exit code reflects the verdict.
  python scripts/_rose_a1_dome_pixels.py --control
      Self-test: synthesized frames must classify correctly. Run this after ANY
      edit to this file; it exits 0 only if every control passes.

CONTROL INVARIANTS  (what --control proves, so the gate cannot be false-green)
  positive : a frame GREEN in the sky band                         -> must PASS
  negative : a frame GOLD/warm in the sky band                     -> must FAIL
  band-trap: a frame GOLD in the sky band but GREEN on the floor   -> must FAIL
             (proves the top-40% sky band is actually doing the work, so a green
             floor / gameplay layer can never fake a green sky)
"""
import sys
import os
import numpy as np
from PIL import Image

D = "docs/assets/gate3"
SKY_BAND_FRAC = 0.40   # top 40% of the frame is where a sky dome lands
FRAC_MIN = 0.50        # >=50% of sky pixels must have G as the strict max channel
DOM_MIN = 1.50         # mean G/(R+B) must reach 1.5 (the documented green contract)

KEY = [
    "_probe_base.png",         # baseline (no debug) — the gold reference
    "_dbg_dome_emissive.png",  # legacy DEBUG-A1: green emissive active -> green or gold?
    "_dbg_dome.png",
    "_dbg_dome_vis.png",
    "_dbg_flat.png",
    "_probe_green.png",        # PRIMARY: VOXELFORGE_LOOK_SKYPROBE=1 capture
    "sky-dome.png",
    "sky-domefog.png",
    "sky-flat.png",
]
PRIMARY = "_probe_green.png"


def _band(h):
    """Row slice for the sky band (top SKY_BAND_FRAC of the frame)."""
    return slice(0, int(h * SKY_BAND_FRAC))


def is_green(dom, frac):
    """Verdict from band stats. Single source of truth for PASS/FAIL."""
    return frac >= FRAC_MIN and dom >= DOM_MIN


def stats_array(a):
    """a: (H,W,3) float. Returns full stat dict incl. the green verdict.

    The shared code path for real PNGs AND synthesized control frames, so the
    control exercises the exact same band selection + thresholds as the verdict.
    """
    h, w, _ = a.shape
    bnd = _band(h)
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    rb, gb, bb = r[bnd], g[bnd], b[bnd]
    dom = gb.mean() / (rb.mean() + bb.mean() + 1e-9)
    frac = float(np.mean((gb > rb) & (gb > bb)))
    return dict(
        size=(w, h),
        mean=(r.mean(), g.mean(), b.mean()),
        sky_mean=(rb.mean(), gb.mean(), bb.mean()),
        sky_green_dom=dom,
        sky_frac_gmax=frac,
        green=is_green(dom, frac),
    )


def _resolve(fn):
    if os.path.isabs(fn):
        return fn
    # a bare name (e.g. "_probe_green.png") -> look under D; a path that already
    # exists relative to cwd (e.g. "docs/assets/gate3/x.png") -> use it as-is.
    if os.path.exists(fn):
        return fn
    return os.path.join(D, fn)


def stats_file(fn):
    p = _resolve(fn)
    if not os.path.exists(p):
        return None
    im = Image.open(p).convert("RGB")
    return stats_array(np.asarray(im, dtype=np.float64))


def _synth_frame(top_rgb, bot_rgb, h=200, w=320):
    """Build a frame whose top SKY_BAND_FRAC is `top_rgb` and the rest `bot_rgb`."""
    a = np.zeros((h, w, 3), dtype=np.float64)
    bnd = _band(h)
    a[bnd, :, :] = top_rgb
    a[bnd.stop:, :, :] = bot_rgb
    return a


def run_control():
    """Self-test on synthesized frames. Returns True iff every control classifies correctly."""
    # (label, top-band RGB, lower-band RGB, EXPECTED green?)
    cases = [
        ("positive: green sky",                 (10, 220, 10),  (200, 150, 40), True),
        ("negative: gold sky",                  (210, 150, 40), (30, 30, 40),   False),
        ("band-trap: gold sky + green floor",   (210, 150, 40), (10, 220, 10),  False),
    ]
    ok = True
    for name, top, bot, want_green in cases:
        s = stats_array(_synth_frame(top, bot))
        got = s["green"]
        good = got == want_green
        ok = ok and good
        verdict = "GREEN" if got else "GOLD"
        want = "PASS" if want_green else "FAIL"
        mark = "OK " if good else "BAD"
        print(f"  [{mark}] {name:34s} dom={s['sky_green_dom']:6.2f} "
              f"%Gmax={s['sky_frac_gmax']*100:5.1f}  -> {verdict} (want {want})")
    return ok


def _print_table():
    print("=== A1 dome probe — pixel evidence (8-bit RGB) ===")
    print(f"{'file':28s} {'meanRGB':22s} {'skyRGB':22s} {'Gdom':>5s} {'%Gmax':>6s} {'verdict':>7s}")
    for fn in KEY:
        s = stats_file(fn)
        if s is None:
            print(f"{fn:28s}  (missing)")
            continue
        mr, mg, mb = s["mean"]
        sr, sg, sb = s["sky_mean"]
        tag = "GREEN" if s["green"] else "GOLD"
        print(f"{fn:28s} ({mr:5.0f},{mg:5.0f},{mb:5.0f}) ({sr:5.0f},{sg:5.0f},{sb:5.0f})"
              f" {s['sky_green_dom']:5.2f} {s['sky_frac_gmax']*100:5.1f}% {tag:>7s}")


def _print_diff_block():
    """Diagnostic (non-gating): did the legacy DEBUG-A1 emissive move ANY pixel vs baseline?"""
    base = stats_file("_probe_base.png")
    dbg = stats_file("_dbg_dome_emissive.png")
    if not (base and dbg):
        return
    ba = np.asarray(Image.open(_resolve("_probe_base.png")).convert("RGB"), dtype=np.float64)
    da = np.asarray(Image.open(_resolve("_dbg_dome_emissive.png")).convert("RGB"), dtype=np.float64)
    if ba.shape != da.shape:
        print(f"\n  shape mismatch baseline{ba.shape} vs debug{da.shape} — not diffable")
        return
    diff = np.abs(da - ba)
    print("\n=== _dbg_dome_emissive vs _probe_base (same shape) ===")
    print(f"  max abs pixel diff : {diff.max():.0f}/255")
    print(f"  mean abs pixel diff: {diff.mean():.3f}/255")
    print(f"  % pixels changed >5: {np.mean(diff.max(axis=2) > 5) * 100:.2f}%")
    print("  -> ~0% means the green-emissive debug had NO visible effect = dome NOT rendered.")


def main(argv):
    if "--control" in argv:
        print("=== A1 dome probe self-test (controls) ===")
        ok = run_control()
        print("RESULT:", "PASS — all controls classify correctly" if ok
              else "FAIL — a control misclassified; do not trust this gate")
        return 0 if ok else 1

    # single-frame grade mode: one positional arg
    if len(argv) == 2 and not argv[1].startswith("-"):
        fn = argv[1]
        s = stats_file(fn)
        if s is None:
            print(f"FAIL: missing image {_resolve(fn)}")
            return 1
        green = s["green"]
        tag = "GREEN (dome rendered) — PASS" if green else "GOLD (dome NOT rendered) — FAIL"
        print(f"{fn}: dom={s['sky_green_dom']:.2f} %Gmax={s['sky_frac_gmax']*100:.1f}% -> {tag}")
        return 0 if green else 1

    # default: diagnostic table, then the PRIMARY verdict (the actual gate)
    _print_table()
    _print_diff_block()
    print("\nVERDICT key: Gdom>=1.5 AND %Gmax>=50% = GREEN (dome reached fragment); else GOLD.")
    s = stats_file(PRIMARY)
    if s is None:
        print(f"\nFAIL: primary probe {PRIMARY} missing — dome NOT proven. "
              f"Capture it with VOXELFORGE_LOOK_SKYPROBE=1 first.")
        return 1
    green = s["green"]
    tag = "GREEN — dome renders, fix CONFIRMED (exit 0)" if green \
        else "GOLD — dome still does not render, fix WRONG or binary stale (exit 1)"
    print(f"\nPRIMARY {PRIMARY}: dom={s['sky_green_dom']:.2f} "
          f"%Gmax={s['sky_frac_gmax']*100:.1f}% -> {tag}")
    return 0 if green else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
