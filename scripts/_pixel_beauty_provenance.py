#!/usr/bin/env python3
"""Provenance stamp for the pixel-lane beauty shoot.

Memory lesson "commit time is not build time": a commit clock never dates a
binary.  So this prints the things that actually pin a frame to a build --
exe size/mtime/sha256, the git rev + dirty state at shoot time, the mtime of
every client source newer than the exe (i.e. edits the exe does NOT contain),
and an md5 per frame so a later reviewer can prove the plate they are looking
at is the plate that was graded.
"""
from __future__ import annotations

import hashlib
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "target-pixel" / "release" / "voxelforge_shot.exe"
OUT = ROOT / os.environ.get("OUT", "_fl_beauty_20260816")


def digest(p: Path, algo: str = "sha256") -> str:
    h = hashlib.new(algo)
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def stamp(ts: float) -> str:
    return datetime.fromtimestamp(ts, timezone.utc).astimezone().isoformat(timespec="seconds")


def git(*args: str) -> str:
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, text=True, timeout=30
        ).stdout.strip()
    except Exception as exc:  # pragma: no cover - diagnostics only
        return f"<git failed: {exc}>"


def main() -> int:
    print(f"root      {ROOT}")
    print(f"when      {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    print(f"git rev   {git('rev-parse', 'HEAD')}")
    print(f"git branch{git('rev-parse', '--abbrev-ref', 'HEAD'):>7}")
    dirty = git("status", "--porcelain")
    print(f"git dirty {len(dirty.splitlines())} paths")

    if not EXE.exists():
        print(f"EXE       MISSING {EXE}")
        return 2
    st = EXE.stat()
    print(f"EXE       {EXE.relative_to(ROOT)}")
    print(f"  size    {st.st_size}")
    print(f"  mtime   {stamp(st.st_mtime)}")
    print(f"  sha256  {digest(EXE)}")

    # Sources edited AFTER the link -- those changes are NOT in this binary.
    src = ROOT / "client" / "src"
    newer = sorted(
        (p for p in src.glob("*.rs") if p.stat().st_mtime > st.st_mtime),
        key=lambda p: p.stat().st_mtime,
    )
    print(f"  sources newer than exe: {len(newer)}"
          + ("  (edits NOT in this binary -- other lanes)" if newer else ""))
    for p in newer:
        print(f"    {p.name:<28} {stamp(p.stat().st_mtime)}")

    print("frames")
    frames = sorted(OUT.glob("*.png")) if OUT.exists() else []
    if not frames:
        print("  <none>")
        return 1
    for p in frames:
        fst = p.stat()
        fresh = "NEW " if fst.st_mtime >= st.st_mtime else "ctl "
        print(f"  {fresh}{p.name:<34} {fst.st_size:>9}b  {stamp(fst.st_mtime)}  md5={digest(p, 'md5')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
