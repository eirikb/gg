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
    : Git's GNU tar shadows tar on PATH and cannot take "C:" paths (#291)
    set "GG_TAR=%SystemRoot%\System32\tar.exe"
    if not exist "%GG_TAR%" set "GG_TAR=tar"
    "%GG_TAR%" -zxf m2 -C "%GG_CACHE_DIR%"
    del m2
    if not exist "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" (
        echo gg: could not unpack into "%GG_CACHE_DIR%"
        exit /b 1
    )
    powershell -executionpolicy bypass -file "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" %*
    exit /b %errorlevel%
BATCH
