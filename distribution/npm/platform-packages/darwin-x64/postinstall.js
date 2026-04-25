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

// Prefer the thin release repo, but fall back to the monorepo release page
// while publish choreography is still catching up.
const RELEASE_REPOS = Object.freeze([
  {
    repo: 'Loctree/loct',
    label: 'thin release repo',
  },
  {
    repo: 'Loctree/loctree-ast',
    label: 'monorepo release fallback',
  },
]);
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

function buildDownloadTargets(version, file) {
  return RELEASE_REPOS.map(({ repo, label }) => ({
    label,
    url: `https://github.com/${repo}/releases/download/v${version}/${file}`,
  }));
}

async function downloadReleaseAsset(downloadTargets, destPath) {
  let lastError = null;

  for (const target of downloadTargets) {
    if (existsSync(destPath)) {
      unlinkSync(destPath);
    }

    console.log(`Downloading loct release asset from ${target.url} (${target.label})...`);

    try {
      await downloadFile(target.url, destPath);
      return target;
    } catch (error) {
      lastError = error;
      console.warn(`Download failed from ${target.label}: ${error.message}`);
    }
  }

  throw lastError || new Error('No download targets available');
}

async function install() {
  const packageName = require('./package.json').name;
  const mapping = BINARY_MAPPINGS[packageName];

  if (!mapping) {
    console.error(`Unknown package: ${packageName}`);
    process.exit(1);
  }

  const { file, target } = mapping;
  const downloadTargets = buildDownloadTargets(VERSION, file);
  const targetPath = join(__dirname, target);
  const archivePath = join(__dirname, file);

  // Skip if already exists
  if (existsSync(targetPath)) {
    console.log(`Binary already exists at ${targetPath}`);
    return;
  }

  try {
    const downloadedFrom = await downloadReleaseAsset(downloadTargets, archivePath);

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

    console.log(`Successfully installed loct binary to ${targetPath} via ${downloadedFrom.label}`);
  } catch (error) {
    console.error(`Failed to download loct binary: ${error.message}`);
    console.error('Attempted URLs:');
    downloadTargets.forEach((target) => console.error(`- ${target.url}`));
    console.error('');
    console.error('Possible solutions:');
    console.error('1. Check your internet connection');
    console.error('2. Verify the matching release assets exist on GitHub');
    console.error('3. Install loct manually from the thin release repo: https://github.com/Loctree/loct/releases');
    console.error('4. If the thin repo is still missing the asset, try the monorepo fallback: https://github.com/Loctree/loctree-ast/releases');
    process.exit(1);
  }
}

install();
