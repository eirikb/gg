CACHE_DIR="$HOME/.cache/gg"

if [ -f "$CACHE_DIR/gg-VERVER/stage2.sh" ]; then
  "$CACHE_DIR/gg-VERVER/stage2.sh" "$@"
  exit
fi

mkdir -p "$CACHE_DIR"
tail -c +BBB gg.cmd | tar -zpx -C "$CACHE_DIR" && "$CACHE_DIR/gg-VERVER/stage2.sh" "$@"
exit
