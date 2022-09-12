if [ -f ./.cache/gg/init.sh ]; then
  ./.cache/gg/init.sh "$@"
  exit
fi

tail -c +368 gg.cmd | tar -zpx && ./.cache/gg/init.sh "$@"
exit
