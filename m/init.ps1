echo "Hello..."

cd .cache\m
ls gg* | % {
    $name = $_ + ".exe"
    if (!$_.EndsWith(".exe")) {
        Write-Host "re to the name"
        cp $_ $name
        & $name
        & .\mn "$@"
    } else {
        Write-Host $_
        & $_
        & .\mn "$@"
    }
}
