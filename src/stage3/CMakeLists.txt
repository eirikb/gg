cmake_minimum_required(VERSION 3.10)

project(stage3 C)

set(CMAKE_C_STANDARD 99)

if (NOT EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/sha256-512/CMakeLists.txt)
    message(STATUS "Cloning sha256-512 library...")
    execute_process(
            COMMAND git clone https://github.com/LeFroid/sha256-512.git
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            RESULT_VARIABLE GIT_RESULT
    )

    if (NOT GIT_RESULT EQUAL "0")
        message(FATAL_ERROR "Failed to clone sha256-512 repository")
    endif ()

    message(STATUS "sha256-512 library cloned successfully")
endif ()

# Hacking the sha256-512 CMakeLists.txt to ensure compatibility (shrug)
if (EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/sha256-512/CMakeLists.txt)
    file(READ ${CMAKE_CURRENT_SOURCE_DIR}/sha256-512/CMakeLists.txt CMAKELISTS_CONTENT)
    string(REGEX REPLACE "cmake_minimum_required *\\(VERSION [0-9]+\\.[0-9]+\\)" "cmake_minimum_required(VERSION 3.10)" CMAKELISTS_CONTENT "${CMAKELISTS_CONTENT}")
    file(WRITE ${CMAKE_CURRENT_SOURCE_DIR}/sha256-512/CMakeLists.txt "${CMAKELISTS_CONTENT}")
    message(STATUS "sha256-512 CMakeLists.txt patched for compatibility")
endif ()

add_subdirectory(sha256-512)

if (WIN32)
    add_executable(stage3 main-win.c)
else ()
    add_executable(stage3 main.c hash.c)
    target_compile_options(stage3 PRIVATE -Wall -Wextra)
endif ()

target_link_libraries(stage3 PRIVATE sha512)
target_compile_options(stage3 PRIVATE -Os -s)
