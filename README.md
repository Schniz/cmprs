# cmprs 🗜️

[![CI](https://github.com/Schniz/cmprs/actions/workflows/ci.yml/badge.svg)](https://github.com/Schniz/cmprs/actions/workflows/ci.yml)

> Self-extracting executable compression

## Why does this exist? 🤔

🍎 **macOS Reality Check**: UPX is fantastic for executable compression, but it doesn't work with `bun build --compile` artifacts on MacOS, so I guess it is pretty widespread.

## What does it do? ✨

🗜️ **Compress**: `cmprs` takes any executable and creates a compressed self-extracting version using zstd compression

🎭 **Self-Extract**: The compressed executable automatically decompresses itself when run, replacing the compressed version with the original

⚡ **Process Replacement**: Uses `exec()` to completely replace the decompressor process with the actual program - no wrapper processes or performance overhead

🧹 **One-Time Cost**: After the first run, the executable is permanently decompressed, so subsequent runs have zero decompression overhead

## How it works 🔧

```bash
# Compress an executable
./cmprs my_program -o my_program.cmprs

# Run the compressed version (decompresses automatically)
./my_program.cmprs args...

# Subsequent runs are instant (already decompressed)
./my_program.cmprs more_args...
```

## Architecture 🏗️

🔗 **Format**: `[dcmprs binary][MAGIC_HEADER][SHA256][zstd compressed data]`

🎯 **Smart Execution**: 
  - Finds the magic boundary in the self-extracting executable
  - Decompresses the original program to a temporary file
  - Simultaneously replaces the compressed file with decompressed content
  - Uses `exec()` to become the original program (no wrapper process)

🛡️ **Secure**: Uses proper temporary file handling with automatic cleanup

## Building 🔨

```bash
# Build both tools
cargo build --release

# The magic happens in target/release/
ls target/release/{cmprs,dcmprs}
```

## Testing 🧪

```bash
# Install Bun (required for e2e tests)
curl -fsSL https://bun.sh/install | bash

# Run e2e tests
bun run test
```

The e2e tests include:
- ✅ Basic compression and decompression
- ✅ Argument passing to compressed binaries
- ✅ File size verification
- ✅ macOS universal binary support (macOS only)
- ✅ Cross-platform compatibility (Linux, macOS, Windows)

## Technical Details 🤓

🎛️ **Compression**: zstd for excellent compression ratios and fast decompression

🧵 **Parallel**: File replacement happens in parallel with program execution

📦 **Minimal Overhead**: dcmprs is aggressively optimized for size (opt-level="z", LTO, stripped)

🔄 **Process Hygiene**: Complete process replacement means proper signal handling and exit codes

## Status 📊

🧪 **Experimental**: This is a proof of concept exploring alternatives to UPX on macOS

🎯 **Works**: Successfully compresses and self-extracts executables

🚧 **Evolving**: Open to improvements and real-world testing
