cmake_minimum_required(VERSION 3.9.4)

project(stage3 C)

set(CMAKE_C_STANDARD 99)

file(
        DOWNLOAD
        "https://opensource.apple.com/source/network_cmds/network_cmds-511/unbound/compat/sha512.c"
        "${CMAKE_CURRENT_SOURCE_DIR}/sha512.c"
        SHOW_PROGRESS
)
file(
        DOWNLOAD
        "https://opensource.apple.com/source/network_cmds/network_cmds-481.20.1/unbound/config.h"
        "${CMAKE_CURRENT_SOURCE_DIR}/config.h"
        SHOW_PROGRESS
)

execute_process(COMMAND patch sha512.c sha512-fix.diff WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

if (WIN32)
    add_executable(stage3 main-win.c)
else ()
    add_executable(stage3 main.c)
    target_compile_options(stage3 PRIVATE -Wall -Wextra)
endif ()

target_compile_options(stage3 PRIVATE -Os -s)
