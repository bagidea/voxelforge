#!/usr/bin/env bash
# Proof that the audio lane's evidence-logging pass actually works: real
# `audio.rs` systems, driven by real transform movement inside the isolated
# `voxelforge_audio_proof` bin, print real `AUDIO_PLAY:` / `AUDIO_ZONE:` /
# `AUDIO_SUMMARY:` lines carrying a real `t=<elapsed>` timestamp and a real
# `pos=(x,y,z)` — not hardcoded, not printed by this script.
#
# Same rule as prove_dodge_parry.sh: every PASS below is graded off a line
# `audio.rs` itself printed. This script never computes a result and prints
# PASS for its own arithmetic — it only greps/asserts on the shipping
# system's own stdout.
#
# ── the lines it grades (all from client/src/audio.rs, unedited) ──────────
#   AUDIO_PLAY:<path> t=<secs> pos=(x,y,z)      — spawn_sfx_layer / spawn_ambient / spawn_music
#   AUDIO_ZONE:<Zone> t=<secs> pos=(x,y,z)       — track_ambient_zone, on change only
#   AUDIO_SUMMARY:total=<n> | <path>=<count> ..  — dump_sfx_summary
#
# ── what this bin's driver (audio_proof_main.rs) is honest about ─────────
# Footsteps, ambient spawn/despawn, zone-crossfade, music spawn and the
# scripted walk are REAL audio.rs systems reacting to REAL transform
# movement. The combat SfxEvents are fired by a scripted MessageWriter (the
# driver's file header says so) because combat.rs cannot compile right now
# (docs/LANES.md) — gate 5 below grades that they play back for real once
# fired, it does NOT claim combat.rs itself is wired up. Do not read a PASS
# here as "combat → audio integration proven end to end".
#
# ── why the gate is shaped this way ────────────────────────────────────────
#   G1 stale-binary guard — this office has shipped a false-green build
#      before by grading an exe older than its own source (docs scars).
#   G3/G4 the zone lines carry pos= INSIDE the region the walk actually
#      targeted, and t= lands inside the leg's own time window — a zone
#      print that fired at the wrong place/time is not evidence of real
#      movement tracking, it's a lucky string match.
#   G6 the mixer live-push (`update_music_volume` / `crossfade_ambient`)
#      has NO println in audio.rs today — only the driver narrates
#      before/after. Per the rule above (driver output does not count as
#      system evidence), this script does NOT grade a PASS for it; it says
#      so explicitly instead of silently skipping the claim.
#
# ── usage ──────────────────────────────────────────────────────────────────
#   BIN=./target-yama/debug/voxelforge_audio_proof.exe bash scripts/prove_audio.sh
#   bash scripts/prove_audio.sh --log <file>     # grade an existing log
set -uo pipefail

BIN=${BIN:-./target-yama/debug/voxelforge_audio_proof.exe}
LOGS=_audio_proof_gate
RUN_SECS=${RUN_SECS:-60}
mkdir -p "$LOGS"

stale_guard() {
  [ -f "$BIN" ] || { echo "STALE_BINARY_GUARD: $BIN does not exist => FAIL"; exit 1; }
  newer=$(find client/src -name '*.rs' -newer "$BIN" 2>/dev/null | head -5)
  if [ -n "$newer" ]; then
    echo "STALE_BINARY_GUARD: $BIN is older than these sources => FAIL"
    echo "$newer" | sed 's/^/  /'
    echo "rebuild first (CARGO_TARGET_DIR=target-yama cargo build --bin voxelforge_audio_proof), or pass BIN=<fresh binary>"
    exit 1
  fi
  echo "STALE_BINARY_GUARD: $BIN newer than all client/src/*.rs => PASS"
}

if [ "${1:-}" = "--log" ]; then
  log="${2:?--log requires a log file path}"
  echo "=== AUDIO PROOF ==="
  echo "log: $log (pre-existing — skipping run)"
  rc=$(grep -a '^exit=' "$log" | tail -1 | sed 's/^exit=//')
  rc=${rc:-0}
else
  echo "=== AUDIO PROOF ==="
  echo "binary: $BIN"
  stale_guard
  log="$LOGS/audio_proof.log"
  echo "=== run voxelforge_audio_proof (timeout ${RUN_SECS}s) ==="
  timeout "$RUN_SECS" "$BIN" >"$log" 2>&1
  rc=$?
  echo "exit=$rc" >>"$log"
fi

echo "exit=$rc"
echo ""
echo "--- real audio.rs evidence lines from the run (first 20) ---"
grep -a -E '^AUDIO_(PLAY|ZONE|SUMMARY):' "$log" | head -20 || true
echo ""

FAILS=0
fail() { echo "  => FAIL [$1] $2"; FAILS=$((FAILS + 1)); }
pass() { echo "  + $1"; }
note() { echo "  ~ NOTE [$1] $2"; }
field() { sed -nE "s/.*[[:space:]]$2=([^[:space:]]+).*/\1/p" <<<"$1"; }

# ── gate 0: process health ────────────────────────────────────────────────
[ "$rc" -eq 0 ] || fail EXIT "nonzero exit code: $rc (expected 0)"
if grep -aqE 'panicked|B0001' "$log"; then
  fail PANIC "panic / query conflict in the log"
  grep -anE 'panicked|B0001' "$log" | head -3
fi
if grep -aq 'AUDIO_PROOF_DONE' "$log"; then
  pass "driver ran to completion (AUDIO_PROOF_DONE)"
else
  fail RUN_DONE "the run never reached AUDIO_PROOF_DONE — cut short"
fi

# ── gate 1: startup sounds are REAL AUDIO_PLAY: lines, not bare paths ──────
# A stale pre-instrumentation binary prints "AUDIO_PLAY:audio/x.wav" with no
# t=/pos= — that shape must FAIL here, not silently pass a substring grep.
STARTUP_SOUNDS="ambient_wind.wav ambient_campfire.wav ambient_village.wav music_theme.wav"
BAD=0
for snd in $STARTUP_SOUNDS; do
  line=$(grep -a "^AUDIO_PLAY:audio/$snd " "$log" | head -1)
  if [ -z "$line" ]; then
    echo "  => FAIL [STARTUP_PLAY] audio/$snd never printed AUDIO_PLAY: with a t=/pos= field"
    BAD=1
    continue
  fi
  t=$(field "$line" t)
  if [ -z "$t" ]; then
    echo "  => FAIL [STARTUP_EVIDENCE] audio/$snd printed AUDIO_PLAY: with no t= — evidence is a bare path"
    BAD=1
  fi
done
[ "$BAD" -eq 0 ] && pass "all 4 startup sounds carry real t=/pos= evidence" || FAILS=$((FAILS + 1))

# ── gate 2: footsteps fired from REAL movement during the walk window ─────
N_STEP=$(grep -ac '^AUDIO_PLAY:audio/footstep_' "$log" || true)
if [ "$N_STEP" -lt 1 ]; then
  fail FOOTSTEPS "zero footstep_*.wav plays — footstep_tracker never fired from the scripted walk"
else
  BAD=0
  while IFS= read -r line; do
    t=$(field "$line" t)
    if [ -z "$t" ]; then
      echo "  => FAIL [FOOTSTEP_EVIDENCE] $line — no t="; BAD=1; continue
    fi
    # walk legs run 0..8.5s (see audio_proof_main.rs ProofScript); a footstep
    # outside that window did not come from the scripted movement.
    if ! awk -v t="$t" 'BEGIN{exit (t>=0 && t<=8.6)?0:1}'; then
      echo "  => FAIL [FOOTSTEP_WINDOW] footstep at t=$t is outside the 0..8.5s walk window"; BAD=1
    fi
  done < <(grep -a '^AUDIO_PLAY:audio/footstep_' "$log")
  [ "$BAD" -eq 0 ] && pass "AUDIO_PLAY:footstep_* x$N_STEP — real movement, all inside the walk window" \
                   || FAILS=$((FAILS + 1))
fi

# ── gate 3: zone crossing — real track_ambient_zone reacting to real pos ──
# Leg 1 targets Vec3::ZERO (village_road, -12..12) but its hold_until=4.5 is
# time-based, not arrival-based: at speed 30 the ~141-unit diagonal from
# (-100,-100) takes ~4.7s, so leg 2 (toward (40,0,40), campfire_square) is
# already underway before village_road is actually entered. Worked out from
# the real leg geometry: the diagonal from leg-1's t=4.5 handoff point
# (-25.75,-25.75) toward (40,40) crosses x=-12 at t≈5.47 — which is exactly
# what the real run below logs. So the correct window for Village is inside
# leg 2 (t<=8.5, same leg whose arrival produces Campfire), not leg 1.
VILLAGE_LINE=$(grep -a '^AUDIO_ZONE:Village ' "$log" | head -1)
CAMPFIRE_LINE=$(grep -a '^AUDIO_ZONE:Campfire ' "$log" | head -1)
if [ -z "$VILLAGE_LINE" ]; then
  fail ZONE_VILLAGE "no AUDIO_ZONE:Village line — zone tracking never saw the player enter village_road"
else
  t=$(field "$VILLAGE_LINE" t); px=$(field "$VILLAGE_LINE" pos | tr -d '()' | cut -d, -f1)
  ok=1
  awk -v t="$t" 'BEGIN{exit (t>0 && t<=8.5)?0:1}' || ok=0
  awk -v x="$px" 'BEGIN{v=x; if(v<0)v=-v; exit (v<=12.5)?0:1}' || ok=0
  if [ "$ok" -eq 1 ]; then
    pass "AUDIO_ZONE:Village at t=$t pos.x=$px — real position, inside village_road, inside leg-2 travel window"
  else
    fail ZONE_VILLAGE "AUDIO_ZONE:Village fired at t=$t pos.x=$px — outside the expected window/bounds"
  fi
fi
if [ -z "$CAMPFIRE_LINE" ]; then
  fail ZONE_CAMPFIRE "no AUDIO_ZONE:Campfire line — zone tracking never saw the player enter campfire_square"
else
  t=$(field "$CAMPFIRE_LINE" t); px=$(field "$CAMPFIRE_LINE" pos | tr -d '()' | cut -d, -f1)
  ok=1
  awk -v t="$t" 'BEGIN{exit (t>4.5 && t<=8.6)?0:1}' || ok=0
  awk -v x="$px" 'BEGIN{exit (x>=27.5 && x<=52.5)?0:1}' || ok=0
  if [ "$ok" -eq 1 ]; then
    pass "AUDIO_ZONE:Campfire at t=$t pos.x=$px — real position, inside campfire_square, inside leg-2 window"
  else
    fail ZONE_CAMPFIRE "AUDIO_ZONE:Campfire fired at t=$t pos.x=$px — outside the expected window/bounds"
  fi
fi
# order matters: Village must precede Campfire (same route the walk takes)
if [ -n "$VILLAGE_LINE" ] && [ -n "$CAMPFIRE_LINE" ]; then
  tv=$(field "$VILLAGE_LINE" t); tc=$(field "$CAMPFIRE_LINE" t)
  if awk -v a="$tv" -v b="$tc" 'BEGIN{exit (a<b)?0:1}'; then
    pass "zone order Village(t=$tv) -> Campfire(t=$tc) matches the scripted route"
  else
    fail ZONE_ORDER "Village(t=$tv) did not precede Campfire(t=$tc)"
  fi
fi

# ── gate 4: AUDIO_SUMMARY: totals are non-zero and internally consistent ──
LAST_SUMMARY=$(grep -a '^AUDIO_SUMMARY:' "$log" | tail -1)
if [ -z "$LAST_SUMMARY" ]; then
  fail SUMMARY "no AUDIO_SUMMARY: line — dump_sfx_summary never ran"
else
  total=$(sed -nE 's/^AUDIO_SUMMARY:total=([0-9]+).*/\1/p' <<<"$LAST_SUMMARY")
  if [ -z "$total" ] || [ "$total" -lt 1 ]; then
    fail SUMMARY "AUDIO_SUMMARY: total=$total — expected > 0"
  else
    pass "AUDIO_SUMMARY: total=$total (last dump: $LAST_SUMMARY)"
  fi
fi

# ── gate 5: combat SFX — scripted TRIGGER, but REAL playback ──────────────
# audio_proof_main.rs fires these via a scripted MessageWriter (combat.rs
# can't compile — see file header). This gate does not claim combat->audio
# integration; it only proves play_sfx really spawns audio for every variant
# once told to, each carrying real t=/pos= evidence.
COMBAT_SOUNDS="swing_light.wav swing_heavy.wav hit_light.wav hit_heavy.wav hit_block.wav hit_parry.wav enemy_death.wav player_hurt.wav player_death.wav player_respawn.wav"
BAD=0
for snd in $COMBAT_SOUNDS; do
  line=$(grep -a "^AUDIO_PLAY:audio/$snd " "$log" | head -1)
  if [ -z "$line" ]; then
    echo "  => FAIL [COMBAT_PLAY] audio/$snd never played after the scripted SfxEvent"; BAD=1
    continue
  fi
  t=$(field "$line" t)
  if [ -z "$t" ] || ! awk -v t="$t" 'BEGIN{exit (t>=9.0)?0:1}'; then
    echo "  => FAIL [COMBAT_TIMING] audio/$snd played at t=$t — expected >= 9.0 (scripted fire point)"; BAD=1
  fi
done
if [ "$BAD" -eq 0 ]; then
  pass "all combat SFX variants played with real t=/pos= (trigger is scripted — see header; playback is real)"
else
  FAILS=$((FAILS + 1))
fi

# ── gate 6: mixer live-push — explicitly NOT graded, and said out loud ────
note MIXER_NOT_GATED "update_music_volume/crossfade_ambient print nothing on the volume they push \
to live AudioSinks — only the driver narrates before/after. Per this script's own rule (driver \
output is not system evidence), no PASS/FAIL is claimed for the mixer live-push. If that needs to \
be provable, audio.rs needs its own println on the sink write, not a louder driver."

echo ""
echo "─── VERDICT ───"
if [ "$FAILS" -gt 0 ]; then
  echo "PROVE_AUDIO: FAIL ($FAILS gate(s))"
  exit 1
fi
echo "PROVE_AUDIO: PASS — startup/footstep/zone/summary/combat-playback evidence all sourced from audio.rs's own AUDIO_PLAY:/AUDIO_ZONE:/AUDIO_SUMMARY: lines (mixer live-push not gated — see note above)"
exit 0
