#!/usr/bin/env python3
"""PROOF + GATE for the missing teal voxel in the wide hero shot.

Flamingo, 2026-07-31.  Background: docs/voxel-hole-findings.md.

Three checks, all pure arithmetic over `client/src/hero.rs` and the locked wide
camera recipe -- nothing here is eyeballed off an image:

  1. ONE SPAWN POINT.  Every grid cube in `setup_hero` must go through
     `VoxelGrid`.  A second `commands.spawn` of a unit cube re-opens the whole
     class of bug: two cubes at one transform, winner picked by draw order.

  2. CELL COLLISION.  Re-derive the world cells the hero bowl and the teal
     tumbler each claim, and intersect them.  A shared cell is only allowed if
     it is listed in EXPECTED_OVERWRITES with a reason -- because under
     VoxelGrid a shared cell is no longer a z-fight, it is a silent overwrite,
     and a silent overwrite is what cost us the voxel in the first place.

  3. SCREEN PROJECTION.  Project each shared cell through the CEO-locked wide
     camera and compare against the hole measured off the golden frame in
     docs/web-parity-checklist.md 2.8.  This is what ties the arithmetic to the
     actual pixels that went missing.

usage:  python scripts/voxel_hole_proof.py             # working tree
        python scripts/voxel_hole_proof.py --rev HEAD  # any git rev
exit 0 = clean, 1 = undeclared overlap / raw spawn.
"""
import argparse
import math
import re
import subprocess
import sys

CAM_EYE = (7.6, 6.4, -6.0)
CAM_TGT = (7.6, 2.7, 8.0)
CAM_FOV_DEG = 60.0                   # vertical fov -- Bevy PerspectiveProjection::fov
RES = (1280, 720)
GOLDEN_HOLE = (752, 293, 843, 382)   # measured on docs/assets/wide-hero-final.png

# Cells two objects deliberately share. Under VoxelGrid the later write wins, so
# each entry is a decision about which material shows -- write down the reason.
EXPECTED_OVERWRITES = {
    (5, 4, 2): "teal tumbler top course over the hero bowl's near-left rim corner; "
               "teal is what the ref-matching web frame shows (wasm-hero-v3.png)",
}


def source(rev):
    if rev:
        # bytes + explicit utf-8: hero.rs has non-ASCII comments and the Windows
        # console codepage would mangle them.
        return subprocess.run(["git", "show", f"{rev}:client/src/hero.rs"],
                              capture_output=True, check=True).stdout.decode("utf-8")
    return open("client/src/hero.rs", encoding="utf-8").read()


def read_params(src):
    """Pull the numbers that decide the collision straight out of the source.

    Tolerant of the spawn-helper refactor: the bowl call was
    `bowl(&mut commands, &cube, &ceramic, Some(&ceramic_sh), 7, 3, 4)` before it
    and `bowl(&mut grid, &ceramic, Some(&ceramic_sh), 7, 3, 4)` after.
    """
    m = re.search(r"bowl\([^;]*?Some\(&ceramic_sh\),"
                  r"\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)\s*\)", src)
    if not m:
        sys.exit("could not find the hero bowl() call in hero.rs")
    bowl_args = tuple(map(int, m.groups()))

    m = re.search(r"let \(gx, gz\) = \((-?\d+),\s*(-?\d+)\)", src)
    if not m:
        sys.exit("could not find the tumbler (gx, gz) in hero.rs")
    tumbler = tuple(map(int, m.groups()))

    # Which corner of the top course is the hollow notch.
    m = re.search(r"if x == gx \+ 1 && z == gz(\s*\+\s*1)?\s*\{", src)
    if not m:
        sys.exit("could not find the tumbler's hollow-notch condition in hero.rs")
    notch = (1, 1 if m.group(1) else 0)
    return bowl_args, tumbler, notch


def bowl_cells(cx, base_y, cz):
    """Mirror of hero.rs `fn bowl`: 3x3 solid base + hollow 5x5 rim ring one up."""
    cells = {(x, base_y, z)
             for x in range(cx - 1, cx + 2) for z in range(cz - 1, cz + 2)}
    y = base_y + 1
    for x in range(cx - 2, cx + 3):
        for z in range(cz - 2, cz + 3):
            if x in (cx - 2, cx + 2) or z in (cz - 2, cz + 2):   # edge only
                cells.add((x, y, z))
    return cells


def tumbler_cells(gx, gz, notch):
    """Mirror of hero.rs section 4: 2x2 base course, notched top course, handle."""
    cells = {(x, 3, z) for x in range(gx, gx + 2) for z in range(gz, gz + 2)}
    for x in range(gx, gx + 2):
        for z in range(gz, gz + 2):
            if (x, z) != (gx + notch[0], gz + notch[1]):
                cells.add((x, 4, z))
    cells.add((gx + 2, 3, gz))                                    # handle nub
    return cells


def project(p):
    ex, ey, ez = CAM_EYE
    fx, fy, fz = (CAM_TGT[0] - ex, CAM_TGT[1] - ey, CAM_TGT[2] - ez)
    n = math.sqrt(fx * fx + fy * fy + fz * fz)
    f = (fx / n, fy / n, fz / n)
    up = (0.0, 1.0, 0.0)
    r = (f[1] * up[2] - f[2] * up[1],
         f[2] * up[0] - f[0] * up[2],
         f[0] * up[1] - f[1] * up[0])
    n = math.sqrt(sum(c * c for c in r))
    r = tuple(c / n for c in r)
    u = (r[1] * f[2] - r[2] * f[1],
         r[2] * f[0] - r[0] * f[2],
         r[0] * f[1] - r[1] * f[0])
    d = (p[0] - ex, p[1] - ey, p[2] - ez)
    xv = sum(a * b for a, b in zip(d, r))
    yv = sum(a * b for a, b in zip(d, u))
    zv = sum(a * b for a, b in zip(d, f))
    t = math.tan(math.radians(CAM_FOV_DEG) / 2)
    ndc_x, ndc_y = xv / (zv * t * (RES[0] / RES[1])), yv / (zv * t)
    return ((ndc_x + 1) / 2 * RES[0], (1 - ndc_y) / 2 * RES[1])


def cell_bbox(cell):
    pts = [project((cell[0] + dx, cell[1] + dy, cell[2] + dz))
           for dx in (0, 1) for dy in (0, 1) for dz in (0, 1)]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return (min(xs), min(ys), max(xs), max(ys))


def iou(a, b):
    ix = max(0.0, min(a[2], b[2]) - max(a[0], b[0]))
    iy = max(0.0, min(a[3], b[3]) - max(a[1], b[1]))
    inter = ix * iy
    area_a = (a[2] - a[0]) * (a[3] - a[1])
    area_b = (b[2] - b[0]) * (b[3] - b[1])
    return inter / (area_a + area_b - inter) if inter else 0.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rev", help="read hero.rs from this git rev instead of the worktree")
    args = ap.parse_args()

    src = source(args.rev)
    (cx, by, cz), (gx, gz), notch = read_params(src)
    print(f"source: {args.rev or 'worktree'}   "
          f"bowl(cx={cx}, base_y={by}, cz={cz})   "
          f"tumbler(gx={gx}, gz={gz}, notch=+{notch[0]},+{notch[1]})")

    # --- 1. one spawn point -------------------------------------------------
    raw = len(re.findall(r"Mesh3d\(cube", src))
    grid_ok = "VoxelGrid" in src and raw <= 1
    print(f"RAW_CUBE_SPAWNS={raw} "
          + ("(OK - the only one is VoxelGrid::flush)" if grid_ok
             else "(FAIL - cubes bypass VoxelGrid)"))

    # --- 2. cell collision --------------------------------------------------
    shared = sorted(bowl_cells(cx, by, cz) & tumbler_cells(gx, gz, notch))
    undeclared = [c for c in shared if c not in EXPECTED_OVERWRITES]
    print(f"CELLS_SHARED={len(shared)}  UNDECLARED={len(undeclared)}")

    # --- 3. screen projection ----------------------------------------------
    for c in shared:
        bb = cell_bbox(c)
        tag = "declared" if c in EXPECTED_OVERWRITES else "UNDECLARED"
        print(f"  cell {c} [{tag}] -> screen x {bb[0]:.0f}-{bb[2]:.0f} y {bb[1]:.0f}-{bb[3]:.0f}"
              f"   IoU vs golden hole {GOLDEN_HOLE} = {iou(bb, GOLDEN_HOLE):.2f}")
        if c in EXPECTED_OVERWRITES:
            print(f"      reason: {EXPECTED_OVERWRITES[c]}")

    if not grid_ok:
        print("VERDICT: FAIL - a unit cube is spawned outside VoxelGrid; two cubes can "
              "again land on one cell with the winner picked by draw order.")
        return 1
    if undeclared:
        print("VERDICT: FAIL - undeclared shared cell(s). Under VoxelGrid one material "
              "silently overwrites the other; declare it or move the geometry.")
        return 1
    print("VERDICT: PASS - one spawn point, and every shared cell is declared.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
