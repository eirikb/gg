Write-Host "Time to GOGOGO"

$stage4 = ".\.cache\gg\stage4.exe"

function Run()
{
    echo run
    $htArgs = if ($Args.Count)
    {
        @{ Args = $Args }
    }
    else
    {
        @{ }
    }
    Write-Host $htArgs
    Write-Host $stage4
    $proc = Start-Process $stage4 -WorkingDirectory "$( Get-Location )" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $htArgs
    echo $proc
    Wait-Process -InputObject $proc
    echo $proc
    exit $proc.ExitCode
}

if (Test-Path $stage4)
{
    Run
}

$arch = $Env:PROCESSOR_ARCHITECTURE
if ($arch -Eq "AMD64")
{
    $arch = "x86_64"
}

$hashes = (Get-Content .cache/gg/hashes).Split("`n")
Write-Host $hashes
$hash = ($hashes | Where-Object { $_ -match "$arch.*windows" })
if ($hash)
{
    Write-Host "Found hash $hash"
    $hash = $hash.split("=")[1]
    Write-Host "Actual hash $hash"
    Invoke-WebRequest "https://gg.eirikb.no/$hash" -OutFile $stage4
    if (Test-Path $stage4)
    {
        Run
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
