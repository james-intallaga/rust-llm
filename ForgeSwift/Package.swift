// swift-tools-version: 5.9
// ForgeSwift - Swift wrapper for the Rust-based llama.cpp inference engine

import PackageDescription

let package = Package(
    name: "ForgeSwift",
    platforms: [
        .iOS("16.4"),
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "ForgeSwift",
            targets: ["ForgeSwift"]
        ),
    ],
    targets: [
        // Swift wrapper that provides a clean API
        .target(
            name: "ForgeSwift",
            dependencies: ["ForgeRustCore", "llama"],
            path: "Sources/ForgeSwift"
        ),

        // The Rust static library as a binary target
        // The module name "ForgeRustCore" comes from the framework name
        .binaryTarget(
            name: "ForgeRustCore",
            path: "ForgeRustCore.xcframework"
        ),

        // The llama.cpp framework (dynamic)
        .binaryTarget(
            name: "llama",
            path: "../llama.xcframework"
        ),
    ]
)
