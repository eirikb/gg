: <<BATCH
    @echo off
    : VERSION: VERVER
    
    set "localCache=false"
    for %%a in (%*) do (
        if "%%a"=="-l" set "localCache=true"
        if "%%a"=="--local-cache" set "localCache=true"
    )
    
    if "%localCache%"=="true" (
        set "cacheBase=.cache\gg"
    ) else (
        set "cacheBase=%LOCALAPPDATA%\gg"
    )
    
    set "cacheDir=%cacheBase%\gg-VERVER"
    
    if exist "%cacheDir%\stage2.ps1" (
        powershell -executionpolicy bypass -file "%cacheDir%\stage2.ps1" %*
        exit /b %errorlevel%
    )
    
    if not exist "%cacheBase%" mkdir "%cacheBase%"
    
    powershell -c "sc m2 ([byte[]](gc gg.cmd -Encoding Byte | select -Skip AAA)) -Encoding Byte"
    tar -zxf m2
    del m2
    
    if "%localCache%"=="false" (
        if exist ".cache\gg\gg-VERVER" (
            xcopy /E /I /Y ".cache\gg\gg-VERVER" "%cacheDir%"
            rmdir /S /Q ".cache"
        )
    )
    
    powershell -executionpolicy bypass -file "%cacheDir%\stage2.ps1" %*
    exit /b %errorlevel%
BATCH