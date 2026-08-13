#!/usr/bin/env python3
"""Run `client/src/import.rs`'s closest_block tests headless.

`import.rs` lives in a bin target that pulls in Bevy, and a `cargo test` here
dies at DLL init (0xc0000142) on this machine, so the tests that guard the
importer's colour matching never actually get run. `closest_block` itself is
pure and only touches `voxelforge_sim::block`, so this probe lifts its *source
text* out of `import.rs` verbatim -- together with every `#[test] fn
closest_block_*` in the file's test module -- into a scratch crate, and
compiles that with `rustc --test` against a freshly built sim rlib.

What it proves: the assertions in those tests, against the palette that is on
disk right now. What it does NOT prove: that the rest of `import.rs` compiles.

    python scripts/_poppy_closest_block_probe.py
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
IMPORT_RS = ROOT / "client" / "src" / "import.rs"
# Overridable so the palette can be swapped for a positive control: point this
# at a copy of `sim/` with one colour nudged and the round-trip test must fail.
SIM_LIB = Path(os.environ.get("VOXELFORGE_SIM_LIB", ROOT / "sim" / "src" / "lib.rs"))


def slice_item(src: str, start: int) -> str:
    """Return src[start:] up to and including the item's balanced closing brace."""
    depth = 0
    i = src.index("{", start)
    for j in range(i, len(src)):
        if src[j] == "{":
            depth += 1
        elif src[j] == "}":
            depth -= 1
            if depth == 0:
                return src[start : j + 1]
    raise SystemExit("unbalanced braces while slicing item")


def main() -> int:
    src = IMPORT_RS.read_text(encoding="utf-8")

    m = re.search(r"^pub fn closest_block\(", src, re.M)
    if not m:
        raise SystemExit("closest_block not found in import.rs")
    fn = slice_item(src, m.start())

    tests = [
        slice_item(src, t.start())
        for t in re.finditer(r"^    fn closest_block_\w+\(", src, re.M)
    ]
    if not tests:
        raise SystemExit("no closest_block_* tests found in import.rs")
    body = "\n\n".join("    #[test]\n" + t for t in tests)

    out = Path(tempfile.mkdtemp(prefix="poppy_closest_"))
    try:
        subprocess.run(
            [
                "rustc", "--edition", "2021", "--crate-type=rlib",
                "--crate-name", "voxelforge_sim", str(SIM_LIB),
                "-o", str(out / "libvoxelforge_sim.rlib"),
            ],
            check=True,
        )

        probe = out / "probe.rs"
        probe.write_text(
            "use voxelforge_sim::block::BlockId;\n\n"
            + fn
            + "\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n"
            + body
            + "\n}\n",
            encoding="utf-8",
        )

        subprocess.run(
            [
                "rustc", "--edition", "2021", "--test", str(probe),
                "--extern", f"voxelforge_sim={out / 'libvoxelforge_sim.rlib'}",
                "-o", str(out / "probe.exe"),
            ],
            check=True,
        )
        print(f"-- ran {len(tests)} closest_block test(s) lifted from {IMPORT_RS.name}")
        return subprocess.run([str(out / "probe.exe")]).returncode
    finally:
        shutil.rmtree(out, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
