#!/usr/bin/env node

/**
 * Postinstall script for platform-specific packages
 * Downloads the loctree binary from GitHub releases
 */

const https = require('https');
const { createWriteStream, chmodSync, existsSync, mkdirSync } = require('fs');
const { join } = require('path');
const { pipeline } = require('stream');
const { promisify } = require('util');

const streamPipeline = promisify(pipeline);

// Configuration
const GITHUB_REPO = 'Loctree/Loctree';
const VERSION = require('./package.json').version;

// Platform-specific binary names
const BINARY_MAPPINGS = {
  '@loctree/darwin-arm64': {
    file: 'loctree-aarch64-apple-darwin',
    target: 'loctree',
  },
  '@loctree/darwin-x64': {
    file: 'loctree-x86_64-apple-darwin',
    target: 'loctree',
  },
  '@loctree/linux-arm64-gnu': {
    file: 'loctree-aarch64-unknown-linux-gnu',
    target: 'loctree',
  },
  '@loctree/linux-arm64-musl': {
    file: 'loctree-aarch64-unknown-linux-musl',
    target: 'loctree',
  },
  '@loctree/linux-x64-gnu': {
    file: 'loctree-x86_64-unknown-linux-gnu',
    target: 'loctree',
  },
  '@loctree/linux-x64-musl': {
    file: 'loctree-x86_64-unknown-linux-musl',
    target: 'loctree',
  },
  '@loctree/win32-arm64-msvc': {
    file: 'loctree-aarch64-pc-windows-msvc.exe',
    target: 'loctree.exe',
  },
  '@loctree/win32-x64-msvc': {
    file: 'loctree-x86_64-pc-windows-msvc.exe',
    target: 'loctree.exe',
  },
};

async function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    https.get(url, {
      headers: { 'User-Agent': 'loctree-npm-installer' },
      followRedirect: true,
    }, (response) => {
      // Handle redirects
      if (response.statusCode === 301 || response.statusCode === 302) {
        return downloadFile(response.headers.location, destPath).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
        return;
      }

      const fileStream = createWriteStream(destPath);
      streamPipeline(response, fileStream)
        .then(resolve)
        .catch(reject);
    }).on('error', reject);
  });
}

async function install() {
  const packageName = require('./package.json').name;
  const mapping = BINARY_MAPPINGS[packageName];

  if (!mapping) {
    console.error(`Unknown package: ${packageName}`);
    process.exit(1);
  }

  const { file, target } = mapping;
  const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/${file}`;
  const targetPath = join(__dirname, target);

  // Skip if already exists
  if (existsSync(targetPath)) {
    console.log(`Binary already exists at ${targetPath}`);
    return;
  }

  console.log(`Downloading loctree binary from ${downloadUrl}...`);

  try {
    await downloadFile(downloadUrl, targetPath);

    // Make executable (Unix-like systems)
    if (process.platform !== 'win32') {
      chmodSync(targetPath, 0o755);
    }

    console.log(`Successfully installed loctree binary to ${targetPath}`);
  } catch (error) {
    console.error(`Failed to download loctree binary: ${error.message}`);
    console.error(`URL: ${downloadUrl}`);
    console.error('');
    console.error('Possible solutions:');
    console.error('1. Check your internet connection');
    console.error('2. Verify the release exists on GitHub');
    console.error('3. Install loctree manually from: https://github.com/Loctree/Loctree/releases');
    process.exit(1);
  }
}

install();
