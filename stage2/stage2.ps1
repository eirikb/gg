if (Test-Path .cache\gg\stage4) {
    return Start-Process .cache\gg\stage4 $args
}

cd .cache\gg
Get-Item stage3* | % {
    pwd
    $name = $_.Name + ".exe"
    Test-Path $_.Name
    if (!$_.Name.EndsWith(".exe")) {
        cp $_.Name $name
    }
    $proc = Start-Process ".\$name" -PassThru -NoNewWindow -ErrorAction SilentlyContinue
    Wait-Process -InputObject $proc
    if ($proc.ExitCode -eq 0) {
        Out-File -Encoding ascii -LiteralPath system -InputObject $_.Name
    }
}

cd ../..

if (Test-Path ".cache\gg\stage4") {
    cat .cache\gg\system
    $proc = Start-Process ".\.cache\gg\stage4" -WorkingDirectory "$(Get-Location)" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
    Wait-Process -InputObject $proc

    $proc = Start-Process ".\.cache\gg\stage4" -WorkingDirectory "$(Get-Location)" -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
    Wait-Process -InputObject $proc

    $htArgs  = if ($Args.Count) { @{ Args = $Args } } else { @{} }
    $proc = Start-Process ".\.cache\gg\stage4" -WorkingDirectory "$(Get-Location)" -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $htArgs
    Wait-Process -InputObject $proc
} else {
    Write-Host "stage4 not found :("
}

