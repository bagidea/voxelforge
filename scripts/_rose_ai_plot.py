"""Top-down path plot for VOXELFORGE_AI_TRACE CSVs (Rose, 2026-08-14).

Reads one or more `t,entity,x,z,state,dist,evading` traces and draws every
husk's ground path plus the player's, so "fans out and takes turns" vs
"stacks into one clump and beelines" is visible as pixels, not a claim.

Usage:
    python scripts/_rose_ai_plot.py out.png before.csv [after.csv ...]
    python scripts/_rose_ai_plot.py out.png --side-by-side before.csv after.csv

PIL only (no matplotlib in this env). World coords map straight to pixels:
-X left/+X right, -Z up/+Z down, with a 1 block = 8 px scale and a margin.
The player path is the warm solid line; each husk is its own cool hue, its
line dashed whenever it was mid-commitment (attacking) so bursts of swings
show as dotted segments along the orbit.
"""
import sys
from PIL import Image, ImageDraw

SCALE = 8  # px per world block
MARGIN = 48

ATTACK = {"attack(swing1)", "attack(swing2)", "attack(lunge-dash)", "telegraph",
          "telegraph(lunge)", "feint"}

HUSK_COLORS = ["#7fd4ff", "#a78bfa", "#f472b6", "#facc15", "#4ade80", "#fb923c"]
PLAYER_COLOR = "#ffb35c"


def load(path):
    """-> {entity: [(t, x, z, state, evading), ...]}, bounds"""
    rows = {}
    lo = [1e9, 1e9]
    hi = [-1e9, -1e9]
    with open(path, encoding="utf-8") as f:
        for line in f:
            parts = line.strip().split(",")
            if len(parts) < 6 or parts[0] == "t":
                continue
            t, ent, x, z, state = parts[0], parts[1], float(parts[2]), float(parts[3]), parts[4]
            evading = bool(int(parts[5])) if len(parts) > 5 and parts[5] in "01" else False
            rows.setdefault(ent, []).append((t, x, z, state, evading))
            lo[0], lo[1] = min(lo[0], x), min(lo[1], z)
            hi[0], hi[1] = max(hi[0], x), max(hi[1], z)
    return rows, lo, hi


def draw_panel(draw, rows, ox, oy, label):
    """Draw one trace into a panel. (ox, oy) = panel top-left in image px."""
    # world (x, z) -> image (px, py):  x -> ox + MARGIN + (x - lo)*SCALE
    def pt(p):
        return (ox + MARGIN + p[1] * SCALE, oy + MARGIN + p[2] * SCALE)

    for i, (ent, path) in enumerate(rows.items()):
        color = PLAYER_COLOR if ent == "player" else HUSK_COLORS[i % len(HUSK_COLORS)]
        # Split the polyline into solid (free) / dotted (committed) segments.
        segs = [[path[0]]]
        for prev, cur in zip(path, path[1:]):
            committed = cur[3] in ATTACK or cur[4]
            if committed == (len(segs) % 2 == 1):
                segs[-1].append(cur)
            else:
                segs.append([prev, cur])
        for j, seg in enumerate(segs):
            if len(seg) < 2:
                continue
            pts = [pt(p) for p in seg]
            # player always solid; husks dotted when committed (odd segments)
            if ent != "player" and j % 2 == 1:
                draw.line(pts, fill=color, width=2)
                for k in range(0, len(pts) - 1, 3):
                    draw.ellipse([pts[k][0] - 2, pts[k][1] - 2, pts[k][0] + 2, pts[k][1] + 2], fill=color)
            else:
                draw.line(pts, fill=color, width=3)
        start, end = pt(path[0]), pt(path[-1])
        r = 5 if ent == "player" else 4
        draw.ellipse([end[0] - r, end[1] - r, end[0] + r, end[1] + r], outline=color, width=2)
        draw.ellipse([start[0] - 2, start[1] - 2, start[0] + 2, start[1] + 2], fill=color)
    draw.text((ox + MARGIN, oy + 12), label, fill="#e8e8f0")


def main():
    args = sys.argv[1:]
    side_by_side = "--side-by-side" in args
    args = [a for a in args if not a.startswith("--")]
    out, csvs = args[0], args[1:]

    traces = [load(p) for p in csvs]
    lo = [min(t[1][0] for t in traces), min(t[1][1] for t in traces)]
    hi = [max(t[2][0] for t in traces), max(t[2][1] for t in traces)]
    w = int((hi[0] - lo[0]) * SCALE) + 2 * MARGIN
    h = int((hi[1] - lo[1]) * SCALE) + 2 * MARGIN + 24

    if side_by_side and len(traces) == 2:
        img = Image.new("RGB", (w * 2 + 16, h), "#101218")
        d = ImageDraw.Draw(img)
        draw_panel(d, traces[0][0], 0, 0, "BEFORE — legacy (VOXELFORGE_AI_LEGACY)")
        draw_panel(d, traces[1][0], w + 16, 0, "AFTER — perception + tactics")
    else:
        img = Image.new("RGB", (w, h), "#101218")
        d = ImageDraw.Draw(img)
        for i, (rows, _, _) in enumerate(traces):
            draw_panel(d, rows, 0, i * (h // len(traces)) if len(traces) > 1 else 0,
                       csvs[i].split("/")[-1])
    img.save(out)
    print(f"wrote {out} ({img.size[0]}x{img.size[1]})")


if __name__ == "__main__":
    main()
