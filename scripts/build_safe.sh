#!/usr/bin/env bash
# build_safe.sh — canonical build entry point for Voxelforge.
#
# Why a wrapper exists (the STATUS_DLL_INIT_FAILED trap):
#   A plain `cargo build` on this box intermittently dies mid-spawn:
#
#       error: process didn't exit successfully: `...rustc.exe ...`
#         (exit code: 0xc0000142, STATUS_DLL_INIT_FAILED)
#
#   0xc0000142 fires while the OS is CREATING a process — at spawn, before any
#   code compiles — and lands on a DIFFERENT crate each build (syn /
#   zerocopy-derive / ntapi). It is NOT a code error.
#
#   The old "out of memory / too many parallel rustc jobs" theory is WRONG and
#   was disproven here: at the failure RAM was 9.2 GB free, commit 11.4/20.2
#   GB; it still dies at -j2 and even CARGO_BUILD_JOBS=1; link.exe alone runs
#   fine; a 150-process + 60-link.exe spawn test failed 0. The OS-level root
#   cause is still OPEN. Full write-up: docs/LANES.md "Build safety".
#
# What this wrapper does:
#   1. Sets three GENTLE DEFAULTS — these do NOT prevent STATUS_DLL_INIT_FAILED
#      (proven: JOBS=1 still dies); they only keep the build light:
#        CARGO_BUILD_JOBS=2          low concurrency
#        CARGO_PROFILE_DEV_DEBUG=0   drop debuginfo -> lighter linking
#        CARGO_INCREMENTAL=0         no per-crate incr-cache thrash
#   2. Forwards every argument to cargo and returns cargo's REAL exit code
#      (cargo is never piped — see the PIPE warning below).
#
# The real lever against STATUS_DLL_INIT_FAILED is SPAWN COUNT: build into a
# target dir that is already WARM (incremental = few rustc spawns), and NEVER
# `cargo clean` / wipe target (that = maximum spawns = maximum failure risk).
#
# Usage — pass anything you'd pass to cargo:
#   bash scripts/build_safe.sh build --bin voxelforge
#   bash scripts/build_safe.sh check
#   bash scripts/build_safe.sh --version
#
# Exit code: cargo's REAL exit code, propagated unchanged. cargo is never
# piped into tail/head here — that would return the pipe's exit 0 and launder
# a failed build into a green one. If you ever need to pipe cargo's output,
# capture ${PIPESTATUS[0]} and exit that, never $?.
set -uo pipefail

# Build-safety env. Gentle defaults; each may still be overridden from the
# caller's environment. NOTE: these do NOT cure STATUS_DLL_INIT_FAILED.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"

# Hand off to cargo with no pipe in the way, so $? below is cargo's own code.
cargo "$@"
rc=$?
exit "$rc"
