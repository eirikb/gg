echo 1
if which powershell &>/dev/null; then
echo 2
  echo found
  which powershell
  powershell ./.cache/gg/stage2.ps1 "$@"
  exit $?
fi
echo 3

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
  chmod +x ./.cache/gg/stage4
  ./.cache/gg/stage4 "$@"
  exit $?
fi

echo "Your system is not supported"
exit 1
