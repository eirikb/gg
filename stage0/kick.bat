: <<BATCH
    @echo off
    powershell -c "sc m2 ([byte[]](gc m.cmd -Encoding Byte | select -Skip 361)) -Encoding Byte"
    tar -zxf m2
    powershell -executionpolicy bypass -file .cache\m\init.ps1 %s
    exit /b
BATCH
