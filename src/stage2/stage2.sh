#!/bin/bash

local_cache=false
for arg in "$@"; do
  if [ "$arg" = "-l" ] || [ "$arg" = "--local-cache" ]; then
    local_cache=true
    break
  fi
done

if [ "$local_cache" = "true" ]; then
  cache_base=".cache/gg"
else
  if [ "$(uname)" = "Darwin" ]; then
    cache_base="$HOME/Library/Caches/gg"
  elif [ -n "$XDG_CACHE_HOME" ]; then
    cache_base="$XDG_CACHE_HOME/gg"
  else
    cache_base="$HOME/.cache/gg"
  fi
fi

cache_dir="$cache_base/gg-VERVER"

if [ "$OSTYPE" = "cygwin" ] || [ "$OSTYPE" = "msys" ]; then
  which powershell
  powershell "$cache_dir/stage2.ps1" "$@"
  exit $?
fi

if [ ! -f "$cache_dir/stage4" ]; then
  mkdir -p "$cache_dir"
  cd "$cache_dir" || exit
  for stage3 in stage3*; do
    chmod +x "$stage3"
    if "./$stage3" "$@" 2>/dev/null; then
      echo "$stage3" >system
      break
    fi
  done
fi

if [ -f "$cache_dir/stage4" ]; then
  chmod +x "$cache_dir/stage4"
  "$cache_dir/stage4" "$@"
  exit $?
fi

echo "Your system is not supported. Please check out https://github.com/eirikb/gg"
exit 1