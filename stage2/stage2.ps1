echo "Hello..."

cd .cache\gg
Get-Item stage3* | % {
    $name = $_.Name + ".exe"
    if (!$_.Name.EndsWith(".exe")) {
        Write-Host "re to the name"
        cp $_.Name $name
        & .\$name
        cd ../..
        & .cache\gg\stage4 $args
    } else {
        Write-Host $_.Name
        & .\$_.Name
        cd ../..
        & .\cache\gg\stage4 $args
    }
}
