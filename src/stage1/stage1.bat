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
    : Bare "tar" picks up Git's GNU tar from PATH in Git Bash and on CI runners,
    : which shells out to a separate gzip and reads "C:" as a remote host (#291).
    : Sysnative first so a 32-bit cmd gets the native bsdtar, not SysWOW64's.
    set "GG_TAR=tar"
    if exist "%SystemRoot%\Sysnative\tar.exe" set "GG_TAR=%SystemRoot%\Sysnative\tar.exe"
    if exist "%SystemRoot%\System32\tar.exe" set "GG_TAR=%SystemRoot%\System32\tar.exe"
    "%GG_TAR%" -zxf m2 -C "%GG_CACHE_DIR%"
    set "GG_UNTAR_ERR=%errorlevel%"
    del m2
    set "GG_TAR="
    if not "%GG_UNTAR_ERR%"=="0" (
        echo gg: failed to unpack into "%GG_CACHE_DIR%" ^(tar exit %GG_UNTAR_ERR%^)
        exit /b %GG_UNTAR_ERR%
    )
    : A zero exit does not prove stage2 landed, and handing PowerShell a missing
    : -file is the confusing error #291 actually reported.
    if not exist "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" (
        echo gg: unpack produced no stage2.ps1 in "%GG_CACHE_DIR%"
        exit /b 1
    )
    powershell -executionpolicy bypass -file "%GG_CACHE_DIR%\gg-VERVER\stage2.ps1" %*
    exit /b %errorlevel%
BATCH
