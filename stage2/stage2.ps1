echo "Hello..."

if (Test-Path .cache\gg\stage4) {
    Write-Host "Run 1";
    return Start-Process .cache\gg\stage4 $args;
}

ls
Write-Host "cd..."
cd .cache\gg
ls
pwd
Get-Item stage3* | % {
    pwd
    $name = $_.Name + ".exe"
    Write-Host $_.Name
    Write-Host "Hello " + $_.Name
    Test-Path $_.Name
    if (!$_.Name.EndsWith(".exe")) {
        Write-Host "re to the name"
        cp $_.Name $name
        Start-Process -ErrorAction SilentlyContinue $name
    } else {
        Write-Host $_.Name
        Start-Process -ErrorAction SilentlyContinue $name
    }
}

cd ../..

ls
ls .cache
ls .cache\gg
if (Test-Path .cache\gg\stage4.exe) {
    Write-Host "Run 2";
    return Start-Process .cache\gg\stage4 $args;
} else {
    Write-Host "stage4 not found :(";
}
