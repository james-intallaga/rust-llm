# Contributing to Forge SDK

Thank you for helping improve private, on-device AI.

## Before you start

- Search existing issues and discussions before opening a duplicate.
- Use a focused branch and keep unrelated changes separate.
- Do not commit model weights, generated binaries, credentials, local paths, or
  machine-specific project files.
- For substantial API or architecture changes, open an issue first so the design
  can be discussed before implementation.

## Set up the repository

```bash
git clone --recurse-submodules <repository-url>
cd amma-forge-sdk
rustup show
```

The pinned Rust version is in `rust-toolchain.toml`. Platform build instructions
are in `docs/developer/building-from-source.md`.

## Validate a change

Run the checks relevant to your change. Rust changes must pass:

```bash
cd forge-rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets --no-fail-fast
```

Run `./scripts/check-release.sh` from the repository root before submitting a
pull request.

## Pull requests

- Explain the problem and the chosen solution.
- Include tests or explain why a test is not practical.
- Note the platforms and devices used for verification.
- Update public documentation when behavior or APIs change.
- Confirm that no personal data, secrets, model files, or generated binaries are
  included.

By contributing, you agree that your contribution is licensed under the MIT
License in this repository.
