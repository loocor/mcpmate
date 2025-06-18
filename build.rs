//! Build script for MCPMate
//!
//! For now, we'll generate Swift bindings manually rather than using swift-bridge-build
//! due to toolchain complexity.

fn main() {
    // Print rerun conditions
    println!("cargo:rerun-if-changed=src/ffi/");
    println!("cargo:rerun-if-changed=build.rs");

    // Create directories for manual Swift binding generation
    #[cfg(feature = "ffi")]
    {
        std::fs::create_dir_all("./generated").unwrap_or_default();
        std::fs::create_dir_all("./swift-package").unwrap_or_default();

        println!("cargo:warning=FFI feature enabled. Swift bindings should be generated manually.");
        println!(
            "cargo:warning=Run: cargo build --features ffi --lib to generate libmcpmate.dylib"
        );
    }
}
