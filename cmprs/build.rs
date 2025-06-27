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

    let status = std::process::Command::new("cargo")
        .current_dir("../dcmprs")
        .args(["build", "--release"])
        .status()
        .expect("Failed to build dcmprs");
    assert!(status.success());

    // Try both target-specific and general release directories
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
        panic!("dcmprs binary not found at {} or {}. Please run 'cargo build --package dcmprs --release' first.", 
               dcmprs_path_specific.display(), dcmprs_path_general.display());
    };

    let dest_path = Path::new(&out_dir).join("dcmprs");

    // Check that dcmprs binary doesn't contain our magic header
    let dcmprs_data = std::fs::read(&dcmprs_path).expect("Failed to read dcmprs binary");
    let magic_header = b"DCMPRS_DATA_HERE";

    if dcmprs_data
        .windows(magic_header.len())
        .any(|window| window == magic_header)
    {
        panic!("ERROR: dcmprs binary contains the magic header '{}'. This would break boundary detection. Please change the magic header to something else.", 
               String::from_utf8_lossy(magic_header));
    }

    std::fs::copy(&dcmprs_path, &dest_path).expect("Failed to copy dcmprs binary");

    println!("cargo:rerun-if-changed=../dcmprs/src/main.rs");
    println!("cargo:rerun-if-changed=../dcmprs/Cargo.toml");
}
