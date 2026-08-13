#!/usr/bin/env python3
"""Prove a binary carries a specific clasp-z build, by its own f32 constants.

An exe's mtime dates the LINK, not the source it was built from — this project
has already been burned by trusting a timestamp ("commit time is not build time":
two binaries straddling a commit differed in 30 of 80.7 MB, all link stamps). So
before any A6.2 verdict is attached to a binary, scan it for the literal little-
endian f32 byte patterns of the z constants that pass is supposed to contain.

    8f09601 (current HEAD) : housing 0.1675   gem 0.1825
    4d24d30 (superseded)   : housing 0.1225   gem 0.1375
    90ace55 (original)     : housing 0.08     gem 0.095

A binary carrying the HEAD pair and NOT the superseded pair is unambiguous.

Usage:  python scripts/_flamingo_a62_binscan.py target-quest/debug/voxelforge.exe
"""
import struct
import sys
from pathlib import Path

WANTED = [
    ("8f09601 HEAD      housing", 0.1675),
    ("8f09601 HEAD      gem    ", 0.1825),
    ("4d24d30 superseded housing", 0.1225),
    ("4d24d30 superseded gem    ", 0.1375),
]


def main():
    if len(sys.argv) < 2:
        raise SystemExit("usage: _flamingo_a62_binscan.py <exe> [exe ...]")
    for path in sys.argv[1:]:
        p = Path(path)
        if not p.exists():
            print(f"{path}: MISSING")
            continue
        blob = p.read_bytes()
        print(f"\n{path}  ({len(blob):,} bytes)")
        for label, val in WANTED:
            pat = struct.pack("<f", val)
            n = blob.count(pat)
            print(f"  {label} = {val:<8}  occurrences: {n}"
                  + ("   <- present" if n else "   <- absent"))


if __name__ == "__main__":
    main()
