
cmake_minimum_required(VERSION 3.9.4)

set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)

project(gg C)

set(CMAKE_C_STANDARD 99)

if (WIN32)
    add_executable(gg main-win.c)
else ()
    add_executable(gg main.c)
endif ()

project(print_hash C)

add_executable(print_hash print_hash.c)