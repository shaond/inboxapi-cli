#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const PLATFORM_PACKAGES = {
  'darwin-arm64': '@inboxapi/cli-darwin-arm64',
  'darwin-x64': '@inboxapi/cli-darwin-x64',
  'linux-x64': '@inboxapi/cli-linux-x64',
  'linux-arm64': '@inboxapi/cli-linux-arm64',
  'win32-x64': '@inboxapi/cli-win32-x64',
};

const binName = process.platform === 'win32' ? 'inboxapi-cli.exe' : 'inboxapi-cli';

function findBinary() {
  // 1. Try platform-specific npm package (production install)
  const platformKey = `${process.platform}-${process.arch}`;
  const pkg = PLATFORM_PACKAGES[platformKey];
  if (pkg) {
    try {
      const pkgDir = path.dirname(require.resolve(`${pkg}/package.json`));
      const binPath = path.join(pkgDir, binName);
      if (fs.existsSync(binPath)) return binPath;
    } catch {}
  }

  // 2. Fall back to local dev paths
  const devPaths = [
    path.join(__dirname, 'target', 'release', binName),
    path.join(__dirname, 'target', 'debug', binName),
  ];
  const found = devPaths.find(p => fs.existsSync(p));
  if (found) return found;

  return null;
}

const binPath = findBinary();
const args = process.argv.slice(2);

if (binPath) {
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
  // 3. Final fallback: cargo run (development)
  const child = spawn('cargo', ['run', '--quiet', '--', ...args], { stdio: 'inherit' });
  child.on('exit', (code, signal) => {
    if (code !== null) process.exit(code);
    if (signal) process.kill(process.pid, signal);
    process.exit(1);
  });
  child.on('error', () => {
    const platformKey = `${process.platform}-${process.arch}`;
    console.error(
      `No pre-built binary available for ${platformKey}. Supported platforms: ${Object.keys(PLATFORM_PACKAGES).join(', ')}.\n` +
      `Fallback to 'cargo run' also failed. Install Rust (https://rustup.rs) or use a supported platform.`
    );
    process.exit(1);
  });
}
