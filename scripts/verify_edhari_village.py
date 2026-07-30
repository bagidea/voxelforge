#!/usr/bin/env python3
"""Village-specific checks for maps/edhari.json that _shino_verify_edhari.py
does not cover: does it match the loader's ACTUAL accepted schema
(client/src/mapfile.rs), and will the engine's own spawn search
(client/src/scene.rs::map_spawn/boot_scene) actually put the player inside
the shelter, standing on solid ground, with headroom?

Read-only: does not touch client/src, does not build/run Rust.
"""
import json
import os

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

print()
if fail:
    print(f"RESULT: FAIL ({len(fail)} check(s) failed)")
else:
    print("RESULT: PASS (all checks green)")
