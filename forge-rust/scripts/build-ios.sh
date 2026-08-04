#!/bin/bash
# Build forge-core for iOS targets
# Creates static libraries for iOS device and simulators

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${PROJECT_DIR}/target/universal"

# Keep developer-local absolute paths out of distributed Rust archives.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${PROJECT_DIR}=."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Building forge-rust for iOS ===${NC}"
echo "Project dir: ${PROJECT_DIR}"
echo "Output dir: ${OUTPUT_DIR}"

# Check for required tools
command -v rustup >/dev/null 2>&1 || { echo -e "${RED}Error: rustup not found${NC}"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo -e "${RED}Error: cargo not found${NC}"; exit 1; }

# Ensure iOS targets are installed
echo -e "\n${YELLOW}Checking Rust targets...${NC}"
rustup target add aarch64-apple-ios 2>/dev/null || true
rustup target add aarch64-apple-ios-sim 2>/dev/null || true
rustup target add x86_64-apple-ios 2>/dev/null || true

# Create output directory
mkdir -p "${OUTPUT_DIR}"

# Function to build for a specific target
build_target() {
    local target=$1
    local description=$2

    echo -e "\n${YELLOW}Building for ${description} (${target})...${NC}"

    # Build the library
    cargo build --release --target "${target}" -p forge-core

    # Check if build succeeded
    local lib_path="${PROJECT_DIR}/target/${target}/release/libforge_core.a"
    if [[ -f "${lib_path}" ]]; then
        echo -e "${GREEN}✓ Built: ${lib_path}${NC}"
        ls -lh "${lib_path}"
    else
        echo -e "${RED}✗ Failed to build for ${target}${NC}"
        return 1
    fi
}

# Build for each target
cd "${PROJECT_DIR}"

# iOS device (arm64)
build_target "aarch64-apple-ios" "iOS Device (arm64)"

# iOS Simulator (arm64 - Apple Silicon Macs)
build_target "aarch64-apple-ios-sim" "iOS Simulator (arm64)"

# iOS Simulator (x86_64 - Intel Macs)
build_target "x86_64-apple-ios" "iOS Simulator (x86_64)"

# Build the native macOS slice used by ForgeSwift on macOS.
echo -e "\n${YELLOW}Building for macOS...${NC}"
cargo build --release -p forge-core

# Create universal binary for simulator (arm64 + x86_64)
echo -e "\n${YELLOW}Creating universal simulator library...${NC}"
lipo -create \
    "${PROJECT_DIR}/target/aarch64-apple-ios-sim/release/libforge_core.a" \
    "${PROJECT_DIR}/target/x86_64-apple-ios/release/libforge_core.a" \
    -output "${OUTPUT_DIR}/libforge_core-ios-sim.a"

echo -e "${GREEN}✓ Created universal simulator library${NC}"
ls -lh "${OUTPUT_DIR}/libforge_core-ios-sim.a"

# Copy device library
cp "${PROJECT_DIR}/target/aarch64-apple-ios/release/libforge_core.a" \
   "${OUTPUT_DIR}/libforge_core-ios.a"

echo -e "${GREEN}✓ Copied iOS device library${NC}"
ls -lh "${OUTPUT_DIR}/libforge_core-ios.a"

# Copy headers
mkdir -p "${OUTPUT_DIR}/include"
cp "${PROJECT_DIR}/include/forge_ffi.h" "${OUTPUT_DIR}/include/"

echo -e "\n${GREEN}=== Build Complete ===${NC}"
echo "Output files:"
echo "  - ${OUTPUT_DIR}/libforge_core-ios.a (device)"
echo "  - ${OUTPUT_DIR}/libforge_core-ios-sim.a (simulator universal)"
echo "  - ${OUTPUT_DIR}/include/forge_ffi.h"
echo ""
echo "Next step: Run ./scripts/package-xcframework.sh to create ForgeRustCore.xcframework"
