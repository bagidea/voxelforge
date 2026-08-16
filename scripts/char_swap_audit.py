#!/usr/bin/env python3
"""Prove the swap plates came off ONE body, from the render logs alone.

A three-frame "costume change" is only interesting if it is one entity being
re-dressed. Three separate spawns photographed once each would look identical on
the contact sheet. This reads the stage stdout and checks:

  1. every SWAP_CAPTURE line carries the SAME root entity id
  2. that id also matches the SWAP_ROOT line printed once at startup
  3. the wearing[...] string CHANGES between consecutive captures
  4. for the weapon stage, EQUIP touched the weapon slot and ONLY the weapon slot

Exit: 0 = proven   1 = measured and failed   2 = refused (no usable log).
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

CAP = re.compile(r"SWAP_CAPTURE (\d+)/(\d+) root=(\S+) wearing\[(.*?)\] -> (.*)")
ROOTLINE = re.compile(r"SWAP_ROOT root=(\S+)")
APPLY = re.compile(r"SWAP_APPLY (\w+)=(\S+) on root=(\S+)")
EQUIP = re.compile(r"EQUIP root=(\S+) slot=(\w+) (\S+) -> (\S+) boxes=(\d+)")


def audit(stage, weapon_only=False):
    log = ROOT / f"_fl_stage_{stage}.log"
    if not log.exists():
        print(f"REFUSE [{stage}]: no log at {log}")
        return 2
    text = log.read_text(errors="replace")

    caps = CAP.findall(text)
    roots = ROOTLINE.findall(text)
    equips = EQUIP.findall(text)

    if not caps:
        print(f"REFUSE [{stage}]: log has no SWAP_CAPTURE lines")
        return 2

    print(f"-- {stage}: {len(caps)} captures, {len(equips)} equips --")
    bad = 0

    # 1 + 2. one root, everywhere
    ids = {c[2] for c in caps} | set(roots)
    if len(ids) == 1:
        print(f"  ONE_ROOT root={ids.pop()} across {len(caps)} captures + startup  PASS")
    else:
        print(f"  ONE_ROOT FAIL - {len(ids)} distinct root ids seen: {sorted(ids)}")
        bad += 1

    # 3. the loadout actually changed between captures
    worn = [c[3] for c in caps]
    for i in range(len(worn) - 1):
        if worn[i] != worn[i + 1]:
            print(f"  CHANGED {i+1}->{i+2}  PASS")
        else:
            print(f"  CHANGED {i+1}->{i+2}  FAIL - identical loadout '{worn[i]}'")
            bad += 1
    for i, w in enumerate(worn):
        print(f"    capture {i+1} wearing: {w}")

    # 4. weapon stage must not touch any other slot
    if weapon_only:
        after_first = text.split("SWAP_CAPTURE 1/")[-1]
        slots = {m[1] for m in EQUIP.findall(after_first)}
        if slots and slots == {"weapon"}:
            print(f"  WEAPON_ONLY PASS - slots touched after capture 1: {sorted(slots)}")
        else:
            print(f"  WEAPON_ONLY FAIL - slots touched: {sorted(slots) or 'none'}")
            bad += 1

    return 1 if bad else 0


def main():
    rc = 0
    rc = max(rc, audit("swap"))
    rc = max(rc, audit("weapons", weapon_only=True))
    print(f"\nAUDIT rc={rc}")
    return rc


if __name__ == "__main__":
    sys.exit(main())
