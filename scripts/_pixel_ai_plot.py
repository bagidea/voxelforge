#!/usr/bin/env python3
"""Top-down before/after panel for the enemy-AI pass (pixel, enemy-AI lane).

Input: two `VOXELFORGE_AI_TRACE` CSVs produced by the SAME binary, one run with
`VOXELFORGE_AI_LEGACY=1` (before) and one without (after). Output: a single
side-by-side PNG showing where the husks actually went, plus the numbers the
caption is allowed to claim.

Why a plot and not just the state log: "they fan out and take turns instead of
stacking into one clump" is a statement about POSITIONS. The AI state log can
show `reposition` firing and still be consistent with three bodies standing in
the same block. Only the trace can tell those apart, so the trace is what gets
drawn and what the headline numbers are computed from.

Every number printed on the panel is derived here from the CSV rows — nothing
is typed in by hand. If a metric cannot be computed (e.g. a run with a single
husk, where "pack spacing" is meaningless) it is reported as `n/a`, never as a
flattering default.
"""
from __future__ import annotations

import csv
import sys
from collections import defaultdict
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# Panel geometry
TILE = 760           # px per panel (square world view)
PAD = 26
HEADER = 96
FOOTER = 168
BG = (18, 16, 22)
INK = (236, 230, 220)
DIM = (150, 142, 134)
GRID = (44, 40, 50)
PLAYER = (255, 209, 102)
# Per-husk track colours (up to 4 husks; the AI demo spawns 3)
TRACKS = [(239, 83, 80), (79, 195, 247), (129, 199, 132), (186, 104, 200)]
# States that hold a squad attack slot — must mirror combat.rs::husk_attacking
ATTACK_STATES = {
    "telegraph",
    "feint",
    "attack(swing1)",
    "attack(swing2)",
    "telegraph(lunge)",
    "attack(lunge-dash)",
}
CLUMP_DIST = 1.2     # blocks; two bodies this close are visually one blob


def load(path: Path):
    """CSV -> (frames, husk_ids). frames = [(t, {id: (x, z, state)})] in time order."""
    by_t: dict[float, dict[str, tuple[float, float, str]]] = defaultdict(dict)
    with path.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            by_t[float(row["t"])][row["entity"]] = (
                float(row["x"]), float(row["z"]), row["state"],
            )
    frames = [(t, by_t[t]) for t in sorted(by_t)]
    husks = sorted({e for _, f in frames for e in f if e != "player"})
    return frames, husks


def metrics(frames, husks):
    """The numbers the caption may claim. All derived, none assumed."""
    clump_frames = 0
    spacings: list[float] = []
    max_attackers = 0
    states: set[str] = set()
    measurable = 0
    for _, f in frames:
        live = [f[h] for h in husks if h in f]
        for _, _, s in live:
            states.add(s)
        atk = sum(1 for _, _, s in live if s in ATTACK_STATES)
        max_attackers = max(max_attackers, atk)
        if len(live) < 2:
            continue
        measurable += 1
        near = min(
            ((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2) ** 0.5
            for i, a in enumerate(live) for b in live[i + 1:]
        )
        spacings.append(near)
        if near <= CLUMP_DIST:
            clump_frames += 1
    spacings.sort()
    return {
        "frames": len(frames),
        # Pack metrics need >=2 live husks; a 1-husk run cannot measure them.
        "measurable": measurable,
        "median_gap": spacings[len(spacings) // 2] if spacings else None,
        "min_gap": spacings[0] if spacings else None,
        "clump_pct": (100.0 * clump_frames / measurable) if measurable else None,
        "max_attackers": max_attackers,
        "states": states,
    }


def world_box(runs):
    """One shared world extent across both panels, so the two are comparable."""
    xs, zs = [], []
    for frames, _ in runs:
        for _, f in frames:
            for x, z, _ in f.values():
                xs.append(x)
                zs.append(z)
    cx, cz = (min(xs) + max(xs)) / 2, (min(zs) + max(zs)) / 2
    half = max(max(xs) - min(xs), max(zs) - min(zs)) / 2 + 2.0
    return cx, cz, max(half, 6.0)


def font(size):
    for name in ("seguisb.ttf", "segoeui.ttf", "arial.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def draw_panel(img, ox, oy, frames, husks, box, title, sub):
    d = ImageDraw.Draw(img)
    cx, cz, half = box

    def px(x, z):
        return (ox + (x - cx) / (2 * half) * TILE + TILE / 2,
                oy + (z - cz) / (2 * half) * TILE + TILE / 2)

    d.rectangle([ox, oy, ox + TILE, oy + TILE], fill=(24, 22, 30), outline=GRID)
    # 2-block grid, so the eye can judge spacing in world units
    step = 2.0
    g = -half
    while g <= half:
        a = px(cx + g, cz - half)
        b = px(cx + g, cz + half)
        d.line([a, b], fill=GRID)
        a = px(cx - half, cz + g)
        b = px(cx + half, cz + g)
        d.line([a, b], fill=GRID)
        g += step

    # Player path (what the husks were converging on)
    ppath = [px(*f["player"][:2]) for _, f in frames if "player" in f]
    if len(ppath) > 1:
        d.line(ppath, fill=(120, 98, 44), width=3)
    if ppath:
        x, y = ppath[-1]
        d.ellipse([x - 7, y - 7, x + 7, y + 7], fill=PLAYER)

    # Husk tracks, one colour each; attack frames marked so "taking turns" shows
    for i, h in enumerate(husks):
        col = TRACKS[i % len(TRACKS)]
        pts = [px(f[h][0], f[h][1]) for _, f in frames if h in f]
        if len(pts) > 1:
            d.line(pts, fill=col, width=2)
        for (_, f), pt in zip([fr for fr in frames if h in fr[1]], pts):
            if f[h][2] in ATTACK_STATES:
                d.ellipse([pt[0] - 2, pt[1] - 2, pt[0] + 2, pt[1] + 2], fill=(255, 255, 255))
        if pts:
            x, y = pts[-1]
            d.ellipse([x - 6, y - 6, x + 6, y + 6], fill=col, outline=INK)

    d.text((ox + 4, oy - 56), title, font=font(30), fill=INK)
    d.text((ox + 4, oy - 22), sub, font=font(17), fill=DIM)


def fmt(v, unit="", nd=2):
    return "n/a" if v is None else f"{v:.{nd}f}{unit}"


def main() -> int:
    if len(sys.argv) < 4:
        print("usage: _pixel_ai_plot.py <before.csv> <after.csv> <out.png>")
        return 2
    before_p, after_p, out_p = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    for p in (before_p, after_p):
        if not p.exists() or p.stat().st_size == 0:
            print(f"REFUSE: trace missing or empty: {p}")
            return 2

    runs = [load(before_p), load(after_p)]
    (bf, bh), (af, ah) = runs
    if not bf or not af:
        print("REFUSE: a trace has no rows — the run never reached Play")
        return 2
    bm, am = metrics(bf, bh), metrics(af, ah)
    box = world_box(runs)

    W = PAD * 3 + TILE * 2
    H = HEADER + TILE + FOOTER
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)
    d.text((PAD, 20), "Guard Husk AI — approach & squad behaviour",
           font=font(34), fill=INK)
    d.text((PAD, 60),
           "Top-down husk paths, one binary, same scripted player route. "
           "White dots = frames holding an attack slot.",
           font=font(17), fill=DIM)

    draw_panel(img, PAD, HEADER, bf, bh, box, "BEFORE — VOXELFORGE_AI_LEGACY=1",
               f"{len(bh)} husks · {bm['frames']} frames · raw-distance aggro, beeline, no squad rules")
    draw_panel(img, PAD * 2 + TILE, HEADER, af, ah, box, "AFTER — perception + squad pass",
               f"{len(ah)} husks · {am['frames']} frames · cone/hearing, alert beat, separation, {am['max_attackers']} max attackers")

    y = HEADER + TILE + 22
    rows = [
        ("median husk-husk spacing", fmt(bm["median_gap"], " b"), fmt(am["median_gap"], " b")),
        (f"frames clumped (<= {CLUMP_DIST} b apart)", fmt(bm["clump_pct"], " %", 1), fmt(am["clump_pct"], " %", 1)),
        ("max husks attacking at once", str(bm["max_attackers"]), str(am["max_attackers"])),
        ("distinct AI states observed", str(len(bm["states"])), str(len(am["states"]))),
    ]
    d.text((PAD, y), "metric", font=font(18), fill=DIM)
    d.text((PAD + 470, y), "before", font=font(18), fill=DIM)
    d.text((PAD + 700, y), "after", font=font(18), fill=DIM)
    for i, (label, b, a) in enumerate(rows):
        yy = y + 30 + i * 30
        d.text((PAD, yy), label, font=font(19), fill=INK)
        d.text((PAD + 470, yy), b, font=font(19), fill=(230, 130, 120))
        d.text((PAD + 700, yy), a, font=font(19), fill=(140, 220, 150))

    new_states = sorted(am["states"] - bm["states"])
    d.text((PAD, y + 30 + len(rows) * 30 + 6),
           "states only the after run reached: " + (", ".join(new_states) if new_states else "(none)"),
           font=font(18), fill=DIM)

    out_p.parent.mkdir(parents=True, exist_ok=True)
    img.save(out_p)
    print(f"wrote {out_p} ({img.size[0]}x{img.size[1]})")
    print(f"BEFORE {bm}")
    print(f"AFTER  {am}")
    # A panel whose two sides are identical is not evidence of anything.
    if bm["states"] == am["states"] and bm["median_gap"] == am["median_gap"]:
        print("WARN: before and after are indistinguishable — check the lever actually applied")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
