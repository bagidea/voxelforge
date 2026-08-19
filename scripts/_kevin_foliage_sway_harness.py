#!/usr/bin/env python3
"""Prove the foliage wind shader actually sways -- measured, never asserted.

Runs `voxelforge_foliage_proof` four times and compares two frame pairs:

  sway:    shot at two wind times (frame 90 = t~1.5s vs 174 = t~2.9s) at the
           default sway amplitude -> the pixels MUST change.
  control: the SAME two frames with VOXELFORGE_FOLIAGE_SWAY_AMP=0.0 -> the
           pixels MUST NOT change (~0%).

PASS is computed from the measured % of pixels whose max channel delta exceeds
a threshold, never from a hardcoded verdict. The control arm is what makes the
sway number mean something: if the camera drifted, or the render is
nondeterministic, or an asset finished loading between the two frames, the
control pair lights up too -- so a sway PASS is rejected unless the control is
(near) zero.

Exit codes: 0 = PASS, 2 = FAIL, 3 = harness error (missing exe, bad image, ...).

Usage:
  python scripts/_kevin_foliage_sway_harness.py                  # default exe/out
  python scripts/_kevin_foliage_sway_harness.py --exe PATH --out DIR
  python scripts/_kevin_foliage_sway_harness.py --selftest       # prove the metric
"""

import argparse
import os
import shutil
import subprocess
import sys
import tempfile

from PIL import Image, ImageChops

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The two wind times, in proof-bin frames (1 frame = 1/60 s under
# ManualDuration). 90 = t~1.5s, 174 = t~2.9s -> 1.4s apart, both long after the
# first frames where shaders/textures are still loading.
SWAY_FRAMES = (90, 174)

# A pixel "changed" when its max |dR|,|dG|,|dB| exceeds this (out of 255).
# Msaa is off and the scene is deterministic, so the control pair is expected to
# be 0.0% -- EPS just swallows any 1-LSB GPU dithering, not real motion.
EPS = 8

CTRL_MAX_PCT = 0.10   # control pair must be (near) static
SWAY_MIN_PCT = 0.10   # sway pair must visibly move

SHADER_SRC = "assets/shaders/foliage_wind.wgsl"
GRASS_TEX = "assets/textures/blocks/vegetation/grass_tall.png"


def pct_changed(a, b, eps=EPS):
    """% of pixels whose max channel |diff| > eps between two same-size images."""
    ima = Image.open(a).convert("RGB")
    imb = Image.open(b).convert("RGB")
    if ima.size != imb.size:
        raise SystemExit("size mismatch: %r %s vs %r %s" % (ima.size, a, imb.size, b))
    # ImageChops.difference -> per-channel |a-b|. HSV's V channel == max(R,G,B),
    # so a pixel is "changed" iff its V exceeds eps. All in C, no pixel loop.
    diff = ImageChops.difference(ima, imb).convert("HSV")
    _, _, v = diff.split()
    hist = v.histogram()
    changed = sum(hist[eps + 1:])
    return 100.0 * changed / (ima.size[0] * ima.size[1])


def selftest():
    """Prove pct_changed measures real pixel change (and 0 for identical)."""
    d = tempfile.mkdtemp(prefix="foliage-sway-selftest-")
    try:
        base = Image.new("RGB", (100, 100), (0, 0, 0))
        moved = Image.new("RGB", (100, 100), (0, 0, 0))
        for y in range(50):
            for x in range(10):
                base.putpixel((x, y), (255, 255, 255))
                moved.putpixel((x + 20, y), (255, 255, 255))
        same = base.copy()  # identical to base AFTER the bar is drawn
        a = os.path.join(d, "a.png")
        s = os.path.join(d, "same.png")
        m = os.path.join(d, "moved.png")
        base.save(a); same.save(s); moved.save(m)

        identical = pct_changed(a, s)
        moved_pct = pct_changed(a, m)

        # identical -> 0.0 exactly. moved: a 10x50 white bar slid 20px right,
        # so both the old and new 500-px regions differ -> 1000/10000 = 10.0%.
        ok = (identical == 0.0) and (9.0 < moved_pct < 11.0)
        print("SELFTEST identical=%.3f%%  moved=%.3f%%  => %s"
              % (identical, moved_pct, "OK" if ok else "FAIL"))
        return 0 if ok else 2
    finally:
        shutil.rmtree(d, ignore_errors=True)


def resolve_exe(explicit):
    if explicit:
        return explicit
    env = os.environ.get("VOXELFORGE_FOLIAGE_EXE")
    if env:
        return env
    # Default: Shino's compile lane. Fall back to any target-*/release build.
    for cand in [
        os.path.join(REPO_ROOT, "target-kevin", "release", "voxelforge_foliage_proof.exe"),
    ]:
        if os.path.isfile(cand):
            return cand
    # scan any lane target dir
    for name in sorted(os.listdir(REPO_ROOT)):
        if name.startswith("target-"):
            p = os.path.join(REPO_ROOT, name, "release", "voxelforge_foliage_proof.exe")
            if os.path.isfile(p):
                return p
    raise SystemExit("no voxelforge_foliage_proof.exe found -- pass --exe (build first)")


def stage_assets(exe):
    """Refresh the two files the proof bin reads, next to the exe it runs.

    The proof bin resolves assets from <exe>/assets, not the repo, and a stale
    copy there silently runs the OLD shader -- the exact 'lever looks dead'
    failure. Copy the current repo files fresh every run.
    """
    exe_dir = os.path.dirname(os.path.abspath(exe))
    for rel in (SHADER_SRC, GRASS_TEX):
        src = os.path.join(REPO_ROOT, rel)
        dst = os.path.join(exe_dir, rel)
        if not os.path.isfile(src):
            raise SystemExit("missing asset source: %s" % src)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(src, dst)


def run_shot(exe, out_png, frame, sway_amp, logf):
    """Run the proof bin once, grab one PNG, return its path."""
    env = dict(os.environ)
    env["VOXELFORGE_SHOT"] = os.path.abspath(out_png)
    env["VOXELFORGE_FOLIAGE_SHOT_FRAME"] = str(frame)
    if sway_amp is not None:
        env["VOXELFORGE_FOLIAGE_SWAY_AMP"] = ("%.6f" % sway_amp)
    else:
        env.pop("VOXELFORGE_FOLIAGE_SWAY_AMP", None)

    logf.write("---- %s frame=%s sway_amp=%s ----\n"
               % (os.path.basename(out_png), frame, sway_amp))
    logf.flush()
    try:
        proc = subprocess.run(
            [exe], env=env, cwd=REPO_ROOT, timeout=240,
            stdout=logf, stderr=subprocess.STDOUT,
        )
    except subprocess.TimeoutExpired:
        raise SystemExit("shot timed out for %s (frame %s)" % (out_png, frame))
    if proc.returncode != 0:
        raise SystemExit("exe exited %d for %s (frame %s) -- see log"
                         % (proc.returncode, out_png, frame))
    if not os.path.isfile(out_png) or os.path.getsize(out_png) == 0:
        raise SystemExit("no PNG written for %s (frame %s)" % (out_png, frame))
    return out_png


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", help="path to voxelforge_foliage_proof.exe")
    ap.add_argument("--out", help="dir for the four PNGs + log")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()

    if args.selftest:
        return selftest()

    exe = resolve_exe(args.exe)
    out_dir = args.out or os.path.join(REPO_ROOT, "_kevin_foliage_sway_out")
    os.makedirs(out_dir, exist_ok=True)

    print("EXE   %s" % exe)
    print("OUT   %s" % out_dir)
    stage_assets(exe)

    f0, f1 = SWAY_FRAMES
    with open(os.path.join(out_dir, "_shots.log"), "w") as logf:
        sway_t0 = run_shot(exe, os.path.join(out_dir, "sway_t0.png"), f0, None, logf)
        sway_t1 = run_shot(exe, os.path.join(out_dir, "sway_t1.png"), f1, None, logf)
        ctrl_t0 = run_shot(exe, os.path.join(out_dir, "ctrl_t0.png"), f0, 0.0, logf)
        ctrl_t1 = run_shot(exe, os.path.join(out_dir, "ctrl_t1.png"), f1, 0.0, logf)

    sway_pct = pct_changed(sway_t0, sway_t1)
    ctrl_pct = pct_changed(ctrl_t0, ctrl_t1)

    ok_ctrl = ctrl_pct <= CTRL_MAX_PCT
    ok_sway = sway_pct >= SWAY_MIN_PCT
    passed = ok_ctrl and ok_sway

    print("")
    print("sway    %.3f%%  (frame %d vs %d, default amplitude)" % (sway_pct, f0, f1))
    print("control %.3f%%  (frame %d vs %d, sway_amp=0.0)" % (ctrl_pct, f0, f1))
    print("")
    for name in ("sway_t0", "sway_t1", "ctrl_t0", "ctrl_t1"):
        print("%s  %s" % (name, os.path.abspath(os.path.join(out_dir, name + ".png"))))
    print("")
    if not ok_ctrl:
        print("FAIL: control moved %.3f%% (must be <= %.2f%%) -- the two frames differ"
              " for a reason other than wind (camera? nondeterminism? loading)."
              % (ctrl_pct, CTRL_MAX_PCT))
    if not ok_sway:
        print("FAIL: sway changed only %.3f%% (must be >= %.2f%%) -- the grass is not"
              " visibly moving." % (sway_pct, SWAY_MIN_PCT))
    print("VERDICT %s" % ("PASS" if passed else "FAIL"))
    return 0 if passed else 2


if __name__ == "__main__":
    sys.exit(main())
