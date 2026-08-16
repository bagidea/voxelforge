"""Before/after plate picker for the enemy-AI proof (Rose, 2026-08-15).

Reads the trace.csv the proof bin wrote (columns: frame,enemy,state,
archetype,x,z,dist — one row per enemy per app frame) and emits BEFORE/AFTER
PNG pairs for each headline behaviour transition, straight from CSV facts:

  detect  a = the enemy's last 'patrol' row before its first patrol->alert
          b = that enemy's first 'alert' row            (idle -> it noticed)
  chase   a = the enemy's row just before it first enters 'chase'
          b = its first 'chase' row with dist<10         (closing in)
  strike  a = its last 'windup'/'crouch' row before the first strike/pounce
          b = the first 'strike'/'pounce' row            (telegraph -> lunge)
  retreat a = the last strike/pounce row for that enemy
          b = its first 'recover' row after it           (post-strike retreat)

Frames are app frames; capture is every 2nd frame in [start,end], so each
pick falls back to frame+-1 when the exact PNG isn't on disk. A behaviour
that never appears is reported MISSING and no file is written for it —
never faked.

Usage: python scripts/_rose_ai_pairs.py <trace.csv> <frames_dir> <outdir>
"""
import csv
import os
import shutil
import sys

PAIRS = [
    ("detect", "idle patrol -> noticed the player", "alert"),
    ("chase", "alert/pre-strike -> closing chase", "chase"),
    ("strike", "telegraph windup -> committed strike", "strike"),
    ("retreat", "strike -> recover (backing off)", "retreat"),
]


def load(trace_path):
    per_enemy = {}
    with open(trace_path, encoding="utf-8-sig") as f:  # -sig: tolerate a BOM
        for r in csv.DictReader(f):
            r["frame"] = int(r["frame"])
            r["dist"] = float(r["dist"])
            per_enemy.setdefault(r["enemy"], []).append(r)
    return per_enemy


def pick(per_enemy):
    """Return {pair_name: (before_row, after_row)} or None per side."""
    out = {}

    # detect: earliest patrol->alert transition of any enemy
    best = None
    for rows in per_enemy.values():
        for prev, cur in zip(rows, rows[1:]):
            if prev["state"] == "patrol" and cur["state"] == "alert":
                if best is None or cur["frame"] < best[1]["frame"]:
                    best = (prev, cur)
                break
    out["detect"] = best

    # chase: earliest entry into 'chase'; b = first chase row with dist<10
    best = None
    for rows in per_enemy.values():
        for i, cur in enumerate(rows):
            if cur["state"] == "chase":
                prev = rows[i - 1] if i else None
                closing = next(
                    (r for r in rows[i:] if r["state"] == "chase" and r["dist"] < 10.0),
                    cur,
                )
                if best is None or cur["frame"] < best[1]["frame"]:
                    best = (prev, closing)
                break
    out["chase"] = best

    # strike: earliest windup/crouch -> strike/pounce transition
    best = None
    for rows in per_enemy.values():
        tele = None
        for cur in rows:
            if cur["state"] in ("windup", "crouch"):
                tele = cur
            elif cur["state"] in ("strike", "pounce"):
                if best is None or cur["frame"] < best[1]["frame"]:
                    best = (tele, cur)
                break
    out["strike"] = best

    # retreat: earliest strike/pounce -> recover transition
    best = None
    for rows in per_enemy.values():
        hit = None
        for cur in rows:
            if cur["state"] in ("strike", "pounce"):
                hit = cur
            elif cur["state"] == "recover" and hit is not None:
                if best is None or cur["frame"] < best[1]["frame"]:
                    best = (hit, cur)
                break
    out["retreat"] = best
    return out


def frame_png(frames_dir, f, side):
    # capture cadence is every 2nd frame, so odd app frames have no PNG.
    # 'a' falls back backwards first, 'b' forwards first — otherwise an
    # adjacent-frame transition can collapse both sides onto the SAME png.
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

    for name, why, _ in PAIRS:
        pair = picks.get(name)
        if not pair or not pair[0] or not pair[1]:
            missing.append(name)
            print(f"MISSING {name}: behaviour never appeared in the trace")
            continue
        a, b = pair
        pa, pb = frame_png(frames_dir, a["frame"], "a"), frame_png(frames_dir, b["frame"], "b")
        if pa and pb and os.path.samefile(pa, pb):
            print(f"MISSING {name}: a and b collapsed onto the same frame png ({pa})")
            continue
        if not pa or not pb:
            missing.append(name)
            print(f"MISSING {name}: frame PNG not on disk (a={a['frame']} b={b['frame']})")
            continue
        fa, fb = os.path.basename(pa), os.path.basename(pb)
        shutil.copyfile(pa, os.path.join(outdir, f"ai-a-before-{name}.png"))
        shutil.copyfile(pb, os.path.join(outdir, f"ai-b-after-{name}.png"))
        ok.append(name)
        print(f"PAIR {name} ({why})")
        print(f"  a ai-a-before-{name}.png <- {fa}  enemy={a['enemy']} {a['state']} dist={a['dist']:.1f}")
        print(f"  b ai-b-after-{name}.png  <- {fb}  enemy={b['enemy']} {b['state']} dist={b['dist']:.1f}")

    print(f"pairs written: {len(ok)} {ok}; missing: {len(missing)} {missing}")
    sys.exit(0 if len(ok) >= 3 else 1)  # Director's bar: at least 3 real pairs


if __name__ == "__main__":
    main()
