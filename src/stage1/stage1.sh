if [ "$1" = "-l" ]; then
  CACHE_DIR=".cache/gg"
  shift
else
  CACHE_DIR="$HOME/.cache/gg"
fi

if [ -f "$CACHE_DIR/gg-VERVER/stage2.sh" ]; then
  "$CACHE_DIR/gg-VERVER/stage2.sh" --cache-dir="$CACHE_DIR" "$@"
  exit
fi

mkdir -p "$CACHE_DIR"
tail -c +BBB gg.cmd | tar -zpx -C "$CACHE_DIR" && "$CACHE_DIR/gg-VERVER/stage2.sh" --cache-dir="$CACHE_DIR" "$@"
exit
