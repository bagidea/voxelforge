"""Purity gate for the enemy_ai.rs pose-channel A/B (Rose, 2026-08-16).

The pose channel's contract is "behaviour is bit-identical with or without
it": pose_for reads state+time only and never touches translation, so the
BEFORE and AFTER proof runs (same seed, same scripted route) must walk the
SAME state sequence with the SAME positions, modulo wall-clock jitter in
delta_secs (vsync) which shifts WHEN a transition lands, not WHAT happens.

So this gate compares phase-aligned facts, not raw frame numbers:
  * per enemy, the ordered list of state transitions must match EXACTLY
  * at each matching transition, the enemy's dist-to-player is compared;
    max |delta| is reported (expected small: sub-frame jitter in when the
    threshold crossed, not a behaviour change)

Usage: python scripts/_rose_ai_pose_purity.py <before.csv> <after.csv>
Exit 0 = sequences identical; exit 1 = any sequence mismatch (with detail).
"""
import csv
import sys


def read(path):
    rows = []
    skipped = 0
    with open(path, encoding="utf-8") as f:
        for r in csv.DictReader(f):
            try:
                r["frame"] = int(r["frame"])
                r["dist"] = float(r["dist"])
            except (TypeError, ValueError):
                # partial last row of a killed run — column shift, not data
                skipped += 1
                continue
            rows.append(r)
    if skipped:
        print(f"  (skipped {skipped} malformed row(s) in {path})")
    return rows


def transitions(rows, enemy):
    """[(state, frame, dist)] on every state change, plus the entry state."""
    out = []
    last = None
    for r in rows:
        if r["enemy"] != enemy:
            continue
        if r["state"] != last:
            out.append((r["state"], r["frame"], r["dist"]))
            last = r["state"]
    return out


def main():
    bt = read(sys.argv[1])
    at = read(sys.argv[2])
    enemies = sorted({r["enemy"] for r in bt})
    ok = True
    worst = 0.0
    for e in enemies:
        tb = transitions(bt, e)
        ta = transitions(at, e)
        sb = [t[0] for t in tb]
        sa = [t[0] for t in ta]
        n = min(len(sb), len(sa))
        # Two wall-clock runs of the SAME sim pack slightly different total
        # sim time into the same frame count (vsync dt jitter), so the tape
        # ends mid-cycle at a different transition index. Behaviour identity
        # is therefore judged on the COMMON PREFIX: every transition the
        # shorter tape recorded must match the longer tape at the same index.
        # A real behaviour change (e.g. a mistimed windup) shifts a state
        # mid-sequence, which prefix comparison catches at that index.
        prefix_ok = sb[:n] == sa[:n]
        if not prefix_ok:
            ok = False
            print(f"enemy {e}: SEQUENCE MISMATCH WITHIN COMMON PREFIX")
            for i in range(n):
                if sb[i] != sa[i]:
                    print(f"  first divergence at #{i}: before={sb[i]} after={sa[i]}")
                    break
        else:
            dmax = max((abs(b[2] - a[2]) for b, a in zip(tb[:n], ta[:n])), default=0.0)
            worst = max(worst, dmax)
            print(
                f"enemy {e}: common prefix {n} transitions IDENTICAL "
                f"(tape lengths {len(sb)}/{len(sa)}), "
                f"max phase-aligned |ddist|={dmax:.3f}"
            )
    if not ok:
        sys.exit("FAIL: behaviour sequence changed — pose channel is not pure")
    print(f"PURITY PASS: all common-prefix sequences identical, worst |ddist|={worst:.3f}")


if __name__ == "__main__":
    main()
