#!/usr/bin/env bash
# Poppy -- poll the detached clouds build. The log is UTF-16LE (PS 5.1 Tee),
# so it MUST be decoded before grep or '^error' can never match.
L="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/client/_poppy_clouds_build2.log"
T=$(iconv -f UTF-16LE -t UTF-8 "$L" 2>/dev/null)
echo "lines=$(printf '%s' "$T" | wc -l)  errors=$(printf '%s\n' "$T" | grep -c '^error')"
printf '%s\n' "$T" | grep -E '^error|^warning: .*generated|Finished|EXITCODE' | tail -5
