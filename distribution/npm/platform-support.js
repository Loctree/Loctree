const SUPPORTED_NPM_TARGETS = Object.freeze([
  {
    key: 'darwin-arm64',
    packageName: '@loctree/darwin-arm64',
    label: 'macOS Apple Silicon (ARM64)',
  },
  {
    key: 'darwin-x64',
    packageName: '@loctree/darwin-x64',
    label: 'macOS Intel (x64)',
  },
  {
    key: 'linux-x64-gnu',
    packageName: '@loctree/linux-x64-gnu',
    label: 'Linux x64 (glibc)',
  },
  {
    key: 'win32-x64-msvc',
    packageName: '@loctree/win32-x64-msvc',
    label: 'Windows x64',
  },
]);

const PLATFORM_PACKAGES = Object.freeze(
  Object.fromEntries(
    SUPPORTED_NPM_TARGETS.map((target) => [target.key, target.packageName]),
  ),
);

function normalizeArch(arch) {
  const archMap = {
    x64: 'x64',
    arm64: 'arm64',
    aarch64: 'arm64',
  };

  return archMap[arch] || arch;
}

function isMuslLibc(spawnSyncImpl) {
  const spawnSync = spawnSyncImpl || require('child_process').spawnSync;
  try {
    const lddVersion = spawnSync('ldd', ['--version'], { encoding: 'utf8' });
    return Boolean(lddVersion.stderr && lddVersion.stderr.includes('musl'));
  } catch (_err) {
    return false;
  }
}

function resolvePlatformKey(options = {}) {
  const platform = options.platform || process.platform;
  const arch = options.arch || process.arch;
  const normalizedArch = normalizeArch(arch);

  if (platform === 'linux') {
    const libcVariant = options.libcVariant || (isMuslLibc(options.spawnSync) ? 'musl' : 'gnu');
    return `${platform}-${normalizedArch}-${libcVariant}`;
  }

  if (platform === 'win32') {
    return `${platform}-${normalizedArch}-msvc`;
  }

  if (platform === 'darwin') {
    return `${platform}-${normalizedArch}`;
  }

  return null;
}

function getPackageNameForPlatformKey(platformKey) {
  return PLATFORM_PACKAGES[platformKey] || null;
}

function supportedTargetSummary() {
  return SUPPORTED_NPM_TARGETS.map((target) => target.label).join(', ');
}

function unsupportedPlatformMessage(options = {}) {
  const platform = options.platform || process.platform;
  const arch = options.arch || process.arch;
  const platformKey = options.platformKey || resolvePlatformKey({ platform, arch });
  const subject = platformKey || `${platform}-${normalizeArch(arch)}`;

  const lines = [
    `No npm package is published for platform: ${subject}.`,
    `Supported npm targets: ${supportedTargetSummary()}.`,
  ];

  if (subject === 'linux-x64-musl') {
    lines.push(
      'Linux musl/Alpine is not packaged on npm yet. Use `cargo install --locked loctree`, a direct release asset, or another supported install channel.',
    );
  } else {
    lines.push(
      'Use `cargo install --locked loctree`, a direct release asset, or another supported install channel instead.',
    );
  }

  return lines.join(' ');
}

module.exports = {
  SUPPORTED_NPM_TARGETS,
  PLATFORM_PACKAGES,
  getPackageNameForPlatformKey,
  isMuslLibc,
  normalizeArch,
  resolvePlatformKey,
  supportedTargetSummary,
  unsupportedPlatformMessage,
};
