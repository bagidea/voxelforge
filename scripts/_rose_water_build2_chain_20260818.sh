#!/usr/bin/env bash
# Rose — clean single-bin rebuild after the 15:27 OOM massacre.
#
# Why single-bin: the 15:27 command built EVERY bin of the package at once
# (9x fat-LTO codegen-units=1 links in parallel) while Poppy's lane was also
# linking -> rustc-LLVM OOM, 9 bins died, log has 9 '^error' lines = unjudgeable.
# We only ever need ONE artifact: target-rose/release/voxelforge.exe.
#
# Chain: (a) wait for the dying 15:27 cargo (pid 4880) to fully exit so we
# never stack a 2nd build in OUR lane; (b) build --bin voxelforge only;
# (c) print the Director's gate: '^error' count, Finished line, exe mtime.
set -u
cd "$(dirname "$0")/.."

stamp() { date "+[%H:%M:%S]"; }

echo "$(stamp) waiting for old cargo pid 4880 to exit..."
i=0
while powershell -NoProfile -Command "exit [int](-not (Get-Process -Id 4880 -ErrorAction SilentlyContinue))" 2>/dev/null; do
  sleep 10
  i=$((i+1))
  if [ $((i % 6)) -eq 0 ]; then
    echo "$(stamp) still waiting for 4880 ($((i*10))s) — remaining rustc:"
    tasklist //NH //FI "IMAGENAME eq rustc.exe" 2>/dev/null | grep -c rustc || true
  fi
  if [ $i -gt 360 ]; then
    echo "$(stamp) 4880 refused to die for 1h — aborting chain"; exit 4
  fi
done
echo "$(stamp) lane free (4880 gone)"

echo "$(stamp) BUILD2_START single-bin"
cargo build --release -p voxelforge --bin voxelforge --target-dir target-rose \
  > _rose_water_build2.log 2>&1
rc=$?
echo "$(stamp) BUILD2_EXIT=$rc"

echo "--- gate: error count ---"
grep -c '^error' _rose_water_build2.log || true
echo "--- gate: Finished line ---"
grep -n '^ *Finished' _rose_water_build2.log || echo "NO FINISHED LINE"
echo "--- gate: exe ---"
if [ -f target-rose/release/voxelforge.exe ]; then
  ls -la --time-style=+'%Y-%m-%d %H:%M:%S' target-rose/release/voxelforge.exe
else
  echo "NO EXE"
fi
echo "$(stamp) chain done"
