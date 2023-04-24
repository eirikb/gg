cmake_minimum_required(VERSION 3.9.4)

project(stage3 C)

set(CMAKE_C_STANDARD 99)

file(
        DOWNLOAD
        "https://opensource.apple.com/source/network_cmds/network_cmds-511/unbound/compat/sha512.c"
        "sha512.c"
        SHOW_PROGRESS
)

execute_process(COMMAND git apply sha512-fix.diff WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

if (WIN32)
    add_executable(stage3 main-win.c)
else ()
    add_executable(stage3 main.c)
    target_compile_options(stage3 PRIVATE -Wall -Wextra)
endif ()

target_compile_options(stage3 PRIVATE -Os -s)
