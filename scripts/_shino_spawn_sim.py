"""Shino's independent re-implementation of scene.rs::map_spawn / boot_scene.
Written from the Rust source, NOT from Yamamoto's validator, so it can disagree.
"""
import json, sys
from collections import defaultdict

CHUNK = 32
FIRE_AHEAD = 3.0

d = json.load(open(sys.argv[1] if len(sys.argv) > 1 else "maps/edhari.json"))
side = max(d["size"]["chunks_x"], d["size"]["chunks_z"])
c = side * CHUNK // 2

col = defaultdict(list)
for b in d["blocks"]:
    if b["block"] != "air":
        col[(b["x"], b["z"])].append(b["y"])

def highest_solid(x, z):
    ys = col.get((x, z))
    return max(ys) if ys else None

def map_spawn(cx, cz):
    for r in range(0, CHUNK * 2):
        for dz in range(-r, r + 1):
            for dx in range(-r, r + 1):
                if r > 0 and abs(dx) != r and abs(dz) != r:
                    continue
                x, z = cx + dx, cz + dz
                h = highest_solid(x, z)
                if h is None:
                    continue
                if h + 3 < CHUNK:
                    return (x, z, h)
    return None

got = map_spawn(c, c)
print(f"side={side} centre=({c},{c}) map_spawn -> {got}")
sx, sz, surface = got
feet = surface + 1
print(f"spawn col=({sx},{sz}) surface={surface} feet={feet}")
print(f"SPAWN_GROUND check: highest_solid({sx},{sz})={highest_solid(sx,sz)} expected={surface} "
      f"=> {'PASS' if highest_solid(sx,sz)==surface else 'FAIL'}")
print(f"centred at map middle? {'YES' if (sx,sz)==(c,c) else 'NO — engine walked away from the middle'}")

# campfire column
fz = int((sz + 0.5) - FIRE_AHEAD // 1) if False else int(((sz + 0.5) - FIRE_AHEAD) // 1)
fh = highest_solid(sx, fz)
print(f"campfire col=({sx},{fz}) highest_solid={fh} fire_y={(fh+1) if fh is not None else feet}")

# body occupancy: feet at `feet`, body needs feet..feet+1 clear in the spawn column
occ = set(col.get((sx, sz), []))
blocked = [y for y in (feet, feet + 1) if y in occ]
print(f"body clearance y={feet},{feet+1} in spawn col -> blocked={blocked} "
      f"=> {'PASS' if not blocked else 'FAIL'}")

# what surrounds spawn (5x5 top-solid map) — is it really an open interior?
print("\ntop-solid y around spawn (rows = z-2..z+2, cols = x-2..x+2):")
for z in range(sz - 2, sz + 3):
    row = []
    for x in range(sx - 2, sx + 3):
        h = highest_solid(x, z)
        row.append("--" if h is None else f"{h:2d}")
    print("   " + " ".join(row) + ("   <= spawn row" if z == sz else ""))

# any column in the whole map where a structure sits directly over the centre area
tall = [(x, z, highest_solid(x, z)) for (x, z) in col if highest_solid(x, z) is not None and highest_solid(x, z) + 3 >= CHUNK]
print(f"\ncolumns too tall for spawn (h+3>=32): {len(tall)}")
