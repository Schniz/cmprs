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
    let is_macos = cfg!(target_os = "macos");

    let dist_dir = Path::new(&out_dir).join("compiled_dcmprs");
    std::fs::create_dir_all(&dist_dir).expect("dist dir");

    // build universal macos
    if is_macos {
        // Use cargo zigbuild for universal binary on macOS
        let status = std::process::Command::new("cargo")
            .current_dir("../dcmprs")
            .args([
                "zigbuild",
                "--target",
                "universal2-apple-darwin",
                "--release",
            ])
            .status()
            .expect("Failed to build dcmprs with zigbuild");
        assert!(status.success());
        let binary_path = Path::new("../dcmprs/target")
            .join("universal2-apple-darwin")
            .join("release")
            .join(dcmprs_name);

        let dest_path = dist_dir.join("macos_universal");
        std::fs::copy(binary_path, &dest_path).expect("Failed to copy dcmprs binary");
    };

    // Standard cargo build
    let status = std::process::Command::new("cargo")
        .current_dir("../dcmprs")
        .args(["build", "--release"])
        .status()
        .expect("Failed to build dcmprs");
    assert!(status.success());

    // Try different target directories based on build type
    let dcmprs_path_specific = Path::new("../dcmprs/target")
        .join(&target)
        .join("release")
        .join(dcmprs_name);
    let dcmprs_path_general = Path::new("../dcmprs/target")
        .join("release")
        .join(dcmprs_name);

    let dcmprs_path = if dcmprs_path_specific.exists() {
        dcmprs_path_specific
    } else if dcmprs_path_general.exists() {
        dcmprs_path_general
    } else {
        panic!("dcmprs binary not found at {}, or {}. Please run 'cargo build --package dcmprs --release' first.", 
               dcmprs_path_specific.display(), dcmprs_path_general.display());
    };

    let dest_path = dist_dir.join("main");
    std::fs::copy(&dcmprs_path, &dest_path).expect("Failed to copy dcmprs binary");

    println!("cargo:rerun-if-changed=../dcmprs/src/main.rs");
    println!("cargo:rerun-if-changed=../dcmprs/Cargo.toml");
    println!("cargo:rerun-if-env-changed=BUILD_UNIVERSAL");
}
