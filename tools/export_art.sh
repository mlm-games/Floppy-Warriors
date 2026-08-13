#!/usr/bin/env bash
# Rasterizes the Inkscape SVGs in assets/images into tight, trimmed PNGs
# that the game loads as sprites. Requires rsvg-convert and ImageMagick.
set -euo pipefail

cd "$(dirname "$0")/../assets/images"
mkdir -p png

for f in body head hand leg upper_arm bow; do
  rsvg-convert -b none -w 1024 "$f.svg" -o "png/$f.png"
  magick "png/$f.png" -trim +repage "png/$f.png"
  echo "exported $f"
done

rsvg-convert -b none -w 2400 bg.svg -o png/bg.png
magick png/bg.png -trim +repage png/bg.png
echo "exported bg"