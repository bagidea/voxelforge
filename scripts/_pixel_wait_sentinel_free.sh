#!/usr/bin/env bash
# Wait until Build Sentinel has no LIVE build (Kevin's run must finish on its own).
# Prints one line per poll so the tail is readable, exits 0 the moment it is free.
API=http://127.0.0.1:8787/plugin/build-sentinel/cmd
for i in $(seq 1 240); do          # 240 * 30s = 2h ceiling
  s=$(curl -s -X POST "$API" -H "content-type: application/json" -d '{"cmd":"status"}')
  st=$(printf '%s' "$s" | sed -n 's/.*"status":"\([a-z]*\)".*/\1/p')
  el=$(printf '%s' "$s" | sed -n 's/.*"elapsedMs":\([0-9]*\).*/\1/p')
  echo "[$i] status=$st elapsedMs=${el:-null}"
  if [ "$st" != "running" ]; then
    echo "FREE: sentinel has no live build (status=$st)"
    printf '%s\n' "$s"
    exit 0
  fi
  sleep 30
done
echo "TIMEOUT: still running after 2h"
exit 1
