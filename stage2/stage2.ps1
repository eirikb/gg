$stage4 = ".\.cache\gg-VERVER\stage4.exe"

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

$hashes = (Get-Content .cache/gg-VERVER/hashes).Split("`n")
$hash = ($hashes | Where-Object { $_ -match "$arch.*windows" })
if ($hash)
{
    "$arch-windows" | Out-File .cache\gg-VERVER\system -Encoding ascii
    $hash = $hash.split("=")[1]
    Invoke-WebRequest "https://gg.eirikb.no/$hash" -OutFile $stage4
    if (Test-Path $stage4)
    {
        $proc = Start-Process $stage4 -WorkingDirectory "$( Get-Location )" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $args
        Wait-Process -InputObject $proc
        exit $proc.ExitCode
    }
    else
    {
        Write-Host "Unable to download. Try again"
    }
}
else
{
    Write-Host "Hash not found :("
}
