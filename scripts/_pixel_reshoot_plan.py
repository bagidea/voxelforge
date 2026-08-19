#!/usr/bin/env python3
"""Build the re-shoot plan for the 64 beauty plates `git clean -fd` destroyed.

THE POINT OF THIS FILE IS THAT IT INVENTS NOTHING.

The plates are gone, but the two run logs that produced them survived the clean
(`.gitignore:26` makes `*.log` ignored, and `git clean -fd` skips ignored files).
Each log carries one line per plate with that plate's exact camera and exact env:

    [shoot] p1-east-level-fog8-45  cine=52,9,48, 52,9,48, 24,9,18, 1  VOXELFORGE_LOOK_FOG=8,45
    [probe] p4-zaxis-level  cine=32,7,58, 32,7,58, 32,9,6, 1

So the plan is PARSED out of those lines rather than re-derived from the
scripts.  That matters twice over:

  * `scripts/_flamingo_beauty_ladder.ps1` — recovered from a transcript — defines
    only 11 rungs, but the manifest names 13 plates per pose.  `fill-up` and
    `fill-down` were added in a third pass whose edit was never captured.  The
    log has all 52; the script has 44.  Reading the script would have silently
    lost 8 plates and no count would have complained.
  * a rung's env is then quoted, not reconstructed, so a typo in a knob value
    cannot pass for a re-shoot.

Pinned env (identical on every plate, from `_flamingo_beauty_ladder.ps1:43-47`
and `_flamingo_beauty_probe.cmd:32-36`) is attached to every entry here so the
shooter has no defaults of its own to get wrong.

Usage:  python scripts/_pixel_reshoot_plan.py [--out _pixel_reshoot/_plan.json]
"""
import argparse
import json
import re
from pathlib import Path

BEAUTY = Path("docs/assets/look/beauty")
LADDER_LOG = BEAUTY / "ladder" / "_ladder.log"
PROBE_LOG = BEAUTY / "probe" / "_probe.log"

# Pinned on every plate in both passes.
PINNED = {
    "VOXELFORGE_PLAY": "1",
    "VOXELFORGE_NOHUD": "1",
    "VOXELFORGE_LOOK_QUALITY": "ultra",
    "VOXELFORGE_CINE_START": "1.0",
    "VOXELFORGE_LOOK_GEN": "v3",
}

# Every knob any rung touches -- cleared before each shot so a rung can never
# inherit the previous rung's env.  Superset of ladder.ps1's list: _LOOK_FILL is
# in the log (fill-up/fill-down) but not in the recovered script.
KNOBS = [
    "VOXELFORGE_LOOK_FOG",
    "VOXELFORGE_LOOK_AMBIENT",
    "VOXELFORGE_LOOK_EXPOSURE",
    "VOXELFORGE_LOOK_ATMOS",
    "VOXELFORGE_LOOK_FILL",
]


def read_log(path):
    """These logs are a PowerShell/cmd append mix: UTF-8 text with stray BOMs and
    NUL runs from the redirected exe output.  Decode permissively and keep only
    the marker lines."""
    raw = path.read_bytes().decode("utf-8", errors="replace")
    return [l.replace("﻿", "").replace("\x00", "").strip()
            for l in raw.splitlines()]


def parse(path, marker):
    """`[marker] NAME  cine=...  [ENV...]` -> (name, cine, {env}).

    Split on the DOUBLE space: the cine value itself contains ', ' single-space
    separators, so a naive split on whitespace mangles every camera.
    """
    out = []
    for line in read_log(path):
        if not line.startswith(f"[{marker}]"):
            continue
        body = line[len(marker) + 2:].strip()
        fields = [f for f in re.split(r"\s{2,}", body) if f]
        name, cine_f = fields[0], fields[1]
        assert cine_f.startswith("cine="), f"no cine in: {line}"
        env = {}
        for extra in fields[2:]:
            if extra == "(no overrides)":
                continue
            for kv in extra.split():
                k, _, v = kv.partition("=")
                env[k] = v
        out.append(dict(name=name, cine=cine_f[len("cine="):].strip(), env=env))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="_pixel_reshoot/_plan.json")
    a = ap.parse_args()

    ladder = parse(LADDER_LOG, "shoot")
    probe = parse(PROBE_LOG, "probe")

    plan = []
    for e in probe:
        plan.append(dict(e, dest="docs/assets/look/beauty/probe", pass_="probe"))
    for e in ladder:
        plan.append(dict(e, dest="docs/assets/look/beauty/ladder", pass_="ladder"))

    names = [e["name"] for e in plan]
    assert len(names) == len(set(names)), "duplicate plate name in plan"

    doc = dict(pinned=PINNED, knobs=KNOBS, plates=plan)
    p = Path(a.out)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(doc, indent=1), encoding="utf-8")

    print(f"probe  {len(probe):>3} plates   from {PROBE_LOG}")
    print(f"ladder {len(ladder):>3} plates   from {LADDER_LOG}")
    print(f"total  {len(plan):>3} plates -> {p}")
    rungs = sorted({n.rsplit("-", 1)[-1] for n in names})
    print(f"distinct env sets: {len({json.dumps(e['env'], sort_keys=True) for e in plan})}")
    for e in plan:
        print(f"  {e['pass_']:<6} {e['name']:<38} {e['env'] or '(no overrides)'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
