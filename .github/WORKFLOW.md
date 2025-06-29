# GitHub Actions Workflow Documentation

## Overview

This repository uses GitHub Actions for continuous integration and deployment across multiple platforms with cross-compilation support via `cargo-zigbuild`.

## Workflow Structure

### Test Job (`test`)

Runs on every push and pull request to validate the codebase across all supported platforms.

**Matrix Strategy:**
- **Linux** (`ubuntu-latest`): `x86_64-unknown-linux-gnu` with zigbuild
- **macOS** (`macos-latest`): `universal2-apple-darwin` with zigbuild + additional targets
- **Windows** (`windows-latest`): `x86_64-pc-windows-msvc` with regular cargo build

**Steps:**
1. Checkout code
2. Install Rust with required targets
3. Install Zig (for zigbuild on Linux/macOS)
4. Install `cargo-zigbuild`
5. Cache Rust dependencies
6. Install Bun for e2e tests
7. Build `cmprs` and `dcmprs` binaries
8. Copy binaries to expected locations
9. Run e2e tests with Bun
10. Upload test artifacts on failure

### Release Job (`build-release`)

Runs only on pushes to `main`/`master` branches after successful tests.

**Matrix Strategy:**
- **Linux x64**: `x86_64-unknown-linux-gnu` → `cmprs-linux-x64.tar.gz`
- **macOS Universal**: `universal2-apple-darwin` → `cmprs-macos-universal.tar.gz`
- **macOS x64**: `x86_64-apple-darwin` → `cmprs-macos-x64.tar.gz`
- **macOS ARM64**: `aarch64-apple-darwin` → `cmprs-macos-arm64.tar.gz`
- **Windows x64**: `x86_64-pc-windows-msvc` → `cmprs-windows-x64.zip`

## Cross-Compilation Setup

### Zigbuild Integration

- **Linux & macOS**: Use `cargo zigbuild` for cross-compilation
- **Windows**: Use regular `cargo build` (native compilation)
- **Zig Version**: 0.11.0 (stable and well-tested)

### Universal macOS Binaries

The macOS test job uses the `universal2-apple-darwin` target which creates binaries that run natively on both Intel and Apple Silicon Macs.

**Additional Targets Installed:**
- `x86_64-apple-darwin` (Intel Macs)
- `aarch64-apple-darwin` (Apple Silicon Macs)

## Local Development

### Setup
```bash
# Run the setup script
./setup-dev.sh

# Or manually install dependencies
cargo install cargo-zigbuild
rustup target add universal2-apple-darwin x86_64-apple-darwin aarch64-apple-darwin
```

### Build Commands
```bash
bun run build           # Regular cargo build
bun run build:zigbuild  # Cross-compilation build
bun run build:universal # Universal macOS binary
```

### Testing
```bash
bun run test           # Run e2e tests
bun run test:watch     # Watch mode
bun run test:coverage  # With coverage
```

## Caching Strategy

- **Rust Dependencies**: Cached by OS and Cargo.lock hash
- **Release Dependencies**: Cached by OS, target, and Cargo.lock hash
- **Cache Keys**: Include OS and target for proper isolation

## Artifact Management

- **Test Artifacts**: Retained for 7 days (failure debugging)
- **Release Artifacts**: Retained for 30 days (distribution)
- **Naming Convention**: `cmprs-{platform}-{arch}.{ext}`

## Validation

Run the workflow validation script:
```bash
./.github/workflows/validate.sh
```

This checks for:
- ✅ Workflow file existence
- ✅ Required sections (name, strategy, platforms)
- ✅ Zigbuild integration
- ✅ Universal macOS target configuration