cmake_minimum_required(VERSION 3.28)
project(simple VERSION 0.5.2 LANGUAGES CXX)

cmake_policy(SET CMP0167 NEW)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

execute_process(
    COMMAND git rev-parse --short HEAD
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    OUTPUT_VARIABLE GIT_HASH
    OUTPUT_STRIP_TRAILING_WHITESPACE
    ERROR_QUIET
)

if(NOT GIT_HASH)
    set(GIT_HASH "unknown")
endif()

configure_file(
    "${CMAKE_SOURCE_DIR}/include/version.hpp.in"
    "${CMAKE_BINARY_DIR}/include/version.hpp"
)

find_package(fmt REQUIRED)
find_package(nlohmann_json REQUIRED)
find_package(yaml-cpp REQUIRED)
find_package(Threads REQUIRED)
find_package(Boost REQUIRED)
find_package(tomlplusplus REQUIRED)

# Find cmark-gfm (GitHub Flavored Markdown with extensions)
find_library(CMARK_GFM_LIB cmark-gfm REQUIRED HINTS /opt/homebrew/lib)
find_library(CMARK_GFM_EXT_LIB cmark-gfm-extensions REQUIRED HINTS /opt/homebrew/lib)
find_path(CMARK_GFM_INCLUDE_DIR cmark-gfm.h REQUIRED HINTS /opt/homebrew/include)

include_directories(
    ${CMAKE_SOURCE_DIR}/include
    ${CMAKE_BINARY_DIR}/include
    ${Boost_INCLUDE_DIRS}
    ${CMARK_GFM_INCLUDE_DIR}
)

set(SOURCES
    src/main.cpp
    src/error.cpp
    src/utils.cpp
    src/new.cpp
    src/minify.cpp
    src/handlers/components.cpp
    src/handlers/templates.cpp
    src/handlers/pages.cpp
    src/handlers/markdown.cpp
    src/handlers/frontmatter.cpp
    src/handlers/entries.cpp
    src/handlers/katex_assets.cpp
    src/dev/dev.cpp
)

add_executable(simple ${SOURCES})

target_link_libraries(simple PRIVATE
    fmt::fmt
    nlohmann_json::nlohmann_json
    yaml-cpp::yaml-cpp
    Threads::Threads
    ${CMARK_GFM_LIB}
    ${CMARK_GFM_EXT_LIB}
    Boost::headers
    tomlplusplus::tomlplusplus
)

install(TARGETS simple DESTINATION bin)
