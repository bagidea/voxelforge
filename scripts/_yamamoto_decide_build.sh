#!/usr/bin/env bash
# Decide PASS/FAIL for a cargo build log by counting compiler error lines only.
# Usage: _yamamoto_decide_build.sh <logfile>
# Never trust tail or a piped $? (PIPESTATUS scar) -- grep -c is the sole gate.
set -u
log="${1:?usage: _yamamoto_decide_build.sh <logfile>}"
if [ ! -f "$log" ]; then
    echo "DECIDE FAIL log_missing=$log"
    exit 1
fi
errs=$(grep -c '^error' "$log")
if [ "$errs" -eq 0 ]; then
    echo "DECIDE PASS errors=0 log=$log"
    exit 0
else
    echo "DECIDE FAIL errors=$errs log=$log"
    exit 1
fi
