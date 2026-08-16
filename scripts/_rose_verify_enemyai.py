#!/usr/bin/env python3
"""Verify the enemy-AI proof run from its NUMERIC trace (trace.csv) + stdout.

The clip is the rendering; this is the proof. Asserts, per the spec in
docs/enemy-behavior.md:
  1. every enemy covered its archetype's full state ladder (patrol, alert,
     pursuit, telegraph, strike, recover)
  2. the Swarm CLOSED on a fleeing player (chase 4.4 > player 3.8): min dist
     during/after the flee phase start must drop under strike range 1.5
  3. the Bruiser NEVER went back to patrol after its first alert (deaggro=INF)
  4. the Pouncer crouched and pounced (the dash exists), and its pounce
     overshoot is visible as dist rising again after a pounce frame

Usage: python scripts/_rose_verify_enemyai.py <trace.csv> <stdout.log>
Exit 0 + PASS lines = green; any FAIL = exit 1 with the reason.
"""
import sys
from collections import defaultdict

FLEE_T = 7.8  # when the scripted player starts fleeing

def main(csv_path: str, log_path: str) -> int:
    frames = []
    with open(csv_path, encoding="utf-8") as f:
        next(f)  # header
        for line in f:
            parts = line.strip().split(",")
            if len(parts) != 7:
                continue
            fr, eid, state, arch, x, z, dist = parts
            frames.append((int(fr), int(eid), state, arch, float(x), float(z), float(dist)))

    # ~60 app fps assumption is only for phase labelling; assertions on
    # states are frame-independent.
    states_by_enemy = defaultdict(set)
    arch_by_enemy = {}
    dists_by_enemy = defaultdict(list)
    for fr, eid, state, arch, x, z, dist in frames:
        states_by_enemy[eid].add(state)
        arch_by_enemy[eid] = arch
        dists_by_enemy[eid].append((fr, dist))

    ok = True
    def check(cond: bool, label: str, detail: str = ""):
        nonlocal ok
        print(("PASS " if cond else "FAIL ") + label + (f" — {detail}" if detail else ""))
        if not cond:
            ok = False

    for eid, arch in sorted(arch_by_enemy.items()):
        s = states_by_enemy[eid]
        base = {"patrol", "alert", "recover"}
        want = {
            "swarm": base | {"chase", "windup", "strike"},
            "bruiser": base | {"advance", "windup", "strike"},
            "pouncer": base | {"stalk", "crouch", "pounce"},
        }[arch]
        missing = want - s
        check(not missing, f"enemy#{eid} ({arch}) state ladder complete",
              f"missing={sorted(missing)}" if missing else f"covered={len(s)} states")

    # 2. swarm closes on the flee: min dist after flee start < 1.5 (strike range)
    flee_frame = int(FLEE_T * 60)
    for eid, arch in sorted(arch_by_enemy.items()):
        if arch != "swarm":
            continue
        after = [d for fr, d in dists_by_enemy[eid] if fr >= flee_frame]
        mn = min(after) if after else 999.0
        check(mn < 1.5, f"enemy#{eid} (swarm) closed to strike range during/after flee",
              f"min_dist={mn:.2f} (want <1.5)")

    # 3. bruiser never returns to patrol after first alert
    for eid, arch in sorted(arch_by_enemy.items()):
        if arch != "bruiser":
            continue
        seq = [st for fr, e, st, a, *_ in frames if e == eid]
        seen_alert = "alert" in seq
        relapsed = False
        if seen_alert and "patrol" in seq:
            # patrol AFTER the first alert is the violation
            i_alert = seq.index("alert")
            relapsed = "patrol" in seq[i_alert:]
        check(not (seen_alert and relapsed), f"enemy#{eid} (bruiser) never de-aggroed back to patrol")

    # 4. pouncer pounces: dist rises right after a pounce (overshoot past target)
    for eid, arch in sorted(arch_by_enemy.items()):
        if arch != "pouncer":
            continue
        seq = [(fr, d) for fr, e, st, a, x, z, d in frames if e == eid]
        st_seq = [(fr, st) for fr, e, st, a, *_ in frames if e == eid]
        pounce_frames = [fr for fr, st in st_seq if st == "pounce"]
        check(len(pounce_frames) > 0, f"enemy#{eid} (pouncer) pounced at least once",
              f"pounce frames={len(pounce_frames)}")
        # overshoot: exists a pounce frame where dist is rising vs 10 frames earlier
        dmap = dict(seq)
        overshoot = any(
            fr in dmap and (fr - 10) in dmap and dmap[fr] > dmap[fr - 10] + 0.3
            for fr in pounce_frames
        )
        check(overshoot, f"enemy#{eid} (pouncer) dash overshoot visible (dist rising mid-pounce)")

    # 5. stdout side: transitions + commits actually printed
    try:
        log = open(log_path, encoding="utf-8", errors="replace").read()
    except OSError:
        log = ""
    check("-> alert" in log, "stdout shows alert beats")
    check("TELEGRAPH->COMMIT" in log, "stdout shows committed telegraphs")
    n_commit = log.count("TELEGRAPH->COMMIT")
    print(f"INFO commits printed: {n_commit}; frames in trace: {frames[-1][0] if frames else 0}")
    check(n_commit >= 3, "at least 3 attacks committed across the pack")

    print("VERDICT:", "GREEN" if ok else "RED")
    return 0 if ok else 1

if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
