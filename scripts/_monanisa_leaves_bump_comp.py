"""Monanisa — bump-depth compensation pass on make_leaves() only.

Runs make_leaves() standalone (never main()), regenerates leaves/_n/_r into
_polish_v2/ from the freshly-generated albedo (albedo math itself untouched),
then measures normal-map bump stats (mean|xy|, p95|xy|, z mean) and albedo
luminance percentiles against the live shipped leaves textures, and drops a
side-by-side normal-map comparison PNG.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _pixel_blocks_gen64 import SIZE, avg_hex, normal_from_height, roughness_img  # noqa: E402
from _pixel_blocks_gen64_polish_v2 import LIVE_DIR, OUT_DIR, make_leaves, save  # noqa: E402


def normal_stats(path):
    arr = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64) / 255.0
    n = arr * 2.0 - 1.0
    nx, ny, nz = n[..., 0], n[..., 1], n[..., 2]
    xy_mag = np.sqrt(nx ** 2 + ny ** 2)
    return {
        "mean_xy": float(xy_mag.mean()),
        "p95_xy": float(np.percentile(xy_mag, 95)),
        "z_mean": float(nz.mean()),
    }


def luma_percentiles(path):
    arr = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)
    luma = 0.299 * arr[..., 0] + 0.587 * arr[..., 1] + 0.114 * arr[..., 2]
    return {
        "p5": float(np.percentile(luma, 5)),
        "p50": float(np.percentile(luma, 50)),
        "p95": float(np.percentile(luma, 95)),
    }


def main():
    albedo, height, rough = make_leaves()
    assert albedo.size == (SIZE, SIZE)
    save(albedo, "leaves")
    save(normal_from_height(height), "leaves", "_n")
    save(roughness_img(rough), "leaves", "_r")
    hexcode, _ = avg_hex(albedo)
    print(f"leaves v2 64x64 albedo {hexcode}  +_n +_r  -> {OUT_DIR}")

    live_n = LIVE_DIR / "leaves_n.png"
    new_n = OUT_DIR / "leaves_n.png"
    live_a = LIVE_DIR / "leaves.png"
    new_a = OUT_DIR / "leaves.png"

    ls, ns = normal_stats(live_n), normal_stats(new_n)
    print("\n-- normal map bump stats --")
    print(f"{'metric':10s} {'live':>10s} {'new':>10s}")
    for k in ("mean_xy", "p95_xy", "z_mean"):
        print(f"{k:10s} {ls[k]:10.4f} {ns[k]:10.4f}")

    la, na = luma_percentiles(live_a), luma_percentiles(new_a)
    print("\n-- albedo luma percentiles --")
    print(f"{'metric':10s} {'live':>10s} {'new':>10s}")
    for k in ("p5", "p50", "p95"):
        print(f"{k:10s} {la[k]:10.2f} {na[k]:10.2f}")

    # side-by-side compare image: live normal | new normal, 4x nearest upscale
    cell = SIZE * 4
    pad = 8
    label_h = 20
    live_img = Image.open(live_n).convert("RGB").resize((cell, cell), Image.NEAREST)
    new_img = Image.open(new_n).convert("RGB").resize((cell, cell), Image.NEAREST)
    from PIL import ImageDraw, ImageFont

    W = cell * 2 + pad * 3
    H = cell + label_h + pad * 2
    sheet = Image.new("RGB", (W, H), (26, 27, 31))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("arial.ttf", 14)
    except Exception:
        font = ImageFont.load_default()
    draw.text((pad, 2), "LIVE normal (leaves_n.png)", fill=(210, 190, 190), font=font)
    draw.text((pad * 2 + cell, 2), "NEW normal (bump-compensated)", fill=(190, 210, 190), font=font)
    sheet.paste(live_img, (pad, label_h + pad))
    sheet.paste(new_img, (pad * 2 + cell, label_h + pad))
    out_path = OUT_DIR / "leaves_normal_compare.png"
    sheet.save(out_path)
    print(f"\nside-by-side normal compare -> {out_path}")


if __name__ == "__main__":
    main()
