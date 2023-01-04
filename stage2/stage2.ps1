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
        ls
        cp $_.Name $name
        ls
        Write-Host "start $name"
        Start-Process -Wait -NoNewWindow -ErrorAction SilentlyContinue $name
        Write-Host "start 2 $name"
        Start-Process $name -Wait -NoNewWindow -ErrorAction SilentlyContinue
        Write-Host "start 3 $name"
        Start-Process ".\$name" -Wait -NoNewWindow -ErrorAction SilentlyContinue
    } else {
        Write-Host $_.Name
        Start-Process -Wait -NoNewWindow -ErrorAction SilentlyContinue $name
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
