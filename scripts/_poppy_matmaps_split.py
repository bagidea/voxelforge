#!/usr/bin/env python3
# ===========================================================================
# Poppy — split the authored map set so ONE channel moves at a time.
#
# WHY THIS EXISTS
#   The matmaps A/B turned `_n` and `_r` on together, so M4 (value-span loss,
#   -2.570 vs a 1.159 floor on atlas16/night-firelit) names no culprit. A
#   normal map redistributes light; a roughness map changes how much of it
#   comes back specular. They are not the same finding and they do not get the
#   same fix.
#
#   `voxel.rs::face_maps` looks the two up INDEPENDENTLY —
#       authored_map(id, face, NORMAL_SUFFIXES).or_else(build_face_normal_map)
#       ... else authored_map(id, face, ROUGHNESS_SUFFIXES) ... else derived
#   — so a folder that carries only `*_n.png` gives authored normals with the
#   derived roughness the shipped build already uses, and vice versa. No engine
#   change, no new env var, nothing to un-review later.
#
# WHAT IT WRITES (copies of `_poppy_matmaps/atlas16`, nothing is moved)
#   atlas16_nonly   albedo + atlas.json + every *_n.png   (no *_r.png)
#   atlas16_ronly   albedo + atlas.json + every *_r.png   (no *_n.png)
#
#   The albedo tiles and the manifest are byte-identical to atlas16's, which is
#   the point: against the same `off` plate, the ONLY thing that differs
#   between the two arms is which map file the loader can find.
#
# USAGE
#   python scripts/_poppy_matmaps_split.py [srcdir]
# ===========================================================================
import hashlib
import os
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "_poppy_matmaps", "atlas16")

VARIANTS = {
    "atlas16_nonly": "_n",   # keep the normals, drop the roughness
    "atlas16_ronly": "_r",   # keep the roughness, drop the normals
}


def md5(path):
    with open(path, "rb") as fh:
        return hashlib.md5(fh.read()).hexdigest()


def kind(name):
    """albedo | _n | _r | manifest — by the same suffix convention the loader uses."""
    stem, ext = os.path.splitext(name)
    if ext == ".json":
        return "manifest"
    for suf in ("_n", "_r"):
        if stem.endswith(suf):
            return suf
    return "albedo"


def main():
    if not os.path.isdir(SRC):
        raise SystemExit(f"REFUSED  {SRC} not found — run scripts/_poppy_matmaps_prep.py first")

    names = sorted(os.listdir(SRC))
    by_kind = {}
    for n in names:
        by_kind.setdefault(kind(n), []).append(n)
    print(f"source {SRC}")
    for k in ("manifest", "albedo", "_n", "_r"):
        print(f"  {k:9s} {len(by_kind.get(k, [])):3d} files")

    for out_name, keep in VARIANTS.items():
        out = os.path.join(os.path.dirname(SRC), out_name)
        if os.path.isdir(out):
            shutil.rmtree(out)
        os.makedirs(out)
        copied = {"manifest": 0, "albedo": 0, keep: 0}
        for n in names:
            k = kind(n)
            if k in ("manifest", "albedo") or k == keep:
                shutil.copy2(os.path.join(SRC, n), os.path.join(out, n))
                copied[k] = copied.get(k, 0) + 1
        # The albedo has to be identical or the pair differs in more than the lever.
        drift = [
            n for n in by_kind.get("albedo", [])
            if md5(os.path.join(SRC, n)) != md5(os.path.join(out, n))
        ]
        print(
            f"{out_name}: manifest={copied['manifest']} albedo={copied['albedo']} "
            f"{keep}={copied[keep]}  albedo-drift={len(drift)}"
        )
        if drift:
            raise SystemExit(f"REFUSED  albedo differs in {out_name}: {drift[:3]}")
        # And the dropped suffix must really be absent, or the arm is not isolated.
        dropped = "_r" if keep == "_n" else "_n"
        left = [n for n in os.listdir(out) if kind(n) == dropped]
        if left:
            raise SystemExit(f"REFUSED  {out_name} still carries {len(left)} {dropped} files")
        print(f"           {dropped} files present: 0  -> the other channel stays derived")


if __name__ == "__main__":
    main()
