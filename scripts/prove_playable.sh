#!/usr/bin/env bash
# Proof that `--play` boots into a game you can actually play.
#
# WHICH WORLD: `--play` no longer means "procedural". `main.rs:448` calls
# `scene::play_map()`, which returns `maps/edhari.json` whenever that file is on
# disk (scene.rs:37) — so now that Shiba's village has landed, all four shots
# below stand in Edhari, and procedural terrain is only the fallback for when
# the map is missing. Expect `MAP_LOAD ok path=maps/edhari.json` in EVERY log;
# seeing it is correct, not a leaked environment variable.
#
# Four captures, all from the SAME binary, each shot at exactly t=3.2 s by
# `main.rs::screenshot_once` — so the only variable between "before" and
# "after" is whether anything pressed a key:
#
#   boot   : `--play`      — no input at all. Avatar standing in the voxel world.
#   before : `--play`      — same, the A-side of the walk pair.
#   after  : `--play-demo` — `scene::play_proof` drives the REAL input resources
#                            (W, D, a left-click to capture the cursor, and
#                            MouseMotion to orbit the camera) through the real
#                            controller. Body and camera both end up elsewhere.
#   edhari : `--play` with VOXELFORGE_MAP_LOAD set explicitly — proves the
#                            caller-supplied map path works, not just the
#                            implicit `play_map()` default the first three take.
#
# Run from the repo root. Every run prints SCENE_READY; the demo also prints
# PLAY_LOOK / PLAY_WALK with the numbers behind the pictures.
#
# A shot only counts if ALL of these hold — `cargo check` being green says
# nothing about whether the schedule actually runs. A B0001 query-conflict is a
# *runtime* panic: it compiles clean, then dies on the first frame. That cost us
# a full round, so the gate is mechanical now instead of by-eye:
#   exit 0 · PNG on disk & non-empty · SCENE_READY printed ·
#   no `panicked` / `B0001` / `thread ... panicked` anywhere in the log.
set -uo pipefail

BIN=${BIN:-./target/debug/voxelforge.exe}
OUT=docs/assets
LOGS=_poppy_proof
mkdir -p "$OUT" "$LOGS"

# The first three shots must exercise the IMPLICIT play_map() path, so make sure
# an inherited VOXELFORGE_MAP_LOAD from the caller's shell can't stand in for it
# and quietly turn shot 4 into a duplicate of shots 1-3.
unset VOXELFORGE_MAP_LOAD VOXELFORGE_SHOT

fails=0

shot() { # <png> <extra-arg...>
  local png="$1"; shift
  local log="$LOGS/$(basename "$png").log"
  echo "=== $png  ($*) ==="
  VOXELFORGE_SHOT="$png" "$BIN" "$@" >"$log" 2>&1
  local rc=$?
  grep -E 'MAP_LOAD|MAP_APPLY|SPAWN_GROUND|SCENE_READY|PLAY_LOOK|PLAY_WALK|SHOT saved' "$log" || true

  local bytes=0
  [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "exit=$rc  bytes=$bytes"

  # --- the gate ---
  local bad=""
  [ "$rc" -ne 0 ]                             && bad="$bad exit=$rc"
  [ "$bytes" -lt 1024 ]                       && bad="$bad no-png"
  grep -qE 'panicked|B0001' "$log"            && bad="$bad PANIC"
  grep -q 'SCENE_READY'     "$log"            || bad="$bad no-SCENE_READY"
  # The engine grades its own movement. A run that boots, screenshots and exits 0
  # while the avatar is falling through the void still prints `=> FAIL` here — so
  # never let a clean exit code paper over the controller's own verdict.
  grep -qE 'PLAY_(WALK|LOOK).*=> FAIL' "$log" && bad="$bad play-FAIL"
  # Same idea, one layer earlier: scene.rs asserts the spawn column really is solid
  # under the avatar's feet. A FAIL here means the body was placed over air at t=0 —
  # the fall-through bug's actual signature, caught before gravity has run a frame.
  grep -qE 'SPAWN_GROUND.*=> FAIL'    "$log" && bad="$bad spawn-over-air"
  grep -q  'SPAWN_GROUND'             "$log" || bad="$bad no-SPAWN_GROUND"

  if [ -n "$bad" ]; then
    echo "  ✗ FAIL:$bad"
    grep -nE 'panicked|B0001|error\[' "$log" | head -5
    fails=$((fails + 1))
  else
    echo "  ✓ PASS"
  fi
}

shot "$OUT/playable-boot.png"        --play
shot "$OUT/playable-walk-before.png" --play
shot "$OUT/playable-walk-after.png"  --play-demo

# Shiba's authored level, loaded through the real engine path (not procedural).
# Exported deliberately: `VAR=x func` leaks past the call in bash, and this is
# the one shot that must NOT use the procedural world.
export VOXELFORGE_MAP_LOAD=${VOXELFORGE_MAP_LOAD:-maps/edhari.json}
shot "$OUT/edhari-load.png" --play
unset VOXELFORGE_MAP_LOAD

# The whole point of the before/after pair: if input never reached the controller
# both frames are the same picture. Byte-identical PNGs from a deterministic world
# are the signature of a dead input path, so that is a hard failure, not a note.
BEFORE="$OUT/playable-walk-before.png"
AFTER="$OUT/playable-walk-after.png"
if [ -f "$BEFORE" ] && [ -f "$AFTER" ]; then
  if cmp -s "$BEFORE" "$AFTER"; then
    echo "=== before/after ==="
    echo "  ✗ FAIL: before and after are byte-identical — input never moved anything"
    fails=$((fails + 1))
  else
    echo "=== before/after ==="
    echo "  ✓ PASS: frames differ ($(wc -c <"$BEFORE" | tr -d ' ') vs $(wc -c <"$AFTER" | tr -d ' ') bytes)"
  fi
fi

echo
if [ "$fails" -eq 0 ]; then
  echo "PROVE_PLAYABLE: PASS — 4/4 shots clean (exit 0, PNG written, no panic, walk graded PASS, before≠after)"
else
  echo "PROVE_PLAYABLE: FAIL — $fails shot(s) bad"
fi
exit "$fails"
