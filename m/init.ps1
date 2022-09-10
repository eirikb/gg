echo "Hello..."

cd .cache/m
ls *gg*.exe | % {
    & $_
    & .\mn "$@"
}
