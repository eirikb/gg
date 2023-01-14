if [ ! -f .cache/gg/stage4 ]; then
  cd .cache/gg || exit
  for stage3 in stage3*; do
    chmod +x "$stage3"
    # shellcheck disable=SC2086
    if "./$stage3" 2>/dev/null; then
      echo "$stage3" >system
      cd ../..
      break
    fi
  done
fi

if [ -f ./.cache/gg/stage4 ]; then
  echo "has stage4!"
  chmod +x ./.cache/gg/stage4
  echo "2 system is"
  cat ./.cache/gg/system
  ./.cache/gg/stage4 "$@"
  exit $?
fi

echo "Your system is not supported"
exit 1
