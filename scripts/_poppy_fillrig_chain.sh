#!/usr/bin/env bash
# _poppy_fillrig_chain.sh — wait for the retry build, then shoot the A/B, then measure.
# One process so the whole proof either lands or fails loudly; no polling by hand.
#
# 60-MINUTE CAP, not 25. The first cap was set against a link on an idle box; this
# box has run 5 concurrent cargo lanes all afternoon and my own build is deliberately
# -j 2 / BelowNormal to stay out of their way. A cap that expires before the link is
# a self-inflicted "no evidence".
set -uo pipefail
cd "$(dirname "$0")/.."

DONE=_poppy_fillrig_build.done
LOG=_poppy_fillrig_build.log
BIN=./target-poppy/perf/voxelforge.exe

for i in $(seq 1 720); do
  [ -f "$DONE" ] && break
  # A heartbeat every 5 min, so a stall is visible as a stall and not as silence.
  if [ $((i % 60)) = 0 ]; then
    echo "CHAIN: still waiting ($((i * 5 / 60)) min) — log $(wc -c <"$LOG" 2>/dev/null || echo 0) bytes"
  fi
  sleep 5
done

if [ ! -f "$DONE" ]; then echo "CHAIN: build never finished (no $DONE after 60m)"; exit 3; fi
echo "CHAIN: build $(tr -d '\r' <"$DONE")"
# Judge by errors, never by tail — the scar is on file. And by EXIT=, because a
# 0xc0000142 DLL-init death prints no `^error` line at all.
errs=$(grep -c '^error' "$LOG" || true)
echo "CHAIN: grep '^error' = $errs"
if [ "$errs" != "0" ]; then grep -A6 '^error' "$LOG" | head -40; exit 4; fi
case "$(tr -d '\r' <"$DONE")" in
  EXIT=0*) ;;
  *) echo "CHAIN: build exit was not 0 — refusing to shoot"; exit 4;;
esac
[ -s "$BIN" ] || { echo "CHAIN: no exe at $BIN"; exit 5; }

bash scripts/_poppy_fillrig_shoot.sh; shot_rc=$?
echo "CHAIN: shoot rc=$shot_rc  (gate failures are non-zero)"
python scripts/_poppy_fillrig_sheet.py; sheet_rc=$?
echo "CHAIN: sheet rc=$sheet_rc  (gate failures are non-zero)"
exit $(( shot_rc != 0 ? shot_rc : sheet_rc ))
