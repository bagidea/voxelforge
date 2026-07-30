import json, os, collections
p = os.path.join(os.path.dirname(__file__), "..", "maps", "edhari.json")
d = json.load(open(p, encoding="utf-8"))
cx, cz = d["size"]["chunks_x"], d["size"]["chunks_z"]
W, D = cx * 32, cz * 32
ok_names = {"grass", "dirt", "stone", "sand"}
bad = collections.Counter()
seen = set()
dup = 0
ys = set()
minx = miny = minz = 10**9
maxx = maxy = maxz = -10**9
for b in d["blocks"]:
    x, y, z, t = b["x"], b["y"], b["z"], b["block"]
    if not (0 <= x < W and 0 <= z < D): bad["xz"] += 1
    if not (0 <= y < 32): bad["y"] += 1
    if t.lower() not in ok_names: bad["type:" + str(t)] += 1
    k = (x, y, z)
    if k in seen: dup += 1
    seen.add(k)
    ys.add(y)
    minx, maxx = min(minx, x), max(maxx, x)
    miny, maxy = min(miny, y), max(maxy, y)
    minz, maxz = min(minz, z), max(maxz, z)
floor = sum(1 for b in d["blocks"] if b["y"] == 0)
types = collections.Counter(b["block"] for b in d["blocks"])
print("version", d.get("version"), "| name", d.get("name"), "| size", cx, "x", cz, "->", W, "x", D)
print("blocks", len(d["blocks"]), "| unique", len(seen), "| duplicates", dup)
print("bbox x", minx, maxx, "| y", miny, maxy, "| z", minz, maxz)
print("y levels used", sorted(ys))
print("floor(y=0) count", floor, "of", W * D, "->", round(100 * floor / (W * D), 1), "% covered")
print("types", dict(types))
print("BAD", dict(bad) if bad else "none")
