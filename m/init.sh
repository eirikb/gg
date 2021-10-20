cd .cache/m || exit
for gg in gg*
do
  echo "Running $gg..."
  file "$gg"
  ldd "$gg"
  chmod +x "$gg"
  # shellcheck disable=SC2086
  if "./$gg"; then
    file mn
    ldd mn
    chmod +x mn
    ./mn -- "$USER_PWD" $1
    exit
  fi
done

echo "Failed?!"
exit 1