"""Director's independent walkability audit for maps/edhari.json.

Deliberately does NOT import or reuse verify_edhari_village.py — the point is a
second implementation, derived straight from the engine's own rules in
client/src/main.rs, so a bug shared between the generator and its sibling
verifier cannot hide in both.

Engine rules mirrored here:
  * sim/src/block.rs  is_opaque()  -> every non-air block is solid
  * main.rs solid_at()             -> y outside 0..32 is air
  * PLAYER_HEIGHT 1.8              -> the body occupies feet cell + 1 above
  * STEP_HEIGHT 1.0                -> auto-climb at most one block
  * gravity                        -> falling any distance is free
Movement is 4-neighbour only (a 0.6-wide body cutting a diagonal corner is not
assumed) — conservative, so a PASS here is a real PASS.
"""

import json
import sys
from collections import deque

CHUNK = 32
MAP = "maps/edhari.json"
SPAWN = (32, 32)

solid = set()
with open(MAP, encoding="utf-8") as f:
    doc = json.load(f)
for b in doc["blocks"]:
    if b["block"] != "air":
        solid.add((b["x"], b["y"], b["z"]))

xs = [p[0] for p in solid]
zs = [p[2] for p in solid]
XMIN, XMAX = min(xs), max(xs)
ZMIN, ZMAX = min(zs), max(zs)


def is_solid(x, y, z):
    if y < 0 or y >= CHUNK:
        return False
    return (x, y, z) in solid


def standable(x, y, z):
    """Feet at y: floor below is solid, feet cell and head cell are clear."""
    return is_solid(x, y - 1, z) and not is_solid(x, y, z) and not is_solid(x, y + 1, z)


def feet_levels(x, z):
    return [y for y in range(0, CHUNK) if standable(x, y, z)]


def landing(x, z, from_y):
    """Where the player ends up walking into column (x,z) from feet level from_y.

    Prefers the highest standable level reachable by a <=1 step up, otherwise the
    highest one below (gravity does the rest). None = wall / no footing.
    """
    cands = feet_levels(x, z)
    up = [y for y in cands if from_y < y <= from_y + 1]
    if up:
        return min(up)
    down = [y for y in cands if y <= from_y]
    if down:
        return max(down)
    return None


def reachable_from(start_xz):
    sx, sz = start_xz
    starts = feet_levels(sx, sz)
    if not starts:
        print(f"  !! no standable footing at start {start_xz}")
        return {}
    sy = min(starts)
    seen = {(sx, sz): sy}
    q = deque([(sx, sy, sz)])
    while q:
        x, y, z = q.popleft()
        for dx, dz in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nx, nz = x + dx, z + dz
            if not (XMIN <= nx <= XMAX and ZMIN <= nz <= ZMAX):
                continue
            ny = landing(nx, nz, y)
            if ny is None:
                continue
            # head must clear while stepping across
            if ny > y and is_solid(x, y + 2, z):
                continue
            prev = seen.get((nx, nz))
            if prev is None or ny < prev:
                seen[(nx, nz)] = ny
                q.append((nx, ny, nz))
    return seen


print(f"map: {MAP}  blocks={len(doc['blocks'])}  x[{XMIN}..{XMAX}] z[{ZMIN}..{ZMAX}]")
reach = reachable_from(SPAWN)
print(f"reachable columns from spawn {SPAWN}: {len(reach)}")

fails = 0


def check(label, ok, detail=""):
    global fails
    print(("  PASS  " if ok else "  FAIL  ") + label + (f"   {detail}" if detail else ""))
    if not ok:
        fails += 1


def walk_check(label, x, z):
    y = reach.get((x, z))
    check(label, y is not None, f"feet y={y}" if y is not None else "unreachable from spawn")


def near_check(label, x, y, z):
    """A prop cell itself is solid — assert the player can stand right beside it."""
    has = is_solid(x, y, z)
    adj = [
        (x + dx, z + dz)
        for dx, dz in ((1, 0), (-1, 0), (0, 1), (0, -1))
        if (x + dx, z + dz) in reach
    ]
    check(label, has and bool(adj), f"block={has} standable-neighbours={len(adj)}")


print("\n-- interiors, from the real spawn (not from a convenient nearby door) --")
walk_check("west_house interior (16,24)", 16, 24)
walk_check("east_house interior (47,24)", 47, 24)

print("\n-- the doorways Shiba moved --")
walk_check("west_house south door (15,19)", 15, 19)
walk_check("east_house north door (49,19)", 49, 19)

print("\n-- story props --")
near_check("Toma's toy (15,1,24)", 15, 1, 24)
near_check("child's drawing (16,1,24)", 16, 1, 24)
near_check("Builder fresco (52,1,7)", 52, 1, 7)

print("\n-- landmarks that must not have regressed --")
walk_check("campfire (32,29)", 32, 29)
walk_check("first husk (32,25)", 32, 25)

print("\nRESULT:", "PASS (independent)" if fails == 0 else f"FAIL ({fails} check(s))")
sys.exit(1 if fails else 0)
