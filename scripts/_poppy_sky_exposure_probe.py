#!/usr/bin/env python3
"""Why the sky reads as clipped white: the dome's exposure compensation.

Two independent measurements, no rebuild required:

  1. THE MATHS. `build_sky_dome_mesh` pre-multiplies every vertex colour by
     `1 / Exposure{ev100}.exposure()` on the stated premise that Bevy applies
     `Exposure` to unlit fragments. It does not (bevy_pbr-0.19.0
     `src/render/pbr.wgsl:80-84`: the unlit branch assigns `base_color`
     straight out and never reaches the `view.exposure *` in
     `pbr_functions.wgsl:863`). So nothing divides the factor back out and the
     dome ships at `exp_comp` times the radiance the flat `ClearColor` sky had.

  2. THE FRAMES. Measure the actual sky pixels in the two captures the Director
     pulled off the 05:35 binary.

Read-only: opens PNGs, writes nothing.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# 1. The maths
# ---------------------------------------------------------------------------

def exposure(ev100: float) -> float:
    """bevy_camera-0.19.0 `Exposure::exposure()` — `exp2(-ev100) / 1.2`."""
    return 2.0 ** (-ev100) / 1.2


def srgb_to_linear(c: float) -> float:
    """Bevy `Color::srgb(..).to_linear()` component transfer."""
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


# Values read off client/src/look.rs `GOLDEN` (the shipped hour).
EV100 = 10.3          # look.rs:841
SKY = (0.36, 0.60, 0.90)  # look.rs:759
SKY_GAIN = 2.4        # look.rs:760


def report_maths() -> float:
    exp = exposure(EV100)
    exp_comp = 1.0 / exp
    print("== 1. exposure compensation ==")
    print(f"  ev100                     = {EV100}")
    print(f"  Exposure::exposure()      = {exp:.6e}")
    print(f"  exp_comp = 1/exposure()   = {exp_comp:.1f}x")
    print()
    lin = [srgb_to_linear(c) * SKY_GAIN for c in SKY]
    print("  zenith radiance written into the HDR target:")
    print(f"    flat ClearColor (correct)  = "
          f"[{lin[0]:.3f}, {lin[1]:.3f}, {lin[2]:.3f}]")
    dome = [c * exp_comp for c in lin]
    print(f"    sky dome as shipped        = "
          f"[{dome[0]:.1f}, {dome[1]:.1f}, {dome[2]:.1f}]")
    print(f"  -> the dome is {exp_comp:.0f}x over. Anything past ~16.0 linear")
    print("     tonemaps to 255 under TonyMcMapface, so the whole dome clips")
    print("     to white and the gradient is unmeasurable.")
    print()
    return exp_comp


# ---------------------------------------------------------------------------
# 2. The frames
# ---------------------------------------------------------------------------

def load_png(path: Path):
    """Decode a PNG to (w, h, rows-of-RGB) with no third-party dependency."""
    import struct
    import zlib

    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", f"{path} is not a PNG"
    pos, idat, w = 8, bytearray(), None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        ctype = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if ctype == b"IHDR":
            w, h, depth, colour = struct.unpack(">IIBB", body[:10])
            assert depth == 8, f"unsupported bit depth {depth}"
            assert colour in (2, 6), f"unsupported colour type {colour}"
            nchan = 3 if colour == 2 else 4
        elif ctype == b"IDAT":
            idat += body
        elif ctype == b"IEND":
            break
        pos += 12 + length

    raw = zlib.decompress(bytes(idat))
    stride = w * nchan
    rows, prev = [], bytearray(stride)
    p = 0
    for _ in range(h):
        f = raw[p]
        line = bytearray(raw[p + 1:p + 1 + stride])
        p += 1 + stride
        # PNG filters (spec 9.2)
        for i in range(stride):
            a = line[i - nchan] if i >= nchan else 0
            b = prev[i]
            c = prev[i - nchan] if i >= nchan else 0
            if f == 1:
                line[i] = (line[i] + a) & 0xFF
            elif f == 2:
                line[i] = (line[i] + b) & 0xFF
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 0xFF
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        rows.append([tuple(line[i:i + 3]) for i in range(0, stride, nchan)])
        prev = line
    return w, h, rows


def report_frame(path: Path, rects) -> None:
    w, h, rows = load_png(path)
    # The sky can only be in the upper band, and only where nothing occludes it.
    # "Sky candidate" = a pixel at or near the top of the range. The floor is on
    # RED alone: the frame is graded warm, so a blown sky loses blue first and a
    # min-over-all-channels floor would throw the sky away as "not white enough".
    band = rows[40:int(h * 0.45)]
    px = [p for row in band for p in row]
    bright = [p for p in px if p[0] >= 250]
    print(f"== 2. {path.name} ({w}x{h}) ==")
    print(f"  upper-band pixels sampled : {len(px)}")
    print(f"  sky candidates (R>=250)   : {len(bright)}"
          f"  ({100.0 * len(bright) / len(px):.1f}% of band)")
    if bright:
        at255 = sum(1 for p in bright if p[0] == 255)
        print(f"  of those, R clipped at 255: {at255}"
              f"  ({100.0 * at255 / len(bright):.1f}%)")
        for ch, name in enumerate("RGB"):
            vals = [p[ch] for p in bright]
            print(f"    {name}: min {min(vals):3d}  mean {sum(vals) / len(vals):6.1f}"
                  f"  max {max(vals):3d}")

    # THE GRADIENT TEST. Hand-picked rectangles can clip a building edge in and
    # invent a "span"; instead take only pixels that ARE the sky (R>=250 and the
    # blue that no lit terrain block in this frame reaches) and ask how the
    # colour moves with screen height. A live dome gradient MUST vary with
    # elevation — that is the entire point of it. A blown one cannot: every
    # vertex is past the tonemapper's shoulder, so they all land on one value.
    # Skip the HUD band: the status line is pure white text and would otherwise
    # be counted as the brightest "sky" row in the frame.
    HUD_ROWS = 46
    sky_rows = {}
    for y, row in enumerate(rows[:int(h * 0.45)]):
        if y < HUD_ROWS:
            continue
        keep = [p for p in row if p[0] >= 250 and p[2] >= 170]
        if len(keep) >= 30:  # enough to be an open patch, not an edge pixel
            n = len(keep)
            sky_rows[y] = tuple(sum(p[c] for p in keep) / n for c in range(3))
    if not sky_rows:
        print("  no sky rows detected")
        print()
        return
    ys = sorted(sky_rows)
    print(f"  sky rows detected: {len(ys)}  (y {ys[0]}..{ys[-1]})")
    for y in (ys[0], ys[len(ys) // 2], ys[-1]):
        c = sky_rows[y]
        print(f"    y={y:3d}  mean sky RGB = "
              f"[{c[0]:5.1f}, {c[1]:5.1f}, {c[2]:5.1f}]")
    spans = [max(sky_rows[y][c] for y in ys) - min(sky_rows[y][c] for y in ys)
             for c in range(3)]
    print(f"    top-to-bottom span   = "
          f"R {spans[0]:.1f}  G {spans[1]:.1f}  B {spans[2]:.1f}")
    print(f"    -> the sky moves {max(spans):.1f} levels over "
          f"{ys[-1] - ys[0]} rows of elevation.")
    print()


# Sky windows picked by eye off each frame — gaps in the skyline where the dome
# is the only thing that can be drawn.
FRAMES = [
    ("docs/assets/playable-walk-after.png", [
        ("top-left gap", (8, 50, 100, 200)),
        ("centre gap", (650, 70, 790, 170)),
        ("right gap", (1010, 80, 1140, 160)),
    ]),
    ("docs/assets/edhari-load.png", [
        ("left gap", (40, 48, 130, 90)),
        ("centre gap", (300, 50, 420, 95)),
        ("right gap", (990, 50, 1090, 100)),
    ]),
]


def main() -> int:
    report_maths()
    for rel, rects in FRAMES:
        p = ROOT / rel
        if p.exists():
            report_frame(p, rects)
        else:
            print(f"missing: {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
