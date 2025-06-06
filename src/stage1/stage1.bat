: <<BATCH
    @echo off
    : VERSION: VERVER
    if exist .cache\gg\gg-VERVER\stage2.ps1 (
        powershell -executionpolicy bypass -file .cache\gg\gg-VERVER\stage2.ps1 %*
        exit /b %errorlevel%
    )
    powershell -c "sc m2 ([byte[]](gc gg.cmd -Encoding Byte | select -Skip AAA)) -Encoding Byte"
    tar -zxf m2
    del m2
    powershell -executionpolicy bypass -file .cache\gg\gg-VERVER\stage2.ps1 %*
    exit /b %errorlevel%
BATCH
