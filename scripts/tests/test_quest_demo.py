"""Quest demo proof harness — Sun's lane.

Runs ``voxelforge.exe --quest-demo`` and proves every PASS line was
emitted BY THE GAME, not by this script.  The harness prints nothing
that looks like a PASS itself — it only greps, counts, and gates.

Gate: all 13 PASS lines must appear in the game output AND the final gate marker must be present.  If either is missing, exit 1.

Self-audit: this script's own source MUST NOT contain any line that
matches the PASS patterns it searches for (excluding the pattern
definitions themselves).  If it did, we'd be cheating.
"""

import subprocess, sys, os, time, re
from pathlib import Path

# --- Self-audit: this script must not emit fake PASS lines ---
_SELF = Path(__file__).read_text(encoding="utf-8")
# Check print() calls only — constants and comments can name PASS patterns.
_FAKE = False
for line in _SELF.split("\n"):
    s = line.strip()
    if s.startswith("print(") and ("=> PASS" in s or "=> PASS" in s.replace(" ", "")):
        _FAKE = True
if _FAKE:
    print("SELF_AUDIT FAIL: harness print() contains PASS — would fake results")
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[2]
BINARY = os.environ.get("VOXELFORGE_BIN", str(ROOT / "target-quest" / "debug" / "voxelforge.exe"))

# --- PASS lines the GAME must emit (not us) ---
# Each regex matches exactly one line the quest engine writes.
PASS_PATTERNS = [
    r"QUEST_STAGE_COMPLETE qid=q1_embers oid=o1_campfire => PASS",
    r"QUEST_COMPLETE id=q1_embers => PASS",
    r"QUEST_ACCEPT id=q2_voice_in_stone => PASS",
    r"QUEST_NEXT_OPEN next=q2_voice_in_stone => PASS",
    r"QUEST_STAGE_COMPLETE qid=q2_voice_in_stone oid=o1_gate => PASS \(reach_zone\)",
    r"QUEST_STAGE_COMPLETE qid=q2_voice_in_stone oid=o2_listen => PASS",
    r"QUEST_COMPLETE id=q2_voice_in_stone => PASS",
    r"QUEST_ACCEPT id=q3_gatekeeper => PASS",
    r"QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS \(reach_zone\)",
    r"QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o2_observe => PASS \(approach",
    r"QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS \(defeat\)",
    r"QUEST_COMPLETE id=q3_gatekeeper => PASS",
    r"QUEST_PROOF.*ALL GATES PASS => PASS",
]

# --- Gates (all must appear in output) ---
FINAL_GATE = "ACT1_COMPLETE"

def main():
    if not os.path.isfile(BINARY):
        print(f"SKIP: binary not found at {BINARY} — build it first")
        sys.exit(0)

    print(f"QUEST_DEMO_PROOF binary={BINARY}")
    t0 = time.time()

    # Run headless — stdout/stderr captured, no window.
    env = os.environ.copy()
    env.setdefault("VOXELFORGE_QUEST_DEMO", "1")

    proc = subprocess.Popen(
        [BINARY, "--quest-demo"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=env,
    )

    # Read line by line so we don't buffer 90s of walk debug.
    lines = []
    try:
        for line in proc.stdout:
            lines.append(line.rstrip("\n"))
    except Exception:
        pass
    proc.wait(timeout=10)

    elapsed = time.time() - t0
    output = "\n".join(lines)

    # --- Prove PASS lines come FROM THE GAME ---
    # We never print a PASS ourselves — we only count and match.
    missing = []
    for pat in PASS_PATTERNS:
        if not re.search(pat, output):
            missing.append(pat)

    has_final = FINAL_GATE in output

    # --- Report ---
    print(f"QUEST_DEMO_PROOF elapsed={elapsed:.1f}s exit={proc.returncode}")
    matched = len(PASS_PATTERNS) - len(missing)
    print(f"QUEST_DEMO_PROOF pass_lines: {matched}/{len(PASS_PATTERNS)} matched")
    if missing:
        print(f"QUEST_DEMO_PROOF MISSING ({len(missing)}):")
        for m in missing:
            print(f"  ✗ {m}")
    if has_final:
        print(f"QUEST_DEMO_PROOF final_gate: {FINAL_GATE} ✓")
    else:
        print(f"QUEST_DEMO_PROOF final_gate: {FINAL_GATE} ✗ NOT FOUND")

    # Write the full log for human inspection.
    log_path = ROOT / "_quest_proof" / "quest-demo-harness.log"
    log_path.parent.mkdir(exist_ok=True)
    log_path.write_text(output, encoding="utf-8")
    print(f"QUEST_DEMO_PROOF log: {log_path}")

    if missing or not has_final:
        print("QUEST_DEMO_VERDICT: insufficient evidence")
        sys.exit(1)
    # Harness never prints "PASS" — the game owns every PASS line.
    # This line is a meta-verdict, not a pass claim.
    print("QUEST_DEMO_VERDICT: all game-emitted proof lines confirmed")


if __name__ == "__main__":
    main()
