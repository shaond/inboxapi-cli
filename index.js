#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * InboxAPI CLI Wrapper
 * 
 * In a real production package, this script would:
 * 1. Detect the platform (OS/Arch)
 * 2. Find the pre-compiled binary packaged with the npm module
 * 3. Or download it if not present
 * 
 * For development, it checks target/release or target/debug, 
 * or falls back to 'cargo run --quiet -- ...'.
 */

const binName = process.platform === 'win32' ? 'inboxapi-cli.exe' : 'inboxapi-cli';
const searchPaths = [
  path.join(__dirname, binName),
  path.join(__dirname, 'target', 'release', binName),
  path.join(__dirname, 'target', 'debug', binName),
];

let binPath = searchPaths.find(p => fs.existsSync(p));

const args = process.argv.slice(2);

if (binPath) {
  // Run the native binary
  const child = spawn(binPath, args, { stdio: 'inherit' });
  child.on('exit', (code, signal) => {
    if (code !== null) process.exit(code);
    if (signal) process.kill(process.pid, signal);
    process.exit(1);
  });
  child.on('error', (err) => {
    console.error(`Failed to start binary at ${binPath}:`, err);
    process.exit(1);
  });
} else {
  // Fallback to cargo run for development
  // Note: this assumes 'cargo' is in the PATH
  const child = spawn('cargo', ['run', '--quiet', '--', ...args], { stdio: 'inherit' });
  child.on('exit', (code, signal) => {
    if (code !== null) process.exit(code);
    if (signal) process.kill(process.pid, signal);
    process.exit(1);
  });
  child.on('error', (err) => {
    console.error("Binary not found and 'cargo' failed to start. Have you built the project?");
    process.exit(1);
  });
}
