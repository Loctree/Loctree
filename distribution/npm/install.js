#!/usr/bin/env node

const { existsSync } = require('fs');
const { join } = require('path');
const { spawnSync } = require('child_process');

// Platform mapping to package names
const PLATFORMS = {
  'darwin-arm64': '@loctree/darwin-arm64',
  'darwin-x64': '@loctree/darwin-x64',
  'linux-arm64-gnu': '@loctree/linux-arm64-gnu',
  'linux-arm64-musl': '@loctree/linux-arm64-musl',
  'linux-x64-gnu': '@loctree/linux-x64-gnu',
  'linux-x64-musl': '@loctree/linux-x64-musl',
  'win32-arm64-msvc': '@loctree/win32-arm64-msvc',
  'win32-x64-msvc': '@loctree/win32-x64-msvc',
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;

  // Normalize architecture names
  const archMap = {
    'x64': 'x64',
    'arm64': 'arm64',
    'aarch64': 'arm64',
  };

  const normalizedArch = archMap[arch] || arch;

  // Determine libc variant on Linux
  if (platform === 'linux') {
    const isMusl = isMuslLibc();
    const libc = isMusl ? 'musl' : 'gnu';
    return `${platform}-${normalizedArch}-${libc}`;
  }

  // Windows uses MSVC ABI
  if (platform === 'win32') {
    return `${platform}-${normalizedArch}-msvc`;
  }

  // macOS
  if (platform === 'darwin') {
    return `${platform}-${normalizedArch}`;
  }

  return null;
}

function isMuslLibc() {
  // Check if we're running on musl (Alpine Linux, etc.)
  try {
    const lddVersion = spawnSync('ldd', ['--version'], { encoding: 'utf8' });
    return lddVersion.stderr && lddVersion.stderr.includes('musl');
  } catch (err) {
    return false;
  }
}

function validateInstallation() {
  const platformKey = getPlatformKey();

  if (!platformKey) {
    console.error(`Unsupported platform: ${process.platform}-${process.arch}`);
    console.error('Supported platforms:');
    Object.keys(PLATFORMS).forEach(key => console.error(`  - ${key}`));
    process.exit(1);
  }

  const packageName = PLATFORMS[platformKey];

  if (!packageName) {
    console.error(`No package available for platform: ${platformKey}`);
    process.exit(1);
  }

  // Check if the platform-specific package was installed
  const binaryName = process.platform === 'win32' ? 'loctree.exe' : 'loctree';
  const binaryPath = join(__dirname, 'node_modules', packageName, binaryName);

  if (!existsSync(binaryPath)) {
    console.warn(`Warning: loctree binary not found at ${binaryPath}`);
    console.warn('This may happen if optionalDependencies are disabled.');
    console.warn('The package may not work correctly.');
    return;
  }

  // Verify binary is executable and correct version
  try {
    const result = spawnSync(binaryPath, ['--version'], { encoding: 'utf8' });
    if (result.status === 0) {
      console.log(`loctree binary installed successfully: ${result.stdout.trim()}`);
    } else {
      console.warn(`Warning: loctree binary may not be working correctly`);
    }
  } catch (err) {
    console.warn(`Warning: Could not verify loctree binary: ${err.message}`);
  }
}

// Run validation
validateInstallation();
