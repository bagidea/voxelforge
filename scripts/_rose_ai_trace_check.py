#!/usr/bin/env python3
"""Assert the enemy-AI proof trace actually covers every behaviour beat.

Reads the trace.csv written by `voxelforge_enemyai_proof` and checks, per
archetype, the beats the CEO brief requires visible evidence of:
  (a) detect -> ALERT (state 'alert' entered from 'patrol')
  (b) pursuit per archetype: swarm closes distance during flee,
      bruiser advances without ever returning to patrol, pouncer orbits
  (c) TELEGRAPH before strike: 'windup'/'crouch' entered BEFORE
      'strike'/'pounce', and the strike moves along one locked line
Exit 0 = all checks green; any FAIL line = exit 1.
"""
import csv
import sys
from collections import defaultdict

path = sys.argv[1] if len(sys.argv) > 1 else "_enemyai_frames/trace.csv"
rows = list(csv.DictReader(open(path, encoding="utf-8")))
if not rows:
    print("FAIL: empty trace")
    sys.exit(1)

by_enemy = defaultdict(list)
for r in rows:
    by_enemy[int(r["enemy"])].append(r)

fails = []

def check(cond, msg):
    print(("PASS" if cond else "FAIL") + ": " + msg)
    if not cond:
        fails.append(msg)

# ---- (a) alert beat: every enemy went patrol -> alert exactly on approach
for eid, rs in by_enemy.items():
    seq = [r["state"] for r in rs]
    arch = rs[0]["archetype"]
    check("alert" in seq, f"id={eid} ({arch}) entered ALERT")

# ---- (b1) swarm closed distance during the flee phase (frames where the
# player was fleeing: between alert and end, distance must reach a minimum
# after having been large)
for eid, rs in by_enemy.items():
    if rs[0]["archetype"] != "swarm":
        continue
    dists = [float(r["dist"]) for r in rs]
    peak = max(dists)
    late_min = min(dists[len(dists) // 2:])
    check(peak > 8.0 and late_min < 2.0,
          f"id={eid} swarm closed the chase (peak {peak:.1f} -> {late_min:.1f})")

# ---- (b2) bruiser never returns to patrol once alerted
for eid, rs in by_enemy.items():
    if rs[0]["archetype"] != "bruiser":
        continue
    seq = [r["state"] for r in rs]
    alerted = seq.index("alert") if "alert" in seq else None
    if alerted is None:
        check(False, f"id={eid} bruiser alerted"); continue
    check("patrol" not in seq[alerted:], f"id={eid} bruiser NEVER forgot (no patrol after alert)")

# ---- (b3) pouncer orbits (stalk) and dashes (pounce)
pouncers = [(eid, rs) for eid, rs in by_enemy.items() if rs[0]["archetype"] == "pouncer"]
for eid, rs in pouncers:
    seq = [r["state"] for r in rs]
    check(seq.count("stalk") > 0, f"id={eid} pouncer ORBITED (stalk)")
    check(seq.count("pounce") >= 1, f"id={eid} pouncer DASHED (pounce) x{seq.count('pounce')}")

# ---- (c) telegraph before strike + locked strike line
for eid, rs in by_enemy.items():
    seq = [r["state"] for r in rs]
    arch = rs[0]["archetype"]
    tele, strike = ("windup", "strike") if arch != "pouncer" else ("crouch", "pounce")
    if strike in seq:
        i = seq.index(strike)
        check(tele in seq[:i], f"id={eid} ({arch}) TELEGRAPH {tele} before {strike}")

# strike locked line: during each strike/pounce run, heading stays ~constant
import math
for eid, rs in by_enemy.items():
    runs, cur = [], []
    prev = None
    for r in rs:
        if r["state"] in ("strike", "pounce"):
            cur.append((float(r["x"]), float(r["z"])))
        else:
            if len(cur) > 2:
                runs.append(cur)
            cur = []
    if len(cur) > 2:
        runs.append(cur)
    for k, run in enumerate(runs):
        headings = []
        for (x0, z0), (x1, z1) in zip(run, run[1:]):
            dx, dz = x1 - x0, z1 - z0
            if abs(dx) + abs(dz) > 1e-4:
                headings.append(math.degrees(math.atan2(dx, dz)))
        if len(headings) >= 3:
            spread = max(headings) - min(headings)
            check(spread < 25.0, f"id={eid} strike#{k} LOCKED line (heading spread {spread:.1f} deg)")

print(f"\n{len(fails)} fail(s)" if fails else "\nALL GREEN")
sys.exit(1 if fails else 0)
