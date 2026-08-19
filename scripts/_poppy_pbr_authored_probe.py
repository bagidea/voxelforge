#!/usr/bin/env python3
"""Poppy — prove the authored-PBR loader actually loads, without shipping art.

WHY THIS EXISTS. `voxel.rs::authored_map` looks for `<tile>_n.png` / `_r.png` /
`_ao.png` beside each tile and falls back to the derived maps when they are
missing. Monanisa's drop has not landed, so on a normal run EVERY lookup misses
and the authored branch never executes — which means shipping it untested and
finding out it was broken on the day the art arrives. That is the failure this
script exists to prevent.

It builds a THROWAWAY art folder (a copy of `assets/textures/blocks` plus
synthesised companion maps), points the engine at it with `VOXELFORGE_ATLAS_DIR`,
and greps the run's stdout for the loader's own `BLOCK_PBR authored` line. The
shipped assets are never touched, so this cannot contaminate the before/after
plates.

Two runs, because "it printed something" is not the claim:
  1. WITH the companion maps      -> expect N `BLOCK_PBR authored` lines
  2. same folder, VOXELFORGE_MAT_MAPS=off -> expect ZERO
Run 2 is the negative control: without it, a line printed unconditionally
somewhere else in startup would read as a pass.

Usage:  _poppy_pbr_authored_probe.py [exe]
"""
import os
import re
import shutil
import subprocess
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "assets", "textures", "blocks")
TMP = os.path.join(ROOT, "_poppy_pbr_probe_art")
SHOT = os.path.join(ROOT, "_poppy_pbr_probe_art", "probe.png")

# The tiles that get synthesised companions. Deliberately a SUBSET: the run must
# show the loader taking the authored path for these and the derived path for
# everything else, which a folder where every tile had maps could not show.
AUTHORED = ["brick", "stone_bricks", "oak_planks"]
SUFFIXES = {
    "_n": "normal",
    "_r": "roughness",
    "_ao": "occlusion",
}


def synth(kind, stem, px=32):
    """A visibly non-default map, so a frame shot through it is not ambiguous."""
    y, x = np.mgrid[0:px, 0:px]
    if kind == "normal":
        # A ridged surface: x tilts with a sawtooth, z holds the rest.
        nx = np.sin(x / px * 2 * np.pi) * 0.6
        ny = np.sin(y / px * 2 * np.pi) * 0.6
        nz = np.sqrt(np.maximum(1e-6, 1.0 - nx**2 - ny**2))
        a = np.stack([(nx * 0.5 + 0.5), (ny * 0.5 + 0.5), (nz * 0.5 + 0.5)], -1)
    elif kind == "roughness":
        # Grey PNG on purpose — R=G=B. This is the shape that turns every
        # surface to chrome if `metallic` is not held at the table value, so the
        # probe folder is built to trip that bug if it is ever reintroduced.
        g = 0.35 + 0.5 * ((x // 4 + y // 4) % 2)
        a = np.stack([g, g, g], -1)
    else:  # occlusion
        g = np.where((x % 8 < 2) | (y % 8 < 2), 0.4, 1.0)
        a = np.stack([g, g, g], -1)
    rgb = (np.clip(a, 0, 1) * 255).round().astype(np.uint8)
    alpha = np.full((px, px, 1), 255, np.uint8)
    Image.fromarray(np.concatenate([rgb, alpha], -1), "RGBA").save(
        os.path.join(TMP, f"{stem}{ [k for k, v in SUFFIXES.items() if v == kind][0] }.png")
    )


def build_probe_dir():
    if os.path.isdir(TMP):
        shutil.rmtree(TMP)
    shutil.copytree(SRC, TMP)
    for stem in AUTHORED:
        if not os.path.isfile(os.path.join(TMP, f"{stem}.png")):
            print(f"  !! {stem}.png not in the art set — probe would prove nothing")
            return False
        for kind in ("normal", "roughness", "occlusion"):
            synth(kind, stem)
    made = sorted(f for f in os.listdir(TMP) if re.search(r"_(n|r|ao)\.png$", f))
    print(f"  probe art dir: {TMP}")
    print(f"  synthesised {len(made)}: {', '.join(made)}")
    return True


def run(exe, extra_env):
    env = dict(os.environ)
    env.update({
        "VOXELFORGE_ATLAS_DIR": TMP,
        "VOXELFORGE_PLAY": "1",
        "VOXELFORGE_NOHUD": "1",
        "VOXELFORGE_CINE_START": "1.0",
        "VOXELFORGE_CINE": "44,14,44, 44,14,44, 32.5,2.0,29.5, 1",
        "VOXELFORGE_SHOT": SHOT,
    })
    env.update(extra_env)
    p = subprocess.run([exe, "--play"], env=env, capture_output=True, text=True,
                       errors="replace", timeout=300)
    out = (p.stdout or "") + (p.returncode and (p.stderr or "") or "")
    return p.returncode, out


def main():
    exe = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        ROOT, "target-poppy", "release", "voxelforge.exe")
    if not os.path.isfile(exe):
        print("no exe at", exe)
        return 2
    print("EXE", exe)
    if not build_probe_dir():
        return 2

    print("\n--- run 1: authored maps present ---")
    rc1, out1 = run(exe, {})
    hits1 = re.findall(r"^BLOCK_PBR authored (.+)$", out1, re.M)
    for h in sorted(set(hits1)):
        print("   ", h)
    print(f"  rc={rc1}  authored lines: {len(hits1)}")

    print("\n--- run 2: same folder, VOXELFORGE_MAT_MAPS=off (negative control) ---")
    rc2, out2 = run(exe, {"VOXELFORGE_MAT_MAPS": "off"})
    hits2 = re.findall(r"^BLOCK_PBR authored (.+)$", out2, re.M)
    print(f"  rc={rc2}  authored lines: {len(hits2)}")

    # 3 tiles x 3 maps, but a tile may be shared across faces/kinds, so the
    # assertion is on DISTINCT paths reaching the expected count, not on the
    # raw line count (grass_side and grass_top are different files; brick is one
    # file worn by all three faces, so it is looked up three times).
    distinct = len(set(hits1))
    expect = len(AUTHORED) * len(SUFFIXES)
    ok_pos = distinct == expect
    ok_neg = len(hits2) == 0
    print("\n=== VERDICT ===")
    print(f"  run 1 distinct authored files : {distinct} (expect {expect})  {'PASS' if ok_pos else 'FAIL'}")
    print(f"  run 2 authored lines          : {len(hits2)} (expect 0)   {'PASS' if ok_neg else 'FAIL'}")
    if not ok_pos:
        print("\n  stdout of run 1 (BLOCK_* lines):")
        for line in out1.splitlines():
            if line.startswith("BLOCK_") or line.startswith("ATLAS"):
                print("   ", line)
    return 0 if (ok_pos and ok_neg) else 1


if __name__ == "__main__":
    sys.exit(main())
