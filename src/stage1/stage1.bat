: <<BATCH
    @echo off
    : VERSION: VERVER
    powershell -c "sc m2 ([byte[]](gc gg.cmd -Encoding Byte | select -Skip AAA)) -Encoding Byte"
    tar -zxf m2
    powershell -executionpolicy bypass -file .cache\gg-VERVER\stage2.ps1 %*
    exit /b
BATCH
