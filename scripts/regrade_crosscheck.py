#!/usr/bin/env python3
"""Print EVERY measurer's G3/G5/G6 verdict side by side, from a regrade.json.

regrade.py runs three tools that grade the same gates by different means. They do not
always agree. The summary table is hand-written, and a hand-written table is exactly
where one of the columns gets quietly dropped -- which is what happened on the
2026-08-07 shotset: only grade_gate.py was reported while grade_hero.py disagreed on
5 of 8 plates and even pointed the total the other way. Picking one column is half a
verdict, and the verdict is not the measurer's job. So this prints all of them, marks
the disagreements, and refuses to choose.

Usage: python scripts/regrade_crosscheck.py <regrade.json> [out.txt]
"""
import json
import sys
from pathlib import Path

GATES = ("G3", "G5", "G6")

# tool -> (display name, key per gate, one-line description of HOW it measures)
TOOLS = [
    ("grade_gate.py", ("G3", "G5", "G6"), "auto-LOCATES the window / warm patch anywhere "
     "in frame. Generic, runs on any plate."),
    ("grade_hero.py", ("G3_pass", "G5_pass", "G6_pass"), "FIXED eyedrop regions tuned for "
     "the golden kitchen hero shot (window assumed screen-left, sunlit wood assumed in "
     "the lower 55%). On a non-hero plate those regions point at whatever is there."),
    ("grade_look.py", ("G3_pass", "G5_pass", None), "G3/G5 only, third opinion."),
]


def verdict(side, tool, key):
    """PASS flag for one gate, or None when that tool does not measure it."""
    if key is None:
        return None
    blob = side.get(tool) or {}
    v = blob.get(key)
    # grade_gate.py nests its flag one level down: {"G3": {"pass": true, ...}}
    if isinstance(v, dict):
        v = v.get("pass")
    return v


def pf(v):
    return "-" if v is None else ("P" if v else "F")


def main():
    if not 2 <= len(sys.argv) <= 3:
        print(__doc__.strip())
        return 2
    data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8-sig"))
    pairs = data.get("pairs", [])
    if not pairs:
        print("CROSSCHECK: no pairs in that json")
        return 1

    out = [
        "VOXELFORGE regrade -- GATE CROSS-CHECK  (every measurer, none dropped)",
        "cell = beforePass->afterPass   P=PASS F=FAIL  -=not measured by that tool",
        "",
    ]
    for tool, _keys, how in TOOLS:
        out.append(f"  {tool:<14}: {how}")
    out += [
        "",
        "  -> Where they disagree, which one is authoritative is the grader's call, not",
        "     this script's. Both readings are printed; neither is dropped.",
        "",
    ]

    cols = []
    for tool, keys, _ in TOOLS:
        n = sum(1 for k in keys if k is not None)
        cols.append((tool, keys, n, max(16, n * 8)))

    # Header names the GATES, not the tool's internal key names -- "grade_hero G3_pass/G5_"
    # is what you get when a column label is built out of dict keys.
    hdr = f"{'plate':<14} | " + " | ".join(
        f"{tool.replace('.py', '') + ' ' + '/'.join(g for g, k in zip(GATES, keys) if k):<{w}}"
        for tool, keys, _n, w in cols) + " | disagree"
    out += [hdr, "-" * len(hdr)]

    ref_tool, ref_keys = TOOLS[0][0], TOOLS[0][1]
    disagreeing = 0
    totals = {tool: [0, 0, 0] for tool, _k, _h in TOOLS}   # before, after, measured

    for p in pairs:
        before, after = p["before"], p["after"]
        cells = []
        for tool, keys, _n, w in cols:
            txt = "  ".join(
                f"{pf(verdict(before, tool, k))}->{pf(verdict(after, tool, k))}"
                for k in keys if k is not None)
            cells.append(f"{txt:<{w}}")
            for k in keys:
                if k is None:
                    continue
                totals[tool][2] += 1
                totals[tool][0] += bool(verdict(before, tool, k))
                totals[tool][1] += bool(verdict(after, tool, k))

        dis = []
        for tool, keys, _h in TOOLS[1:]:
            for gate, rk, k in zip(GATES, ref_keys, keys):
                a = (verdict(before, ref_tool, rk), verdict(after, ref_tool, rk))
                b = (verdict(before, tool, k), verdict(after, tool, k))
                if None not in b and a != b and gate not in dis:
                    dis.append(gate)
        disagreeing += bool(dis)
        out.append(f"{p['label']:<14} | " + " | ".join(cells) + " | " +
                   (",".join(dis) if dis else "-"))

    out.append("")
    out.append(f"plates where a measurer disagrees with {ref_tool}: {disagreeing}/{len(pairs)}")
    for tool, _keys, _h in TOOLS:
        b, a, n = totals[tool]
        arrow = "up" if a > b else ("down" if a < b else "flat")
        out.append(f"  {tool:<14} gates PASS all plates: {b}/{n} -> {a}/{n}  ({arrow})")

    txt = "\n".join(out) + "\n"
    print(txt, end="")
    if len(sys.argv) == 3:
        Path(sys.argv[2]).write_text(txt, encoding="utf-8")
        print(f"[written] {sys.argv[2]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
