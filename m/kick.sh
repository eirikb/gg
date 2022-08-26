#!/bin/bash
if [ -f ./.cache/m/mn ]; then
  ./.cache/m/mn "$@"
  exit
fi
tail -c +135 "$0" | tar -zpx && ./.cache/m/init.sh "$@"
exit
