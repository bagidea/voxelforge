"""Rose — prove river_sunset.json vs river_sunset_dry.json differ by WATER only.

Loads both maps, deep-compares every top-level key, and for `blocks` compares
the non-water voxels exactly and counts the water-only delta. Prints PASS/FAIL.
"""
import json
import sys

WET = "maps/river_sunset.json"
DRY = "maps/river_sunset_dry.json"

wet = json.load(open(WET))
dry = json.load(open(DRY))

fail = []

# every key except `blocks` must match verbatim (camera, spawn, time, sun...).
# `name` is the map's own label, not scene data — allowed to differ.
for k in sorted(set(wet) | set(dry)):
    if k in ("blocks", "name"):
        continue
    if wet.get(k) != dry.get(k):
        fail.append(f"key '{k}' differs: {wet.get(k)!r} vs {dry.get(k)!r}")

def norm(blocks):
    out = {}
    water = 0
    for b in blocks:
        key = (b["x"], b["y"], b["z"])
        if b.get("block") == "water":
            water += 1
            continue
        out[key] = b.get("block")
    return out, water

wn, ww = norm(wet["blocks"])
dn, dw = norm(dry["blocks"])

print(f"wet : {len(wet['blocks'])} blocks, {ww} water, {len(wn)} non-water")
print(f"dry : {len(dry['blocks'])} blocks, {dw} water, {len(dn)} non-water")

only_wet = {k: v for k, v in wn.items() if dn.get(k) != v}
only_dry = {k: v for k, v in dn.items() if wn.get(k) != v}
print(f"non-water cells only in wet: {len(only_wet)}")
print(f"non-water cells only in dry: {len(only_dry)}")
if only_wet:
    sample = list(only_wet.items())[:5]
    print(f"  sample wet-only: {sample}")
if only_dry:
    sample = list(only_dry.items())[:5]
    print(f"  sample dry-only: {sample}")

if fail:
    print("STRUCTURAL DIFFS:")
    for f in fail[:10]:
        print(" ", f)

ok = (not fail) and not only_wet and not only_dry and ww > 0 and dw == 0
print("MAP IDENTITY:", "PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
