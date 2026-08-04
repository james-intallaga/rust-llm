//! Build script for forge-core
//!
//! Generates C headers using cbindgen for Swift interop.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = PathBuf::from(&crate_dir).parent().unwrap().join("include");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let framework_dir = PathBuf::from(&crate_dir)
            .parent()
            .and_then(|path| path.parent())
            .expect("forge-core must be inside the workspace")
            .join("llama.xcframework/macos-arm64_x86_64");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            framework_dir.display()
        );
    }

    // Create include directory if it doesn't exist
    std::fs::create_dir_all(&output_dir).ok();

    // Generate C header
    let config = cbindgen::Config::from_file("cbindgen.toml").unwrap_or_default();

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(output_dir.join("forge_ffi.h"));
            println!(
                "cargo:warning=Generated header: {:?}",
                output_dir.join("forge_ffi.h")
            );
        }
        Err(e) => {
            // Don't fail the build, just warn
            println!("cargo:warning=cbindgen failed: {}", e);
        }
    }
}
