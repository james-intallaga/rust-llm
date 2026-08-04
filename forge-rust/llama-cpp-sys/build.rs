//! Build script for llama-cpp-sys
//!
//! Generates Rust FFI bindings from llama.cpp C headers using bindgen,
//! and configures linking against prebuilt llama libraries.
//!
//! Supported platforms:
//! - macOS: Links against llama.xcframework
//! - iOS: Links against llama.xcframework
//! - Android: Links against prebuilt .so files

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();

    // Determine target platform
    let target = env::var("TARGET").unwrap_or_else(|_| "aarch64-apple-darwin".to_string());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "macos".to_string());

    println!("cargo:warning=Target: {}", target);
    println!("cargo:warning=Target OS: {}", target_os);

    // Dispatch to platform-specific handling
    match target_os.as_str() {
        "android" => configure_android(project_root, &target, &manifest_dir),
        "macos" | "ios" => configure_apple(project_root, &target, &target_os, &manifest_dir),
        _ => {
            println!("cargo:warning=Unsupported target OS: {}", target_os);
        }
    }
}

/// Configure linking for Android
fn configure_android(project_root: &Path, target: &str, _manifest_dir: &Path) {
    // Map Rust target to Android ABI
    let abi = match target {
        "aarch64-linux-android" => "arm64-v8a",
        "armv7-linux-androideabi" => "armeabi-v7a",
        "i686-linux-android" => "x86",
        "x86_64-linux-android" => "x86_64",
        _ => {
            println!("cargo:warning=Unknown Android target: {}", target);
            return;
        }
    };

    // Check for LLAMA_LIB_DIR environment variable first
    let lib_dir = if let Ok(dir) = env::var("LLAMA_LIB_DIR") {
        PathBuf::from(dir)
    } else {
        // Default to build/android/{abi}
        project_root
            .join("forge-rust")
            .join("build")
            .join("android")
            .join(abi)
    };

    println!("cargo:warning=Android ABI: {}", abi);
    println!("cargo:warning=Library dir: {:?}", lib_dir);

    if !lib_dir.exists() {
        println!(
            "cargo:warning=Android libraries not found at {:?}. Run scripts/build-android.sh first.",
            lib_dir
        );
        return;
    }

    // Tell Cargo where to find the libraries
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Android packages mtmd separately from llama. Declare both shared-library
    // dependencies so libforge_core carries DT_NEEDED entries for the dynamic
    // loader; preloading mtmd from Kotlin is not enough on modern Android,
    // where dlopen defaults to local symbol visibility.
    println!("cargo:rustc-link-lib=dylib=mtmd");
    println!("cargo:rustc-link-lib=dylib=llama");

    // Android system libraries
    println!("cargo:rustc-link-lib=log");
    println!("cargo:rustc-link-lib=android");

    // C++ standard library for Android
    println!("cargo:rustc-link-lib=c++_shared");

    // Use stub bindings for Android (generated on macOS)
    println!("cargo:warning=Using pre-generated bindings for Android");
}

/// Configure linking for Apple platforms (macOS, iOS)
fn configure_apple(
    project_root: &std::path::Path,
    target: &str,
    target_os: &str,
    manifest_dir: &Path,
) {
    let xcframework_path = project_root.join("llama.xcframework");

    // Select the correct framework slice based on target
    let (framework_slice, headers_path) = get_framework_paths(&xcframework_path, target, target_os);

    println!("cargo:warning=Framework slice: {:?}", framework_slice);
    println!("cargo:warning=Headers path: {:?}", headers_path);

    // Check if framework exists
    if !framework_slice.exists() {
        println!(
            "cargo:warning=Framework not found at {:?}, skipping linking",
            framework_slice
        );
        return;
    }

    // Configure linking against the dynamic framework
    configure_apple_linking(&framework_slice, target_os);

    // Skip bindgen for iOS cross-compilation (use stubs instead)
    // Bindgen runs on the host machine and can't parse iOS headers properly
    if target_os == "ios" {
        println!("cargo:warning=Cross-compiling for iOS, using stub bindings (bindgen skipped)");
        return;
    }

    // Generate bindings if headers exist (macOS only for now)
    if headers_path.exists() {
        generate_bindings(&headers_path, manifest_dir);
    } else {
        println!(
            "cargo:warning=Headers not found at {:?}, using stub bindings",
            headers_path
        );
    }
}

/// Get the correct framework slice and headers path for Apple targets
fn get_framework_paths(
    xcframework_path: &Path,
    target: &str,
    target_os: &str,
) -> (PathBuf, PathBuf) {
    // Determine if this is a simulator target
    // x86_64-apple-ios is always simulator (no x86_64 iOS devices exist)
    // aarch64-apple-ios-sim is explicitly simulator
    let is_simulator = target.contains("sim")
        || target.contains("simulator")
        || (target_os == "ios" && target.contains("x86_64"));

    let slice_name = match (target_os, is_simulator) {
        // macOS (universal binary)
        ("macos", _) => "macos-arm64_x86_64",
        // iOS simulator (arm64 + x86_64 universal)
        ("ios", true) => "ios-arm64_x86_64-simulator",
        // iOS device (arm64 only)
        ("ios", false) => "ios-arm64",
        // Default to macOS for development
        _ => "macos-arm64_x86_64",
    };

    let framework_slice = xcframework_path.join(slice_name).join("llama.framework");

    let headers_path = framework_slice.join("Headers");

    (framework_slice, headers_path)
}

/// Configure Cargo to link against the llama framework (Apple)
fn configure_apple_linking(framework_path: &Path, target_os: &str) {
    // Get the parent directory containing the framework
    let framework_dir = framework_path.parent().unwrap();

    // Tell Cargo where to find the framework
    println!(
        "cargo:rustc-link-search=framework={}",
        framework_dir.display()
    );

    // Link against the llama framework (dynamic)
    println!("cargo:rustc-link-lib=framework=llama");

    // Cargo test binaries need an explicit runtime search path for the dynamic
    // framework. Application bundles still provide their normal @rpath entries.
    if target_os == "macos" {
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            framework_dir.display()
        );
    }

    // On macOS/iOS, we also need Metal and other system frameworks
    if target_os == "macos" || target_os == "ios" {
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }

    // For C++ interop (llama.cpp is C++)
    println!("cargo:rustc-link-lib=c++");

    println!(
        "cargo:warning=Configured linking: framework=llama at {}",
        framework_dir.display()
    );
}

/// Generate Rust bindings from C headers using bindgen
fn generate_bindings(headers_path: &Path, manifest_dir: &Path) {
    let llama_h = headers_path.join("llama.h");
    let mtmd_h = headers_path.join("mtmd.h");
    let ggml_h = headers_path.join("ggml.h");

    println!("cargo:rerun-if-changed={}", llama_h.display());
    println!("cargo:rerun-if-changed={}", mtmd_h.display());
    println!("cargo:rerun-if-changed={}", ggml_h.display());

    // Only generate if llama.h exists
    if !llama_h.exists() {
        println!("cargo:warning=llama.h not found, skipping bindgen");
        return;
    }

    // Generate bindings for llama.h
    let llama_bindings = bindgen::Builder::default()
        .header(llama_h.to_string_lossy())
        .clang_arg(format!("-I{}", headers_path.display()))
        // Only generate bindings for llama_ prefixed items
        .allowlist_function("llama_.*")
        .allowlist_type("llama_.*")
        .allowlist_var("LLAMA_.*")
        // Also include ggml types that llama.h depends on
        .allowlist_type("ggml_.*")
        .allowlist_function("ggml_.*")
        // Parse C++ as C
        .clang_arg("-xc")
        .clang_arg("-std=c11")
        // Generate Rust-friendly types
        .derive_debug(true)
        .derive_default(true)
        .derive_copy(true)
        .impl_debug(true)
        .size_t_is_usize(true)
        // Don't generate layout tests (they fail on cross-compilation)
        .layout_tests(false)
        .generate()
        .expect("Failed to generate llama bindings");

    // Generate bindings for mtmd.h (multimodal) if it exists
    let mtmd_bindings = if mtmd_h.exists() {
        Some(
            bindgen::Builder::default()
                .header(mtmd_h.to_string_lossy())
                .clang_arg(format!("-I{}", headers_path.display()))
                // Only generate bindings for mtmd_ prefixed items
                .allowlist_function("mtmd_.*")
                .allowlist_type("mtmd_.*")
                .allowlist_var("MTMD_.*")
                // Block llama_ types - they're already in llama_bindings.rs
                .blocklist_type("llama_.*")
                .blocklist_function("llama_.*")
                .blocklist_type("ggml_.*")
                .blocklist_function("ggml_.*")
                // Parse as C
                .clang_arg("-xc")
                .clang_arg("-std=c11")
                // Generate Rust-friendly types
                .derive_debug(true)
                .derive_default(true)
                .derive_copy(true)
                .impl_debug(true)
                .size_t_is_usize(true)
                .layout_tests(false)
                .generate()
                .expect("Failed to generate mtmd bindings"),
        )
    } else {
        println!("cargo:warning=mtmd.h not found, skipping multimodal bindings");
        None
    };

    // Write bindings to src/
    let out_dir = manifest_dir.join("src");

    llama_bindings
        .write_to_file(out_dir.join("llama_bindings.rs"))
        .expect("Failed to write llama bindings");

    if let Some(bindings) = mtmd_bindings {
        bindings
            .write_to_file(out_dir.join("mtmd_bindings.rs"))
            .expect("Failed to write mtmd bindings");
    }

    println!("cargo:warning=Generated bindings to {:?}", out_dir);
}
