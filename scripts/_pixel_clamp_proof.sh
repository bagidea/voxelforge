#!/usr/bin/env bash
# Step 3 proof: fire a -j 4 build at the LIVE plugin and prove two things —
#   (a) it is rejected, and
#   (b) nothing was spawned: no new cargo/rustc process, no new history row,
#       and the sentinel did not go `running`.
API=http://127.0.0.1:8787/plugin/build-sentinel/cmd
HERE="$(cd "$(dirname "$0")" && pwd)"

procs() {
  powershell.exe -NoProfile -Command \
    '$p = @(Get-Process cargo,rustc -ErrorAction SilentlyContinue); Write-Output $p.Count' 2>/dev/null | tr -d '\r'
}
ask() { curl -s -X POST "$API" -H "content-type: application/json" -d "{\"cmd\":\"$1\"}"; }

echo "=== BEFORE ==="
B_PROCS=$(procs);            echo "cargo+rustc processes : $B_PROCS"
B_STATUS=$(ask status);      echo "status                : $B_STATUS"
B_HIST=$(ask history);       B_TOP=$(printf '%s' "$B_HIST" | sed -n 's/.*"runs":\[{"id":\([0-9]*\).*/\1/p')
echo "newest history id     : ${B_TOP:-none}"

echo
echo "=== FIRE: cargo build ... -j 4 ==="
RESP=$(curl -s -X POST "$API" -H "content-type: application/json" \
  --data-binary @"$HERE/_pixel_clamp_reject_probe.json")
echo "$RESP"

echo
echo "=== AFTER (immediately) ==="
A_PROCS=$(procs);            echo "cargo+rustc processes : $A_PROCS"
A_STATUS=$(ask status);      echo "status                : $A_STATUS"
A_HIST=$(ask history);       A_TOP=$(printf '%s' "$A_HIST" | sed -n 's/.*"runs":\[{"id":\([0-9]*\).*/\1/p')
echo "newest history id     : ${A_TOP:-none}"

echo
echo "=== VERDICT ==="
ok=0
case "$RESP" in *'"ok":false'*) echo "PASS  rejected (ok:false)";; *) echo "FAIL  not rejected"; ok=1;; esac
case "$RESP" in *'"rejected":"jobs-clamp"'*) echo "PASS  rejected BY the jobs clamp";; *) echo "FAIL  rejected by something else"; ok=1;; esac
case "$A_STATUS" in *'"status":"running"'*) echo "FAIL  sentinel went running"; ok=1;; *) echo "PASS  sentinel did not go running";; esac
if [ "$B_PROCS" = "$A_PROCS" ]; then echo "PASS  no new cargo/rustc process ($B_PROCS -> $A_PROCS)"; else echo "FAIL  process count moved ($B_PROCS -> $A_PROCS)"; ok=1; fi
if [ "${B_TOP:-none}" = "${A_TOP:-none}" ]; then echo "PASS  no new history row (${B_TOP:-none})"; else echo "FAIL  a run was recorded (${B_TOP:-none} -> ${A_TOP:-none})"; ok=1; fi
echo
[ $ok -eq 0 ] && echo "ALL PASS — the clamp rejects -j 4 without spawning anything." || echo "SOMETHING FAILED — do not trust the clamp."
exit $ok
