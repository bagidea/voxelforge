#!/usr/bin/env python3
"""Positive + negative control for the two refusals in `_poppy_sky_after_plate`.

A gate that refuses everything is not a gate, it is a brick, and both new
refusals fire on every pair currently on disk — so neither has yet been shown to
be *reachable*. This runs the plate's own `main()` twice against synthetic
inputs in a temp directory:

  PASS case  — probe frame with the masked pixels blacked out, i.e. a mask that
               really is sitting on the dome. Purity 100%, drift unchanged at
               2.69, and the plate must WRITE and exit 0.
  FAIL case  — the same frames with one pixel block of the mask left bright, so
               purity drops below the floor. The plate must exit 2 and write
               nothing.

The frames are the committed A/B pair, so the PASS case is the real plate with
one input swapped; nothing about the geometry or the thresholds is faked.

    python scripts/_flamingo_sky_gate_control.py
"""
import importlib.util
import shutil
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "docs" / "assets"


def load_plate(after_dir: Path, out: Path):
    """Fresh module instance pointed at a scratch directory."""
    spec = importlib.util.spec_from_file_location(
        "plate_ctl", ROOT / "scripts" / "_poppy_sky_after_plate.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod.AFTER_DIR = after_dir
    mod.OUT = out
    return mod


def build(tmp: Path, purity: str) -> None:
    """Copy the A/B pair in, then synthesise a probe with the wanted purity."""
    names = ["_poppy_sky_ab_fixed.png", "_poppy_sky_ab_prefix.png"]
    for n in names:
        shutil.copy(ASSETS / n, tmp / n)
    before = np.asarray(Image.open(tmp / names[1]).convert("RGB")).astype(np.int16)
    after = np.asarray(Image.open(tmp / names[0]).convert("RGB"))

    spec = importlib.util.spec_from_file_location(
        "plate_mask", ROOT / "scripts" / "_poppy_sky_after_plate.py")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    mask = m.sky_mask(before)

    probe = after.copy()
    probe[mask] = 0                       # every masked pixel reacts to the dome
    if purity == "low":
        ys, xs = np.where(mask)
        keep = slice(0, int(len(ys) * 0.5))   # half of them stop reacting
        probe[ys[keep], xs[keep]] = after[ys[keep], xs[keep]]
    Image.fromarray(probe).save(tmp / "_poppy_sky_ab_probe.png")


def run(purity: str, want_rc: int) -> bool:
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        build(tmp, purity)
        out = tmp / "plate.png"
        rc = load_plate(tmp, out).main()
        wrote = out.exists()
        ok = rc == want_rc and wrote == (want_rc == 0)
        print(f"  purity={purity:<4} exit={rc} (want {want_rc})  "
              f"plate written={wrote}  ->  {'OK' if ok else 'BROKEN'}")
        return ok


def main() -> int:
    print("=== the purity refusal has to be reachable in BOTH directions ===")
    good = run("high", 0)
    bad = run("low", 2)
    print()
    if good and bad:
        print("CONTROL PASS: the gate writes a plate when the mask really is the "
              "dome, and refuses when it is not. It is a gate, not a brick.")
        return 0
    print("CONTROL FAIL: the refusal does not behave as documented.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
