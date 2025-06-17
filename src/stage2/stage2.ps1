$cacheDir = if ($env:GG_CACHE_DIR)
{
    $env:GG_CACHE_DIR
}
else
{
    "$env:UserProfile\.cache\gg"
}
$stage4 = "$cacheDir\gg-VERVER\stage4.exe"

$quotedArgs = $args | ForEach-Object {
    if ($_ -match '\s')
    {
        '"{0}"' -f $_
    }
    else
    {
        $_
    }
}

if (Test-Path $stage4)
{
    if ((Get-Item $stage4).Length -gt 0)
    {
        $allArgs = $quotedArgs
        $proc = Start-Process $stage4 -WorkingDirectory "$( Get-Location )" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $allArgs
        if (-not $proc.HasExited)
        {
            Wait-Process -InputObject $proc
        }
        exit $proc.ExitCode
    }
    else
    {
        Remove-Item $stage4 -Force
    }
}

$arch = $Env:PROCESSOR_ARCHITECTURE
if ($arch -Eq "AMD64")
{
    $arch = "x86_64"
}

$hashes = (Get-Content "$cacheDir\gg-VERVER\hashes").Split("`n")
$hash = ($hashes | Where-Object { $_ -match "$arch.*windows" })
if ($hash)
{
    "$arch-windows" | Out-File "$cacheDir\gg-VERVER\system" -Encoding ascii
    $hash = $hash.split("=")[1]
    $tempFile = "$stage4.tmp"

    if (Test-Path $tempFile)
    {
        Remove-Item $tempFile -Force
    }

    try
    {
        Invoke-WebRequest "https://ggcmd.z13.web.core.windows.net/$hash" -OutFile $tempFile
        if ((Test-Path $tempFile) -and ((Get-Item $tempFile).Length -gt 0))
        {
            Move-Item $tempFile $stage4 -Force
        }
        else
        {
            Write-Host "Download failed: incomplete file"
            if (Test-Path $tempFile)
            {
                Remove-Item $tempFile -Force
            }
            exit 1
        }
    }
    catch
    {
        Write-Host "Download error: $_"
        if (Test-Path $tempFile)
        {
            Remove-Item $tempFile -Force
        }
        exit 1
    }

    if (Test-Path $stage4)
    {
        $allArgs = $args
        $proc = Start-Process $stage4 -WorkingDirectory "$( Get-Location )" -PassThru -NoNewWindow -ErrorAction SilentlyContinue -ArgumentList $allArgs
        if (-not $proc.HasExited)
        {
            Wait-Process -InputObject $proc
        }
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
