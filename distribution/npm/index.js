#!/usr/bin/env node

const { execFileSync } = require('child_process');
const { join } = require('path');
const { existsSync } = require('fs');

const {
  getPackageNameForPlatformKey,
  resolvePlatformKey,
  unsupportedPlatformMessage,
} = require('./platform-support');

function getBinaryPath() {
  const platformKey = resolvePlatformKey();

  if (!platformKey) {
    throw new Error(`Unsupported platform: ${process.platform}-${process.arch}`);
  }

  const packageName = getPackageNameForPlatformKey(platformKey);

  if (!packageName) {
    throw new Error(unsupportedPlatformMessage({ platformKey }));
  }

  const binaryName = process.platform === 'win32' ? 'loct.exe' : 'loct';
  const binaryPath = join(__dirname, 'node_modules', packageName, binaryName);

  if (!existsSync(binaryPath)) {
    throw new Error(
      `loct binary not found at ${binaryPath}. ` +
      `This may happen if optionalDependencies are disabled. ` +
      `Please ensure "${packageName}" is installed.`
    );
  }

  return binaryPath;
}

/**
 * Execute loct with given arguments
 * @param {string[]} args - Command line arguments
 * @param {object} options - Execution options
 * @returns {Buffer} - stdout from loct
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
 * Execute loct and return result as string
 * @param {string[]} args - Command line arguments
 * @returns {string} - stdout from loct
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
