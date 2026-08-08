#!/usr/bin/env bash
# G7b — build, PROVE the exe is the one that was just built, then shoot the table.
#
# Detached and self-contained on purpose: a session that ends mid-link must not
# orphan the shoot, and a shoot must never run against a stale exe. The build is
# judged by `grep -c '^error'` plus the `Finished` line and NOTHING else --
# cargo prints `Compiling ...` after errors, and piping its output through `tail`
# has already produced a false "build green" on this box once (v0.9.45), so the
# exit code is captured directly, not through a pipe.
set -u
cd "$(dirname "$0")/.."

OUT=_flamingo_g7b
LOG=$OUT/build.log
EXE=target-flamingo/perf/voxelforge.exe
mkdir -p "$OUT"
: > "$LOG"

before_mtime=$(stat -c %Y "$EXE" 2>/dev/null || echo 0)
before_size=$(stat -c %s "$EXE" 2>/dev/null || echo 0)
echo "[$(date +%T)] exe before: mtime=$before_mtime size=$before_size" >> "$LOG"

# Same ceiling rule as the G7 build: two other cargo invocations (four cargo.exe,
# shim + toolchain each) is the documented max before STATUS_DLL_INIT_FAILED.
for _ in $(seq 1 240); do
  n=$(tasklist 2>/dev/null | grep -c 'cargo.exe' || true)
  [ "${n:-0}" -le 4 ] && { echo "[$(date +%T)] slot free (cargo procs=$n)" >> "$LOG"; break; }
  echo "[$(date +%T)] waiting: cargo procs=$n" >> "$LOG"
  sleep 20
done

echo "[$(date +%T)] cargo build --profile perf --bin voxelforge --target-dir target-flamingo -j 4" >> "$LOG"
cargo build --profile perf --bin voxelforge --target-dir target-flamingo -j 4 >> "$LOG" 2>&1
rc=$?
echo "EXIT=$rc $(date +%T)" >> "$LOG"

errs=$(grep -c '^error' "$LOG")
fin=$(grep -c '^ *Finished' "$LOG")
after_mtime=$(stat -c %Y "$EXE" 2>/dev/null || echo 0)
after_size=$(stat -c %s "$EXE" 2>/dev/null || echo 0)
echo "[$(date +%T)] verdict: rc=$rc errors=$errs finished=$fin exe mtime $before_mtime->$after_mtime size $before_size->$after_size" >> "$LOG"

if [ "$rc" -ne 0 ] || [ "$errs" -ne 0 ] || [ "$fin" -eq 0 ]; then
  echo "BUILD FAILED — not shooting" >> "$LOG"; exit 1
fi
if [ "$after_mtime" -le "$before_mtime" ]; then
  echo "EXE NOT RELINKED (mtime did not advance) — not shooting" >> "$LOG"; exit 2
fi

echo "[$(date +%T)] shooting $OUT/plan.txt" >> "$LOG"
powershell.exe -NonInteractive -ExecutionPolicy Bypass \
  -File scripts/_flamingo_g7_sweep.ps1 "$OUT\\plan.txt" "$OUT" >> "$OUT/sweep.run.log" 2>&1
echo "SWEEP EXIT=$? $(date +%T)" >> "$LOG"

# A2 is an A/B axis: grade every row against this build's OWN zero-fog frame.
for f in "$OUT"/*-nohud2.png; do
  b=$(basename "$f" -nohud2.png)
  [ "$b" = "hazeoff" ] && continue
  {
    echo "===== $b ====="
    python scripts/grade_g7.py --ab-haze "$OUT/hazeoff-nohud2.png" "$f"
  } >> "$OUT/a2.log" 2>&1
done
echo "=== CHAIN DONE $(date +%T) ===" >> "$LOG"
