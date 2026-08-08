#!/usr/bin/env bash
# Cut the two raw captures into one deliverable clip.
#
# NO CROP, NO INPAINT -- and that is the point of this pass. The previous clip
# had to be cropped to 1280x502 and TELEA-inpainted because there was no way to
# turn the HUD off; `VOXELFORGE_NOHUD` now removes it at the source, so this
# script ships the full 1280x720 frame the game actually renders.
#
# Usage: bash scripts/_poppy_cine_cut.sh <vista.mkv> <gameplay.mkv> <out.mp4>
set -eu
cd "$(dirname "$0")/.."

VISTA="$1"
PLAY="$2"
OUT="$3"

# Trim points. The vista raw starts while the map is still settling and the
# scripted dolly runs on the binary's own clock, so the usable window is picked
# by hand, not by a fixed offset.
VISTA_SS="${VISTA_SS:-5}"
VISTA_T="${VISTA_T:-13}"
PLAY_SS="${PLAY_SS:-1}"
PLAY_T="${PLAY_T:-25}"
XFADE="${XFADE:-0.8}"

mkdir -p "$(dirname "$OUT")"

# One filter graph, one encode: re-encoding each shot and then concat'ing would
# put the pieces through x264 twice for no reason.
ffmpeg -hide_banner -loglevel warning \
  -ss "$VISTA_SS" -t "$VISTA_T" -i "$VISTA" \
  -ss "$PLAY_SS"  -t "$PLAY_T"  -i "$PLAY" \
  -filter_complex "
    [0:v]setpts=PTS-STARTPTS,fps=60,format=yuv420p[a];
    [1:v]setpts=PTS-STARTPTS,fps=60,format=yuv420p[b];
    [a][b]xfade=transition=fade:duration=$XFADE:offset=$(python -c "print($VISTA_T-$XFADE)")[v]
  " \
  -map "[v]" -c:v libx264 -preset slow -crf 20 -pix_fmt yuv420p \
  -movflags +faststart -an -y "$OUT"

echo "OUT=$OUT"
ffprobe -v error -show_entries format=duration,size -show_entries stream=width,height,r_frame_rate,nb_frames \
  -of default=nw=1 "$OUT"
