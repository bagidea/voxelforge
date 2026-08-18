#!/usr/bin/env python3
# ===========================================================================
# Poppy — build the atlas dir the VOXELFORGE_MAT_MAPS lever can actually MOVE.
#
# WHY THIS EXISTS
#   `voxel.rs` pins `TILE_PX = 16` and `tile_set()` only returns a set whose
#   manifest `tile_px` matches it. The working-tree art drop is `tile_px: 64`,
#   so every run prints
#       BLOCK_ART tile_px=64 != 16 — file set ignored, procedural tiles kept
#   and `authored_map()` bails on its FIRST line (`let set = tile_set()?`).
#   With no set there is no `_n`/`_r` lookup at all, so MAT_MAPS=on and
#   MAT_MAPS=off render the SAME frame. A 0.00 diff there measures the gate,
#   not the maps.
#
#   So this writes a second atlas dir that clears the gate without touching
#   either the engine or Monanisa's files:
#       albedo  = the 16x16 set from HEAD (what TILE_PX expects)
#       _n/_r   = the 64x64 authored maps from the working tree, AS THEY ARE
#                 (voxel.rs::authored_map accepts any resolution — chunk UVs
#                 are measured in blocks and the sampler repeats)
#       atlas.json = HEAD's manifest (tile_px 16); the tile FILE NAMES are
#                 identical in both manifests, and the suffix convention is
#                 what the loader searches, not the manifest fields.
#
#   Point the engine at it with VOXELFORGE_ATLAS_DIR. Nothing is overwritten:
#   assets/textures/blocks/ is read-only to this script.
#
# USAGE
#   python scripts/_poppy_matmaps_prep.py [outdir]
# ===========================================================================
import json
import os
import struct
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "assets", "textures", "blocks")
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "_poppy_matmaps", "atlas16")

MAP_SUFFIXES = ("_n", "_r")


def png_size(blob):
    return struct.unpack(">II", blob[16:24])


def git_show(path):
    r = subprocess.run(["git", "show", f"HEAD:{path}"], cwd=ROOT, capture_output=True)
    if r.returncode != 0:
        raise SystemExit(f"git show HEAD:{path} failed: {r.stderr.decode(errors='replace')}")
    return r.stdout


def main():
    os.makedirs(OUT, exist_ok=True)

    manifest = json.loads(git_show("assets/textures/blocks/atlas.json"))
    tile_px = manifest["tile_px"]
    print(f"manifest from HEAD: tile_px={tile_px} tiles={len(manifest['tiles'])}")
    if tile_px != 16:
        raise SystemExit(f"HEAD manifest is tile_px={tile_px}, expected 16 — gate would still reject it")

    with open(os.path.join(OUT, "atlas.json"), "wb") as f:
        f.write(git_show("assets/textures/blocks/atlas.json"))

    n_albedo = n_map = 0
    missing = []
    for entry in manifest["tiles"]:
        fn = entry["file"]
        blob = git_show(f"assets/textures/blocks/{fn}")
        w, h = png_size(blob)
        if (w, h) != (tile_px, tile_px):
            raise SystemExit(f"{fn} from HEAD is {w}x{h}, manifest says {tile_px} — load_tiles would error")
        with open(os.path.join(OUT, fn), "wb") as f:
            f.write(blob)
        n_albedo += 1

        stem = os.path.splitext(fn)[0]
        for suffix in MAP_SUFFIXES:
            name = f"{stem}{suffix}.png"
            src = os.path.join(SRC, name)
            if not os.path.isfile(src):
                missing.append(name)
                continue
            with open(src, "rb") as f:
                mblob = f.read()
            with open(os.path.join(OUT, name), "wb") as f:
                f.write(mblob)
            n_map += 1

    mw, mh = png_size(open(os.path.join(OUT, "brick_n.png"), "rb").read(33))
    print(f"albedo tiles copied (16x16, from HEAD): {n_albedo}")
    print(f"authored maps copied ({mw}x{mh}, working tree): {n_map}")
    if missing:
        print(f"maps NOT found (derived map will be used for these): {', '.join(missing)}")
    print(f"out: {OUT}")


if __name__ == "__main__":
    main()
