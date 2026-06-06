CACHE_DIR="${GG_CACHE_DIR:-$HOME/.cache/gg}"

if [ "$OSTYPE" = "cygwin" ] || [ "$OSTYPE" = "msys" ]; then
  which powershell >/dev/null 2>&1
  powershell -executionpolicy bypass "$CACHE_DIR/gg-VERVER/stage2.ps1" "$@"
  exit $?
fi

stage3_executed=""
if [ ! -f "$CACHE_DIR/gg-VERVER/stage4" ]; then
  cd "$CACHE_DIR/gg-VERVER" || exit
  # Glob order matters: aarch64* sorts before x86_64*, so on Apple Silicon the
  # native binary is tried before the x86_64 one (which Rosetta could also run)
  for stage3 in stage3*; do
    chmod +x "$stage3"
    "./$stage3" 2>/dev/null
    stage3_result=$?
    if [ "$stage3_result" = 0 ]; then
      echo "$stage3" >system
      cd - >/dev/null
      break
    fi
    # 126/127 mean the binary could not execute at all (wrong arch/OS).
    # Anything else means it ran but failed (network, hash mismatch, ...)
    if [ "$stage3_result" != 126 ] && [ "$stage3_result" != 127 ]; then
      stage3_executed=1
    fi
  done
fi

if [ -f "$CACHE_DIR/gg-VERVER/stage4" ]; then
  chmod +x "$CACHE_DIR/gg-VERVER/stage4"
  "$CACHE_DIR/gg-VERVER/stage4" "$@"
  exit $?
fi

if [ -n "$stage3_executed" ]; then
  echo "gg failed to download its runtime. Check your network connection (any errors above may have details)."
else
  echo "Your system is not supported. Please check out https://github.com/eirikb/gg"
fi
exit 1
