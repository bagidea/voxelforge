#!/usr/bin/env bash
# The lane's OWN verification of the R-key arbiter lane (`docs/LANES.md`):
# `cargo check` through build_safe.sh, scoped to Poppy's warm `target-poppy`.
# `--tests` is the load-bearing flag — without it the `#[cfg(test)]` module
# holding the six RKeyRoute tests is never type-checked at all.
#
# This is NOT the runtime proof. The runtime proof needs a full binary build,
# which is the integration lead's to run: scripts/_poppy_rkey_proof.sh.
set -uo pipefail
cd "$(dirname "$0")/.."

export CARGO_TARGET_DIR=target-poppy

echo "=== queue behind any build already on the box @ $(date +%H:%M:%S) ==="
if ! bash scripts/_poppy_wait_for_free_box.sh; then
  echo "CHECK_EXIT=1 (box never freed)"
  exit 1
fi

echo "=== cargo check -p voxelforge --tests @ $(date +%H:%M:%S) ==="
bash scripts/build_safe.sh check -p voxelforge --tests > _poppy_rkey_check.detail.log 2>&1
echo "CHECK_EXIT=$?"
echo "CHECK_ERRORS=$(grep -c '^error' _poppy_rkey_check.detail.log)"
grep -E '^error|^warning: unused|^ *--> client/src/(combat|quest)\.rs' _poppy_rkey_check.detail.log | head -20
tail -3 _poppy_rkey_check.detail.log
