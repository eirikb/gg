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

if [ -f "$cache_dir/stage2.sh" ]; then
  "$cache_dir/stage2.sh" "$@"
  exit
fi

mkdir -p "$cache_dir"
tail -c +BBB gg.cmd | tar -xzC "$cache_base" && "$cache_dir/stage2.sh" "$@"
exit