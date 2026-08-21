#!/usr/bin/env bash
# Turn the two showcase PNG sequences into the three artefacts the CEO actually
# looks at: the AFTER film, the BEFORE film, and a labelled side-by-side of the
# pair. Plus a contact sheet of the named beats, for the places a video will not
# embed.
#
#   bash scripts/_poppy_herofilm_encode.sh _poppy_hero
#
# The frames come out of the engine already evenly spaced in SIM time (main.rs
# pins the clock at FEEL_FILM_DT while filming), so the fps below is not a guess:
# it is the reciprocal of that constant, and the film plays at true speed.
set -euo pipefail

OUT="${1:-_poppy_hero}"
FPS=30                     # 1 / FEEL_FILM_DT, see main.rs
AFTER="$OUT/film_after"
BEFORE="$OUT/film_before"

have() { [ -d "$1" ] && [ "$(ls -1 "$1"/f*.png 2>/dev/null | wc -l)" -gt 0 ]; }

if ! have "$AFTER"; then
  echo "FILM FAIL: no frames in $AFTER"
  exit 2
fi

na=$(ls -1 "$AFTER"/f*.png | wc -l)
echo "after frames: $na"

# The frames are numbered from the first FILMED frame (f00000), so -start_number
# is 0 and the sequence is dense: any gap means a dropped screenshot, which would
# show up as a jump in the film, so count before encoding rather than after.
first=$(ls -1 "$AFTER"/f*.png | head -1)
last=$(ls -1 "$AFTER"/f*.png | tail -1)
expect=$(( $(basename "$last" .png | tr -d 'f' | sed 's/^0*//;s/^$/0/') + 1 ))
if [ "$na" -ne "$expect" ]; then
  echo "FILM WARN: $na files but the last is $(basename "$last") -- $((expect-na)) frame(s) missing"
fi
echo "range: $(basename "$first") .. $(basename "$last")"

enc() { # dir out label
  ffmpeg -y -loglevel error -framerate "$FPS" -start_number 0 -i "$1/f%05d.png" \
    -c:v libx264 -pix_fmt yuv420p -crf 18 -movflags +faststart "$2"
  echo "wrote $2 ($(du -h "$2" | cut -f1))"
}

enc "$AFTER" "$OUT/hero-rig-after.mp4"

if have "$BEFORE"; then
  nb=$(ls -1 "$BEFORE"/f*.png | wc -l)
  echo "before frames: $nb"
  enc "$BEFORE" "$OUT/hero-rig-before.mp4"

  # Side by side, each half labelled in the burn so a screenshot of the video is
  # still self-describing (a caption in a doc gets separated from the file).
  # drawtext needs an EXPLICIT fontfile here. The Windows gyan build ships no
  # fontconfig, so a bare drawtext does not fall back to a default face -- it
  # prints "Cannot load default config file" and SEGFAULTS, taking the whole
  # comparison cut with it. Caught by running this script against synthetic
  # frames before the real ones existed; hence the fallback below rather than a
  # hard dependency on one font being installed.
  lbl_b="BEFORE - placeholder capsule"
  lbl_a="AFTER - humanoid rig"
  if [ -f "/c/Windows/Fonts/arial.ttf" ]; then
    FONT="fontfile='C\\:/Windows/Fonts/arial.ttf':"
  else
    FONT=""
    echo "FILM WARN: no arial.ttf -- burning labels without an explicit font may fail"
  fi
  style="x=12:y=12:fontsize=20:fontcolor=white:box=1:boxcolor=black@0.6:boxborderw=6"

  ffmpeg -y -loglevel error \
    -framerate "$FPS" -start_number 0 -i "$BEFORE/f%05d.png" \
    -framerate "$FPS" -start_number 0 -i "$AFTER/f%05d.png" \
    -filter_complex "\
      [0:v]scale=640:-1,drawtext=${FONT}text='${lbl_b}':${style}[l];\
      [1:v]scale=640:-1,drawtext=${FONT}text='${lbl_a}':${style}[r];\
      [l][r]hstack=inputs=2" \
    -c:v libx264 -pix_fmt yuv420p -crf 18 -movflags +faststart "$OUT/hero-rig-compare.mp4"
  echo "wrote $OUT/hero-rig-compare.mp4 ($(du -h "$OUT/hero-rig-compare.mp4" | cut -f1))"

  # An animated GIF of the same pair, because chat clients embed GIFs where they
  # will not autoplay an mp4. Two-pass palette or the rig turns into 8-colour mud.
  ffmpeg -y -loglevel error -i "$OUT/hero-rig-compare.mp4" \
    -vf "fps=15,scale=900:-1:flags=lanczos,palettegen=stats_mode=diff" "$OUT/_pal.png"
  ffmpeg -y -loglevel error -i "$OUT/hero-rig-compare.mp4" -i "$OUT/_pal.png" \
    -lavfi "fps=15,scale=900:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=3" \
    "$OUT/hero-rig-compare.gif"
  rm -f "$OUT/_pal.png"
  echo "wrote $OUT/hero-rig-compare.gif ($(du -h "$OUT/hero-rig-compare.gif" | cut -f1))"
else
  echo "FILM NOTE: no before/ frames -- skipping the comparison cuts"
fi

echo "FILM OK"
