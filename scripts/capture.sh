#!/bin/bash
# Headless visual test: Xvfb + lavapipe, dumps screenshots at timed moments.
# Usage: capture.sh <mode> <out_prefix>
#   mode = menu | mission | mission2 | debug | <mission_id>
set -e
MODE="${1:-mission}"
PREFIX="${2:-cap}"
ROOT=/home/z/my-project/mdungeons
OUT=$ROOT/scripts/preview
mkdir -p "$OUT"

export LD_LIBRARY_PATH=/tmp/egldirect:/tmp/vkroot/usr/lib/x86_64-linux-gnu:/tmp/localprefix/usr/lib/x86_64-linux-gnu
export VK_DRIVER_FILES=/tmp/vkroot/usr/share/vulkan/icd.d/lvp_icd.json
export VK_ICD_FILENAMES=/tmp/vkroot/usr/share/vulkan/icd.d/lvp_icd.json
export LIBGL_ALWAYS_SOFTWARE=1
export PKG_CONFIG_PATH=/tmp/localprefix/usr/lib/x86_64-linux-gnu/pkgconfig
export DISPLAY=:97
export PATH=/tmp/vkroot/usr/bin:$PATH

pkill -9 -f "Xvfb :97" 2>/dev/null || true
sleep 0.5
rm -f /tmp/.X97-lock /tmp/.X11-unix/X97 2>/dev/null || true
# Xvfb must NOT inherit our LD_LIBRARY_PATH (mesa symlinks crash it)
env -i Xvfb :97 -screen 0 1280x720x24 > "$OUT/xvfb.log" 2>&1 < /dev/null &
XVFB_PID=$!
# wait until the display answers
for i in $(seq 1 30); do
    if xset -display :97 -q >/dev/null 2>&1; then break; fi
    sleep 0.4
done
sleep 0.5

shot() {  # shot <name>
    xwd -root -silent -display :97 | ffmpeg -y -loglevel error -i - -vf "crop=1280:720:0:0" "$OUT/$1.png"
}

case "$MODE" in
  menu)
    "$ROOT/target/release/mdungeons" > "$OUT/${PREFIX}_log.txt" 2>&1 &
    GAME_PID=$!
    sleep 6; shot "${PREFIX}_menu"
    ;;
  camp)
    MD_SCREEN=camp "$ROOT/target/release/mdungeons" > "$OUT/${PREFIX}_log.txt" 2>&1 &
    GAME_PID=$!
    sleep 7; shot "${PREFIX}_camp"
    ;;
  debug)
    MD_DEBUG=1 "$ROOT/target/release/mdungeons" > "$OUT/${PREFIX}_log.txt" 2>&1 &
    GAME_PID=$!
    sleep 6; shot "${PREFIX}_menu_debug"
    kill $GAME_PID 2>/dev/null || true
    MD_DEBUG=1 MD_AUTO=0 "$ROOT/target/release/mdungeons" > "$OUT/${PREFIX}_log2.txt" 2>&1 &
    GAME_PID=$!
    sleep 6; shot "${PREFIX}_mission_debug"
    ;;
  *)
    MID="${MODE}"
    MD_AUTO="$MID" MD_BOT=1 "$ROOT/target/release/mdungeons" > "$OUT/${PREFIX}_log.txt" 2>&1 &
    GAME_PID=$!
    sleep 6; shot "${PREFIX}_mission_spawn"
    sleep 5; shot "${PREFIX}_mission_later"
    sleep 4; shot "${PREFIX}_mission_combat"
    ;;
esac

kill $GAME_PID 2>/dev/null || true
kill $XVFB_PID 2>/dev/null || true
echo "captures -> $OUT/"
ls -la "$OUT" | tail -8
