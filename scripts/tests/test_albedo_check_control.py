#!/usr/bin/env python3
"""Lock the --albedo-check control against the azimuth that broke it (Poppy 2026-08-09).

The measurement this guards once shipped a false claim to the CEO. `--albedo-check`
compared a plate against the PRE-LIGHT plate of the same camera -- the same azimuth
205 deg, only 17 deg instead of 22 -- so the same walls threw the same bands onto the
same grass in both frames, a real cast shadow that barely moved scored 89-94 %
"already dark before", and "THE GRASS STILL HAS NO CAST SHADOW ON IT" went into
docs/note-to-director-N6-regrade-2026-08-08.md. It has a cast shadow. The control was
blind, not the renderer.

The fix is a control the albedo cannot follow (move the sun's AZIMUTH; painted texture
is bolted to the blocks), and the failure mode that has to stay dead is not a number --
it is somebody handing the tool a same-azimuth plate again, six weeks from now, and
reading the verdict it prints. So this test locks the REFUSAL, not just the maths:

  1. the four constants are exactly what the note quotes (relaxing one turns this red);
  2. `azim_sep` wraps (350 vs 10 deg is 20 deg apart, not 340);
  3. same-azimuth control -> REFUSED, and NO verdict word is printed at all;
  4. no provenance on disk -> REFUSED (it does not fall back to "probably GOLDEN");
  5. a `before` plate is never given the shot's own sun -- it came off another run;
  6. a control that keeps under half the plate's grass -> UNRELIABLE, no verdict;
  7. synthetic positive controls: structure that MOVES with the sun reads CAST SHADOW,
     structure bolted to the blocks reads PAINTED. Both built here, no capture needed.
  8. the same four verdicts through the CLI, so the flag parsing is locked too.

Legs 1-8 run in a clean clone. The probe-plate leg (`_poppy_pcss_probe/`, local capture,
gitignored) asserts the three real numbers when present and SKIPS with a reason when not.

Run:   python scripts/tests/test_albedo_check_control.py
Exit 0 only if every present assertion holds. No pytest, no build, no engine, no network.
"""
import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))    # .../scripts/tests
SCRIPTS = os.path.dirname(HERE)                      # .../scripts
ROOT = os.path.dirname(SCRIPTS)                      # .../Voxelforge
sys.path.insert(0, SCRIPTS)

import cast_shadow_penumbra as csp  # noqa: E402

PY = sys.executable
PROBE = os.path.join(ROOT, "_poppy_pcss_probe")
N6 = os.path.join(ROOT, "_pixel_shotset_N6")
VERDICT_WORDS = ("CAST SHADOW", "PAINTED IN THE ALBEDO")

FAILS = []


def check(cond, msg):
    print(("  [PASS] " if cond else "  [FAIL] ") + msg)
    if not cond:
        FAILS.append(msg)


def skip(msg):
    print("  [SKIP] " + msg)


# ---- synthetic plates -------------------------------------------------------
# A field of Voxelforge-ish grass (hue ~77 deg, sat ~54 %, so `grass_select` keeps
# it) carrying two separable signals: a faint per-pixel albedo texture, and hard
# vertical bands. Moving the BANDS between the two plates is a cast shadow that
# followed the sun; moving nothing but the exposure is paint. That is the whole
# question the flag exists to answer, in 20 lines and no GPU.
GRASS = np.array([110.0, 130.0, 60.0])
W, H = 480, 320
BAND_PERIOD, BAND_DARK = 16, 0.55


def plate(band_phase, gain=1.0, seed=7):
    rng = np.random.default_rng(seed)                     # same albedo in both
    tex = 1.0 + 0.02 * rng.standard_normal((H, W))        # faint, ~2 % of level
    x = np.arange(W)
    band = np.where(((x + band_phase) // (BAND_PERIOD // 2)) % 2 == 0, BAND_DARK, 1.0)
    f = gain * tex * band[None, :]
    return np.clip(f[:, :, None] * GRASS[None, None, :], 0, 255).astype(np.uint8)


def write_case(d, name, arr_a, arr_b, sun_a, sun_b, manifest=True):
    """A plate + its control in `<d>/<name>/{after,control}/`, each with the
    manifest.json the shoot script would have written beside it."""
    paths = []
    for tag, arr, sun in (("after", arr_a, sun_a), ("control", arr_b, sun_b)):
        sub = os.path.join(d, f"{name}-{tag}")
        os.makedirs(os.path.join(sub, "after"), exist_ok=True)
        p = os.path.join(sub, "after", f"{name}-nohud2.png")
        Image.fromarray(arr).save(p)
        if manifest:
            env = f"VOXELFORGE_LOOK_SUN={sun[0]},{sun[1]},22000 VOXELFORGE_PLAY=1"
            with open(os.path.join(sub, "manifest.json"), "w", encoding="utf-8") as f:
                json.dump({"extra_env": "", "shots": [{"key": name, "env": env, "after": p}]}, f)
        paths.append(p)
    return paths


def verdict_of(plate_png, control_png, **kw):
    """`albedo_check`'s console output, captured — the console IS the verdict."""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        csp.albedo_check(plate_png, control_png, **kw)
    return buf.getvalue()


def says(out, word):
    return word in out


def no_verdict(out):
    return not any(says(out, w) for w in VERDICT_WORDS)


def main():
    print("--albedo-check control regression lock")

    print("\n1. the constants the note quotes")
    check(csp.MIN_AZIMUTH_MOVE == 90.0,
          f"MIN_AZIMUTH_MOVE == 90.0 (is {csp.MIN_AZIMUTH_MOVE}) -- lowering it re-admits "
          "the same-azimuth control")
    check(csp.PAINTED_R == 0.80, f"PAINTED_R == 0.80 (is {csp.PAINTED_R})")
    check(csp.MIN_SHARED_GRASS == 0.5, f"MIN_SHARED_GRASS == 0.5 (is {csp.MIN_SHARED_GRASS})")
    check(csp.SHIPPED_SUN == (22.0, 205.0),
          f"SHIPPED_SUN == Hour::GOLDEN 22/205 (is {csp.SHIPPED_SUN})")

    print("\n2. azimuth separation wraps")
    check(csp.azim_sep(205.0, 25.0) == 180.0, "205 vs 25 = 180 deg")
    check(csp.azim_sep(205.0, 205.0) == 0.0, "205 vs 205 = 0 deg")
    check(abs(csp.azim_sep(350.0, 10.0) - 20.0) < 1e-9, "350 vs 10 = 20 deg (wraps, not 340)")
    check(abs(csp.azim_sep(25.0, 205.0) - 180.0) < 1e-9, "symmetric")

    d = tempfile.mkdtemp(prefix="albedoctl_")
    try:
        # The two synthetic pairs. `moved` = bands half a period across (the shadow
        # followed the sun); `bolted` = identical bands, only the exposure differs.
        moved = write_case(d, "moved", plate(0), plate(BAND_PERIOD // 2), (22, 205), (22, 25))
        bolted = write_case(d, "bolted", plate(0), plate(0, gain=0.8), (22, 205), (22, 25))
        same = write_case(d, "same", plate(0), plate(BAND_PERIOD // 2), (22, 205), (17, 205))
        bare = write_case(d, "bare", plate(0), plate(BAND_PERIOD // 2),
                          (22, 205), (22, 25), manifest=False)

        print("\n3. a same-azimuth control is REFUSED, with no verdict printed")
        out = verdict_of(*same)
        check(says(out, "REFUSED"), "prints REFUSED")
        check(no_verdict(out), "prints NO verdict word (this is the a99841f failure mode)")
        check(says(out, "note-to-director-N6-regrade-2026-08-08.md"),
              "names the retracted note, so the reader can see what this cost")

        print("\n4. no provenance on disk is REFUSED, not assumed to be GOLDEN")
        out = verdict_of(*bare)
        check(says(out, "REFUSED"), "prints REFUSED")
        check(no_verdict(out), "prints no verdict")
        out = verdict_of(*bare, sun=(22.0, 205.0), control_sun=(22.0, 25.0))
        check(says(out, "CAST SHADOW"), "--sun/--control-sun declare it and the check proceeds")

        print("\n5. a `before` plate is not given its shot's sun")
        sub = os.path.join(d, "beforecase")
        os.makedirs(os.path.join(sub, "before"), exist_ok=True)
        bp = os.path.join(sub, "before", "moved-nohud2.png")
        Image.fromarray(plate(0)).save(bp)
        with open(os.path.join(sub, "manifest.json"), "w", encoding="utf-8") as f:
            json.dump({"extra_env": "", "shots": [{"key": "moved", "env": "VOXELFORGE_PLAY=1",
                                                   "after": os.path.join(sub, "after", "x.png"),
                                                   "before": bp,
                                                   "before_src": "gone/x.png"}]}, f)
        check(csp.plate_sun(bp) is None,
              "plate_sun(<before>) is None -- a before plate came off ANOTHER run's binary")
        check(csp.plate_sun(moved[0])[:2] == (22.0, 205.0), "plate_sun(<after>) reads its env")

        print("\n6. positive controls: structure that moves vs structure that is bolted on")
        out = verdict_of(*moved)
        check(says(out, "CAST SHADOW"), "bands that moved with the sun -> CAST SHADOW")
        check("azimuth separation 180deg" in out, "reports the separation it checked")
        out = verdict_of(*bolted)
        check(says(out, "PAINTED IN THE ALBEDO"), "bands bolted to the blocks -> PAINTED")

        print("\n7. a control that loses most of the plate's grass gives no verdict")
        # Same azimuth move, but the control's grass is crushed out of the hue/sat
        # window -- the real `az115` failure (35,802 px of 313,846) in miniature.
        dark = np.clip(plate(BAND_PERIOD // 2).astype(np.float64) * 0.06, 0, 255).astype(np.uint8)
        thin = write_case(d, "thin", plate(0), dark, (22, 205), (22, 25))
        out = verdict_of(*thin)
        check(says(out, "UNRELIABLE"), "prints UNRELIABLE")
        check(no_verdict(out), "prints no verdict")

        print("\n8. the same four verdicts through the CLI")
        for label, (a, b), want in (("moved -> CAST", moved, "CAST SHADOW"),
                                    ("bolted -> PAINTED", bolted, "PAINTED IN THE ALBEDO"),
                                    ("same azimuth -> REFUSED", same, "REFUSED"),
                                    ("thin mask -> UNRELIABLE", thin, "UNRELIABLE")):
            r = subprocess.run([PY, os.path.join(SCRIPTS, "cast_shadow_penumbra.py"), a,
                                "--albedo-check", b],
                               capture_output=True, text=True, cwd=ROOT)
            check(r.returncode == 0 and want in r.stdout, f"CLI {label}")
    finally:
        for root, dirs, files in os.walk(d, topdown=False):
            for fn in files:
                os.remove(os.path.join(root, fn))
            for dn in dirs:
                os.rmdir(os.path.join(root, dn))
        os.rmdir(d)

    print("\n9. the real probe plates (local capture)")
    az205 = os.path.join(PROBE, "az205", "after", "s1-vista-nohud2.png")
    az025 = os.path.join(PROBE, "az025", "after", "s1-vista-nohud2.png")
    rpt = os.path.join(PROBE, "az205_nocontact", "after", "s1-vista-nohud2.png")
    if not os.path.isfile(az205) or not os.path.isfile(az025):
        skip("_poppy_pcss_probe/ absent (local capture, gitignored) -- real-plate leg not run")
    else:
        out = verdict_of(az205, az025)
        check(says(out, "CAST SHADOW"),
              "s1-vista 205 deg vs 25 deg -> CAST SHADOW (the claim a99841f denied)")
        check("r = 0.010" in out, "r = 0.010, the number in commit 21f6675 -- unchanged")
        if os.path.isfile(rpt):
            out = verdict_of(az205, rpt)
            check(says(out, "REFUSED") and no_verdict(out),
                  "a repeat shot as control -> REFUSED (separation 0 deg)")
    n6a = os.path.join(N6, "after", "s1-vista-nohud2.png")
    n6b = os.path.join(N6, "before", "s1-vista-nohud2.png")
    if not os.path.isfile(n6a) or not os.path.isfile(n6b):
        skip("_pixel_shotset_N6/ absent -- the retracted workflow's own leg not run")
    else:
        out = verdict_of(n6a, n6b)
        check(says(out, "REFUSED") and no_verdict(out),
              "THE RETRACTED WORKFLOW ITSELF (N6 after vs N6 before) -> REFUSED, no number")

    print()
    if FAILS:
        print(f"FAILED ({len(FAILS)}):")
        for f in FAILS:
            print("  - " + f)
        return 1
    print("all present assertions hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
