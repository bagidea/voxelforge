#!/usr/bin/env bash
# Build the Voxelforge client for the web (wasm32 + WebGPU) via trunk.
#
# getrandom backend (wasm32 has no OS entropy):
#   `getrandom` (pulled in transitively by ahash → dot_vox / bevy_egui) refuses
#   to compile until a backend is named. It needs BOTH halves or it errors: the
#   `wasm_js` cargo feature (client/Cargo.toml — owned by Poppy, do NOT touch
#   the getrandom_03/getrandom_04 block) and this --cfg. Keep the two in sync.
#
# WASM OPTIMIZATION (why this script is short on purpose):
#   trunk runs wasm-opt ITSELF during `trunk build`, using a copy of binaryen it
#   fetches/owns (wasm-opt is NOT expected on $PATH). The optimization level is
#   pinned explicitly in index.html via `data-wasm-opt="-O"` so the result is
#   deterministic rather than depending on a trunk default that could change.
#   trunk also writes SRI integrity hashes for the .wasm into dist/index.html,
#   computed FROM those optimized bytes — so NEVER strip / re-run wasm-opt /
#   otherwise mutate the .wasm after `trunk build`, or the hashes go stale and
#   every browser refuses to load the module ("Failed to find a valid digest").
#   (The old comment here claimed this script "strips debuginfo via RUSTFLAGS
#   and re-runs wasm-opt itself" — that was never true and following it would
#   break SRI; do not bring it back.)
#
# Usage: scripts/web-build.sh [--debug]
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'

# trunk maps NO_COLOR onto its --no-color flag, which only accepts true/false —
# the conventional NO_COLOR=1 makes it exit 2 before it builds anything.
unset NO_COLOR

MODE="--release"
if [ "${1:-}" = "--debug" ]; then MODE=""; fi

# No `exec`: we want to VERIFY the artifact after trunk returns. `set -e` still
# propagates trunk's exit code, so a failed build aborts before verification.
trunk build ${MODE}

# ── Verify the artifact before declaring success ────────────────────────────
# trunk can exit 0 even when wasm-opt choked and left an unoptimized/broken
# module — that is exactly how a broken web artifact shipped before. Verify
# here so a bad build fails LOUD instead of silently producing an unusable dist.
shopt -s nullglob
js=(dist/*.js)
wasm=(dist/*_bg.wasm)

if [ ! -f dist/index.html ] || [ ${#js[@]} -eq 0 ] || [ ${#wasm[@]} -eq 0 ]; then
  echo "web-build: VERIFY FAIL — dist/ incomplete (need index.html + *.js + *_bg.wasm)" >&2
  ls -la dist/ >&2 || true
  exit 1
fi

wasm_file="${wasm[0]}"
magic="$(od -An -tx1 -N4 "$wasm_file" | tr -d ' \n')"
if [ "$magic" != "0061736d" ]; then
  echo "web-build: VERIFY FAIL — $(basename "$wasm_file") is not a valid wasm module" >&2
  echo "           magic bytes = ${magic:-<empty>}, expected 0061736d (\\0asm)" >&2
  exit 1
fi

wasm_size=$(wc -c < "$wasm_file")
# wasm-opt'd release module is ~tens of MB; the raw (unoptimized) release is
# ~96MB. A module over ~80MB means wasm-opt almost certainly did not run — the
# broken-artifact failure mode — so warn loudly. (Not a hard fail: it still
# loads, but it's a regression signal the owner should see.)
if [ "$wasm_size" -gt 83886080 ]; then
  echo "web-build: WARN — $(basename "$wasm_file") is ${wasm_size} bytes (>80MB);" >&2
  echo "           wasm-opt likely did NOT run; artifact is valid but unoptimized" >&2
fi

# Confirm the SRI integrity hash in dist/index.html still matches the .wasm on
# disk (catches any accidental post-trunk mutation of the module).
sri_expected="$(grep -oE 'integrity="sha384-[A-Za-z0-9+/]+"' dist/index.html | sed -n '2p' \
  | sed -E 's/integrity="sha384-//; s/"$//')"
sri_actual="$(od -An -tx1 "$wasm_file" | tr -d ' \n' \
  | python -c "import sys,hashlib,base64;h=hashlib.sha384(bytes.fromhex(sys.stdin.read())).digest();print(base64.b64encode(h).decode())" 2>/dev/null || true)"
if [ -n "$sri_expected" ] && [ -n "$sri_actual" ] && [ "$sri_expected" != "$sri_actual" ]; then
  echo "web-build: VERIFY FAIL — SRI integrity hash in dist/index.html does not" >&2
  echo "           match $wasm_file; browsers will refuse to load it." >&2
  exit 1
fi

echo "web-build: VERIFY OK — $(basename "$wasm_file") ${wasm_size} bytes, magic ok," \
  "SRI matches; js glue + index.html present; serve dist/ (e.g. trunk serve)"
