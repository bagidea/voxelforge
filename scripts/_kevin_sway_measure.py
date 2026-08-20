#!/usr/bin/env python3
"""Sway proof grader — does the grass ACTUALLY move?

Reads two frames from the wind arm (`sway_amp` at its default) and two from the
control arm (`sway_amp = 0`), each pair shot at two different wind times from
the SAME binary, and reports the share of pixels that changed between the pair.

The control is the whole point: the app pins its clock to
`TimeUpdateStrategy::ManualDuration`, so with the sway zeroed the two control
frames must be bit-identical. Any motion the wind arm shows above the control's
number is the shader moving vertices, not the renderer wobbling.

## Why the guards exist

`foliage_proof_main.rs` pins its assets to `<exe>/assets`, and `client/build.rs`
WIPES that directory (`remove_dir_all`) on every build. If the shader or the
grass sprite is missing at run time the app still renders — a ground plane under
a flat sky, with no plants. Both wind frames then come back identical and a
naive reader calls that "the grass does not sway" when the truth is "there was
no grass". So a stillness verdict is only allowed to be FAIL once three guards
pass:

  * plants  — the run log printed `FOLIAGE plants=N` with N > 0
  * assets  — the run log carries no asset/shader load ERROR
  * detail  — the frame carries blade-shaped edge detail, not two flat bands

Any guard failing returns UNMEASURABLE (exit 2), never FAIL.

Usage:
    python _kevin_sway_measure.py <dir>

expects <dir>/{wind_a,wind_b,ctrl_a,ctrl_b}.png and the matching *.log/*.err.log
"""
import hashlib
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image

# A pixel counts as "moved" once any channel shifts by more than this many 8-bit
# levels — above dither/tonemap noise, below a real blade sliding off a pixel.
THRESH = 8
# Share of pixels that must carry a strong local gradient for the frame to hold
# a plant field at all. A ground plane + flat sky scores near zero; 625 grass
# cross-quads score far above this.
MIN_EDGE_PCT = 1.0
# Gradient magnitude (8-bit levels) that counts as an edge.
EDGE_LEVEL = 20


def load(p: Path) -> np.ndarray:
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.int16)


def md5(p: Path) -> str:
    return hashlib.md5(p.read_bytes()).hexdigest()


def edge_pct(img: np.ndarray) -> float:
    """Share of pixels sitting on a strong luminance edge — the blade detector."""
    lum = img.mean(axis=2)
    gx = np.abs(np.diff(lum, axis=1))
    gy = np.abs(np.diff(lum, axis=0))
    g = np.zeros_like(lum)
    g[:, :-1] = np.maximum(g[:, :-1], gx)
    g[:-1, :] = np.maximum(g[:-1, :], gy)
    return 100.0 * (g > EDGE_LEVEL).mean()


def read_log(root: Path, name: str) -> str:
    text = ""
    for suffix in (".log", ".err.log"):
        p = root / f"{name}{suffix}"
        if p.exists():
            text += p.read_text(encoding="utf-8", errors="replace")
    return text


def guards(root: Path, frame: np.ndarray) -> tuple[bool, list[str]]:
    """Prove the frame is measurable BEFORE any stillness is called a failure."""
    log = read_log(root, "wind_a")
    lines = []
    ok = True

    m = re.search(r"FOLIAGE plants=(\d+)", log)
    plants = int(m.group(1)) if m else 0
    if not m:
        ok = False
        lines.append("plants  FAIL  run log never printed `FOLIAGE plants=` (app died early?)")
    elif plants == 0:
        ok = False
        lines.append("plants  FAIL  the stage spawned 0 plants")
    else:
        lines.append(f"plants  ok    {plants} plants spawned")

    asset_err = [
        ln.strip()
        for ln in log.splitlines()
        if "ERROR" in ln and any(k in ln.lower() for k in ("asset", "shader", "load", "wgsl", "png"))
    ]
    if asset_err:
        ok = False
        lines.append(f"assets  FAIL  {len(asset_err)} asset/shader error(s): {asset_err[0][:110]}")
    else:
        lines.append("assets  ok    no asset/shader load error in the run log")

    e = edge_pct(frame)
    if e < MIN_EDGE_PCT:
        ok = False
        lines.append(f"detail  FAIL  edge density {e:.2f}% < {MIN_EDGE_PCT}% — frame holds no blades")
    else:
        lines.append(f"detail  ok    edge density {e:.2f}% (>= {MIN_EDGE_PCT}%)")

    return ok, lines


def arm(a: Path, b: Path) -> dict:
    ia, ib = load(a), load(b)
    if ia.shape != ib.shape:
        raise SystemExit(f"shape mismatch: {a.name} {ia.shape} vs {b.name} {ib.shape}")
    d = np.abs(ia - ib).max(axis=2)
    moved = d > THRESH
    return {
        "img_a": ia,
        "px": int(ia.shape[0] * ia.shape[1]),
        "moved_pct": 100.0 * moved.mean(),
        "mean_abs": float(np.abs(ia - ib).mean()),
        "max_abs": int(d.max()),
        "md5_a": md5(a),
        "md5_b": md5(b),
        "shape": f"{ia.shape[1]}x{ia.shape[0]}",
    }


def main() -> int:
    root = Path(sys.argv[1])
    missing = [n for n in ("wind_a", "wind_b", "ctrl_a", "ctrl_b") if not (root / f"{n}.png").exists()]
    if missing:
        print(f"UNMEASURABLE — missing frames: {', '.join(missing)}")
        return 2

    wind = arm(root / "wind_a.png", root / "wind_b.png")
    ctrl = arm(root / "ctrl_a.png", root / "ctrl_b.png")

    print(f"resolution     {wind['shape']}  ({wind['px']} px)")
    for name, r in (("WIND", wind), ("CTRL", ctrl)):
        same = "IDENTICAL" if r["md5_a"] == r["md5_b"] else "differ"
        print(
            f"{name}  moved={r['moved_pct']:6.3f}%  mean|d|={r['mean_abs']:6.3f}  "
            f"max|d|={r['max_abs']:3d}  md5 {same}"
        )

    print("\nguards (a still frame is only FAIL once these pass):")
    measurable, lines = guards(root, wind["img_a"])
    for ln in lines:
        print(f"  {ln}")

    # The control is the floor. Anything the wind arm does above it is the sway.
    delta = wind["moved_pct"] - ctrl["moved_pct"]
    print(f"\nwind - control = {delta:+.3f} pp of pixels moved")

    if ctrl["md5_a"] != ctrl["md5_b"]:
        print("CONTROL DIRTY — sway_amp=0 frames differ; the floor is not zero.")

    if not measurable:
        print("VERDICT: UNMEASURABLE — a guard failed, so stillness here says nothing")
        print("         about the wind shader. Fix the guard and re-shoot.")
        return 2
    if wind["md5_a"] == wind["md5_b"]:
        print("VERDICT: FAIL — the two wind frames are byte-identical. Nothing moved.")
        return 1
    if delta < 0.5:
        print(f"VERDICT: FAIL — {delta:.3f} pp over control is not visible sway.")
        return 1
    print(f"VERDICT: PASS — grass moves on {delta:.3f}% of the frame the control holds still.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
