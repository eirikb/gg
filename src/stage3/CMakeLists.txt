cmake_minimum_required(VERSION 3.9.4)

project(stage3 C)

set(CMAKE_C_STANDARD 99)

add_subdirectory(sha256-512)

if (WIN32)
    add_executable(stage3 main-win.c)
else ()
    add_executable(stage3 main.c hash.c)
    target_compile_options(stage3 PRIVATE -Wall -Wextra)
endif ()

target_link_libraries(stage3 PRIVATE sha512)
target_compile_options(stage3 PRIVATE -Os -s)
