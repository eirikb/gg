: <<BATCH
    @echo off
    : VERSION: VERVER
    set CACHE_DIR=%LocalAppData%\gg.cmd
    if exist "%CACHE_DIR%\gg-VERVER\stage2.ps1" (
        powershell -executionpolicy bypass -file "%CACHE_DIR%\gg-VERVER\stage2.ps1" %*
        exit /b %errorlevel%
    )
    if not exist "%CACHE_DIR%" mkdir "%CACHE_DIR%"
    powershell -c "sc m2 ([byte[]](gc gg.cmd -Encoding Byte | select -Skip AAA)) -Encoding Byte"
    tar -zxf m2 -C "%CACHE_DIR%"
    del m2
    powershell -executionpolicy bypass -file "%CACHE_DIR%\gg-VERVER\stage2.ps1" %*
    exit /b %errorlevel%
BATCH
