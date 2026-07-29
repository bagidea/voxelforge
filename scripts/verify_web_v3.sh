#!/usr/bin/env bash
# Capture the WEB half of the web-parity v3 pair (docs/web-parity-checklist.md §2.6).
#
# The query string is NOT written here: `scripts/hero_recipe.py --query` renders the
# SAME dict that `--env` feeds to scripts/render_native_control_v3.sh, so the two
# halves of the pair cannot drift onto different recipes. Both come from
# scripts/render_wide_hero.sh — the CEO-approved TILT-DOWN hero that produced the
# baseline docs/assets/wide-hero-final.png.
#
# `?hero` selects the scene (main.rs:153) and is injected by hero_recipe.py; the
# native shot binary doesn't need it, the browser does.
#
# Produces, next to each other (the grader auto-detects the .console.txt sibling):
#   docs/assets/wasm-hero-v3.png
#   docs/assets/wasm-hero-v3.png.console.txt   <- AdapterInfo (W0-A) + VOXELFORGE_URL (W0-C)
#   docs/assets/wasm-hero-v3.png.browser.log   <- Chrome's own log (Tint diagnostics)
set -euo pipefail
cd "$(dirname "$0")/.."

SHOT="${1:-docs/assets/wasm-hero-v3.png}"
QUERY="?$(python scripts/hero_recipe.py --query)"

echo "recipe: $QUERY"

# Settle 8s > the native hero's 3.2s TAA wait, so the web frame is never the one
# still converging (checklist §2.6 "TAA settle ต้องเท่ากันหรือมากกว่า").
node scripts/web-verify.mjs \
  --query "$QUERY" \
  --shot "$SHOT" \
  --wait 60000 \
  --settle 8000

# Fail loudly here rather than at grading time: the console dump is the only
# evidence for W0-A (backend) and W0-C (recipe), and a capture missing either is
# NOT GRADEABLE no matter how good the frame looks.
LOG="$SHOT.console.txt"
test -f "$LOG" || { echo "verify_web_v3: no $LOG — the grader cannot admit this frame" >&2; exit 1; }
grep -q 'AdapterInfo {' "$LOG" || { echo "verify_web_v3: no AdapterInfo line in $LOG (W0-A unprovable)" >&2; exit 1; }
URL=$(grep -m1 -o 'http[^ ]*?[^ ]*' "$LOG")
python scripts/hero_recipe.py --check-url "$URL"
