const test = require('node:test');
const assert = require('node:assert/strict');

const packageJson = require('../package.json');
const {
  SUPPORTED_NPM_TARGETS,
  getPackageNameForPlatformKey,
  resolvePlatformKey,
  unsupportedPlatformMessage,
} = require('../platform-support');

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
