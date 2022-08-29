if [ -f ./.cache/m/init.sh ]; then
  ./.cache/m/init.sh "$@"
  exit
fi

tail -c +331 m.cmd | tar -zpx && ./.cache/m/init.sh "$@"
exit
