#!/usr/bin/env python3
"""Render the sky-overexposure evidence as one plate.

Three panels per frame:
  1. the frame, with the detected sky pixels tinted magenta so you can see
     exactly which pixels the numbers came from;
  2. the raw sky crop;
  3. the same crop with its own range stretched to full 0..255 — a live gradient
     survives that, a crushed one turns to noise.

Plus a per-row plot of the mean sky RGB against screen Y: the authored dome goes
deep blue at the zenith to warm haze at the horizon, so those three lines should
diverge across the frame. They are flat.

    python scripts/_poppy_sky_evidence_plate.py
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "assets" / "_poppy_sky_overexposed_2026-08-14.png"

HUD_ROWS = 46          # the status line is white text, not sky
FRAMES = [
    ("playable-walk-after.png", (620, 60, 820, 190)),
    ("edhari-load.png", (280, 46, 440, 110)),
]

BG = (18, 18, 22)
FG = (232, 232, 238)
DIM = (140, 140, 150)


def sky_mask(a: np.ndarray) -> np.ndarray:
    """Sky = near-clipped red plus a blue no lit terrain block here reaches."""
    m = (a[:, :, 0] >= 250) & (a[:, :, 2] >= 170)
    m[:HUD_ROWS, :] = False
    m[int(a.shape[0] * 0.45):, :] = False
    return m


def panel(img: Image.Image, w: int, h: int) -> Image.Image:
    return img.resize((w, h), Image.NEAREST)


def main() -> int:
    PW, PH = 420, 236       # panel size
    PAD, HDR = 14, 30
    rows = []

    for name, (x0, y0, x1, y1) in FRAMES:
        src = Image.open(ROOT / "docs" / "assets" / name).convert("RGB")
        a = np.asarray(src).astype(np.int16)
        m = sky_mask(a)

        # 1. frame with sky pixels marked
        marked = np.asarray(src).copy()
        marked[m] = (255, 0, 200)
        p1 = panel(Image.fromarray(marked), PW, PH)

        # 2. raw crop
        crop = src.crop((x0, y0, x1, y1))
        p2 = panel(crop, PW, PH)

        # 3. same crop, per-channel range stretched to 0..255
        c = np.asarray(crop).astype(np.float32)
        lo, hi = c.min(axis=(0, 1)), c.max(axis=(0, 1))
        span = np.maximum(hi - lo, 1e-6)
        p3 = panel(Image.fromarray(
            ((c - lo) / span * 255).clip(0, 255).astype(np.uint8)), PW, PH)

        # per-row means for the plot
        prof = []
        for y in range(a.shape[0]):
            sel = a[y][m[y]]
            if len(sel) >= 30:
                prof.append((y, sel.mean(axis=0)))
        rows.append((name, (lo, hi, span), [p1, p2, p3], prof))

    W = PAD + 3 * (PW + PAD)
    ROWH = HDR + PH + 128
    H = 44 + len(rows) * (ROWH + PAD)
    plate = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(plate)
    d.text((PAD, 12), "SKY IS OVEREXPOSED — dome renders, but every vertex is past "
                      "the tonemapper's shoulder", fill=FG)
    d.text((PAD, 26), "root cause: build_sky_dome_mesh multiplied by 1/exposure(); "
                      "bevy_pbr 0.19 never applies Exposure to unlit fragments "
                      "(pbr.wgsl:80-84)", fill=DIM)

    y = 46
    for name, (lo, hi, span), panels, prof in rows:
        d.text((PAD, y), name, fill=FG)
        labels = ["frame — magenta = pixels measured",
                  "sky crop, as shipped",
                  "same crop, range stretched to 0..255"]
        for i, (p, lab) in enumerate(zip(panels, labels)):
            x = PAD + i * (PW + PAD)
            plate.paste(p, (x, y + HDR))
            d.text((x, y + HDR + PH + 4), lab, fill=DIM)

        # profile plot spanning the row, caption ABOVE the box so nothing overlaps
        cap = y + HDR + PH + 20
        py = cap + 14
        ph, pw = 64, W - 2 * PAD
        if prof:
            ys = [p[0] for p in prof]
            y_lo, y_hi = min(ys), max(ys)
            d.text((PAD, cap),
                   f"mean sky R/G/B vs screen Y, y {y_lo}..{y_hi}, plotted on the "
                   f"full 0..255 scale — the dome is authored deep blue at the "
                   f"zenith to warm haze at the horizon, so these three lines "
                   f"should diverge across the frame:",
                   fill=DIM)
            d.rectangle([PAD, py, PAD + pw, py + ph], outline=(60, 60, 70))
            for ch, col in enumerate([(255, 90, 90), (90, 230, 120), (110, 160, 255)]):
                pts = []
                for yy, mean in prof:
                    px = PAD + (yy - y_lo) / max(y_hi - y_lo, 1) * pw
                    pv = py + ph - mean[ch] / 255.0 * ph
                    pts.append((px, pv))
                d.line(pts, fill=col, width=2)
                d.text((PAD + pw + 4 - 46, pts[-1][1] - 6),
                       f"{'RGB'[ch]} {prof[-1][1][ch]:.0f}", fill=col)
            d.text((PAD, py + ph + 4),
                   f"flat. across {y_hi - y_lo} rows the sky moves "
                   f"R {max(p[1][0] for p in prof) - min(p[1][0] for p in prof):.1f}  "
                   f"G {max(p[1][1] for p in prof) - min(p[1][1] for p in prof):.1f}  "
                   f"B {max(p[1][2] for p in prof) - min(p[1][2] for p in prof):.1f} "
                   f"levels — the gradient is crushed onto one value",
                   fill=(255, 140, 140))
        y += ROWH + PAD

    plate.save(OUT)
    print(f"wrote {OUT}  ({plate.size[0]}x{plate.size[1]})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
