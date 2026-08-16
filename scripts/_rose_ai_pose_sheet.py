"""Telegraph pose before/after contact sheets (Rose, 2026-08-16).

Composes the consecutive-frame evidence for the enemy_ai.rs pose channel:
for each archetype it finds one full strike CYCLE in the run's own trace CSV
(windup|crouch -> strike|pounce -> recover), cuts an app-frame window around
it, samples the captured PNGs evenly across that window, and stacks BEFORE
(pose-less source) over AFTER (posed source) as two labelled rows.

Picking discipline (no eyeballing):
  * the cycle is the LAST complete one for that enemy in the cornered phase —
    late cycles happen next to the camera-followed player, so the enemy is
    large in frame
  * the window is cut per-run from that run's OWN trace (state-aligned), so
    vsync jitter between runs cannot mis-align panel phases
  * every panel is labelled with its own run's frame number + state, so the
    sheet is checkable against its trace

Usage:
  python scripts/_rose_ai_pose_sheet.py \
      <before_trace.csv> <before_frames_dir> \
      <after_trace.csv>  <after_frames_dir>  <out_prefix> \
      [--panels 8] [--pre 6] [--recover-frac 0.6]
"""
import argparse
import csv
import os
import sys
from PIL import Image, ImageDraw

CAP_MOD = 2  # the proof bin captures every 2nd app frame


def read_trace(path):
    rows = []
    with open(path, encoding="utf-8") as f:
        for r in csv.DictReader(f):
            r["frame"] = int(r["frame"])
            r["dist"] = float(r["dist"])
            rows.append(r)
    return rows


def cycles(rows, enemy):
    """(windup_start, strike_start, recover_start, recover_end) per full cycle."""
    mine = [r for r in rows if r["enemy"] == enemy]
    out = []
    i = 0
    while i < len(mine):
        if mine[i]["state"] in ("windup", "crouch"):
            w0 = mine[i]["frame"]
            j = i
            while j < len(mine) and mine[j]["state"] in ("windup", "crouch"):
                j += 1
            if j < len(mine) and mine[j]["state"] in ("strike", "pounce"):
                s0 = mine[j]["frame"]
                k = j
                while k < len(mine) and mine[k]["state"] in ("strike", "pounce"):
                    k += 1
                if k < len(mine) and mine[k]["state"] == "recover":
                    r0 = mine[k]["frame"]
                    m = k
                    while m < len(mine) and mine[m]["state"] == "recover":
                        m += 1
                    r1 = mine[m - 1]["frame"] if m > k else r0
                    out.append((w0, s0, r0, r1))
                    i = m
                    continue
        i += 1
    return out


def state_at(rows, enemy, frame):
    for r in rows:
        if r["enemy"] == enemy and r["frame"] == frame:
            return r["state"]
    return "?"


def window_frames(w0, s0, r0, r1, recover_frac, pre):
    """App-frame [start, end] around the cycle + the even frames to sample."""
    start = w0 - CAP_MOD * pre
    end = int(r0 + recover_frac * max(r1 - r0, 1))
    evens = [f for f in range(start, end + 1) if f % CAP_MOD == 0]
    return start, end, evens


def sample(evens, panels):
    if len(evens) <= panels:
        return evens
    step = (len(evens) - 1) / (panels - 1)
    return [evens[round(i * step)] for i in range(panels)]


def load(frames_dir, frame):
    path = os.path.join(frames_dir, f"f{frame:04d}.png")
    return Image.open(path).convert("RGB")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("before_trace")
    ap.add_argument("before_dir")
    ap.add_argument("after_trace")
    ap.add_argument("after_dir")
    ap.add_argument("out_prefix")
    ap.add_argument("--panels", type=int, default=8)
    ap.add_argument("--pre", type=int, default=6)
    ap.add_argument("--recover-frac", type=float, default=0.6)
    ap.add_argument(
        "--cycle-offset",
        type=int,
        default=0,
        help="0 = last full cycle (default); 1 = second-to-last, etc. The very "
        "last cycle can outrun the PNG tape (screenshots race AppExit), so "
        "offsets pick a cycle whose window is fully captured.",
    )
    args = ap.parse_args()

    bt = read_trace(args.before_trace)
    at = read_trace(args.after_trace)
    archs = {}
    for r in bt:
        archs.setdefault(r["archetype"], r["enemy"])

    written = []
    for arch, enemy in sorted(archs.items()):
        bc = cycles(bt, enemy)
        ac = cycles(at, enemy)
        if not bc or not ac:
            print(f"{arch}: no complete cycle for enemy {enemy}, skipped")
            continue
        b = bc[-1 - args.cycle_offset]
        a = ac[-1 - args.cycle_offset]
        bs, be, bev = window_frames(*b, args.recover_frac, args.pre)
        as_, ae, aev = window_frames(*a, args.recover_frac, args.pre)
        bf = sample(bev, args.panels)
        af = sample(aev, args.panels)
        missing = [
            f
            for f, d in ((bf, args.before_dir), (af, args.after_dir))
            for f in f
            if not os.path.exists(os.path.join(d, f"f{f:04d}.png"))
        ]
        if missing:
            print(f"{arch}: missing PNGs {missing[:4]}..., skipped")
            continue

        pw, ph = 320, 180
        lab, head, gut, labw = 18, 26, 4, 104
        x0 = labw + gut
        W = x0 + gut + args.panels * (pw + gut)
        H = head + 2 * (ph + lab) + gut
        sheet = Image.new("RGB", (W, H), (16, 17, 20))
        d = ImageDraw.Draw(sheet)
        commit_b, commit_a = b[1], a[1]
        row_labels = ("BEFORE\n(flat)", "AFTER\n(posed)")
        row_colors = ((170, 170, 170), (240, 200, 120))
        for col, (fb, fa) in enumerate(zip(bf, af)):
            x = x0 + gut + col * (pw + gut)
            for row, (f, t, dirn, commit) in enumerate(
                ((fb, bt, args.before_dir, commit_b), (fa, at, args.after_dir, commit_a))
            ):
                y = head + row * (ph + lab)
                img = load(dirn, f).resize((pw, ph))
                if abs(f - commit) <= CAP_MOD:  # the whip frame
                    d.rectangle([x - 1, y - 1, x + pw + 1, y + ph + 1], outline=(255, 200, 80))
                sheet.paste(img, (x, y))
                st = state_at(t, enemy, f)
                d.text(
                    (x + 4, y + ph + 3),
                    f"f{f:04d} {st}",
                    fill=(215, 215, 215) if row else (150, 150, 150),
                )
        # row labels in the left column (drawn after panels so they sit clear)
        for row, (txt, colr) in enumerate(zip(row_labels, row_colors)):
            y = head + row * (ph + lab)
            for i, line in enumerate(txt.split("\n")):
                d.text((8, y + ph // 2 - 8 + i * 14), line, fill=colr)
        d.text((x0, 8), f"{arch} (enemy {enemy}) — last full strike cycle, before f{bf[0]}..{bf[-1]} / after f{af[0]}..{af[-1]}; orange frame = commit (whip)", fill=(200, 200, 200))
        out = f"{args.out_prefix}_{arch}.png"
        sheet.save(out)
        written.append(out)
        print(
            f"{arch}: before f{bf[0]}..{bf[-1]} after f{af[0]}..{af[-1]} -> {out}"
        )
    if not written:
        sys.exit("no sheets written")


if __name__ == "__main__":
    main()
