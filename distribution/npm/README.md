# loctree

Fast, language-aware codebase analyzer for detecting dead exports, circular imports, and generating dependency graphs.

This is the official npm wrapper for the [loctree](https://github.com/Loctree/Loctree) Rust binary.

## Features

- **Dead Export Detection**: Find unused exports in your codebase
- **Circular Import Detection**: Identify circular dependencies
- **Dependency Graph Generation**: Visualize module relationships
- **Multi-language Support**: TypeScript, JavaScript, Rust, and more
- **Fast Performance**: Written in Rust for maximum speed
- **Tauri Integration**: Built-in presets for Tauri projects

## Installation

```bash
npm install loctree
# or
pnpm add loctree
# or
yarn add loctree
```

The correct binary for your platform will be automatically downloaded during installation.

## Usage

### Command Line

```bash
# Analyze a directory
loctree src/

# Analyze multiple directories
loctree src/ lib/

# Detect circular imports
loctree src/ --circular

# Find dead exports
loctree src/ --dead

# Use Tauri preset (analyzes frontend + backend)
loctree src src-tauri/src --preset-tauri

# Generate dependency graph
loctree src/ --graph

# Fail on missing Tauri command handlers
loctree src src-tauri/src --preset-tauri --fail-on-missing-handlers

# Show all options
loctree --help
```

### Programmatic API

```javascript
const { execLoctreeSync, execLoctree, getBinaryPath } = require('loctree');

// Execute loctree and get output as string
const output = execLoctreeSync(['src/', '--dead']);
console.log(output);

// Execute loctree with inherited stdio (for interactive output)
execLoctree(['src/', '--circular']);

// Get path to binary (useful for custom spawning)
const binaryPath = getBinaryPath();
console.log('Binary location:', binaryPath);
```

### TypeScript

```typescript
import { execLoctreeSync, execLoctree, getBinaryPath } from 'loctree';

const result: string = execLoctreeSync(['src/', '--dead']);
console.log(result);
```

## Supported Platforms

This package provides pre-built binaries for:

- **macOS**: Intel (x64) and Apple Silicon (ARM64)
- **Linux**: x64 and ARM64 (both glibc and musl)
- **Windows**: x64 and ARM64

The correct binary for your platform is automatically selected during installation.

## Platform-Specific Packages

This package uses optional dependencies to install platform-specific binaries:

- `@loctree/darwin-arm64` - macOS Apple Silicon (M1/M2/M3)
- `@loctree/darwin-x64` - macOS Intel
- `@loctree/linux-arm64-gnu` - Linux ARM64 (glibc)
- `@loctree/linux-arm64-musl` - Linux ARM64 (musl/Alpine)
- `@loctree/linux-x64-gnu` - Linux x64 (glibc)
- `@loctree/linux-x64-musl` - Linux x64 (musl/Alpine)
- `@loctree/win32-arm64-msvc` - Windows ARM64
- `@loctree/win32-x64-msvc` - Windows x64

## Common Use Cases

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Install loctree
  run: pnpm add -D loctree

- name: Check for dead exports
  run: pnpm loctree src/ --dead --confidence high

- name: Check for circular imports
  run: pnpm loctree src/ --circular
```

### Package.json Scripts

```json
{
  "scripts": {
    "analyze": "loctree src/",
    "analyze:dead": "loctree src/ --dead",
    "analyze:circular": "loctree src/ --circular",
    "analyze:tauri": "loctree src src-tauri/src --preset-tauri --circular --dead"
  }
}
```

### Pre-commit Hook

```json
{
  "husky": {
    "hooks": {
      "pre-commit": "loctree src/ --dead --circular"
    }
  }
}
```

## Configuration

loctree can be configured via command-line flags. Common options:

- `-A, --all` - Enable all checks
- `--circular` - Detect circular imports
- `--dead` - Find dead exports
- `--graph` - Generate dependency graph
- `--preset-tauri` - Use Tauri-specific analysis
- `--fail-on-missing-handlers` - Exit with error if Tauri handlers are missing
- `--confidence <level>` - Set confidence level (low, medium, high)

For a complete list of options, run:

```bash
loctree --help
```

## Troubleshooting

### Binary Not Found

If you get an error about the binary not being found:

1. Ensure `optionalDependencies` are not disabled in your package manager
2. Check that the correct platform package is installed in `node_modules/@loctree/`
3. Try removing `node_modules` and reinstalling

### Platform Not Supported

If your platform is not supported, you can:

1. Install loctree directly via Cargo: `cargo install loctree`
2. Request support for your platform in the [GitHub issues](https://github.com/Loctree/Loctree/issues)

### Download Failures

If the binary download fails during installation:

1. Check your internet connection
2. Verify the release exists on [GitHub releases](https://github.com/Loctree/Loctree/releases)
3. Install the binary manually and place it in the appropriate package directory

## Development

This npm package is a wrapper around the Rust-based loctree binary. The source code for loctree itself is at:

https://github.com/Loctree/Loctree

## License

MIT

## Links

- [GitHub Repository](https://github.com/Loctree/Loctree)
- [Documentation](https://docs.rs/loctree)
- [crates.io](https://crates.io/crates/loctree)
- [npm Package](https://www.npmjs.com/package/loctree)
