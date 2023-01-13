echo "Hello..."

Write-Host "Args? $args"

if (Test-Path .cache\gg\stage4) {
    Write-Host "Run 1"
    return Start-Process .cache\gg\stage4 $args
}

Write-Host "cd..."
cd .cache\gg
Get-Item stage3* | % {
    pwd
    $name = $_.Name + ".exe"
    Write-Host $_.Name
    Write-Host "Hello " + $_.Name
    Test-Path $_.Name
    if (!$_.Name.EndsWith(".exe")) {
        Write-Host "re to the name"
        cp $_.Name $name
    }
    $proc = Start-Process ".\$name" -PassThru -NoNewWindow -ErrorAction SilentlyContinue
    Wait-Process -InputObject $proc
    if ($proc.ExitCode -eq 0) {
        Out-File -Encoding ascii -LiteralPath system -InputObject $_.Name
    } else {
        Write-Host "Didn't work $($proc.ExitCode)"
        Write-Host "proc is $proc"
    }
}

cd ../..

if (Test-Path ".cache\gg\stage4") {
    Write-Host "Run 2"
    cat .cache\gg\system
    echo location is "$(Get-Location)"
    echo "location is $(Get-Location)"
    echo "args is $args"
    $proc = Start-Process ".\.cache\gg\stage4" -WorkingDirectory "$(Get-Location)" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
    Write-Host $proc
    Wait-Process -InputObject $proc
    echo $proc.ExitCode

    $proc = Start-Process ".\.cache\gg\stage4" -WorkingDirectory "$(Get-Location)" -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
    Write-Host $proc
    Wait-Process -InputObject $proc
    echo $proc.ExitCode

    echo "NOW WHAT"
    $htArgs  = if ($Args.Count) { @{ Args = $Args } } else { @{} }
    $proc = Start-Process ".\.cache\gg\stage4" -WorkingDirectory "$(Get-Location)" -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $htArgs
    Write-Host $proc
    Wait-Process -InputObject $proc
    echo $proc.ExitCode
} else {
    Write-Host "stage4 not found :("
}

