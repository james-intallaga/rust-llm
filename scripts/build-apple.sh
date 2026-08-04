#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

git -C "${ROOT_DIR}" submodule update --init --recursive

"${ROOT_DIR}/build-xcframework.sh"
"${ROOT_DIR}/forge-rust/scripts/build-ios.sh"
"${ROOT_DIR}/forge-rust/scripts/package-xcframework.sh"

rsync -a --delete \
  "${ROOT_DIR}/forge-rust/build/ForgeRustCore.xcframework/" \
  "${ROOT_DIR}/ForgeSwift/ForgeRustCore.xcframework/"

echo "Apple frameworks are ready for ForgeSwift."
