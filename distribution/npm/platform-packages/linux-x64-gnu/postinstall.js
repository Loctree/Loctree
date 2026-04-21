#!/usr/bin/env node

/**
 * Postinstall script for platform-specific packages.
 * Downloads the current release asset from GitHub releases and extracts it in place.
 */

const https = require('https');
const { createWriteStream, chmodSync, existsSync, unlinkSync } = require('fs');
const { join } = require('path');
const { pipeline } = require('stream');
const { promisify } = require('util');
const { execSync } = require('child_process');

const streamPipeline = promisify(pipeline);

// Configuration
const GITHUB_REPO = 'Loctree/loct';
const VERSION = require('./package.json').version;

// Platform-specific release assets currently shipped by CI
const BINARY_MAPPINGS = {
  '@loctree/darwin-arm64': {
    file: 'loct-darwin-aarch64.tar.gz',
    target: 'loct',
  },
  '@loctree/darwin-x64': {
    file: 'loct-darwin-x86_64.tar.gz',
    target: 'loct',
  },
  '@loctree/linux-x64-gnu': {
    file: 'loct-linux-x86_64.tar.gz',
    target: 'loct',
  },
  '@loctree/win32-x64-msvc': {
    file: 'loct-windows-x86_64.zip',
    target: 'loct.exe',
  },
};

async function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    https.get(url, {
      headers: { 'User-Agent': 'loct-npm-installer' },
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
  const archivePath = join(__dirname, file);

  // Skip if already exists
  if (existsSync(targetPath)) {
    console.log(`Binary already exists at ${targetPath}`);
    return;
  }

  console.log(`Downloading loct release asset from ${downloadUrl}...`);

  try {
    await downloadFile(downloadUrl, archivePath);

    if (file.endsWith('.tar.gz')) {
      execSync(`tar -xzf "${archivePath}" -C "${__dirname}"`, { stdio: 'inherit' });
    } else if (file.endsWith('.zip')) {
      if (process.platform === 'win32') {
        execSync(
          `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${__dirname}' -Force"`,
          { stdio: 'inherit' }
        );
      } else {
        execSync(`unzip -o "${archivePath}" -d "${__dirname}"`, { stdio: 'inherit' });
      }
    }

    if (existsSync(archivePath)) {
      unlinkSync(archivePath);
    }

    // Make executable (Unix-like systems)
    if (process.platform !== 'win32') {
      chmodSync(targetPath, 0o755);
    }

    console.log(`Successfully installed loct binary to ${targetPath}`);
  } catch (error) {
    console.error(`Failed to download loct binary: ${error.message}`);
    console.error(`URL: ${downloadUrl}`);
    console.error('');
    console.error('Possible solutions:');
    console.error('1. Check your internet connection');
    console.error('2. Verify the release exists on GitHub');
    console.error('3. Install loct manually from: https://github.com/Loctree/loct/releases');
    process.exit(1);
  }
}

install();
