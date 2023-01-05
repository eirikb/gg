echo "Hello..."

Write-Host "Args? $args"

if (Test-Path .cache\gg\stage4) {
    Write-Host "Run 1"
    return Start-Process .cache\gg\stage4 $args
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
    }
    Start-Process ".\$name" -Wait -NoNewWindow -ErrorAction SilentlyContinue
    if ($LastExitCode -eq 0) {
        Write-Output "It worked! Write system!"
        $_.Name | Out-File -FilePath system
    }
}

cd ../..

ls
ls .cache
ls .cache\gg
if (Test-Path ".cache\gg\stage4") {
    Write-Host "Run 2"
    return Start-Process ".\.cache\gg\stage4" -Wait -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
} else {
    Write-Host "stage4 not found :("
}

if (Test-Path ".\.cache\gg\stage4") {
    Write-Host "Run 3"
    return Start-Process ".\.cache\gg\stage4" -Wait -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
} else {
    Write-Host "stage4 not found :("
}
