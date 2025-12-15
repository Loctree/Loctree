#!/usr/bin/env node

/**
 * Postinstall script for loctree npm package
 * Downloads the correct binary for the current platform from GitHub releases
 */

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const zlib = require("zlib");

const PACKAGE_VERSION = require("../package.json").version;
const REPO = "Loctree/Loctree";

// Platform/arch mapping to release asset names
// Must match artifacts from .github/workflows/publish.yml
const PLATFORM_MAP = {
  "darwin-arm64": "loctree-darwin-aarch64.tar.gz",
  "darwin-x64": "loctree-darwin-x86_64.tar.gz",
  "linux-x64": "loctree-linux-x86_64.tar.gz",
  "win32-x64": "loctree-windows-x86_64.exe.zip",
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  return `${platform}-${arch}`;
}

function getAssetName() {
  const key = getPlatformKey();
  const asset = PLATFORM_MAP[key];
  if (!asset) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported platforms: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    process.exit(1);
  }
  return asset;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    const request = (url) => {
      https.get(url, { headers: { "User-Agent": "loctree-npm" } }, (response) => {
        // Follow redirects
        if (response.statusCode === 302 || response.statusCode === 301) {
          request(response.headers.location);
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Failed to download: ${response.statusCode}`));
          return;
        }

        response.pipe(file);
        file.on("finish", () => {
          file.close();
          resolve();
        });
      }).on("error", reject);
    };

    request(url);
  });
}

async function extractTarGz(archive, destDir) {
  // Use tar command (available on macOS/Linux)
  // nosemgrep: detect-child-process - archive and destDir are internal paths, not user input
  execSync(`tar -xzf "${archive}" -C "${destDir}"`, { stdio: "inherit" });
}

async function extractZip(archive, destDir) {
  // Use unzip command or PowerShell on Windows
  if (process.platform === "win32") {
    // nosemgrep: javascript.lang.security.detect-child-process.detect-child-process
    execSync(`powershell -Command "Expand-Archive -Path '${archive}' -DestinationPath '${destDir}'"`, { stdio: "inherit" });
  } else {
    // nosemgrep: javascript.lang.security.detect-child-process.detect-child-process
    execSync(`unzip -o "${archive}" -d "${destDir}"`, { stdio: "inherit" });
  }
}

async function main() {
  const binDir = path.join(__dirname, "..", "bin");
  const assetName = getAssetName();
  const downloadUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${assetName}`;

  console.log(`Installing loctree v${PACKAGE_VERSION} for ${getPlatformKey()}...`);
  console.log(`Downloading from: ${downloadUrl}`);

  // Create bin directory
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const archivePath = path.join(binDir, assetName);

  try {
    // Download archive
    await downloadFile(downloadUrl, archivePath);
    console.log("Download complete, extracting...");

    // Extract based on file type
    if (assetName.endsWith(".tar.gz")) {
      await extractTarGz(archivePath, binDir);
    } else if (assetName.endsWith(".zip")) {
      await extractZip(archivePath, binDir);
    }

    // Clean up archive
    fs.unlinkSync(archivePath);

    // Make binaries executable (Unix only)
    if (process.platform !== "win32") {
      const loctreeBin = path.join(binDir, "loctree");
      const loctBin = path.join(binDir, "loct");

      if (fs.existsSync(loctreeBin)) {
        fs.chmodSync(loctreeBin, 0o755);
      }
      if (fs.existsSync(loctBin)) {
        fs.chmodSync(loctBin, 0o755);
      }
    }

    console.log("loctree installed successfully!");
    console.log("Run 'loct --help' to get started.");

  } catch (error) {
    console.error("Installation failed:", error.message);
    console.error("");
    console.error("You can install loctree manually:");
    console.error("  cargo install loctree");
    console.error("");
    console.error("Or download from: https://github.com/Loctree/Loctree/releases");
    process.exit(1);
  }
}

main();
