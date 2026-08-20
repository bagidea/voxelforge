"""
Monanisa / art lane — audit value-range + hue-spread of the LIVE interior block
textures (assets/textures/blocks/*.png, NOT the _n/_r maps, NOT _backup dirs).

For each tile: convert to HSV over opaque pixels only, report:
  - V (value/brightness): min, max, range, std   <- "flat" = tiny range/std
  - hue spread: number of populated 10-degree bins (weight = S*V, threshold
    0.5% of total weight) out of 36  <- "flat" = 1-2 bins (near-monochrome)
  - S (saturation): mean, std

Read-only. No .rs, no asset writes.
"""
import colorsys
import json
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
BLOCKS = ROOT / "assets/textures/blocks"

TARGETS = [
    "oak_planks.png",
    "oak_log_side.png",
    "oak_log_top.png",
    "floorboards.png",
    "brick.png",
    "stone_bricks.png",
    "clay_plaster.png",
]


def analyze(path: Path):
    im = Image.open(path).convert("RGBA")
    px = list(im.getdata())
    vs = []
    hue_weight = {}
    total_w = 0.0
    sats = []
    for r, g, b, a in px:
        if a < 8:
            continue
        h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
        vs.append(v)
        sats.append(s)
        w = s * v
        total_w += w
        binno = int(h * 36) % 36
        hue_weight[binno] = hue_weight.get(binno, 0.0) + w

    vmin, vmax = min(vs), max(vs)
    vmean = sum(vs) / len(vs)
    vstd = (sum((x - vmean) ** 2 for x in vs) / len(vs)) ** 0.5

    smean = sum(sats) / len(sats)

    populated = 0
    if total_w > 0:
        for w in hue_weight.values():
            if w / total_w >= 0.005:
                populated += 1

    return {
        "file": path.name,
        "n_px": len(vs),
        "v_min": round(vmin, 3),
        "v_max": round(vmax, 3),
        "v_range": round(vmax - vmin, 3),
        "v_std": round(vstd, 4),
        "hue_bins_used": populated,
        "s_mean": round(smean, 3),
    }


def main():
    names = sys.argv[1:] or TARGETS
    results = []
    for name in names:
        p = BLOCKS / name
        if not p.exists():
            print(f"MISSING: {p}", file=sys.stderr)
            continue
        results.append(analyze(p))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
