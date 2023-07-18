if [ -f ./.cache/gg/gg-VERVER/stage2.sh ]; then
  ./.cache/gg/gg-VERVER/stage2.sh "$@"
  exit
fi

tail -c +BBB gg.cmd | tar -zpx && ./.cache/gg/gg-VERVER/stage2.sh "$@"
exit
