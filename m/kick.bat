: <<BATCH
    @echo off
    powershell -c "sc m2 ([byte[]](gc m.cmd -Encoding Byte | select -Skip 331)) -Encoding Byte"
    tar -zxf m2
    powershell .cache\m\init.ps1 %s
    exit /b
BATCH
