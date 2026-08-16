"""Self-test for _rose_ai_arch_pairs.py (Rose, 2026-08-16).

Builds synthetic traces + dummy frame PNGs in a temp dir, runs the picker on
a positive control (all 3 archetypes commit) and a negative control (pouncer
never pounces, bruiser never strikes) and checks the exit codes + outputs.
Run: python scripts/_rose_ai_arch_pairs_selftest.py
"""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PICKER = os.path.join(HERE, "_rose_ai_arch_pairs.py")


def write_trace(path):
    rows = ["frame,enemy,state,archetype,x,z,dist"]
    for f in range(2, 10):
        rows.append(f"{f},1,patrol,swarm,0,0,18.0")
    for f in range(10, 14):
        rows.append(f"{f},1,alert,swarm,0,0,15.0")
    for f in range(14, 30):
        rows.append(f"{f},1,chase,swarm,0,0,{max(8.0, 15.0 - (f - 14)):.2f}")
    for f in range(2, 20):
        rows.append(f"{f},3,patrol,bruiser,0,0,20.0")
    for f in range(20, 30):
        rows.append(f"{f},3,advance,bruiser,0,0,12.0")
    for f in range(30, 34):
        rows.append(f"{f},3,windup,bruiser,0,0,3.0")
    for f in range(34, 40):
        rows.append(f"{f},3,strike,bruiser,0,0,1.5")
    for f in range(2, 40):
        rows.append(f"{f},4,patrol,pouncer,0,0,22.0")
    for f in range(40, 50):
        rows.append(f"{f},4,stalk,pouncer,0,0,7.0")
    for f in range(50, 53):
        rows.append(f"{f},4,crouch,pouncer,0,0,5.0")
    for f in range(53, 60):
        rows.append(f"{f},4,pounce,pouncer,0,0,2.0")
    open(path, "w").write("\n".join(rows) + "\n")
    # negative: pouncer never pounces, bruiser never strikes (chop the tails)
    neg = [
        r for r in rows
        if not (r.split(",")[1] == "4" and int(r.split(",")[0]) >= 50)
        and not (r.split(",")[1] == "3" and int(r.split(",")[0]) >= 30)
    ]
    open(path.replace("pos", "neg"), "w").write("\n".join(neg) + "\n")


def main():
    tmp = tempfile.mkdtemp(prefix="ai_arch_selftest_")
    frames = os.path.join(tmp, "frames")
    out = os.path.join(tmp, "out")
    os.makedirs(frames)
    pos = os.path.join(tmp, "trace_pos.csv")
    neg = os.path.join(tmp, "trace_neg.csv")
    write_trace(pos)
    for f in range(2, 62, 2):  # capture cadence: even frames only, size>0
        open(os.path.join(frames, "f%04d.png" % f), "wb").write(b"x")

    fails = []

    r = subprocess.run([sys.executable, PICKER, pos, frames, out],
                       capture_output=True, text=True)
    print("--- POSITIVE ---")
    print(r.stdout)
    want = ["ai-arch-swarm-a-before.png", "ai-arch-swarm-b-after.png",
            "ai-arch-bruiser-a-before.png", "ai-arch-bruiser-b-after.png",
            "ai-arch-pouncer-a-before.png", "ai-arch-pouncer-b-after.png"]
    if r.returncode != 0:
        fails.append(f"positive control exit={r.returncode} (want 0)")
    for w in want:
        if not os.path.exists(os.path.join(out, w)):
            fails.append(f"positive control missing output {w}")
    # swarm 'a' must be a pre-chase row (alert f10..13), 'b' a chase row (f>=14)
    if "enemy=1 alert" not in r.stdout or "enemy=1 chase" not in r.stdout:
        fails.append("positive: swarm pair rows not (alert -> chase)")

    r2 = subprocess.run([sys.executable, PICKER, neg, frames, out],
                        capture_output=True, text=True)
    print("--- NEGATIVE ---")
    print(r2.stdout)
    if r2.returncode == 0:
        fails.append(f"negative control exit=0 (want !=0)")
    if "MISSING bruiser" not in r2.stdout or "MISSING pouncer" not in r2.stdout:
        fails.append("negative: bruiser/pouncer not reported MISSING")
    if "MISSING swarm" in r2.stdout:
        fails.append("negative: swarm wrongly MISSING")

    if fails:
        print("SELFTEST FAIL:")
        for f in fails:
            print(" -", f)
        sys.exit(1)
    print("SELFTEST PASS (positive 3 pairs, negative flags bruiser+pouncer)")
    print("tmpdir:", tmp)


if __name__ == "__main__":
    main()
