#!/bin/bash
# Build llama.cpp and forge-rust for Android
#
# Prerequisites:
#   - Android NDK installed (ANDROID_NDK_HOME set)
#   - cargo-ndk installed (cargo install cargo-ndk)
#   - Rust Android targets installed (rustup target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi)
#
# Usage:
#   ./scripts/build-android.sh [--release]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FORGE_RUST_DIR="$PROJECT_ROOT/forge-rust"
LLAMA_CPP_DIR="$PROJECT_ROOT/llama.cpp"
BUILD_TYPE="${1:-Release}"

# Keep developer-local absolute paths out of distributed native libraries.
PATH_REMAP_FLAGS="-g0 -ffile-prefix-map=${PROJECT_ROOT}=. -fdebug-prefix-map=${PROJECT_ROOT}=. -ffile-prefix-map=${HOME}=~ -fdebug-prefix-map=${HOME}=~"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${PROJECT_ROOT}=. --remap-path-prefix=${HOME}=~"

# Android API level (24 = Android 7.0)
ANDROID_API=24
DEFAULT_NDK_VERSION="27.0.12077973"

# ABIs to build
ABIS=("arm64-v8a" "x86_64")

echo "=============================================="
echo "Building llama.cpp and forge-rust for Android"
echo "=============================================="
echo "Project root: $PROJECT_ROOT"
echo "Build type: $BUILD_TYPE"

# Check prerequisites
if [ -z "$ANDROID_NDK_HOME" ]; then
    # Try common locations
    if [ -d "$HOME/Library/Android/sdk/ndk" ]; then
        if [ -d "$HOME/Library/Android/sdk/ndk/$DEFAULT_NDK_VERSION" ]; then
            export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/$DEFAULT_NDK_VERSION"
        else
            export ANDROID_NDK_HOME="$(find "$HOME/Library/Android/sdk/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
        fi
    fi
fi

if [ -z "$ANDROID_NDK_HOME" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "ERROR: ANDROID_NDK_HOME is not set or invalid"
    echo "Please set it to your Android NDK path, e.g.:"
    echo "  export ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/27.0.12077973"
    exit 1
fi

echo "Using NDK: $ANDROID_NDK_HOME"

if ! command -v cargo-ndk &> /dev/null; then
    echo "ERROR: cargo-ndk not found. Install with: cargo install cargo-ndk"
    exit 1
fi

# Create output directory for prebuilt libraries
ANDROID_LIBS_DIR="$FORGE_RUST_DIR/build/android"
mkdir -p "$ANDROID_LIBS_DIR"

# Determine CMake path
CMAKE_BIN="cmake"
if [ -f "$ANDROID_NDK_HOME/prebuilt/darwin-x86_64/bin/cmake" ]; then
    CMAKE_BIN="$ANDROID_NDK_HOME/prebuilt/darwin-x86_64/bin/cmake"
fi

echo ""
echo "Step 1: Building llama.cpp for Android (with 16KB page alignment)"
echo "-------------------------------------------------------------------"

for ABI in "${ABIS[@]}"; do
    echo ""
    echo "Building for $ABI..."

    BUILD_DIR="$PROJECT_ROOT/llama.cpp/build-android-$ABI"
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"

    # Set architecture-specific flags
    case $ABI in
        "arm64-v8a")
            ARCH="aarch64"
            GGML_FLAGS="-DGGML_CPU_KLEIDIAI=ON"
            ;;
        "x86_64")
            ARCH="x86_64"
            GGML_FLAGS=""
            ;;
        "armeabi-v7a")
            ARCH="armv7a"
            GGML_FLAGS=""
            ;;
    esac

    # Configure with CMake - use ANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES for 16KB alignment
    "$CMAKE_BIN" -S "$LLAMA_CPP_DIR" -B "$BUILD_DIR" \
        -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
        -DANDROID_ABI="$ABI" \
        -DANDROID_PLATFORM=android-$ANDROID_API \
        -DCMAKE_BUILD_TYPE="$BUILD_TYPE" \
        -DCMAKE_C_FLAGS="$PATH_REMAP_FLAGS" \
        -DCMAKE_CXX_FLAGS="$PATH_REMAP_FLAGS" \
        -DCMAKE_C_FLAGS_RELEASE="-O3 -DNDEBUG $PATH_REMAP_FLAGS" \
        -DCMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG $PATH_REMAP_FLAGS" \
        -DBUILD_SHARED_LIBS=ON \
        -DLLAMA_BUILD_COMMON=OFF \
        -DLLAMA_BUILD_TOOLS=OFF \
        -DLLAMA_BUILD_MTMD=ON \
        -DLLAMA_BUILD_APP=OFF \
        -DLLAMA_CURL=OFF \
        -DGGML_NATIVE=OFF \
        -DGGML_LLAMAFILE=OFF \
        -DGGML_OPENMP=ON \
        -DGGML_CCACHE=OFF \
        -DMTMD_VIDEO=OFF \
        -DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON \
        $GGML_FLAGS

    # Build
    "$CMAKE_BIN" --build "$BUILD_DIR" -j$(sysctl -n hw.ncpu)

    # Copy built libraries
    ABI_OUT_DIR="$ANDROID_LIBS_DIR/$ABI"
    rm -rf "$ABI_OUT_DIR"
    mkdir -p "$ABI_OUT_DIR"

    # Copy .so files from bin directory
    cp "$BUILD_DIR/bin/"*.so "$ABI_OUT_DIR/"

    # Also copy libmtmd.so if it's in a different location (some CMake versions put it elsewhere)
    find "$BUILD_DIR" -name "libmtmd.so" -exec cp {} "$ABI_OUT_DIR/" \; 2>/dev/null || true

    # Remove compiler debug sections before packaging release libraries.
    STRIP_BIN="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -path '*/bin/llvm-strip' -print -quit)"
    "$STRIP_BIN" --strip-debug "$ABI_OUT_DIR"/*.so

    # Copy libomp.so from NDK for OpenMP support
    case $ABI in
        "arm64-v8a")
            OMP_ARCH="aarch64"
            ;;
        "x86_64")
            OMP_ARCH="x86_64"
            ;;
    esac
    OMP_LIB="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -path "*/lib/clang/*/lib/linux/$OMP_ARCH/libomp.so" -print -quit)"
    if [ -f "$OMP_LIB" ]; then
        cp "$OMP_LIB" "$ABI_OUT_DIR/"
        echo "Copied libomp.so from NDK"
    fi

    for REQUIRED_LIB in libggml-base.so libggml-cpu.so libggml.so libllama.so libmtmd.so libomp.so; do
        if [ ! -f "$ABI_OUT_DIR/$REQUIRED_LIB" ]; then
            echo "ERROR: Required Android library missing: $ABI_OUT_DIR/$REQUIRED_LIB"
            exit 1
        fi
    done

    echo "Libraries for $ABI copied to $ABI_OUT_DIR"
    ls -la "$ABI_OUT_DIR"
done

echo ""
echo "Step 2: Building forge-rust for Android (with 16KB page alignment)"
echo "-------------------------------------------------------------------"

# Now build the Rust library
cd "$FORGE_RUST_DIR"

for ABI in "${ABIS[@]}"; do
    echo ""
    echo "Building Rust for $ABI..."

    # Map ABI to Rust target
    case $ABI in
        "arm64-v8a")
            RUST_TARGET="aarch64-linux-android"
            ;;
        "x86_64")
            RUST_TARGET="x86_64-linux-android"
            ;;
        "armeabi-v7a")
            RUST_TARGET="armv7-linux-androideabi"
            ;;
    esac

    # Set library path for linking
    export LLAMA_LIB_DIR="$ANDROID_LIBS_DIR/$ABI"

    # Set 16KB page alignment for Rust linker
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384 -C link-arg=-Wl,-soname,libforge_core.so"
    export CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384 -C link-arg=-Wl,-soname,libforge_core.so"

    # Build with cargo-ndk
    cargo ndk -t $ABI --platform $ANDROID_API -o "$ANDROID_LIBS_DIR" build --release -p forge-core

    if [ ! -f "$ANDROID_LIBS_DIR/$ABI/libforge_core.so" ]; then
        echo "ERROR: Rust library missing for $ABI"
        exit 1
    fi

    STRIP_BIN="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -path '*/bin/llvm-strip' -print -quit)"
    "$STRIP_BIN" --strip-debug "$ANDROID_LIBS_DIR/$ABI/libforge_core.so"

    echo "Built Rust library for $ABI"
done

echo ""
echo "=============================================="
echo "Build complete!"
echo "=============================================="
echo ""
echo "Output libraries are in: $ANDROID_LIBS_DIR"
ls -laR "$ANDROID_LIBS_DIR"
