cd .cache/m || exit
for gg in .cache/m/gg*; do
  chmod +x "$gg"
  # shellcheck disable=SC2086
  if "./$gg" 2>/dev/null; then
    chmod +x mn
    echo "$gg" >system
    ./mn "$@"
    exit
  fi
done

echo "Your system is not supported"
exit 1
