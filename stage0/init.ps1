echo "Hello..."

cd .cache\gg
Get-Item stage1* | % {
    $name = $_.Name + ".exe"
    if (!$_.Name.EndsWith(".exe")) {
        Write-Host "re to the name"
        cp $_.Name $name
        & $name
        & .\mn "$@"
    } else {
        Write-Host $_.Name
        & $_.Name
        & .\mn "$@"
    }
}
