#!/usr/bin/env python3
"""Stamp a provenance banner onto a generated grid table.

`_flamingo_beauty_table.py` writes a bare table with no statement of which
binary or which plates produced it.  That was survivable while one lane shot one
grid; after the 2026-08-16 `git clean -fd` there are now two vintages of the same
filenames on disk -- the 08-16 numbers (recovered from `_grid.json`) and the
08-17 re-shoot on a newer exe -- and a reader opening either file cannot tell
them apart.  A number whose origin is unreadable is the office's oldest scar.

So the banner is not decoration; it is the thing that keeps the recovered table
from being mistaken for the re-shot one.  Idempotent: re-stamping replaces the
existing banner instead of stacking a second one.

Usage:
  python scripts/_pixel_grid_banner.py FILE.md --text "line" [--text "line" ...]
"""
import argparse
from pathlib import Path

MARK_A = "<!-- provenance:begin -->"
MARK_B = "<!-- provenance:end -->"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("file")
    ap.add_argument("--text", action="append", required=True)
    a = ap.parse_args()

    p = Path(a.file)
    body = p.read_text(encoding="utf-8")
    if MARK_A in body:
        body = body.split(MARK_B, 1)[1].lstrip("\n")

    banner = "\n".join([MARK_A] + [f"# {t}" for t in a.text] + [MARK_B, "", ""])
    p.write_text(banner + body, encoding="utf-8")
    print(f"stamped {p}  ({len(a.text)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
