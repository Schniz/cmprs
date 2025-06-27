use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("TARGET").unwrap();

    // Copy the dcmprs binary to OUT_DIR (must be built first)
    let dcmprs_name = if target.contains("windows") {
        "dcmprs.exe"
    } else {
        "dcmprs"
    };

    // Check if we should build universal binary on macOS
    let build_universal = std::env::var("BUILD_UNIVERSAL").unwrap_or_default() == "1";
    let is_macos = cfg!(target_os = "macos");

    let status = if build_universal && is_macos {
        // Use cargo zigbuild for universal binary on macOS
        std::process::Command::new("cargo")
            .current_dir("../dcmprs")
            .args([
                "zigbuild",
                "--target",
                "universal2-apple-darwin",
                "--release",
            ])
            .status()
            .expect("Failed to build dcmprs with zigbuild")
    } else {
        // Standard cargo build
        std::process::Command::new("cargo")
            .current_dir("../dcmprs")
            .args(["build", "--release"])
            .status()
            .expect("Failed to build dcmprs")
    };
    assert!(status.success());

    // Try different target directories based on build type
    let build_universal = std::env::var("BUILD_UNIVERSAL").unwrap_or_default() == "1";
    let is_macos = cfg!(target_os = "macos");

    let dcmprs_path_universal = Path::new("../dcmprs/target")
        .join("universal2-apple-darwin")
        .join("release")
        .join(dcmprs_name);
    let dcmprs_path_specific = Path::new("../dcmprs/target")
        .join(&target)
        .join("release")
        .join(dcmprs_name);
    let dcmprs_path_general = Path::new("../dcmprs/target")
        .join("release")
        .join(dcmprs_name);

    let dcmprs_path = if build_universal && is_macos && dcmprs_path_universal.exists() {
        dcmprs_path_universal
    } else if dcmprs_path_specific.exists() {
        dcmprs_path_specific
    } else if dcmprs_path_general.exists() {
        dcmprs_path_general
    } else {
        panic!("dcmprs binary not found at {}, {}, or {}. Please run 'cargo build --package dcmprs --release' first.", 
               dcmprs_path_universal.display(), dcmprs_path_specific.display(), dcmprs_path_general.display());
    };

    let dest_path = Path::new(&out_dir).join("dcmprs");

    std::fs::copy(&dcmprs_path, &dest_path).expect("Failed to copy dcmprs binary");

    println!("cargo:rerun-if-changed=../dcmprs/src/main.rs");
    println!("cargo:rerun-if-changed=../dcmprs/Cargo.toml");
    println!("cargo:rerun-if-env-changed=BUILD_UNIVERSAL");
}
