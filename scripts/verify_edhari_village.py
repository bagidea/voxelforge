#!/usr/bin/env python3
"""Village-specific checks for maps/edhari.json that _shino_verify_edhari.py
does not cover: does it match the loader's ACTUAL accepted schema
(client/src/mapfile.rs), and will the engine's own spawn search
(client/src/scene.rs::map_spawn/boot_scene) actually put the player inside
the shelter, standing on solid ground, with headroom?

This AAA pass also verifies authored-world intent:
- readable silhouette landmarks,
- a guiding main street,
- rest fire spots,
- combat cover around the first encounter,
- a lived-in southern half that is no longer empty.

Read-only: does not touch client/src, does not build/run Rust.
"""
import json
import os
from collections import deque

MAP_PATH = os.path.join(os.path.dirname(__file__), "..", "maps", "edhari.json")
CHUNK = 32

# The set client/src/mapfile.rs::block_id_from_name() actually recognises.
# BlockId has 15 palette entries (block.rs), but the map *loader* only knows
# these 5 — anything else is silently skipped on load (reported, not applied).
LOADER_KNOWN_BLOCKS = {"air", "grass", "dirt", "stone", "sand"}

fail = []


def check(label, ok):
    print(("PASS" if ok else "FAIL"), "-", label)
    if not ok:
        fail.append(label)


with open(MAP_PATH, encoding="utf-8") as f:
    raw = f.read()
d = json.loads(raw)

# ---- 1. Struct shape matches client/src/mapfile.rs::MapFile exactly ----
check("top-level keys are exactly version/name/size/blocks",
      set(d.keys()) == {"version", "name", "size", "blocks"})
check("version is int 1", isinstance(d.get("version"), int) and d["version"] == 1)
check("name is a string", isinstance(d.get("name"), str))
size = d.get("size", {})
check("size has only chunks_x/chunks_z (both int)",
      set(size.keys()) == {"chunks_x", "chunks_z"}
      and isinstance(size.get("chunks_x"), int) and isinstance(size.get("chunks_z"), int))
check("blocks is a non-empty list", isinstance(d.get("blocks"), list) and len(d["blocks"]) > 0)

cx, cz = size["chunks_x"], size["chunks_z"]
W, D = cx * CHUNK, cz * CHUNK

shape_ok = True
for b in d["blocks"]:
    if set(b.keys()) != {"x", "y", "z", "block"}:
        shape_ok = False
        break
    if not (isinstance(b["x"], int) and isinstance(b["y"], int) and isinstance(b["z"], int)):
        shape_ok = False
        break
    if not isinstance(b["block"], str):
        shape_ok = False
        break
check("every block entry is exactly {x,y,z,block} with correct types", shape_ok)

# ---- 2. Every block name is one the loader actually resolves ----
unknown = sorted({b["block"] for b in d["blocks"] if b["block"].lower() not in LOADER_KNOWN_BLOCKS})
check(f"all block names are in the loader's known set {sorted(LOADER_KNOWN_BLOCKS)}"
      + (f" (found unknown: {unknown})" if unknown else ""),
      not unknown)

# ---- 3. Reproduce map_spawn()'s own search and confirm it lands well ----
top = {}
for b in d["blocks"]:
    x, y, z = b["x"], b["y"], b["z"]
    if b["block"].lower() == "air":
        continue
    if (x, z) not in top or y > top[(x, z)]:
        top[(x, z)] = y

side = max(cx, cz)
c = side * CHUNK // 2  # matches client/src/scene.rs boot_scene(): c = world_side * CHUNK / 2


def highest_solid(x, z):
    return top.get((x, z))


def map_spawn(cx0, cz0):
    for r in range(CHUNK * 2):
        for dz in range(-r, r + 1):
            for dx in range(-r, r + 1):
                if r > 0 and abs(dx) != r and abs(dz) != r:
                    continue
                x, z = cx0 + dx, cz0 + dz
                h = highest_solid(x, z)
                if h is None:
                    continue
                if h + 3 < CHUNK:
                    return x, z, h
    return None


spawn = map_spawn(c, c)
check(f"map_spawn({c},{c}) resolves to a standable column", spawn is not None)
if spawn:
    sx, sz, h = spawn
    print(f"    -> spawn column ({sx},{sz}) surface(top solid y)={h}")
    check("engine spawns EXACTLY at the map centre (32,32), inside the shelter",
          (sx, sz) == (c, c))
    check(f"spawn column ({sx},{sz}) has >=3 voxels of headroom above the surface",
          h + 3 < CHUNK)

blocks_set = {(b["x"], b["y"], b["z"]) for b in d["blocks"]}
check("spawn column (32,32) itself carries no wall block above the floor "
      "(open interior, not embedded in the shelter's own wall)",
      not any((32, y, 32) in blocks_set for y in range(1, 6)))

fx, fz = 32, 29  # SPAWN_X, SPAWN_Z - FIRE_AHEAD(3.0) from scene.rs
check("the procedural campfire column (32,29) is clear of walls at head height",
      not any((fx, y, fz) in blocks_set for y in range(1, 4)))

# ---- 4. The dungeon-gate "glowing sigil" must have a face a player can
# actually see, not just a block sitting mid-slab with every neighbour solid.
# "sand" is only ever used for the sigil accent block (gen_edhari.py), so it
# uniquely identifies it. A face only counts if it opens toward the village
# (+Z, since -Z is "north"/into the sealed gate) or toward the sky (+Y) —
# a -Y (underside) opening doesn't count, since that pocket sits inside the
# sealed door and a player can never stand under it to look up.
sigil_blocks = [b for b in d["blocks"] if b["block"].lower() == "sand"]
check("exactly one 'sand' sigil block in the map", len(sigil_blocks) == 1)
for b in sigil_blocks:
    sgx, sgy, sgz = b["x"], b["y"], b["z"]
    visible_plus_z = (sgx, sgy, sgz + 1) not in blocks_set
    visible_plus_y = (sgx, sgy + 1, sgz) not in blocks_set
    check(f"sigil block ({sgx},{sgy},{sgz}) has a player-visible face "
          "(+Z toward the village or +Y toward the sky), not buried mid-slab",
          visible_plus_z or visible_plus_y)

# ===========================================================================
# AAA assertions — verify the authored-world intent is actually in the data.
# ===========================================================================

by_pos = {(b["x"], b["y"], b["z"]): b["block"].lower() for b in d["blocks"]}
by_type = {name: set() for name in LOADER_KNOWN_BLOCKS}
for b in d["blocks"]:
    by_type[b["block"].lower()].add((b["x"], b["y"], b["z"]))

# ---- 5. World scale: enough blocks to feel like a place, not a flat slab ----
block_count = len(d["blocks"])
check(f"map has AAA block count (>= 7000)", block_count >= 7000)
print(f"    -> block count: {block_count}")

# ---- 6. Landmark silhouettes: at least three distinct clusters reach high ----
# We raise the skyline bar to y>=18 so only genuine landmarks count; short walls
# and house stubs no longer pollute the cluster count.
TALL_Y = 18
skyline = {(b["x"], b["z"]) for b in d["blocks"] if b["y"] >= TALL_Y}
check(f"landmark skyline has blocks at y >= {TALL_Y}", len(skyline) >= 10)

# Cluster the skyline columns; we expect at least 3 separate landmarks.
def cluster_columns(cols):
    if not cols:
        return 0
    seen = set()
    clusters = 0
    for start in cols:
        if start in seen:
            continue
        clusters += 1
        q = deque([start])
        seen.add(start)
        while q:
            x, z = q.popleft()
            for dx, dz in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
                nb = (x + dx, z + dz)
                if nb in cols and nb not in seen:
                    seen.add(nb)
                    q.append(nb)
    return clusters

def cluster_cells(cols):
    """Return list of sets, each set is one connected cluster."""
    if not cols:
        return []
    seen = set()
    clusters = []
    for start in cols:
        if start in seen:
            continue
        cluster = set()
        q = deque([start])
        seen.add(start)
        cluster.add(start)
        while q:
            x, z = q.popleft()
            for dx, dz in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
                nb = (x + dx, z + dz)
                if nb in cols and nb not in seen:
                    seen.add(nb)
                    cluster.add(nb)
                    q.append(nb)
        clusters.append(cluster)
    return clusters

landmark_clusters = cluster_columns(skyline)
check(f"skyline forms at least 3 readable landmark silhouettes", landmark_clusters >= 3)
print(f"    -> skyline columns: {len(skyline)}, clusters: {landmark_clusters}")

# ---- 6b. Dominant vista landmark: one silhouette must tower above the rest ----
# This is the "see something far away and want to walk to it" check.
VISTA_Y = 20
vista_skyline = {(b["x"], b["z"]) for b in d["blocks"] if b["y"] >= VISTA_Y}
vista_clusters = cluster_cells(vista_skyline)

# Pick the cluster whose highest block is tallest — that is the intentional
# vista, not a coincidental tall wall.
def cluster_max_y(cluster):
    return max(b["y"] for b in d["blocks"] if (b["x"], b["z"]) in cluster)

vista_clusters.sort(key=cluster_max_y, reverse=True)
dominant = vista_clusters[0] if vista_clusters else set()
check(f"dominant vista landmark reaches y >= {VISTA_Y}", len(dominant) >= 10)
check(f"dominant vista landmark has at least 10 skyline columns", len(dominant) >= 10)
print(f"    -> vista (y>={VISTA_Y}) columns: {len(vista_skyline)}, dominant cluster: {len(dominant)}")

# ---- 6c. Sight-line: the dominant landmark is visible from spawn -------------
# The player wakes at (32,32) facing -Z.  We trace from roughly eye height to the
# highest point of the dominant landmark; a real vista fails if a wall stands on
# the line.
def surface_at(x, z):
    for y in range(31, -1, -1):
        if (x, y, z) in by_pos:
            return y
    return None


def is_solid(x, y, z):
    return (x, y, z) in by_pos


def line_of_sight(x0, y0, z0, x1, y1, z1):
    """DDA-like voxel ray from (x0,y0,z0) to (x1,y1,z1).  Returns True if no
    solid voxel blocks the line.  Sampling is conservative: we test every voxel
    the ray passes through."""
    dx, dy, dz = x1 - x0, y1 - y0, z1 - z0
    steps = max(abs(dx), abs(dy), abs(dz), 1)
    for i in range(steps + 1):
        t = i / steps
        x = int(round(x0 + dx * t))
        y = int(round(y0 + dy * t))
        z = int(round(z0 + dz * t))
        # Don't count the start/destination voxels as blockers.
        if (x, z) == (x0, z0) or (x, z) == (x1, z1):
            continue
        if is_solid(x, y, z):
            return False
    return True


if dominant:
    # target = highest solid voxel inside the dominant cluster
    dom_blocks = [(b["x"], b["y"], b["z"]) for b in d["blocks"]
                  if (b["x"], b["z"]) in dominant]
    dom_blocks.sort(key=lambda t: t[1], reverse=True)
    tx, ty, tz = dom_blocks[0]
    # Eye height is approximately 1.6 voxels above the surface (EYE_HEIGHT in scene.rs).
    spawn_eye_y = (surface_at(32, 32) or 0) + 2
    los_ok = line_of_sight(32, spawn_eye_y, 32, tx, ty, tz)
    check(f"dominant landmark ({tx},{ty},{tz}) is visible from spawn (line-of-sight)", los_ok)
    print(f"    -> dominant landmark top: ({tx},{ty},{tz}), spawn eye y={spawn_eye_y}")

# ---- 6d. Reachability: the player can walk from spawn to the landmark -------
def walkable_neighbours(x, z):
    h = surface_at(x, z)
    if h is None:
        return []
    out = []
    for dx, dz in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
        nx, nz = x + dx, z + dz
        if not (0 <= nx < W and 0 <= nz < D):
            continue
        nh = surface_at(nx, nz)
        if nh is None:
            continue
        # Step up/down at most one voxel; headroom for a 2-voxel-tall body.
        if abs(nh - h) > 1:
            continue
        if is_solid(nx, nh + 2, nz):
            continue
        out.append((nx, nz))
    return out


if dominant:
    # The landmark itself may be vertical (spire shaft) with no walkable cell on
    # its skyline columns, so we also accept reaching any walkable cell that is
    # directly adjacent to the dominant silhouette.
    goals = set(dominant)
    for cx, cz in list(dominant):
        for dx, dz in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
            goals.add((cx + dx, cz + dz))
    q = deque([(32, 32)])
    seen = {(32, 32)}
    reached = False
    while q:
        cur = q.popleft()
        if cur in goals:
            reached = True
            break
        for nxt in walkable_neighbours(*cur):
            if nxt not in seen:
                seen.add(nxt)
                q.append(nxt)
    check("dominant landmark is reachable on foot from spawn", reached)
    print(f"    -> reachable cells explored: {len(seen)}")

# ---- 7. Main street spine remains clear and paved --------------------------
main_street_cells = [(x, z) for x in range(30, 35) for z in range(6, 30)]
street_paved = sum(1 for (x, z) in main_street_cells
                   if (x, 0, z) in by_pos and by_pos[(x, 0, z)] == "stone")
check("main street (x30-34, z6-29) is majority stone-paved",
      street_paved >= len(main_street_cells) * 0.85)
street_clear = all((x, y, z) not in blocks_set
                   for x in range(30, 35) for z in range(6, 30) for y in range(3, 5))
check("main street is clear of head-height obstacles (y=3-4)", street_clear)

# ---- 8. Village well is recessed (2+ blocks deep) --------------------------
# Well centre (32,17); expect stone curb ring around a dirt bottom.
well_region = [(x, z) for x in range(29, 36) for z in range(14, 21)]
well_stone_y2 = sum(1 for (x, z) in well_region if (x, 2, z) in by_type["stone"])
well_dirt_low = sum(1 for (x, z) in well_region
                    if (x, 0, z) in by_type["dirt"] or (x, 1, z) in by_type["dirt"])
check("well has raised stone curb at y=2", well_stone_y2 >= 6)
check("well has dirt bottom below ground level", well_dirt_low >= 4)

# ---- 9. Skeleton remains near the well -------------------------------------
# A small plus/cross of stone blocks at y=1 around (37,18).
skeleton_zone = {(x, z) for x in range(36, 40) for z in range(17, 21)}
skel_stone = sum(1 for (x, z) in skeleton_zone if (x, 1, z) in by_type["stone"])
check("skeleton remains (stone cross) placed near the well", skel_stone >= 4)

# ---- 10. Rest fire spots ---------------------------------------------------
# A fire pit is a 2-block-high ring: stone around a dirt centre at y=1.
def find_fire_pits():
    found = 0
    checked = set()
    stone_y1 = {(x, z) for (x, y, z) in by_type["stone"] if y == 1}
    dirt_y1 = {(x, z) for (x, y, z) in by_type["dirt"] if y == 1}
    for cx in range(2, W - 2):
        for cz in range(2, D - 2):
            if (cx, cz) in checked:
                continue
            # centre must be dirt/ash
            if (cx, cz) not in dirt_y1:
                continue
            # 3x3 neighbourhood: at least 6 of the 8 neighbours are stone
            neighbours = [(cx + dx, cz + dz) for dx in (-1, 0, 1) for dz in (-1, 0, 1)
                          if (dx, dz) != (0, 0)]
            stone_neighbours = sum(1 for p in neighbours if p in stone_y1)
            if stone_neighbours >= 6:
                found += 1
                checked.update((cx + dx, cz + dz) for dx in (-2, -1, 0, 1, 2)
                               for dz in (-2, -1, 0, 1, 2))
    return found

fire_pit_count = find_fire_pits()
check("at least 2 rest fire-pit rings exist", fire_pit_count >= 2)
print(f"    -> fire pits found: {fire_pit_count}")

# ---- 11. Combat cover around first husk encounter --------------------------
# Expect low stone walls / pillars in the arena box x24-40, z22-28, y1-2.
arena_stone = sum(1 for (x, y, z) in by_type["stone"]
                  if 24 <= x <= 40 and 22 <= z <= 28 and 1 <= y <= 2)
check("husk arena has combat cover stones (>= 30 blocks)", arena_stone >= 30)
print(f"    -> arena cover stones: {arena_stone}")

# ---- 12. Southern half is no longer empty ----------------------------------
# Before the AAA pass z>=44 was essentially barren; now it should have authored
# content (fields, pond, fences, ruins, boundary wall).
south_blocks = sum(1 for b in d["blocks"] if b["z"] >= 44 and b["y"] >= 1
                   and b["block"].lower() != "air")
check("southern half (z>=44) has authored content (>= 400 blocks)", south_blocks >= 400)
print(f"    -> southern half blocks: {south_blocks}")

# ---- 13. No accidental holes punched through the spawn shelter floor --------
# The back of the shelter interior floor should remain grass at y=0.
# The fire plaza deliberately sits at the north opening (z~29-32).
shelter_floor = all((x, 0, z) in by_type["grass"]
                    for x in range(30, 35) for z in range(33, 36))
check("spawn shelter back floor remains solid grass", shelter_floor)

# ---- 14. Exploration rhythm: narrow -> open -> vista ------------------------
# The level should guide the player through compression and release, ending at
# a vantage point where the dominant landmark is framed.
walkable = {(x, z) for x in range(W) for z in range(D) if surface_at(x, z) is not None
            and not is_solid(x, surface_at(x, z) + 2, z)}

# Neighbour count on the walkable graph.
def walk_neighbours(cell):
    x, z = cell
    return [n for n in [(x + 1, z), (x - 1, z), (x, z + 1), (x, z - 1)]
            if n in walkable]

# Narrow corridor = cells with few walkable neighbours (dead-end or slot).
narrow_cells = {c for c in walkable if len(walk_neighbours(c)) <= 3}
open_cells = walkable - narrow_cells

# Find the longest narrow corridor (connected component of narrow cells).
def largest_component(cells):
    best = 0
    seen = set()
    for start in cells:
        if start in seen:
            continue
        q = deque([start])
        seen.add(start)
        size = 0
        while q:
            cur = q.popleft()
            size += 1
            for nxt in walk_neighbours(cur):
                if nxt in cells and nxt not in seen:
                    seen.add(nxt)
                    q.append(nxt)
        best = max(best, size)
    return best

narrow_corridor_len = largest_component(narrow_cells)
open_area_size = largest_component(open_cells)
check("exploration: a narrow corridor exists (length >= 15 cells)", narrow_corridor_len >= 15)
check("exploration: an open area exists (size >= 80 cells)", open_area_size >= 80)
print(f"    -> narrow corridor length: {narrow_corridor_len}, open area size: {open_area_size}")

# Vista points: cells where the dominant landmark is visible.
if dominant and dom_blocks:
    tx, ty, tz = dom_blocks[0]
    vista_count = 0
    # Sample every 4th cell so this stays cheap.
    sample_cells = [(x, z) for x in range(5, W - 5, 4) for z in range(5, D - 5, 4)
                    if (x, z) in walkable]
    for vx, vz in sample_cells:
        vy = surface_at(vx, vz) + 2
        if line_of_sight(vx, vy, vz, tx, ty, tz):
            vista_count += 1
    check("exploration: multiple vista points can see the dominant landmark (>= 5)", vista_count >= 5)
    print(f"    -> vista points with sight to dominant landmark: {vista_count}")

print()
if fail:
    print(f"RESULT: FAIL ({len(fail)} check(s) failed)")
else:
    print("RESULT: PASS (all checks green)")
