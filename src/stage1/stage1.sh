if command -v realpath >/dev/null 2>&1; then
  export GG_CMD_PATH="$(realpath -s "$0" 2>/dev/null || readlink -f "$0" 2>/dev/null || echo "$0")"
else
  export GG_CMD_PATH="$0"
fi

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
