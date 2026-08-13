#!/usr/bin/env python3
"""Audit `BlockId::base_color()` against the designer's shipped palette spec.

The village is painted from exactly three files, and nothing in the build
checks that they agree:

  * `docs/block-palette.md`  — Monanisa's spec. §6.1's "New hex" column is the
    authority; §2's table is the earlier photo pass and must not contradict it.
  * `sim/src/block.rs`       — `base_color()`, the only albedo the renderer and
    the .vox importer read (`voxel.rs::tile_base`, `import.rs::closest_block`).
  * `maps/edhari.json`       — which block names the village actually places.

So this script parses all three and prints one row per block the map uses.
It is deliberately read-only and cargo-free: the build lane is the Director's.

    python scripts/block_palette_audit.py          # audit the map's blocks
    python scripts/block_palette_audit.py --all    # every palette slot

Exit code 0 = every block the map uses matches its spec, 1 = a mismatch, and
2 = a block the map uses has no colour spec at all (a designer gap, not a bug
to guess at).
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BLOCK_RS = ROOT / "sim" / "src" / "block.rs"
PALETTE_MD = ROOT / "docs" / "block-palette.md"
MAP_JSON = ROOT / "maps" / "edhari.json"


def hexs(rgb):
    return "#%02x%02x%02x" % tuple(rgb)


def parse_base_color(path):
    """`Self::GRASS => [91, 140, 70],` -> {"grass": [91, 140, 70]}."""
    text = path.read_text(encoding="utf-8")
    body = text.split("fn base_color(")[1]
    out = {}
    for const, r, g, b in re.findall(
        r"Self::([A-Z_]+)\s*=>\s*\[\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\]", body
    ):
        out[const.lower()] = [int(r), int(g), int(b)]
    return out


def parse_spec(path):
    """Pull the designer's final hex per block out of the palette doc.

    §6.1's "New hex" wins over §2's "Hex" — it is the later, full-16-slot pass
    and its own table marks the §2 values it carries forward. A block whose two
    tables disagree is reported rather than silently resolved.
    """
    spec = {}
    section = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            section = line
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 3:
            continue
        # §2: | # | BlockId | Hex | Changed? | Texture treatment |
        if section and section.startswith("## 2.") and len(cells) == 5:
            name, hx = cells[1], cells[2]
            col = "s2"
        # §6.1: | BlockId | line | Old hex | New hex | Changed? | Why |
        elif section and section.startswith("## 6.") and len(cells) == 6:
            name, hx = cells[0], cells[3]
            col = "s6"
        else:
            continue
        name = name.strip("`* ").lower()
        m = re.search(r"#([0-9a-fA-F]{6})", hx)
        if not m or not re.fullmatch(r"[a-z_]+", name):
            continue
        rgb = [int(m.group(1)[i : i + 2], 16) for i in (0, 2, 4)]
        spec.setdefault(name, {})[col] = rgb
    return spec


def main():
    show_all = "--all" in sys.argv[1:]
    code = parse_base_color(BLOCK_RS)
    spec = parse_spec(PALETTE_MD)
    used = {}
    for b in json.loads(MAP_JSON.read_text(encoding="utf-8"))["blocks"]:
        used[b["block"]] = used.get(b["block"], 0) + 1

    names = sorted(code) if show_all else sorted(used, key=lambda n: -used[n])
    print(f"{'block':<12} {'used':>6}  {'spec':<9} {'code':<9} verdict")
    print("-" * 60)
    bad = gaps = shown = 0
    for name in names:
        if name == "air":
            continue
        shown += 1
        want = spec.get(name, {})
        got = code.get(name)
        s6, s2 = want.get("s6"), want.get("s2")
        authority = s6 or s2
        n = used.get(name, 0)
        # Printed ASCII-only on purpose: this runs in a Windows console where a
        # section sign comes out as a replacement char and makes a clean audit
        # look like a broken one.
        if got is None:
            verdict, bad = "NO SUCH BLOCK in base_color()", bad + 1
        elif authority is None:
            verdict, gaps = "NO SPEC - designer gap, do not guess", gaps + 1
        elif authority != got:
            verdict, bad = f"MISMATCH (spec says {hexs(authority)})", bad + 1
        elif s6 and s2 and s6 != s2:
            verdict = f"ok (superseded sec.2 value {hexs(s2)})"
        else:
            verdict = "ok"
        print(
            f"{name:<12} {n:>6}  {hexs(authority) if authority else '-':<9} "
            f"{hexs(got) if got else '-':<9} {verdict}"
        )

    print("-" * 60)
    print(f"{shown} block(s) audited | {bad} mismatch | {gaps} unspecified")
    return 1 if bad else (2 if gaps else 0)


if __name__ == "__main__":
    sys.exit(main())
