echo "Hello..."

if (Test-Path .cache\gg\stage4) {
    return Start-Process .cache\gg\stage4 $args;
}

cd .cache\gg
Get-Item stage3* | % {
    $name = $_.Name + ".exe"
    if (!$_.Name.EndsWith(".exe")) {
        Write-Host "re to the name"
        cp $_.Name $name
        Start-Process -ErrorAction SilentlyContinue $name
        cd ../..
    } else {
        Write-Host $_.Name
        Start-Process -ErrorAction SilentlyContinue $name
        cd ../..
    }
}

if (Test-Path .cache\gg\stage4) {
    return Start-Process .cache\gg\stage4 $args;
} else {
    Write-Host "stage4 not found :(";
}
