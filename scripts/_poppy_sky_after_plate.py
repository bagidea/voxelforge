#!/usr/bin/env python3
"""The after-plate for the unlit-exposure fix — same frames, same pixels.

Companion to `_poppy_sky_evidence_plate.py`, which measured the BEFORE frames.
This one measures the pair, and the whole point is that it measures **the same
pixel set on both sides**:

  * the sky mask is computed on the BEFORE frame only (`R>=250 & B>=170` — the
    signature of the blown sky) and then applied verbatim to the AFTER frame.
    A value-based mask re-run on a fixed sky would select nothing and the two
    columns would silently stop being comparable;
  * the frames come from the same deterministic capture (`prove_playable.sh`
    shoots at t=3.2 s), so the same (x, y) is the same ray in both.

The two numbers the fix has to move:

  1. % of those sky pixels pinned at the top of the range (R >= 250, and the
     stricter pure-white 255/255/255);
  2. how far mean sky R travels across the frame's rows of elevation — the
     gradient itself. Before: 0.8 levels over 170 rows, i.e. flat.

An angle guard runs first: mean |diff| over the NON-sky pixels. The fix touches
only the dome's vertex colours, so terrain must come back near-identical; if
that number is large the capture moved and no sky comparison below means
anything.

And one refusal, learned the hard way: if the AFTER frame is byte-identical to
the BEFORE one, the shot has NOT been re-taken with the fixed binary and this
script writes nothing. A plate headed "SKY AFTER THE FIX" whose after column is
the before frame reads as "the fix moved nothing" — a wrong result is worse
than a missing one, so a dry-run must not be able to leave one on disk.

Two pairs can be plated, because the first one turned out not to isolate the fix:

  POPPY_SKY_SET=proof  the two `prove_playable.sh` frames, pre-fix exe (05:35)
                       vs fixed exe (06:15). CONTAMINATED — 72408ad/3262c6e
                       repainted every block in between, so its angle guard
                       fires by design. Kept because the guard firing IS the
                       result.
  POPPY_SKY_SET=ab     (default) `_poppy_sky_gain_ab.sh` — one binary, the dome
                       put back at its pre-fix radiance through
                       `VOXELFORGE_LOOK_SKYGAIN`. Nothing but the dome moves.

    POPPY_SKY_SET=ab python scripts/_poppy_sky_after_plate.py
"""
import os
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
AFTER_DIR = ROOT / "docs" / "assets"

SET = os.environ.get("POPPY_SKY_SET", "ab")
HUD_ROWS = 46          # the status line is white text, not sky

if SET == "proof":
    OUT = AFTER_DIR / "_poppy_sky_fixed_2026-08-14.png"
    # The BEFORE frames live beside the AFTER ones under a `_poppy_sky_before_`
    # prefix, NOT in a scratch `_dir/` — `.gitignore:166` swallows any `_*/`, and
    # a before/after pair whose before half cannot be committed is not a
    # comparison anyone can re-run.
    BEFORE_OF = {n: f"_poppy_sky_before_{n}" for n in
                 ("playable-walk-after.png", "edhari-load.png")}
    FRAMES = [
        ("playable-walk-after.png", (620, 60, 820, 190)),
        ("edhari-load.png", (280, 46, 440, 110)),
    ]
    TITLE = ("SKY, TWO BINARIES -- pre-fix exe 05:35 vs fixed exe 06:15. The "
             "angle guard below is the finding: the pair does NOT isolate the "
             "dome (block palette moved in between)")
else:
    OUT = AFTER_DIR / "_poppy_sky_ab_2026-08-14.png"
    BEFORE_OF = {"_poppy_sky_ab_fixed.png": "_poppy_sky_ab_prefix.png"}
    FRAMES = [("_poppy_sky_ab_fixed.png", (280, 46, 440, 110))]
    TITLE = ("SKY AFTER THE FIX -- ONE binary, one map, one camera: the dome put "
             "back at its pre-fix radiance via VOXELFORGE_LOOK_SKYGAIN=3632.16 "
             "(= 2.4 x 1513.4)")


def before_path(name: str) -> Path:
    return AFTER_DIR / BEFORE_OF[name]

BG = (18, 18, 22)
FG = (232, 232, 238)
DIM = (140, 140, 150)
GOOD = (140, 240, 170)
BAD = (255, 140, 140)


def sky_mask(a: np.ndarray) -> np.ndarray:
    """Sky = near-clipped red plus a blue no lit terrain block here reaches.

    Identical to `_poppy_sky_evidence_plate.sky_mask` — and only ever fed the
    BEFORE frame, so the pixel set is frozen across the pair.
    """
    m = (a[:, :, 0] >= 250) & (a[:, :, 2] >= 170)
    m[:HUD_ROWS, :] = False
    m[int(a.shape[0] * 0.45):, :] = False
    return m


def profile(a: np.ndarray, m: np.ndarray):
    """Per-row mean RGB over the masked pixels, rows with a real patch only."""
    out = []
    for y in range(a.shape[0]):
        sel = a[y][m[y]]
        if len(sel) >= 30:
            out.append((y, sel.mean(axis=0)))
    return out


def stats(a: np.ndarray, m: np.ndarray, prof) -> dict:
    px = a[m]
    spans = [max(p[1][c] for p in prof) - min(p[1][c] for p in prof)
             for c in range(3)] if prof else [0.0, 0.0, 0.0]
    return {
        "n": int(m.sum()),
        "mean": px.mean(axis=0),
        "pin_r250": 100.0 * float((px[:, 0] >= 250).mean()),
        "pure_white": 100.0 * float((px >= 255).all(axis=1).mean()),
        "rows": (prof[0][0], prof[-1][0]) if prof else (0, 0),
        "spans": spans,
    }


def panel(img: Image.Image, w: int, h: int) -> Image.Image:
    return img.resize((w, h), Image.NEAREST)


def plot(d: ImageDraw.ImageDraw, x: int, y: int, w: int, h: int, prof, title: str):
    d.text((x, y - 14), title, fill=DIM)
    d.rectangle([x, y, x + w, y + h], outline=(60, 60, 70))
    if not prof:
        d.text((x + 8, y + h // 2), "no rows", fill=BAD)
        return
    ys = [p[0] for p in prof]
    y_lo, y_hi = min(ys), max(ys)
    for ch, col in enumerate([(255, 90, 90), (90, 230, 120), (110, 160, 255)]):
        pts = [(x + (yy - y_lo) / max(y_hi - y_lo, 1) * w,
                y + h - mean[ch] / 255.0 * h) for yy, mean in prof]
        d.line(pts, fill=col, width=2)
        d.text((x + w - 44, pts[-1][1] - 6), f"{'RGB'[ch]} {prof[-1][1][ch]:.0f}",
               fill=col)


def main() -> int:
    PW, PH = 420, 236
    PAD, HDR = 14, 30
    rows = []

    for name, (x0, y0, x1, y1) in FRAMES:
        b_src = Image.open(before_path(name)).convert("RGB")
        a_src = Image.open(AFTER_DIR / name).convert("RGB")
        if b_src.size != a_src.size:
            print(f"SKIP {name}: size {b_src.size} != {a_src.size}")
            continue
        b = np.asarray(b_src).astype(np.int16)
        a = np.asarray(a_src).astype(np.int16)
        if np.array_equal(b, a):
            print(f"REFUSE {name}: AFTER is byte-identical to BEFORE -- this frame "
                  f"has not been re-shot with the fixed binary. Nothing written.")
            return 2

        m = sky_mask(b)                      # frozen on the BEFORE frame
        guard = np.abs(b - a).mean(axis=2)   # angle guard, off-sky only
        off = ~m
        off[:HUD_ROWS, :] = False
        cam_drift = float(guard[off].mean())

        b_prof, a_prof = profile(b, m), profile(a, m)
        b_st, a_st = stats(b, m, b_prof), stats(a, m, a_prof)

        print(f"=== {name}  ({m.sum()} sky px, frozen from BEFORE) ===")
        print(f"  angle guard: mean |before-after| over non-sky = {cam_drift:.2f} levels")
        # NB: on the BEFORE side `R>=250` is 100% *by construction* — it is the
        # mask predicate. The number that carries information is the AFTER one:
        # of the pixels that were pinned at the top of the range, how many still
        # are once the dome stops shipping at 1513x.
        for tag, st in (("BEFORE", b_st), ("AFTER ", a_st)):
            print(f"  {tag}  mean RGB [{st['mean'][0]:5.1f}, {st['mean'][1]:5.1f}, "
                  f"{st['mean'][2]:5.1f}]   R>=250 {st['pin_r250']:5.1f}%"
                  f"{' (by construction)' if tag == 'BEFORE' else '                  '}"
                  f"   pure-white {st['pure_white']:5.3f}%   "
                  f"y{st['rows'][0]}..{st['rows'][1]}  "
                  f"span R {st['spans'][0]:.1f} G {st['spans'][1]:.1f} "
                  f"B {st['spans'][2]:.1f}")
        print()

        marked = np.asarray(b_src).copy()
        marked[m] = (255, 0, 200)
        p1 = panel(Image.fromarray(marked), PW, PH)
        p2 = panel(a_src, PW, PH)
        crop = a_src.crop((x0, y0, x1, y1))
        p3 = panel(crop, PW, PH)
        c = np.asarray(crop).astype(np.float32)
        lo, hi = c.min(axis=(0, 1)), c.max(axis=(0, 1))
        p4 = panel(Image.fromarray(
            ((c - lo) / np.maximum(hi - lo, 1e-6) * 255).clip(0, 255).astype(np.uint8)),
            PW, PH)
        rows.append((name, cam_drift, b_st, a_st, b_prof, a_prof, [p1, p2, p3, p4]))

    if not rows:
        print("nothing measured")
        return 1

    W = PAD + 4 * (PW + PAD)
    ROWH = HDR + PH + 160
    H = 46 + len(rows) * (ROWH + PAD)
    plate = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(plate)
    d.text((PAD, 12), TITLE, fill=FG)
    d.text((PAD, 26), "same frames, same capture, and the same pixel set: the sky "
                      "mask is computed on the BEFORE frame and applied verbatim "
                      "to the AFTER one", fill=DIM)

    y = 48
    for name, drift, b_st, a_st, b_prof, a_prof, panels in rows:
        d.text((PAD, y), name, fill=FG)
        d.text((PAD + 220, y),
               f"angle guard (non-sky mean abs diff) {drift:.2f} levels",
               fill=GOOD if drift < 3.0 else BAD)
        labels = [
            "BEFORE -- magenta = the pixels measured on both sides",
            "AFTER -- same shot, fixed dome",
            "AFTER sky crop, as shipped",
            "same crop, range stretched to 0..255",
        ]
        for i, (p, lab) in enumerate(zip(panels, labels)):
            x = PAD + i * (PW + PAD)
            plate.paste(p, (x, y + HDR))
            d.text((x, y + HDR + PH + 4), lab, fill=DIM)

        pw = (W - 3 * PAD) // 2
        ph = 76
        py = y + HDR + PH + 46
        plot(d, PAD, py, pw, ph, b_prof,
             f"BEFORE -- mean sky R/G/B vs screen Y, full 0..255 scale "
             f"(y{b_st['rows'][0]}..{b_st['rows'][1]})")
        plot(d, PAD * 2 + pw, py, pw, ph, a_prof,
             f"AFTER -- same rows, same scale "
             f"(y{a_st['rows'][0]}..{a_st['rows'][1]})")
        rowspan = max(b_st['rows'][1] - b_st['rows'][0], 1)
        d.text((PAD, py + ph + 6),
               f"across {rowspan} rows of elevation the sky moves  "
               f"R {b_st['spans'][0]:.1f} -> {a_st['spans'][0]:.1f}   "
               f"G {b_st['spans'][1]:.1f} -> {a_st['spans'][1]:.1f}   "
               f"B {b_st['spans'][2]:.1f} -> {a_st['spans'][2]:.1f} levels        "
               f"pinned R>=250  {b_st['pin_r250']:.1f}% -> {a_st['pin_r250']:.1f}%",
               fill=GOOD if a_st['spans'][0] > b_st['spans'][0] else BAD)
        y += ROWH + PAD

    plate.save(OUT)
    print(f"wrote {OUT}  ({plate.size[0]}x{plate.size[1]})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
