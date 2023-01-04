: <<BATCH
    @echo off
    powershell -c "sc m2 ([byte[]](gc gg.cmd -Encoding Byte | select -Skip 375)) -Encoding Byte"
    tar -zxf m2
    powershell -executionpolicy bypass -file .cache\gg\stage2.ps1 %*
    exit /b
BATCH
