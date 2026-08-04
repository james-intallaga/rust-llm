#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

for required in LICENSE README.md CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md THIRD_PARTY_NOTICES.md; do
  test -s "${required}" || { echo "Missing required file: ${required}"; exit 1; }
done

if git ls-files | grep -E '(^|/)(target|build|DerivedData|xcuserdata|\.cxx|\.gradle|\.idea|\.swiftpm)(/|$)|\.(dSYM|xcframework)(/|$)|\.(gguf|safetensors|pem|p8|p12|jks|keystore)$'; then
  echo "Generated, private, or oversized files are tracked."
  exit 1
fi

if git grep -I -n -E '/Users/[^/]+/|DEVELOPMENT_TEAM = [A-Z0-9]{10};|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY' -- ':!scripts/check-release.sh'; then
  echo "A local path, signing team, or private key marker is tracked."
  exit 1
fi

git submodule status --recursive >/dev/null
echo "Release hygiene checks passed."
