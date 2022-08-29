: <<BATCH
    @echo off
    powershell -c "sc m2 ([byte[]](gc m.cmd -Encoding Byte | select -Skip 326)) -Encoding Byte"
    tar -zxf m2
    powershell .cache\m\init.ps %s
    exit /b
BATCH
