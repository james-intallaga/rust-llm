#!/bin/bash
# Package ForgeRustCore.xcframework from built static libraries
# Run build-ios.sh first to create the static libraries

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_DIR="$(dirname "$PROJECT_DIR")"
INPUT_DIR="${PROJECT_DIR}/target/universal"
OUTPUT_DIR="${PROJECT_DIR}/build"
FRAMEWORK_NAME="ForgeRustCore"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Packaging ${FRAMEWORK_NAME}.xcframework ===${NC}"

# Check input files exist
if [[ ! -f "${INPUT_DIR}/libforge_core-ios.a" ]]; then
    echo -e "${RED}Error: iOS device library not found. Run build-ios.sh first.${NC}"
    exit 1
fi

if [[ ! -f "${INPUT_DIR}/libforge_core-ios-sim.a" ]]; then
    echo -e "${RED}Error: iOS simulator library not found. Run build-ios.sh first.${NC}"
    exit 1
fi

if [[ ! -f "${INPUT_DIR}/include/forge_ffi.h" ]]; then
    echo -e "${RED}Error: Header file not found. Run build-ios.sh first.${NC}"
    exit 1
fi

# Create output directory
mkdir -p "${OUTPUT_DIR}"
rm -rf "${OUTPUT_DIR}/tmp"
mkdir -p "${OUTPUT_DIR}/tmp/ios"
mkdir -p "${OUTPUT_DIR}/tmp/ios-simulator"
mkdir -p "${OUTPUT_DIR}/tmp/macos"

# Create proper framework bundles for each platform
# Note: Framework directory must be named exactly "FrameworkName.framework"
# and contain a binary named exactly "FrameworkName"
create_framework_bundle() {
    local output_subdir=$1
    local lib_path=$2
    local framework_dir="${OUTPUT_DIR}/tmp/${output_subdir}/${FRAMEWORK_NAME}.framework"

    # Status messages go to stderr so they don't get captured with the return value
    echo -e "${YELLOW}Creating framework bundle for ${output_subdir}...${NC}" >&2

    mkdir -p "${framework_dir}/Headers"
    mkdir -p "${framework_dir}/Modules"

    # Copy the static library as the framework binary
    cp "${lib_path}" "${framework_dir}/${FRAMEWORK_NAME}"

    # Copy headers
    cp "${INPUT_DIR}/include/forge_ffi.h" "${framework_dir}/Headers/"

    # Create module.modulemap
    # The module name MUST match the framework bundle name for SPM to work correctly
    cat > "${framework_dir}/Modules/module.modulemap" << EOF
framework module ${FRAMEWORK_NAME} {
    umbrella header "forge_ffi.h"
    export *
    module * { export * }

    link "c++"
}
EOF

    # Create Info.plist
    cat > "${framework_dir}/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${FRAMEWORK_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.forge.${FRAMEWORK_NAME}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${FRAMEWORK_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>MinimumOSVersion</key>
    <string>15.0</string>
</dict>
</plist>
EOF

    echo -e "  Created: ${framework_dir}" >&2
    # Return just the path (to stdout)
    echo "${framework_dir}"
}

# Create framework bundles
IOS_FRAMEWORK=$(create_framework_bundle "ios" "${INPUT_DIR}/libforge_core-ios.a")
SIM_FRAMEWORK=$(create_framework_bundle "ios-simulator" "${INPUT_DIR}/libforge_core-ios-sim.a")

# Create macOS framework if available
MACOS_LIB="${PROJECT_DIR}/target/release/libforge_core.a"
MACOS_FRAMEWORK=""
if [[ -f "${MACOS_LIB}" ]]; then
    MACOS_FRAMEWORK=$(create_framework_bundle "macos" "${MACOS_LIB}")
else
    echo -e "${YELLOW}Note: macOS library not found, skipping macOS in xcframework${NC}"
fi

# Remove existing xcframework
XCFRAMEWORK_PATH="${OUTPUT_DIR}/${FRAMEWORK_NAME}.xcframework"
rm -rf "${XCFRAMEWORK_PATH}"

# Create xcframework
echo -e "\n${YELLOW}Creating ${FRAMEWORK_NAME}.xcframework...${NC}"

if [[ -n "${MACOS_FRAMEWORK}" && -d "${MACOS_FRAMEWORK}" ]]; then
    xcodebuild -create-xcframework \
        -framework "${IOS_FRAMEWORK}" \
        -framework "${SIM_FRAMEWORK}" \
        -framework "${MACOS_FRAMEWORK}" \
        -output "${XCFRAMEWORK_PATH}"
else
    xcodebuild -create-xcframework \
        -framework "${IOS_FRAMEWORK}" \
        -framework "${SIM_FRAMEWORK}" \
        -output "${XCFRAMEWORK_PATH}"
fi

# Cleanup temporary frameworks
rm -rf "${OUTPUT_DIR}/tmp"

# Verify
echo -e "\n${GREEN}=== XCFramework Created ===${NC}"
echo "Location: ${XCFRAMEWORK_PATH}"
echo ""
echo "Contents:"
ls -la "${XCFRAMEWORK_PATH}"
echo ""
echo "Slices:"
for slice in "${XCFRAMEWORK_PATH}"/*; do
    if [[ -d "$slice" ]]; then
        echo "  - $(basename "$slice")"
    fi
done

# Show size
echo ""
echo "Total size:"
du -sh "${XCFRAMEWORK_PATH}"

echo -e "\n${GREEN}Done!${NC}"
echo "Copy ${XCFRAMEWORK_PATH} to your Swift project."
