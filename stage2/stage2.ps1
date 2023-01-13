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
        $_.Name > system
        Out-File -Encoding unicode -LiteralPath system -InputObject $_.Name
        Out-File -Encoding utf8 -LiteralPath system2 -InputObject $_.Name
        Out-File -Encoding ASCII  -LiteralPath system3 -InputObject $_.Name
        Out-File -Encoding utf8BIM -LiteralPath system4 -InputObject $_.Name

        echo "GO!"
        cat system
        cat system2
        cat system3
        pwd
        ls
        [System.IO.File]::ReadAllBytes("$(pwd)\system") | echo
        echo "..."
        [System.IO.File]::ReadAllBytes("$(pwd)\system2") | echo
        echo "..."
        [System.IO.File]::ReadAllBytes("$(pwd)\system3") | echo
        echo "..."
        [System.IO.File]::ReadAllBytes("$(pwd)\system4") | echo
        echo "..."
        Write-Host $([System.IO.File]::ReadAllBytes("$(pwd)\system"))
        echo "..."
        Write-Host $([System.IO.File]::ReadAllBytes("$(pwd)\system2"))
        echo "..."
        Write-Host $([System.IO.File]::ReadAllBytes("$(pwd)\system3"))
        echo "..."
        Write-Host $([System.IO.File]::ReadAllBytes("$(pwd)\system4"))
        echo "TEST"
        echo $([system.Text.Encoding]::UTF8.GetBytes("test"))
        echo $([system.Text.Encoding]::UTF8.GetBytes($_.Name))
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
    Wait-Process -InputObject $proc
    return $proc.ExitCode
} else {
    Write-Host "stage4 not found :("
}

