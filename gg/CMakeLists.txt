cmake_minimum_required(VERSION 3.9.4)

project(gg C)

set(CMAKE_C_STANDARD 99)

if (WIN32)
    add_executable(gg main-win.c)
else ()
    add_executable(gg main.c)
    target_compile_options(gg PRIVATE -Wall -Wextra)
endif ()

target_compile_options(gg PRIVATE -Os -s)