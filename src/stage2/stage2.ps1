$stage4 = ".\.cache\gg\gg-VERVER\stage4.exe"

if (Test-Path $stage4)
{
    $proc = Start-Process $stage4 -WorkingDirectory "$( Get-Location )" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
    Wait-Process -InputObject $proc
    exit $proc.ExitCode
}

$arch = $Env:PROCESSOR_ARCHITECTURE
if ($arch -Eq "AMD64")
{
    $arch = "x86_64"
}

$hashes = (Get-Content .cache/gg/gg-VERVER/hashes).Split("`n")
$hash = ($hashes | Where-Object { $_ -match "$arch.*windows" })
if ($hash)
{
    "$arch-windows" | Out-File .cache\gg\gg-VERVER\system -Encoding ascii
    $hash = $hash.split("=")[1]
    Invoke-WebRequest "https://ggcmd.z13.web.core.windows.net/$hash" -OutFile $stage4
    if (Test-Path $stage4)
    {
        $proc = Start-Process $stage4 -WorkingDirectory "$( Get-Location )" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
        Wait-Process -InputObject $proc
        exit $proc.ExitCode
    }
    else
    {
        Write-Host "Unable to download. Try again"
        exit 1
    }
}
else
{
    Write-Host "Hash not found :("
    exit 1
}
