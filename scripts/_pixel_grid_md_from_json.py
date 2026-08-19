#!/usr/bin/env python3
"""Regenerate `_grid.md` from `_grid.json` — the ORIGINAL numbers, not new ones.

`docs/assets/look/beauty/_grid.md` was destroyed by the `git clean -fd` of
2026-08-16 and is not in any transcript (a script emitted it, no `Write` tool
call ever carried it).  But the run that wrote it also wrote `_grid.json` in the
SAME `_flamingo_beauty_table.py` invocation (`--md` and `--json` are written back
to back, from one `cells` dict), and `_grid.json` survived because it was never
untracked-and-swept.  So the table is fully recoverable from its own JSON.

HOW THIS AVOIDS RE-DERIVING ANYTHING.  It does not re-implement the table.  It
imports `_flamingo_beauty_table.py` and runs that module's real `main()`, with
exactly two things swapped out:

  * `axes.grade()` -> a lookup into `_grid.json` (by plate stem, and by the REF
    label for the reference row).  Every number printed therefore comes from the
    2026-08-16 measurement, verbatim.
  * the plate directory -> a temp dir of empty `<stem>.png` files, so the
    module's own `sorted(dir.glob("*.png"))` enumerates the same 44 plates.

The formatting, the column order, the per-axis targets and the PASS/fail logic
are the shipped code's, untouched.  If `_grid.json` ever disagrees with this
output, the bug is in this shim, not in the numbers.

The JSON holds 44 cells (4 poses x 11 rungs) — `fill-up`/`fill-down` were shot in
a later pass and never re-tabled, so the recovered `_grid.md` is the 44-plate
table that actually existed, not a 52-plate table that never did.

Usage:
  python scripts/_pixel_grid_md_from_json.py \
      [--json docs/assets/look/beauty/_grid.json] \
      [--md   docs/assets/look/beauty/_grid.md]
"""
import argparse
import importlib
import json
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

RUNGS = ("r0,fog8-45,fog6-55,fog5-70,amb1700,amb1400,amb1100,ev11.3,ev12.0,"
         "atmos-off,win-atmosfog")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", default="docs/assets/look/beauty/_grid.json")
    ap.add_argument("--md", default="docs/assets/look/beauty/_grid.md")
    ap.add_argument("--rungs", default=RUNGS)
    a = ap.parse_args()

    doc = json.loads(Path(a.json).read_text(encoding="utf-8"))
    ref, cells = doc["ref"], doc["cells"]

    by_stem = {}
    for key, cell in cells.items():
        by_stem[cell["label"]] = cell
    print(f"{a.json}: ref + {len(by_stem)} plates")

    table = importlib.import_module("_flamingo_beauty_table")

    def grade(path, crop=None, hud_bottom=0, label=None):
        if str(path) == table.REF:
            return ref
        stem = Path(str(path)).stem
        if stem not in by_stem:
            raise KeyError(f"{stem} is not in {a.json}; refusing to invent a row")
        return by_stem[stem]

    table.axes.grade = grade

    with tempfile.TemporaryDirectory() as td:
        for stem in by_stem:
            (Path(td) / f"{stem}.png").touch()
        sys.argv = ["_flamingo_beauty_table.py", "--dir", td,
                    "--rungs", a.rungs, "--md", a.md]
        rc = table.main()

    if rc == 0:
        txt = Path(a.md).read_text(encoding="utf-8")
        # The temp dir must not leak into the artefact: the header prints only
        # counts, but assert it rather than trust it.
        assert "Temp" not in txt and "tmp" not in txt.lower(), "temp path leaked into _grid.md"
        print(f"\n{a.md}: {len(txt)} chars, {len(txt.splitlines())} lines")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
