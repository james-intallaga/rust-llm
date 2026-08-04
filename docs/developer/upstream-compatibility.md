# Upstream Compatibility

Forge pins its native and Rust inputs so platform builds are reproducible.

| Component | Pinned baseline | Policy |
|---|---|---|
| Rust | 1.97.0 | Controlled by `rust-toolchain.toml`; warnings and Clippy findings fail verification. |
| llama.cpp | b10012 (`c71854292f7c367cc3b35939f88121d81945472f`) | Git submodule; update deliberately and rebuild every native artifact. |
| Android NDK | 27.0.12077973 | Matches the Android Gradle configuration and produces 16KB-aligned shared libraries. |
| iOS | 15.0 minimum | Device and universal simulator slices are packaged into XCFrameworks. |

## Upgrade checklist

1. Move the llama.cpp submodule to a tagged release, not an arbitrary working-tree snapshot.
2. Build `llama.xcframework`, including mtmd, without modifying upstream source files.
3. Regenerate Rust bindings and `forge_ffi.h`.
4. Run strict Rust checks, model-backed text and multi-turn tests, and strict documentation builds.
5. Rebuild the Forge Rust XCFramework and both Android ABIs.
6. Build ForgeSwift, the Android library, and the example Android application.
7. Verify every packaged Android `.so` has a 16KB (`0x4000`) LOAD alignment.

## Model compatibility

Architecture support in llama.cpp does not guarantee that every community GGUF is valid. Vision and audio models also require a matching projector file and prompt/media marker. The model-backed Forge test uses Qwen 2.5 1.5B Instruct Q4_K_M; other models listed in [Recommended Models](../troubleshooting-and-todos/recommended-models.md) must be tested on the target device before release.
