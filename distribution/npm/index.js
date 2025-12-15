#!/usr/bin/env node

const { execFileSync } = require('child_process');
const { join } = require('path');
const { existsSync } = require('fs');

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

  const archMap = {
    'x64': 'x64',
    'arm64': 'arm64',
    'aarch64': 'arm64',
  };

  const normalizedArch = archMap[arch] || arch;

  if (platform === 'linux') {
    const isMusl = isMuslLibc();
    const libc = isMusl ? 'musl' : 'gnu';
    return `${platform}-${normalizedArch}-${libc}`;
  }

  if (platform === 'win32') {
    return `${platform}-${normalizedArch}-msvc`;
  }

  if (platform === 'darwin') {
    return `${platform}-${normalizedArch}`;
  }

  return null;
}

function isMuslLibc() {
  const { spawnSync } = require('child_process');
  try {
    const lddVersion = spawnSync('ldd', ['--version'], { encoding: 'utf8' });
    return lddVersion.stderr && lddVersion.stderr.includes('musl');
  } catch (err) {
    return false;
  }
}

function getBinaryPath() {
  const platformKey = getPlatformKey();

  if (!platformKey) {
    throw new Error(`Unsupported platform: ${process.platform}-${process.arch}`);
  }

  const packageName = PLATFORMS[platformKey];

  if (!packageName) {
    throw new Error(`No package available for platform: ${platformKey}`);
  }

  const binaryName = process.platform === 'win32' ? 'loctree.exe' : 'loctree';
  const binaryPath = join(__dirname, 'node_modules', packageName, binaryName);

  if (!existsSync(binaryPath)) {
    throw new Error(
      `loctree binary not found at ${binaryPath}. ` +
      `This may happen if optionalDependencies are disabled. ` +
      `Please ensure "${packageName}" is installed.`
    );
  }

  return binaryPath;
}

/**
 * Execute loctree with given arguments
 * @param {string[]} args - Command line arguments
 * @param {object} options - Execution options
 * @returns {Buffer} - stdout from loctree
 */
function execLoctree(args = [], options = {}) {
  const binaryPath = getBinaryPath();

  const execOptions = {
    stdio: 'inherit',
    ...options,
  };

  try {
    return execFileSync(binaryPath, args, execOptions);
  } catch (err) {
    if (err.status !== undefined) {
      process.exit(err.status);
    }
    throw err;
  }
}

/**
 * Execute loctree and return result as string
 * @param {string[]} args - Command line arguments
 * @returns {string} - stdout from loctree
 */
function execLoctreeSync(args = []) {
  const binaryPath = getBinaryPath();

  try {
    return execFileSync(binaryPath, args, { encoding: 'utf8' });
  } catch (err) {
    if (err.stdout) return err.stdout;
    throw err;
  }
}

// Export API
module.exports = {
  execLoctree,
  execLoctreeSync,
  getBinaryPath,
};

// CLI execution
if (require.main === module) {
  const args = process.argv.slice(2);
  execLoctree(args);
}
