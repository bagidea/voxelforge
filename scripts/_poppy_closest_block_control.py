#!/usr/bin/env python3
"""Positive control for the closest_block guards — prove they can go RED.

A passing test is worth nothing until you have seen it fail for the reason it
was written for. This copies `sim/` to a scratch dir, nudges ONE palette entry
so `stone` sits almost on top of `cobblestone` (d² 22 -> 3, i.e. 4.69 -> 1.73
apart in sRGB), and re-runs `_poppy_closest_block_probe.py` against that copy
via its `VOXELFORGE_SIM_LIB` hook. Nothing under the repo is written.

Expected, and asserted below:
  * closest_block_palette_pairs_stay_far_enough_apart  -> FAILS (this is the
    regression it exists to catch)
  * closest_block_snaps_a_near_colour_to_its_neighbour -> FAILS (stone+2 now
    resolves to cobblestone — which is why it was moved off obsidian)
  * closest_block_round_trips_every_palette_colour     -> still PASSES, which is
    the whole point: it cannot see the palette getting tighter, only exact
    duplicates. Its doc comment used to claim otherwise.

    python scripts/_poppy_closest_block_control.py
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROBE = ROOT / "scripts" / "_poppy_closest_block_probe.py"

# stone #8f8776 -> a colour 1.73 from cobblestone #8c8a78 instead of 4.69.
STONE_FROM = "Self::STONE       => [143, 135, 118],"
STONE_TO = "Self::STONE       => [141, 137, 119],"

EXPECT_FAIL = {
    "closest_block_palette_pairs_stay_far_enough_apart",
    "closest_block_snaps_a_near_colour_to_its_neighbour",
}
EXPECT_PASS = {
    "closest_block_round_trips_every_palette_colour",
    "closest_block_produces_only_opaque",
}


def main() -> int:
    scratch = Path(tempfile.mkdtemp(prefix="poppy_ctrl_"))
    try:
        sim = scratch / "sim"
        shutil.copytree(ROOT / "sim", sim)
        block = sim / "src" / "block.rs"
        src = block.read_text(encoding="utf-8")
        if STONE_FROM not in src:
            print(f"!! anchor not found in {block} — palette moved, update this control")
            return 2
        block.write_text(src.replace(STONE_FROM, STONE_TO), encoding="utf-8")
        print(f"-- control palette: {STONE_FROM.strip()}")
        print(f"                 -> {STONE_TO.strip()}")
        print("   (stone<->cobblestone d² 22 -> 3)\n")

        env = dict(os.environ, VOXELFORGE_SIM_LIB=str(sim / "src" / "lib.rs"))
        run = subprocess.run(
            [sys.executable, str(PROBE)],
            env=env, capture_output=True, text=True, cwd=ROOT,
        )
        out = run.stdout + run.stderr
        print(out)

        results = dict(re.findall(r"^test tests::(\w+) \.\.\. (ok|FAILED)$", out, re.M))
        if not results:
            print("!! no test results parsed — the probe itself did not run")
            return 2

        bad = []
        for name in sorted(EXPECT_FAIL | EXPECT_PASS):
            want = "FAILED" if name in EXPECT_FAIL else "ok"
            got = results.get(name, "<missing>")
            mark = "OK " if got == want else "BAD"
            if got != want:
                bad.append(name)
            print(f"  [{mark}] {name}: want {want}, got {got}")

        print()
        if bad:
            print(f"CONTROL FAILED — {len(bad)} test(s) did not behave as expected")
            return 1
        print("CONTROL PASSED — the two new guards go red on a tightened palette, "
              "and the round-trip test is confirmed blind to it")
        return 0
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
