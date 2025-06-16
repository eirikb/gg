CACHE_DIR="$HOME/.cache/gg"
if [ "$1" = "--cache-dir" ] && [ -n "$2" ]; then
  CACHE_DIR="$2"
  shift 2
fi

if [ "$OSTYPE" = "cygwin" ] || [ "$OSTYPE" = "msys" ]; then
  which powershell
  powershell "$CACHE_DIR/gg-VERVER/stage2.ps1" --cache-dir="$CACHE_DIR" "$@"
  exit $?
fi

if [ ! -f "$CACHE_DIR/gg-VERVER/stage4" ]; then
  cd "$CACHE_DIR/gg-VERVER" || exit
  for stage3 in stage3*; do
    chmod +x "$stage3"
    if "./$stage3" 2>/dev/null; then
      echo "$stage3" >system
      cd - >/dev/null
      break
    fi
  done
fi

if [ -f "$CACHE_DIR/gg-VERVER/stage4" ]; then
  chmod +x "$CACHE_DIR/gg-VERVER/stage4"
  "$CACHE_DIR/gg-VERVER/stage4" --cache-dir="$CACHE_DIR" "$@"
  exit $?
fi

echo "Your system is not supported. Please check out https://github.com/eirikb/gg"
exit 1
