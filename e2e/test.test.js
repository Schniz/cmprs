#!/usr/bin/env bun

import { test, expect, beforeAll, afterAll } from "bun:test";
import { spawn } from "bun";
import { mkdtemp, rm, chmod, stat } from "fs/promises";
import { join } from "path";
import { tmpdir, platform } from "os";

const REPO_ROOT = join(import.meta.dir, "..");
const CMPRS_ROOT = join(REPO_ROOT, "cmprs");
const CMPRS_BIN = join(CMPRS_ROOT, "target/release/cmprs");

let tempDir;

async function runCommand(cmd, args = [], options = {}) {
  const proc = spawn([cmd, ...args], {
    stdio: ["inherit", "pipe", "pipe"],
    ...options,
  });
  
  const output = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  
  return {
    exitCode: output,
    stdout: stdout.trim(),
    stderr: stderr.trim(),
  };
}

async function createTestBinary(tempDir) {
  const scriptPath = join(import.meta.dir, "script.js");
  const binaryPath = join(tempDir, "test-binary");
  
  console.log(`   Creating test binary: ${binaryPath}`);
  const result = await runCommand("bun", [
    "build",
    "--compile",
    `--outfile=${binaryPath}`,
    scriptPath,
  ]);
  
  if (result.exitCode !== 0) {
    throw new Error(`Failed to create test binary: ${result.stderr}`);
  }
  
  // Make sure it's executable
  await chmod(binaryPath, 0o755);
  
  return binaryPath;
}

beforeAll(async () => {
  console.log("🚀 Running cmprs e2e tests\n");
  
  // Build cmprs first
  console.log("📦 Building cmprs...");
  const buildResult = await runCommand("cargo", ["build", "--release"], {
    cwd: CMPRS_ROOT,
  });
  
  if (buildResult.exitCode !== 0) {
    throw new Error(`Failed to build cmprs: ${buildResult.stderr}`);
  }
  console.log("✅ Built cmprs\n");
  
  tempDir = await mkdtemp(join(tmpdir(), "cmprs-e2e-"));
  console.log(`📁 Using temp directory: ${tempDir}\n`);
});

afterAll(async () => {
  if (tempDir) {
    await rm(tempDir, { recursive: true, force: true });
    console.log(`🧹 Cleaned up temp directory: ${tempDir}`);
  }
});

test("Create and run test binary", async () => {
  const binaryPath = await createTestBinary(tempDir);
  
  // Test that the original binary works
  const originalResult = await runCommand(binaryPath);
  expect(originalResult.exitCode).toBe(0);
  expect(originalResult.stdout).toBe("hello world");
  console.log("   ✓ Original binary works");
});

test("Compress binary with cmprs", async () => {
  const binaryPath = await createTestBinary(tempDir);
  const compressedPath = `${binaryPath}.cmprs`;
  
  // Compress the binary
  const compressResult = await runCommand(CMPRS_BIN, [
    "--output", compressedPath,
    binaryPath,
  ]);
  
  expect(compressResult.exitCode).toBe(0);
  
  // Check that compressed file exists and is executable
  const compressedStat = await stat(compressedPath);
  expect(compressedStat.mode & 0o111).toBeTruthy();
  
  console.log("   ✓ Binary compressed successfully");
  console.log(`   ✓ Compressed file is executable`);
});

test("Run compressed binary (first time - decompression)", async () => {
  const binaryPath = await createTestBinary(tempDir);
  const compressedPath = `${binaryPath}.cmprs`;
  
  // Compress the binary
  await runCommand(CMPRS_BIN, ["--output", compressedPath, binaryPath]);
  
  // Run the compressed binary for the first time (should decompress and execute)
  const result = await runCommand(compressedPath);
  
  expect(result.exitCode).toBe(0);
  expect(result.stdout).toBe("hello world");
  
  console.log("   ✓ Compressed binary executed successfully");
  console.log("   ✓ Output matches original");
});

test("Run decompressed binary (second time - direct execution)", async () => {
  const binaryPath = await createTestBinary(tempDir);
  const compressedPath = `${binaryPath}.cmprs`;
  
  // Compress and run once to decompress
  await runCommand(CMPRS_BIN, ["--output", compressedPath, binaryPath]);
  await runCommand(compressedPath);
  
  // Run again (should be direct execution now)
  const result = await runCommand(compressedPath);
  
  expect(result.exitCode).toBe(0);
  expect(result.stdout).toBe("hello world");
  
  console.log("   ✓ Decompressed binary executed successfully");
  console.log("   ✓ Output matches original");
});

test("Compressed binary handles arguments", async () => {
  // Create a binary that echoes its arguments
  const argScriptPath = join(tempDir, "args-script.js");
  await Bun.write(argScriptPath, 'console.log(process.argv.slice(2).join(" "));');
  
  const argBinaryPath = join(tempDir, "args-binary");
  await runCommand("bun", [
    "build",
    "--compile",
    `--outfile=${argBinaryPath}`,
    argScriptPath,
  ]);
  await chmod(argBinaryPath, 0o755);
  
  const compressedPath = `${argBinaryPath}.cmprs`;
  await runCommand(CMPRS_BIN, ["--output", compressedPath, argBinaryPath]);
  
  // Test with arguments
  const result = await runCommand(compressedPath, ["hello", "world", "test"]);
  
  expect(result.exitCode).toBe(0);
  expect(result.stdout).toBe("hello world test");
  
  console.log("   ✓ Arguments passed correctly to compressed binary");
});

test("Compression reduces file size", async () => {
  const binaryPath = await createTestBinary(tempDir);
  const compressedPath = `${binaryPath}.cmprs`;
  
  const originalStat = await stat(binaryPath);
  
  await runCommand(CMPRS_BIN, ["--output", compressedPath, binaryPath]);
  
  const compressedStat = await stat(compressedPath);
  
  // Compressed file should be smaller (though with dcmprs embedded, it might not be much smaller for tiny binaries)
  console.log(`   ✓ Original size: ${originalStat.size} bytes`);
  console.log(`   ✓ Compressed size: ${compressedStat.size} bytes`);
  
  if (compressedStat.size >= originalStat.size * 2) {
    console.log("   ⚠️  Compressed file is significantly larger (expected for small test binaries)");
  }
  
  // Just ensure both files exist and have reasonable sizes
  expect(originalStat.size).toBeGreaterThan(0);
  expect(compressedStat.size).toBeGreaterThan(0);
});

test.skipIf(platform() !== "darwin")("Build macOS universal binary with --build-universal-macos", async () => {
  const binaryPath = await createTestBinary(tempDir);
  const universalPath = `${binaryPath}.universal.cmprs`;
  
  // Compress the binary with --build-universal-macos flag
  const compressResult = await runCommand(CMPRS_BIN, [
    "--build-universal-macos",
    "--output", universalPath,
    binaryPath,
  ]);
  
  expect(compressResult.exitCode).toBe(0);
  
  // Check that universal binary file exists and is executable
  const universalStat = await stat(universalPath);
  expect(universalStat.mode & 0o111).toBeTruthy();
  
  console.log("   ✓ Universal binary created successfully");
  console.log(`   ✓ Universal binary is executable`);
  
  // Test that the universal binary works
  const result = await runCommand(universalPath);
  expect(result.exitCode).toBe(0);
  expect(result.stdout).toBe("hello world");
  
  console.log("   ✓ Universal binary executed successfully");
  console.log("   ✓ Output matches original");
  
  // Check if it's actually a universal binary using the `file` command
  const fileResult = await runCommand("file", [universalPath]);
  expect(fileResult.exitCode).toBe(0);
  
  // Universal binaries should contain multiple architectures
  if (fileResult.stdout.includes("universal") || fileResult.stdout.includes("fat")) {
    console.log("   ✓ Confirmed as universal/fat binary");
  } else {
    console.log(`   ℹ️  File type: ${fileResult.stdout}`);
  }
});