if [ -z "$GG_CACHE_DIR" ]; then
  if [ "$1" = "-l" ]; then
    export GG_CACHE_DIR=".cache/gg"
    shift
  else
    export GG_CACHE_DIR="$HOME/.cache/gg"
  fi
elif [ "$1" = "-l" ]; then
  shift
fi

if [ -f "$GG_CACHE_DIR/gg-VERVER/stage2.sh" ]; then
  "$GG_CACHE_DIR/gg-VERVER/stage2.sh" "$@"
  exit
fi

mkdir -p "$GG_CACHE_DIR"
tail -c +BBBB "$0" | tar -zpx -C "$GG_CACHE_DIR" && "$GG_CACHE_DIR/gg-VERVER/stage2.sh" "$@"
exit
