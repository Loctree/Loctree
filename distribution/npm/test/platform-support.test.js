const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const packageJson = require('../package.json');
const {
  SUPPORTED_NPM_TARGETS,
  getPackageNameForPlatformKey,
  resolvePlatformKey,
  unsupportedPlatformMessage,
} = require('../platform-support');
const { execLoctree, execLoctreeSync, getBinaryPath } = require('../index');

test('darwin x64 resolves to a published npm package', () => {
  const platformKey = resolvePlatformKey({ platform: 'darwin', arch: 'x64' });
  assert.equal(platformKey, 'darwin-x64');
  assert.equal(getPackageNameForPlatformKey(platformKey), '@loctree/darwin-x64');
});

test('musl platforms return an actionable unsupported-platform message', () => {
  const message = unsupportedPlatformMessage({
    platform: 'linux',
    arch: 'x64',
    platformKey: 'linux-x64-musl',
  });

  assert.match(message, /Linux musl\/Alpine is not packaged on npm yet/);
  assert.match(message, /cargo install --locked loctree/);
});

test('main npm package optionalDependencies match supported targets', () => {
  const supportedPackages = SUPPORTED_NPM_TARGETS.map((target) => target.packageName).sort();
  const optionalDependencies = Object.keys(packageJson.optionalDependencies).sort();

  assert.deepEqual(optionalDependencies, supportedPackages);
});

test('platform postinstall scripts fall back from thin repo to monorepo release assets', () => {
  const repoRoot = path.resolve(__dirname, '..');
  const templatePath = path.join(repoRoot, 'platform-packages', 'postinstall.js');
  const templateSource = fs.readFileSync(templatePath, 'utf8');

  assert.match(templateSource, /Loctree\/loct/);
  assert.match(templateSource, /Loctree\/loctree-ast/);
  assert.match(templateSource, /Attempted URLs:/);

  for (const target of SUPPORTED_NPM_TARGETS) {
    const packageDir = target.packageName.replace('@loctree/', '');
    const packagePath = path.join(repoRoot, 'platform-packages', packageDir, 'postinstall.js');
    const packageSource = fs.readFileSync(packagePath, 'utf8');

    assert.equal(packageSource, templateSource, `${packageDir} postinstall.js drifted from template`);
  }
});

test('npm API captures stdout for programmatic calls', async (t) => {
  const platformKey = resolvePlatformKey({
    platform: process.platform,
    arch: process.arch,
    libcVariant: 'gnu',
  });
  const packageName = getPackageNameForPlatformKey(platformKey);

  if (!packageName) {
    t.skip(`Current platform is not supported by the npm package: ${process.platform}-${process.arch}`);
    return;
  }

  const repoRoot = path.resolve(__dirname, '..');
  const binaryName = process.platform === 'win32' ? 'loct.exe' : 'loct';
  const binaryDir = path.join(repoRoot, 'node_modules', packageName);
  const binaryPath = path.join(binaryDir, binaryName);
  const hadOriginalBinary = fs.existsSync(binaryPath);
  const originalBinary = hadOriginalBinary ? fs.readFileSync(binaryPath) : null;

  fs.mkdirSync(binaryDir, { recursive: true });

  if (process.platform === 'win32') {
    fs.writeFileSync(
      binaryPath,
      '@echo off\r\nif "%~1"=="--version" (echo loct test-version) else (echo %*)\r\n',
      'utf8',
    );
  } else {
    fs.writeFileSync(
      binaryPath,
      '#!/usr/bin/env node\n' +
      'if (process.argv[2] === "--version") {\n' +
      '  process.stdout.write("loct test-version\\n");\n' +
      '} else {\n' +
      '  process.stdout.write(process.argv.slice(2).join(" "));\n' +
      '}\n',
      'utf8',
    );
    fs.chmodSync(binaryPath, 0o755);
  }

  t.after(() => {
    if (hadOriginalBinary && originalBinary) {
      fs.writeFileSync(binaryPath, originalBinary);
      if (process.platform !== 'win32') {
        fs.chmodSync(binaryPath, 0o755);
      }
      return;
    }

    fs.rmSync(path.join(repoRoot, 'node_modules'), { recursive: true, force: true });
  });

  assert.equal(getBinaryPath(), binaryPath);

  const stdout = execLoctree(['alpha', 'beta']);
  assert.equal(Buffer.isBuffer(stdout), true);
  assert.equal(stdout.toString('utf8'), 'alpha beta');

  const version = execLoctreeSync(['--version']);
  assert.equal(version.trim(), 'loct test-version');
});
