#!/usr/bin/env node

const { existsSync } = require('fs');
const { join } = require('path');
const { spawnSync } = require('child_process');

const {
  getPackageNameForPlatformKey,
  resolvePlatformKey,
  unsupportedPlatformMessage,
} = require('./platform-support');

function validateInstallation() {
  const platformKey = resolvePlatformKey();

  if (!platformKey) {
    console.error(`Unsupported platform: ${process.platform}-${process.arch}`);
    console.error('Supported platforms:');
    console.error('  - macOS Apple Silicon (ARM64)');
    console.error('  - macOS Intel (x64)');
    console.error('  - Linux x64 (glibc)');
    console.error('  - Windows x64');
    process.exit(1);
  }

  const packageName = getPackageNameForPlatformKey(platformKey);

  if (!packageName) {
    console.error(unsupportedPlatformMessage({ platformKey }));
    process.exit(1);
  }

  // Check if the platform-specific package was installed
  const binaryName = process.platform === 'win32' ? 'loct.exe' : 'loct';
  const binaryPath = join(__dirname, 'node_modules', packageName, binaryName);

  if (!existsSync(binaryPath)) {
    console.warn(`Warning: loct binary not found at ${binaryPath}`);
    console.warn('This may happen if optionalDependencies are disabled.');
    console.warn('The package may not work correctly.');
    return;
  }

  // Verify binary is executable and correct version
  try {
    const result = spawnSync(binaryPath, ['--version'], { encoding: 'utf8' });
    if (result.status === 0) {
      console.log(`loct binary installed successfully: ${result.stdout.trim()}`);
    } else {
      console.warn(`Warning: loct binary may not be working correctly`);
    }
  } catch (err) {
    console.warn(`Warning: Could not verify loct binary: ${err.message}`);
  }
}

// Run validation
validateInstallation();
