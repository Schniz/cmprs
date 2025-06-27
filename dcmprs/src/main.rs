use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::{self, Command};
use std::thread;
use tempfile::NamedTempFile;
use zstd::stream::read::Decoder as ZstdDecoder;

// Same magic header as in cmprs
const MAGIC_HEADER: &[u8; 16] = b"DCMPRS_DATA_HERE";

fn main() -> io::Result<()> {
    let current_exe = env::current_exe()?;
    let mut file = File::open(&current_exe)?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Find the boundary between the dcmprs executable and the magic header
    let magic_pos = find_magic_header(&buffer);

    if magic_pos.is_none() {
        eprintln!("No magic header found - this may not be a dcmprs-compressed file");
        process::exit(1);
    }

    let magic_pos = magic_pos.unwrap();
    let data_start = magic_pos + MAGIC_HEADER.len();

    if data_start + 32 >= buffer.len() {
        eprintln!("No SHA256 hash or compressed data found after magic header");
        process::exit(1);
    }

    // Skip the SHA256 hash (32 bytes after magic header) and get compressed data
    let compressed_data = &buffer[data_start + 32..];

    // Decompress the data
    let mut decoder = ZstdDecoder::new(compressed_data)?;
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;

    // Create a temporary file to write the decompressed content
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(&decompressed_data)?;

    // Make sure the temp file is executable
    let metadata = temp_file.as_file().metadata()?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    temp_file.as_file().set_permissions(permissions)?;

    let temp_path = temp_file.path().to_path_buf();

    // Collect command line arguments (excluding the program name)
    let args: Vec<String> = env::args().skip(1).collect();

    // Clone data needed for the replacement thread
    let current_exe_clone = current_exe.clone();
    let decompressed_data_clone = decompressed_data.clone();

    // Start replacement in parallel
    let replacement_handle = thread::spawn(move || {
        // Write decompressed content directly to original file
        if let Ok(mut output_file) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&current_exe_clone)
        {
            let _ = output_file.write_all(&decompressed_data_clone);
            let _ = output_file.sync_all();
        }
    });

    // Execute the decompressed file with the provided arguments and environment
    // This replaces the current process entirely
    let mut cmd = Command::new(&temp_path);
    cmd.args(&args);

    // Preserve all environment variables
    for (key, value) in env::vars() {
        cmd.env(key, value);
    }

    // Wait for replacement to complete before exec
    let _ = replacement_handle.join();

    // Keep temp file alive until exec
    let _temp_file_guard = temp_file;

    // Replace current process with the decompressed executable
    // This never returns if successful
    let err = cmd.exec();

    // If we get here, exec failed
    Err(err)
}

/// Look for our custom magic header
/// The format is: [dcmprs executable][MAGIC_HEADER][32-byte SHA256][zstd compressed data]
/// Search from the beginning to find the FIRST occurrence
fn find_magic_header(buffer: &[u8]) -> Option<usize> {
    (0..buffer.len().saturating_sub(MAGIC_HEADER.len()))
        .find(|&i| &buffer[i..i + MAGIC_HEADER.len()] == MAGIC_HEADER)
}

