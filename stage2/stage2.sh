if [ "$OSTYPE" = "cygwin" ] || [ "$OSTYPE" = "msys" ]; then
  which powershell
  powershell ./.cache/gg-VERVER/stage2.ps1 "$@"
  exit $?
fi

if [ ! -f .cache/gg-VERVER/stage4 ]; then
  cd .cache/gg-VERVER || exit
  for stage3 in stage3*; do
    chmod +x "$stage3"
    if "./$stage3" 2>/dev/null; then
      echo "$stage3" >system
      cd ../..
      break
    fi
  done
fi

if [ -f ./.cache/gg-VERVER/stage4 ]; then
  chmod +x ./.cache/gg-VERVER/stage4
  ./.cache/gg-VERVER/stage4 "$@"
  exit $?
fi

echo "Your system is not supported. Please check out https://github.com/eirikb/gg"
exit 1
