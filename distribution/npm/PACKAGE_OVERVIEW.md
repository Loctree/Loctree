# loctree npm Package - Technical Overview

This document provides a comprehensive technical overview of the loctree npm package structure.

## Package Architecture

This package uses the **optionalDependencies pattern** popularized by esbuild, swc, and turbo for distributing platform-specific Rust binaries via npm.

### Key Components

```
loctree/                          # Main package
├── package.json                  # Main package metadata + optionalDependencies
├── index.js                      # JavaScript wrapper + CLI entry point
├── index.d.ts                    # TypeScript type definitions
├── bin/loctree                   # CLI executable (delegates to index.js)
├── install.js                    # Post-install validation script
├── README.md                     # User documentation
├── LICENSE                       # MIT license
└── platform-packages/            # Platform-specific packages
    ├── darwin-arm64/             # macOS Apple Silicon
    │   ├── package.json
    │   ├── postinstall.js        # Downloads binary from GitHub releases
    │   └── loctree               # Binary (downloaded on install)
    ├── darwin-x64/               # macOS Intel
    ├── linux-arm64-gnu/          # Linux ARM64 (glibc)
    ├── linux-arm64-musl/         # Linux ARM64 (musl/Alpine)
    ├── linux-x64-gnu/            # Linux x64 (glibc)
    ├── linux-x64-musl/           # Linux x64 (musl/Alpine)
    ├── win32-arm64-msvc/         # Windows ARM64
    └── win32-x64-msvc/           # Windows x64
```

## How It Works

### Installation Flow

1. **User runs**: `npm install loctree`

2. **npm resolves dependencies**:
   - Main package `loctree` is installed
   - npm checks all `optionalDependencies`
   - Only the platform-matching package is installed (e.g., `@loctree/darwin-arm64` on M1 Mac)

3. **Platform package postinstall**:
   - `postinstall.js` runs in the platform package
   - Downloads the Rust binary from GitHub releases
   - Verifies download and sets executable permissions

4. **Main package postinstall**:
   - `install.js` runs in the main package
   - Validates that the correct platform binary is present
   - Displays version information

5. **User runs**: `npx loctree --help`
   - `bin/loctree` is executed (shebang: `#!/usr/bin/env node`)
   - Delegates to `index.js`
   - `index.js` finds the platform binary and spawns it with arguments

### Runtime Execution

When calling `loctree` (either via CLI or programmatically):

```
User command
    ↓
bin/loctree (Node.js shebang wrapper)
    ↓
index.js (Main wrapper)
    ↓
getBinaryPath() (Platform detection)
    ↓
Finds: node_modules/@loctree/darwin-arm64/loctree
    ↓
execFileSync(binaryPath, args)
    ↓
Rust binary executes
    ↓
Result returned to user
```

## Platform Detection Logic

### Platform Key Generation

The package uses this logic to determine which binary to use:

```javascript
function getPlatformKey() {
  const platform = process.platform;  // 'darwin', 'linux', 'win32'
  const arch = process.arch;          // 'x64', 'arm64'

  if (platform === 'linux') {
    // Detect glibc vs musl
    const isMusl = isMuslLibc();
    return `linux-${arch}-${isMusl ? 'musl' : 'gnu'}`;
  }

  if (platform === 'win32') {
    return `win32-${arch}-msvc`;
  }

  if (platform === 'darwin') {
    return `darwin-${arch}`;
  }
}
```

### musl Detection

On Linux, the package detects musl (Alpine Linux) by checking `ldd --version`:

```javascript
function isMuslLibc() {
  const lddVersion = spawnSync('ldd', ['--version'], { encoding: 'utf8' });
  return lddVersion.stderr && lddVersion.stderr.includes('musl');
}
```

## Binary Download Strategy

### GitHub Releases Integration

Each platform package downloads its binary from GitHub releases:

```
https://github.com/Loctree/Loctree/releases/download/v{VERSION}/{BINARY_NAME}
```

**Binary naming convention** (matches Rust cross-compilation targets):

| Platform Package | Binary Name | Rust Target |
|-----------------|-------------|-------------|
| `@loctree/darwin-arm64` | `loctree-aarch64-apple-darwin` | `aarch64-apple-darwin` |
| `@loctree/darwin-x64` | `loctree-x86_64-apple-darwin` | `x86_64-apple-darwin` |
| `@loctree/linux-arm64-gnu` | `loctree-aarch64-unknown-linux-gnu` | `aarch64-unknown-linux-gnu` |
| `@loctree/linux-arm64-musl` | `loctree-aarch64-unknown-linux-musl` | `aarch64-unknown-linux-musl` |
| `@loctree/linux-x64-gnu` | `loctree-x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-gnu` |
| `@loctree/linux-x64-musl` | `loctree-x86_64-unknown-linux-musl` | `x86_64-unknown-linux-musl` |
| `@loctree/win32-arm64-msvc` | `loctree-aarch64-pc-windows-msvc.exe` | `aarch64-pc-windows-msvc` |
| `@loctree/win32-x64-msvc` | `loctree-x86_64-pc-windows-msvc.exe` | `x86_64-pc-windows-msvc` |

### Download Implementation

The `postinstall.js` script in each platform package:

1. Constructs download URL from GitHub releases
2. Uses Node.js `https` module (no external dependencies)
3. Follows redirects (GitHub releases use 302 redirects)
4. Streams binary to disk
5. Sets executable permissions on Unix-like systems (`chmod 0o755`)
6. Validates download success

## API Design

### Programmatic API

Three functions are exported:

```typescript
// Execute loctree with inherited stdio (for interactive output)
function execLoctree(args?: string[], options?: ExecFileSyncOptions): Buffer;

// Execute loctree and return output as string
function execLoctreeSync(args?: string[]): string;

// Get path to the binary (useful for custom spawning)
function getBinaryPath(): string;
```

**Examples**:

```javascript
// Get output as string
const output = execLoctreeSync(['src/', '--dead']);
console.log(output);

// Interactive execution (streams to stdout)
execLoctree(['src/', '--circular']);

// Custom spawning
const binaryPath = getBinaryPath();
const { spawn } = require('child_process');
const proc = spawn(binaryPath, ['--help'], { stdio: 'inherit' });
```

## Error Handling

### Installation Errors

1. **Platform not supported**:
   ```
   Unsupported platform: freebsd-x64
   ```
   → User should install via `cargo install loctree`

2. **Binary download failed**:
   ```
   Failed to download loctree binary: HTTP 404
   ```
   → Check GitHub release exists

3. **optionalDependencies disabled**:
   ```
   loctree binary not found at node_modules/@loctree/...
   ```
   → Re-install without `--no-optional` flag

### Runtime Errors

1. **Binary not executable**:
   ```
   EACCES: permission denied
   ```
   → Run `chmod +x` on binary path

2. **Binary not found**:
   ```
   No package available for platform: linux-arm64-gnu
   ```
   → Platform package not installed correctly

## Package Manager Compatibility

### npm

Works out of the box:
```bash
npm install loctree
```

### pnpm

Fully compatible (optionalDependencies are hoisted correctly):
```bash
pnpm add loctree
```

### Yarn

Compatible with Yarn 1.x, 2.x, 3.x:
```bash
yarn add loctree
```

**Note**: Ensure `--ignore-optional` is NOT used, or platform packages won't install.

## CI/CD Integration

### GitHub Actions

```yaml
- name: Install loctree
  run: pnpm add -D loctree

- name: Run analysis
  run: pnpm loctree src/ --dead --circular
```

### Multi-platform Testing

The package includes a GitHub Actions workflow (`.github/workflows/test-install.yml`) that tests installation on:

- Ubuntu (Linux x64 glibc)
- macOS Intel (darwin x64)
- macOS Apple Silicon (darwin arm64)
- Windows (win32 x64)

With Node.js versions: 14.x, 16.x, 18.x, 20.x

## Comparison with Other Approaches

### optionalDependencies (This Package)

**Pros**:
- No downloads during `npm install` (platform packages are on npm)
- Fast installation
- Works offline if npm cache is populated
- No postinstall scripts to trust

**Cons**:
- Requires publishing 9 packages (1 main + 8 platforms)
- npm registry storage costs
- More complex release process

### Binary Download (binary-install pattern)

**Pros**:
- Only 1 package to publish
- Simpler release process

**Cons**:
- Downloads from GitHub during install (slower, requires internet)
- Relies on GitHub releases being stable
- May hit rate limits

### Direct Bundling

**Pros**:
- Fully offline installation
- No external dependencies

**Cons**:
- Huge package sizes (>100MB for all platforms)
- Violates npm package size best practices

## Security Considerations

### Binary Integrity

- Binaries are downloaded from official GitHub releases
- HTTPS ensures transport security
- Consider adding SHA256 checksums in future versions

### Supply Chain Security

- The package has zero runtime dependencies (only Node.js builtins)
- Platform packages also have zero dependencies
- Postinstall scripts are minimal and auditable

### Permissions

- Binaries are set to `0o755` (rwxr-xr-x) on Unix
- No sudo/root required
- All files stay in `node_modules`

## Future Enhancements

### Potential Improvements

1. **SHA256 Checksums**: Verify binary integrity after download
2. **Fallback to Cargo**: Auto-install via `cargo install loctree` if binary missing
3. **WebAssembly Version**: Ship WASM binary as ultimate fallback
4. **CDN Hosting**: Mirror binaries on npm CDN for faster downloads
5. **Version Pinning**: Lock platform packages to exact versions
6. **Binary Caching**: Cache downloaded binaries globally

### Version Sync Automation

Consider automating version bumps across all packages:

```bash
# Update all package.json files to new version
npm version 0.6.15
./scripts/sync-versions.sh 0.6.15
```

## License

MIT License - see LICENSE file

## References

- [esbuild npm package](https://www.npmjs.com/package/esbuild) - Inspiration for optionalDependencies pattern
- [swc npm package](https://www.npmjs.com/package/@swc/core) - Similar architecture
- [Publishing binaries on npm (Sentry)](https://sentry.engineering/blog/publishing-binaries-on-npm) - Best practices guide
- [Packaging Rust for npm (Orhun)](https://blog.orhun.dev/packaging-rust-for-npm/) - Detailed tutorial

---

Created for the loctree npm package wrapper project.
