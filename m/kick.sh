if [ -f ./.cache/m/init.sh ]; then
  ./.cache/m/init.sh "$@"
  exit
fi

tail -c +332 m.cmd | tar -zpx && ./.cache/m/init.sh "$@"
exit
