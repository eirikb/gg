if [ -f ./.cache/m/init.sh ]; then
  ./.cache/m/init.sh "$@"
  exit
fi

tail -c +325 "$0" | tar -zpx && ./.cache/m/init.sh "$@"
