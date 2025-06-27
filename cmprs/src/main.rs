use clap::Parser;
use include_dir::{include_dir, Dir};
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use zstd::stream::write::Encoder as ZstdEncoder;

static DIST_DIR: Dir = include_dir!("$OUT_DIR/compiled_dcmprs");

// Custom magic header to mark the boundary between dcmprs executable and compressed data
// Using a unique 16-byte sequence that's unlikely to appear in binaries
const MAGIC_HEADER: &[u8; 16] = b"DCMPRS_DATA_HERE";

#[cfg(not(windows))]
const SUFFIX: &str = "cmprs";
#[cfg(windows)]
const SUFFIX: &str = "cmprs.exe";

#[derive(Parser)]
#[command(name = "cmprs")]
#[command(about = "Creates self-extracting zstd compressed executables")]
#[command(version)]
struct Args {
    #[arg(
        short,
        long,
        help = "Output file. If not specified, defaults to <input>.<suffix>"
    )]
    output: Option<PathBuf>,

    #[arg(help = "Input file")]
    input: PathBuf,

    #[arg(
        short = 'l',
        long = "level",
        default_value = "3",
        help = "Compression level (1-22, higher = better compression but slower)"
    )]
    compression_level: i32,

    #[arg(
        long,
        default_value = "false",
        help = "Build universal macOS binary",
        hide = cfg!(not(target_os = "macos")),
    )]
    build_universal_macos: bool,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let start_time = Instant::now();

    let args = Args::parse();
    let output_path = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}.{SUFFIX}", args.input.display())));

    info!(
        "Starting compression of {} to {}",
        args.input.display(),
        output_path.display(),
    );

    // Read input file and check permissions
    debug!("Reading input file: {}", args.input.display());
    let read_start = Instant::now();
    let mut input = Vec::new();
    let input_file = File::open(&args.input)?;
    let input_metadata = input_file.metadata()?;
    let input_permissions = input_metadata.permissions();
    let is_executable = input_permissions.mode() & 0o111 != 0;

    if !is_executable {
        warn!("Input file '{}' is not executable", args.input.display());
    }

    let mut input_file = input_file;
    input_file.read_to_end(&mut input)?;
    info!("Read {} bytes in {:?}", input.len(), read_start.elapsed());

    let input_len = input.len();
    info!("Input size: {:.2} MB", input_len as f64 / 1_048_576.0);

    // Share input data between threads using Arc
    debug!("Creating Arc for input data ({} bytes)", input_len);
    let arc_start = Instant::now();
    let input_data = Arc::new(input);
    info!("Arc creation took {:?}", arc_start.elapsed());

    // Start SHA256 calculation in a separate thread
    debug!("Starting SHA256 calculation thread");
    let input_for_hash = Arc::clone(&input_data);
    let hash_thread = thread::spawn(move || {
        let hash_start = Instant::now();
        debug!("SHA256 thread: starting hash calculation");

        let mut hasher = Sha256::new();
        let chunk_size = 1024 * 1024; // 1MB chunks
        let data = &*input_for_hash;

        for (i, chunk) in data.chunks(chunk_size).enumerate() {
            hasher.update(chunk);
            if i % 10 == 0 {
                debug!(
                    "SHA256 thread: processed {} MB",
                    (i + 1) * chunk_size / 1_048_576
                );
            }
        }

        let sha256_hash = hasher.finalize();
        let elapsed = hash_start.elapsed();
        let throughput = data.len() as f64 / elapsed.as_secs_f64() / 1_048_576.0;
        info!(
            "SHA256 calculated in {:?} ({:.1} MB/s): {}",
            elapsed,
            throughput,
            hex::encode(sha256_hash)
        );
        (sha256_hash, elapsed)
    });

    // Start compression in a separate thread
    debug!("Starting compression thread");
    let input_for_compress = Arc::clone(&input_data);
    let compression_level = args.compression_level;

    let compress_thread = thread::spawn(move || {
        let compress_start = Instant::now();
        debug!(
            "Compression thread: starting Zstd compression (level {})",
            compression_level
        );

        let mut compressed = Vec::new();
        {
            let mut encoder = ZstdEncoder::new(&mut compressed, compression_level)
                .expect("Failed to create Zstd encoder");

            let data = &*input_for_compress;
            let chunk_size = 64 * 1024; // 64KB chunks for compression

            for (i, chunk) in data.chunks(chunk_size).enumerate() {
                encoder
                    .write_all(chunk)
                    .expect("Failed to write to encoder");
                if i % 100 == 0 {
                    debug!(
                        "Compression thread: processed {} MB",
                        (i + 1) * chunk_size / 1_048_576
                    );
                }
            }

            encoder.finish().expect("Failed to finish compression");
        }

        let elapsed = compress_start.elapsed();
        let compression_ratio = compressed.len() as f64 / input_for_compress.len() as f64;
        let throughput = input_for_compress.len() as f64 / elapsed.as_secs_f64() / 1_048_576.0;
        info!(
            "Compressed {} bytes to {} bytes ({:.1}%) in {:?} ({:.1} MB/s)",
            input_for_compress.len(),
            compressed.len(),
            compression_ratio * 100.0,
            elapsed,
            throughput
        );

        if elapsed.as_secs() > 5 {
            warn!(
                "Compression took longer than 5 seconds - consider using a lower compression level"
            );
        }

        (compressed, elapsed)
    });

    // Meanwhile, start writing the output file with dcmprs executable
    debug!("Loading embedded dcmprs executable");
    let embed_start = Instant::now();
    let dcmprs_file = if args.build_universal_macos {
        DIST_DIR.get_file("macos_universal").or_else(|| {
            log::error!("Universal macOS binary not found, falling back to main dcmprs");
            DIST_DIR.get_file("main")
        })
    } else {
        DIST_DIR.get_file("main")
    };
    let dcmprs_data = dcmprs_file.unwrap().contents();
    info!(
        "Loaded {} byte dcmprs executable in {:?}",
        dcmprs_data.len(),
        embed_start.elapsed()
    );

    debug!(
        "Creating output file and writing dcmprs executable: {}",
        output_path.display()
    );
    let write_start = Instant::now();
    let mut output = File::create(&output_path)?;
    output.write_all(dcmprs_data)?;
    output.write_all(MAGIC_HEADER)?;
    output.write_all(b";;;")?;
    let dcmprs_write_time = write_start.elapsed();
    info!(
        "Wrote {} byte dcmprs executable + magic header in {:?}",
        dcmprs_data.len() + MAGIC_HEADER.len() + 3,
        dcmprs_write_time
    );

    // Wait for SHA256 calculation to complete and write it
    debug!("Waiting for SHA256 calculation to complete");
    let (sha256_hash, hash_duration) = hash_thread.join().expect("SHA256 thread panicked");
    let sha_write_start = Instant::now();
    output.write_all(&sha256_hash)?;
    let sha_write_time = sha_write_start.elapsed();
    info!("Wrote 32-byte SHA256 hash in {:?}", sha_write_time);

    // Wait for compression to complete and write it
    debug!("Waiting for compression to complete");
    let (compressed, compress_duration) =
        compress_thread.join().expect("Compression thread panicked");
    let compress_write_start = Instant::now();
    output.write_all(&compressed)?;
    let compress_write_time = compress_write_start.elapsed();
    info!(
        "Wrote {} byte compressed data in {:?}",
        compressed.len(),
        compress_write_time
    );

    // Copy permissions from the original file to the compressed file
    debug!("Copying permissions from input to output file");
    let perm_start = Instant::now();
    output.set_permissions(input_permissions)?;
    info!("Set permissions in {:?}", perm_start.elapsed());

    let total_size = dcmprs_data.len() + MAGIC_HEADER.len() + sha256_hash.len() + compressed.len();
    let total_write_time = dcmprs_write_time + sha_write_time + compress_write_time;
    info!(
        "Total output: {} bytes written in {:?}",
        total_size, total_write_time
    );

    let parallel_time = hash_duration.max(compress_duration);
    info!(
        "Parallel processing completed in {:?} (hash: {:?}, compress: {:?})",
        parallel_time, hash_duration, compress_duration
    );
    info!("Total compression completed in {:?}", start_time.elapsed());
    Ok(())
}
