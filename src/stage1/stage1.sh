# Keep the name we were invoked as - a symlink like node.cmd -> gg.cmd is the
# applet, so don't resolve it. realpath -s is GNU-only and readlink -f resolves
# the link, so cd/pwd + basename instead, same on macOS and Linux (#289).
if GG_CMD_PATH="$(cd "$(dirname "$0")" 2>/dev/null && pwd)"; then
  export GG_CMD_PATH="$GG_CMD_PATH/$(basename "$0")"
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
