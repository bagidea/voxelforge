"""Per-archetype before/after picker for the enemy-AI proof (Rose, 2026-08-16).

`_rose_ai_pairs.py` picks behaviour transitions from ANY enemy; the Director's
2026-08-16 bar is stricter — the images must show the behaviour of all THREE
archetypes (Swarm/Bruiser/Pouncer). This script reads the same trace.csv and
emits, per archetype, one BEFORE/AFTER pair pinned to that archetype's
signature transition, straight from CSV facts (never faked):

  swarm   a = last patrol/alert row before a swarm enemy's first 'chase'
          b = its first 'chase' row with dist<12        (pack pursuit closes)
  bruiser a = last advance/chase row before a bruiser's first 'windup'
          b = its first 'strike' row                    (telegraph -> commit)
  pouncer a = last stalk/crouch row before a pouncer's first 'pounce'
          b = its first 'pounce' row                    (orbit -> lunge)

Frame-PNG fallback matches _rose_ai_pairs.py (capture is every 2nd app frame;
'a' falls back backwards, 'b' forwards). A side that never appears is MISSING
and no file is written for it. Exit 0 only if all 3 archetypes got a real pair.

Usage: python scripts/_rose_ai_arch_pairs.py <trace.csv> <frames_dir> <outdir>
"""
import csv
import os
import shutil
import sys

# cast is fixed in enemy_ai_proof_main.rs: id1/2=Reaver(swarm) id3=Sentinel
# (bruiser) id4=Stalker(pouncer) — but read the archetype column, don't assume.
ARCHETYPES = ["swarm", "bruiser", "pouncer"]


def load(trace_path):
    per_enemy = {}
    with open(trace_path, encoding="utf-8-sig") as f:  # -sig: tolerate a BOM
        for r in csv.DictReader(f):
            r["frame"] = int(r["frame"])
            r["dist"] = float(r["dist"])
            per_enemy.setdefault(r["enemy"], []).append(r)
    return per_enemy


def pick_for_archetype(rows, tele_states, commit_state, closing_dist):
    """Rows of ONE enemy: return (before, after) around its signature
    telegraph->commit transition, or None if it never commits."""
    tele = None
    for cur in rows:
        if cur["state"] in tele_states:
            tele = cur
        elif cur["state"] == commit_state:
            if closing_dist is None:
                return tele, cur
            closing = next(
                (r for r in rows if r["state"] == commit_state and r["dist"] < closing_dist),
                cur,
            )
            return tele, closing
    return None


def pick(per_enemy):
    """{archetype: (before_row, after_row)} using each archetype's own cast."""
    plan = {
        # telegraph states -> commit state (with optional closing-dist gate)
        "swarm": (("patrol", "alert"), "chase", 12.0),
        "bruiser": (("advance", "chase"), "windup", None),
        "pouncer": (("stalk", "crouch"), "pounce", None),
    }
    # bruiser commits at windup->strike; widen: before = last pre-windup row,
    # after = first 'strike'. Handle by treating windup as telegraph too.
    out = {}
    for arch, (tele_states, commit, closing) in plan.items():
        best = None
        for rows in per_enemy.values():
            if not rows or rows[0]["archetype"] != arch:
                continue
            if arch == "bruiser":
                # a = last advance/chase before first windup, b = first strike
                pre = None
                hit = None
                seen_windup = False
                for cur in rows:
                    if cur["state"] == "windup":
                        seen_windup = True
                    elif not seen_windup and cur["state"] in tele_states:
                        pre = cur
                    elif seen_windup and cur["state"] == "strike":
                        hit = cur
                        break
                if hit is not None and pre is not None:
                    pair = (pre, hit)
                else:
                    continue
            else:
                pair = pick_for_archetype(rows, tele_states, commit, closing)
            if pair and pair[0] is not None:
                if best is None or pair[1]["frame"] < best[1]["frame"]:
                    best = pair
        out[arch] = best
    return out


def frame_png(frames_dir, f, side):
    order = (f, f - 1, f - 2, f + 1, f + 2) if side == "a" else (f, f + 1, f + 2, f - 1, f - 2)
    for cand in order:
        p = os.path.join(frames_dir, "f%04d.png" % cand)
        if os.path.exists(p) and os.path.getsize(p) > 0:
            return p
    return None


def main():
    trace, frames_dir, outdir = sys.argv[1:4]
    picks = pick(load(trace))
    os.makedirs(outdir, exist_ok=True)
    ok, missing = [], []
    for arch in ARCHETYPES:
        pair = picks.get(arch)
        if not pair or not pair[0] or not pair[1]:
            missing.append(arch)
            print(f"MISSING {arch}: signature transition never appeared in the trace")
            continue
        a, b = pair
        pa, pb = frame_png(frames_dir, a["frame"], "a"), frame_png(frames_dir, b["frame"], "b")
        if pa and pb and os.path.samefile(pa, pb):
            missing.append(arch)
            print(f"MISSING {arch}: a and b collapsed onto the same frame png ({pa})")
            continue
        if not pa or not pb:
            missing.append(arch)
            print(f"MISSING {arch}: frame PNG not on disk (a={a['frame']} b={b['frame']})")
            continue
        shutil.copyfile(pa, os.path.join(outdir, f"ai-arch-{arch}-a-before.png"))
        shutil.copyfile(pb, os.path.join(outdir, f"ai-arch-{arch}-b-after.png"))
        ok.append(arch)
        print(f"ARCHPAIR {arch}")
        print(f"  a ai-arch-{arch}-a-before.png <- {os.path.basename(pa)}  "
              f"enemy={a['enemy']} {a['state']} dist={a['dist']:.1f}")
        print(f"  b ai-arch-{arch}-b-after.png  <- {os.path.basename(pb)}  "
              f"enemy={b['enemy']} {b['state']} dist={b['dist']:.1f}")
    print(f"arch pairs written: {len(ok)} {ok}; missing: {len(missing)} {missing}")
    sys.exit(0 if len(ok) == 3 else 1)  # Director's bar: all 3 archetypes


if __name__ == "__main__":
    main()
