: <<BATCH
    @echo off
    : VERSION: VERVER
    set GG_CMD_PATH=%~f0
    if not defined GG_CACHE_DIR (
        if "%1"=="-l" (
            set GG_CACHE_DIR=.cache\gg
            shift /1
        ) else (
            set GG_CACHE_DIR=%UserProfile%\.cache\gg
        )
    ) else (
        if "%1"=="-l" shift /1
    )
    if exist "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" (
        powershell -executionpolicy bypass -file "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" %*
        exit /b %errorlevel%
    )
    if not exist "%GG_CACHE_DIR%" mkdir "%GG_CACHE_DIR%"
    powershell -c "sc m2 ([byte[]](gc '%0' -Encoding Byte | select -Skip AAAA)) -Encoding Byte"
    tar -zxf m2 -C "%GG_CACHE_DIR%"
    del m2
    powershell -executionpolicy bypass -file "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" %*
    exit /b %errorlevel%
BATCH
