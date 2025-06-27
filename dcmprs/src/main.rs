use log::{debug, info, warn};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::{self, Command};
use std::thread;
use std::time::Instant;
use tempfile::NamedTempFile;
use zstd::stream::read::Decoder as ZstdDecoder;

// Same magic header as in cmprs
const MAGIC_HEADER: &[u8; 16] = b"DCMPRS_DATA_HERE";

fn main() -> io::Result<()> {
    // Initialize logger with custom environment variable
    env_logger::Builder::from_env(env_logger::Env::new().filter("DCMPRS_LOG_LEVEL")).init();

    let start_time = Instant::now();
    let current_exe = env::current_exe()?;
    info!(
        "Starting dcmprs decompression for: {}",
        current_exe.display()
    );

    debug!("Opening current executable file");
    let mut file = File::open(&current_exe)?;

    debug!("Reading executable file into memory");
    let read_start = Instant::now();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    info!("Read {} bytes in {:?}", buffer.len(), read_start.elapsed());

    // Find the boundary between the dcmprs executable and the magic header
    debug!("Searching for magic header in {} byte buffer", buffer.len());
    let magic_pos = find_magic_header(&buffer);

    if magic_pos.is_none() {
        warn!("No magic header found - this may not be a dcmprs-compressed file");
        process::exit(1);
    }

    let magic_pos = magic_pos.unwrap();
    info!("Found magic header at position {}", magic_pos);
    let data_start = magic_pos + MAGIC_HEADER.len();

    if data_start + 32 >= buffer.len() {
        warn!("No SHA256 hash or compressed data found after magic header");
        process::exit(1);
    }

    debug!(
        "Data starts at position {} (after magic header + SHA256)",
        data_start + 32
    );

    // Skip the SHA256 hash (32 bytes after magic header) and get compressed data
    let compressed_data = &buffer[data_start + 32..];
    info!("Found {} bytes of compressed data", compressed_data.len());

    // Decompress the data
    debug!("Starting zstd decompression");
    let decompress_start = Instant::now();
    let mut decoder = ZstdDecoder::new(compressed_data)?;
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    let decompress_time = decompress_start.elapsed();
    info!(
        "Decompressed {} bytes to {} bytes in {:?}",
        compressed_data.len(),
        decompressed_data.len(),
        decompress_time
    );

    // Create a temporary file to write the decompressed content
    debug!("Creating temporary file for decompressed content");
    let temp_start = Instant::now();
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(&decompressed_data)?;

    // Make sure the temp file is executable
    let metadata = temp_file.as_file().metadata()?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    temp_file.as_file().set_permissions(permissions)?;

    let temp_path = temp_file.path().to_path_buf();
    info!(
        "Created executable temp file at {} in {:?}",
        temp_path.display(),
        temp_start.elapsed()
    );

    // Collect command line arguments (excluding the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    debug!("Command line arguments: {:?}", args);

    // Clone data needed for the replacement thread
    let current_exe_clone = current_exe.clone();
    let decompressed_data_clone = decompressed_data.clone();

    // Start replacement in parallel
    debug!("Starting parallel file replacement thread");
    let replacement_handle = thread::spawn(move || {
        let replace_start = Instant::now();
        // Write decompressed content directly to original file
        if let Ok(mut output_file) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&current_exe_clone)
        {
            let _ = output_file.write_all(&decompressed_data_clone);
            let _ = output_file.sync_all();
            debug!(
                "File replacement completed in {:?}",
                replace_start.elapsed()
            );
        } else {
            warn!("Failed to open original file for replacement");
        }
    });

    // Execute the decompressed file with the provided arguments and environment
    // This replaces the current process entirely
    debug!(
        "Preparing to exec decompressed program: {}",
        temp_path.display()
    );
    let mut cmd = Command::new(&temp_path);
    cmd.args(&args);

    // Preserve all environment variables
    let env_count = env::vars().count();
    debug!("Preserving {} environment variables", env_count);
    for (key, value) in env::vars() {
        cmd.env(key, value);
    }

    // Wait for replacement to complete before exec
    debug!("Waiting for file replacement to complete");
    let _ = replacement_handle.join();

    info!("Total dcmprs processing time: {:?}", start_time.elapsed());
    info!("Executing decompressed program with exec()");

    // Keep temp file alive until exec
    let _temp_file_guard = temp_file;

    // Replace current process with the decompressed executable
    // This never returns if successful
    let err = cmd.exec();

    // If we get here, exec failed
    warn!("exec() failed: {}", err);
    Err(err)
}

/// Look for our custom magic header
/// The format is: [dcmprs executable][MAGIC_HEADER][32-byte SHA256][zstd compressed data]
/// Search from the beginning to find the FIRST occurrence
fn find_magic_header(buffer: &[u8]) -> Option<usize> {
    (0..buffer.len().saturating_sub(MAGIC_HEADER.len()))
        .find(|&i| &buffer[i..i + MAGIC_HEADER.len()] == MAGIC_HEADER)
}

