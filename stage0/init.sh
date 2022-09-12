cd .cache/m || exit
for stage1 in stage1*; do
  chmod +x "$stage1"
  # shellcheck disable=SC2086
  if "./$stage1" 2>/dev/null; then
    echo "stage1 is $stage1"
    chmod +x mn
    echo "$stage1" >system
    ls -lah
    echo "system?"
    cat system
    ldd mn
    file mn
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
