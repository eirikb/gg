if [ ! -f .cache/gg/stage2 ]; then
  cd .cache/gg || exit
  for stage1 in stage1*; do
    chmod +x "$stage1"
    # shellcheck disable=SC2086
    if "./$stage1" 2>/dev/null; then
      echo "$stage1" >system
      cd ../..
      break
    fi
  done
fi

if [ -f ./.cache/gg/stage2 ]; then
  echo "has stage2!"
  chmod +x ./.cache/gg/stage2
  ./.cache/gg/stage2 "$@"
  exit $?
fi

echo "Your system is not supported"
exit 1
