#!/usr/bin/env python3
"""
A1 sky-dome green-vs-gold pixel probe (Rose) — 3-state falsification gate.

WHAT IT DOES
  Decides, from a rendered play-path frame, whether the sky DOME actually reached
  the fragment shader (frame goes GREEN) or never rendered (frame stays GOLD —
  we see the flat ClearColor / background, not the dome material) — OR whether the
  frame has NO sky region to grade at all (UNMEASURABLE).

WHY IT EXISTS
  The A1 dome was proven dead at the fragment level (see
  docs/_rose_a1_dome_dead_2026-08-11.md): an unlit HDR green(0,8,0) emissive on
  the dome changed NOTHING — every frame stayed gold. Root cause (fixed in
  client/src/look.rs @ 7299237): the dome spawned without a `Visibility`
  component, so Bevy 0.19 never computed `ViewVisibility` and never extracted it.
  This script is the FALSIFICATION GATE for that fix.

HOW THE PROOF CAPTURE IS SHOT  (confirmed from source — do NOT use voxelforge_shot)
  The dome lives in `look.rs::sky_dome()`, which only runs in the PLAYABLE session:
  `look_enabled` is true only when `cfg.play` (VOXELFORGE_PLAY) is set, or
  VOXELFORGE_LOOK_FORCE. The isolated `voxelforge_shot` binary (shot_main.rs) does
  NOT include look.rs at all — it renders the hero kitchen and CANNOT produce a
  sky-dome frame. So the capture MUST use the main binary in play mode:

    VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_SKYPROBE=1 \
      VOXELFORGE_SHOT=docs/assets/gate3/_probe_green.png \
      [VOXELFORGE_LOOK_CAM=yaw,pitch,dist  -- pitch>0 aims the boom UP at the sky] \
      ./target/release/voxelforge.exe

  VOXELFORGE_PLAY=1   -> enables the look plugin (sky_dome system runs)
  VOXELFORGE_LOOK_SKYPROBE=1 -> swaps the dome for the nuclear-green HDR-8.0 probe
  VOXELFORGE_SHOT=<png>      -> screenshot path (grabbed at t=3.2s by screenshot_once)
  VOXELFORGE_LOOK_CAM=yaw,pitch,dist -> re-pose the orbit boom; pitch range is
                          [-1.35, +1.20] (positive = look up). A play spawn that
                          frames a canyon wall instead of sky returns UNMEASURABLE
                          (exit 2) — re-shoot with pitch aimed at open sky.

WHAT IT MEASURES  (top 40% sky band of the frame, 8-bit RGB)
  sky_green_dom : mean G / (mean R + mean B)            green sky >> 1.5 ; gold ~0.5
  sky_frac_gmax : fraction of sky px where G is STRICT max channel   green ~1.0 ; gold ~0.0
  sky_tex_p90   : 90th-pct LOCAL std of luminance over a 5x5 window
                  sky (clear/gradient/dome) is SMOOTH: real open-sky frames measure
                  ~9-14; textured terrain/UI measures ~21 (synth rock) to ~36 (real
                  canyon wall). The discriminator between "sky" and "not sky".
  sky_rough     : mean abs neighbour diff of luminance (a second, independent
                  texture signal). Open sky ~1.3-1.6 ; textured terrain >=3.

VERDICT  (3-state, exit-coded — matches the convention Flamingo's gates use)
  GREEN      / exit 0 : sky present AND green-dominant           -> dome rendered. FIX OK.
  GOLD       / exit 1 : sky present but NOT green-dominant       -> dome dead/broken.
                       (a REAL verdict: the dome was measured and failed.)
  UNMEASURABLE/ exit 2: no sky region to grade (top band is textured terrain/UI),
                        OR the proof image is missing entirely.
                        This is a CAPTURE problem, never a dome verdict — re-shoot
                        with the camera aimed at open sky (see LOOK_CAM above).

  Why "unmeasurable" is its own state: a play-path capture can spawn the camera into
  an enclosed canyon/dialogue where NO dome pixel is visible — dead OR alive. Grading
  such a frame GOLD would be a false negative ("dome broken") that is actually just
  "wrong camera". exit 2 keeps that from being misread as a dome failure.

USAGE
  python scripts/_rose_a1_dome_pixels.py
      Diagnostic table of every gate3 frame + PRIMARY verdict on _probe_green.png.
  python scripts/_rose_a1_dome_pixels.py IMG.png
      Grade a single frame (absolute path or name under docs/assets/gate3/);
      exit code reflects the verdict (0 green / 1 gold / 2 unmeasurable).
  python scripts/_rose_a1_dome_pixels.py --control
      Self-test on synthesized frames. Run after ANY edit; exits 0 only if every
      control classifies correctly.

CONTROL INVARIANTS  (what --control proves, so the gate cannot lie)
  positive : a frame GREEN in the sky band                         -> GREEN (exit 0)
  negative : a frame GOLD in the sky band                          -> GOLD  (exit 1)
  band-trap: GOLD sky + GREEN floor (top band still gold)          -> GOLD  (exit 1)
  no-sky   : a frame whose top band is TEXTURED rock (no sky)      -> UNMEASURABLE (exit 2)
"""
import sys
import os
import numpy as np
from PIL import Image

D = "docs/assets/gate3"
SKY_BAND_FRAC = 0.40   # top 40% of the frame is where a sky dome lands
FRAC_MIN = 0.50        # >=50% of sky pixels must have G as the strict max channel
DOM_MIN = 1.50         # mean G/(R+B) must reach 1.5 (the documented green contract)
# Sky-coverage guard: the dome / clear-color sky is SMOOTH; terrain + dialogue UI are
# TEXTURED. Calibrated against real frames (docs/assets/gate3): open-sky plates read
# sky_tex_p90 <= 13.6 and sky_rough <= 1.6; the 2026-08-14 no-sky canyon capture read
# p90=36.4 / rough=3.2; a synth textured rock reads p90=21.2 / rough=14.8. Either
# signal clearing its threshold means "the top band is not sky".
TEX_P90_MAX = 18.0     # 90th-pct local std at/above this => not sky
ROUGH_MAX = 2.2        # mean abs neighbour diff at/above this => not sky
TEX_WIN = 5            # local-std window half-width (5x5 neighbourhood)

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


def _texture(lum):
    """Two independent 'is this sky or texture?' signals over a luminance plane.

    Returns (tex_p90, rough):
      tex_p90 — 90th-percentile of per-pixel LOCAL std over a (2*TEX_WIN+1) window,
                computed via an integral image (O(n)). Sky is smooth => low; terrain
                is noisy => high.
      rough   — mean absolute difference between horizontal/vertical neighbours.
                A second, cheaper texture signal that does not depend on the window.
    """
    w = TEX_WIN
    N = 2 * w + 1
    pad = np.pad(lum, ((w, w), (w, w)), mode="edge")
    # integral images for mean and mean-of-squares over the window
    cs = np.cumsum(np.cumsum(pad, axis=0), axis=1)
    cs = np.pad(cs, ((1, 0), (1, 0)))
    s = cs[N:, N:] - cs[:-N, N:] - cs[N:, :-N] + cs[:-N, :-N]
    mean = s / (N * N)
    sq = np.cumsum(np.cumsum(pad * pad, axis=0), axis=1)
    sq = np.pad(sq, ((1, 0), (1, 0)))
    ss = sq[N:, N:] - sq[:-N, N:] - sq[N:, :-N] + sq[:-N, :-N]
    loc_std = np.sqrt(np.clip(ss / (N * N) - mean * mean, 0.0, None))
    tex_p90 = float(np.percentile(loc_std, 90))
    dh = np.abs(np.diff(lum, axis=1))
    dv = np.abs(np.diff(lum, axis=0))
    rough = float(np.concatenate([dh.ravel(), dv.ravel()]).mean())
    return tex_p90, rough


def classify(dom, frac, tex_p90, rough):
    """Single source of truth for the 3-state verdict.

    Order matters: texture is tested FIRST. A textured top band has no sky to grade,
    so its green/gold numbers are meaningless regardless of what they read. Only a
    SMOOTH (sky-like) band is graded green vs gold.
    """
    if tex_p90 >= TEX_P90_MAX or rough >= ROUGH_MAX:
        return "unmeasurable"
    if frac >= FRAC_MIN and dom >= DOM_MIN:
        return "green"
    return "gold"


def stats_array(a):
    """a: (H,W,3) float. Returns full stat dict incl. the 3-state verdict.

    The shared code path for real PNGs AND synthesized control frames, so the
    control exercises the exact same band selection, texture guard + thresholds.
    """
    h, w, _ = a.shape
    bnd = _band(h)
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    rb, gb, bb = r[bnd], g[bnd], b[bnd]
    dom = gb.mean() / (rb.mean() + bb.mean() + 1e-9)
    frac = float(np.mean((gb > rb) & (gb > bb)))
    lum = 0.299 * rb + 0.587 * gb + 0.114 * bb
    tex_p90, rough = _texture(lum)
    return dict(
        size=(w, h),
        mean=(r.mean(), g.mean(), b.mean()),
        sky_mean=(rb.mean(), gb.mean(), bb.mean()),
        sky_green_dom=dom,
        sky_frac_gmax=frac,
        sky_tex_p90=tex_p90,
        sky_rough=rough,
        state=classify(dom, frac, tex_p90, rough),
        green=(frac >= FRAC_MIN and dom >= DOM_MIN),  # legacy alias (ignores guard)
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


def _synth_no_sky(h=200, w=320):
    """A textured top band (rock) with no sky — exercises the coverage guard.

    Smooth-gradient noise would NOT trip the guard (a real sky gradient is smooth);
    this carries enough high-frequency energy to read as terrain, not sky.
    """
    rng = np.random.default_rng(0)
    ys = np.arange(h)[: int(h * SKY_BAND_FRAC), None]
    base = 110 + 30 * np.sin(ys / 9) + 20 * np.sin(ys / 3) + rng.standard_normal((1, w)) * 18
    a = np.zeros((h, w, 3), dtype=np.float64)
    bnd = _band(h)
    a[bnd, :, 0] = np.clip(base, 0, 255)
    a[bnd, :, 1] = np.clip(base * 0.66, 0, 255)
    a[bnd, :, 2] = np.clip(base * 0.30, 0, 255)
    a[bnd.stop:, :, :] = (30, 30, 40)
    return a


def run_control():
    """Self-test on synthesized frames. Returns True iff every control classifies correctly."""
    # (label, frame, EXPECTED state)
    cases = [
        ("positive: green sky",               _synth_frame((10, 220, 10), (200, 150, 40)), "green"),
        ("negative: gold sky",                _synth_frame((210, 150, 40), (30, 30, 40)),   "gold"),
        ("band-trap: gold sky + green floor", _synth_frame((210, 150, 40), (10, 220, 10)),  "gold"),
        ("no-sky: textured rock band",        _synth_no_sky(),                              "unmeasurable"),
    ]
    ok = True
    for name, frame, want in cases:
        s = stats_array(frame)
        got = s["state"]
        good = got == want
        ok = ok and good
        mark = "OK " if good else "BAD"
        print(f"  [{mark}] {name:34s} dom={s['sky_green_dom']:6.2f} "
              f"%Gmax={s['sky_frac_gmax']*100:5.1f} tex90={s['sky_tex_p90']:6.1f} "
              f"rough={s['sky_rough']:5.2f} -> {got.upper():12s} (want {want.upper()})")
    return ok


def _label(state):
    return {"green": "GREEN", "gold": "GOLD", "unmeasurable": "UNMEASURABLE"}.get(state, state)


def _print_table():
    print("=== A1 dome probe — pixel evidence (8-bit RGB, top 40% band) ===")
    print(f"{'file':28s} {'skyRGB':22s} {'Gdom':>5s} {'%Gmax':>6s} "
          f"{'tex90':>6s} {'rough':>6s} {'verdict':>12s}")
    for fn in KEY:
        s = stats_file(fn)
        if s is None:
            print(f"{fn:28s}  (missing)")
            continue
        sr, sg, sb = s["sky_mean"]
        print(f"{fn:28s} ({sr:5.0f},{sg:5.0f},{sb:5.0f}) {s['sky_green_dom']:5.2f} "
              f"{s['sky_frac_gmax']*100:5.1f}% {s['sky_tex_p90']:6.1f} {s['sky_rough']:6.2f} "
              f"{_label(s['state']):>12s}")


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


# exit code per verdict state — single map, used by every mode
_EXIT = {"green": 0, "gold": 1, "unmeasurable": 2}


def _grade_primary(s, fn):
    """Print + return the exit code for a resolved primary frame's stats."""
    state = s["state"]
    if state == "unmeasurable":
        print(f"\n{fn}: dom={s['sky_green_dom']:.2f} %Gmax={s['sky_frac_gmax']*100:.1f}% "
              f"tex90={s['sky_tex_p90']:.1f} rough={s['sky_rough']:.2f} -> UNMEASURABLE (exit 2)\n"
              f"  The top band is textured terrain/UI, not sky — this capture has NO sky region\n"
              f"  to grade, so it says nothing about the dome. Re-shoot with the boom aimed at\n"
              f"  open sky: VOXELFORGE_LOOK_CAM=<yaw>,<pitch-up>,<dist> (pitch range -1.35..+1.20).")
    elif state == "green":
        print(f"\n{fn}: dom={s['sky_green_dom']:.2f} %Gmax={s['sky_frac_gmax']*100:.1f}% "
              f"-> GREEN — dome renders, fix CONFIRMED (exit 0)")
    else:
        print(f"\n{fn}: dom={s['sky_green_dom']:.2f} %Gmax={s['sky_frac_gmax']*100:.1f}% "
              f"tex90={s['sky_tex_p90']:.1f} -> GOLD — sky present, dome still does not render,\n"
              f"  fix WRONG or binary stale (exit 1)")
    return _EXIT[state]


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
            print(f"UNMEASURABLE: missing image {_resolve(fn)} — nothing to grade (exit 2)")
            return 2
        print(f"{fn}: dom={s['sky_green_dom']:.2f} %Gmax={s['sky_frac_gmax']*100:.1f}% "
              f"tex90={s['sky_tex_p90']:.1f} rough={s['sky_rough']:.2f} -> {_label(s['state'])}")
        return _EXIT[s["state"]]

    # default: diagnostic table, then the PRIMARY verdict (the actual gate)
    _print_table()
    _print_diff_block()
    print("\nVERDICT key: tex90>=18 OR rough>=2.2 => UNMEASURABLE (no sky); "
          "else Gdom>=1.5 AND %Gmax>=50% => GREEN, else GOLD.")
    s = stats_file(PRIMARY)
    if s is None:
        print(f"\nUNMEASURABLE: primary probe {PRIMARY} missing — dome NOT proven and NOT graded.\n"
              f"  Capture it first: VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_SKYPROBE=1 "
              f"VOXELFORGE_SHOT=.../{PRIMARY} ./target/release/voxelforge.exe (exit 2)")
        return 2
    return _grade_primary(s, f"PRIMARY {PRIMARY}")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
