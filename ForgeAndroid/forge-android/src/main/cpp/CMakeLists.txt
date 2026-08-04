cmake_minimum_required(VERSION 3.22.1)

project("forge-android" VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Get the project root (go up from ForgeAndroid/forge-android/src/main/cpp to amma-forge-sdk)
# cpp -> main -> src -> forge-android -> ForgeAndroid -> amma-forge-sdk
get_filename_component(PROJECT_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../.." ABSOLUTE)

# Path to the Rust FFI header
set(FORGE_INCLUDE_DIR "${PROJECT_ROOT}/forge-rust/include")

# Path to prebuilt libraries
set(FORGE_LIB_DIR "${PROJECT_ROOT}/forge-rust/build/android/${ANDROID_ABI}")

message(STATUS "Project root: ${PROJECT_ROOT}")
message(STATUS "Forge include dir: ${FORGE_INCLUDE_DIR}")
message(STATUS "Forge lib dir: ${FORGE_LIB_DIR}")

# Verify the library exists
if(NOT EXISTS "${FORGE_LIB_DIR}/libforge_core.so")
    message(FATAL_ERROR "libforge_core.so not found at ${FORGE_LIB_DIR}. Run forge-rust/scripts/build-android.sh first.")
endif()

# Import the prebuilt Rust library
add_library(forge_core SHARED IMPORTED)
set_target_properties(forge_core PROPERTIES
    IMPORTED_LOCATION "${FORGE_LIB_DIR}/libforge_core.so"
    IMPORTED_NO_SONAME TRUE
)

# Import llama.cpp libraries
add_library(llama SHARED IMPORTED)
set_target_properties(llama PROPERTIES
    IMPORTED_LOCATION "${FORGE_LIB_DIR}/libllama.so"
)

add_library(ggml SHARED IMPORTED)
set_target_properties(ggml PROPERTIES
    IMPORTED_LOCATION "${FORGE_LIB_DIR}/libggml.so"
)

add_library(ggml-base SHARED IMPORTED)
set_target_properties(ggml-base PROPERTIES
    IMPORTED_LOCATION "${FORGE_LIB_DIR}/libggml-base.so"
)

add_library(ggml-cpu SHARED IMPORTED)
set_target_properties(ggml-cpu PROPERTIES
    IMPORTED_LOCATION "${FORGE_LIB_DIR}/libggml-cpu.so"
)

add_library(mtmd SHARED IMPORTED)
set_target_properties(mtmd PROPERTIES
    IMPORTED_LOCATION "${FORGE_LIB_DIR}/libmtmd.so"
)

# Build our JNI wrapper
add_library(${CMAKE_PROJECT_NAME} SHARED
    forge_jni.cpp
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE
    ${FORGE_INCLUDE_DIR}
)

target_link_libraries(${CMAKE_PROJECT_NAME}
    forge_core
    llama
    mtmd
    ggml
    ggml-base
    ggml-cpu
    android
    log
)
