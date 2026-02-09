# Runtime API Detection (P1-03)

## Problem
Node.js ES Module loader hooks and other runtime-invoked APIs are flagged as dead code because they're never statically imported. They're invoked by the runtime environment.

## Solution
Loctree 0.7.x automatically detects runtime-invoked exports and excludes them from dead code detection.

## Supported Runtime APIs

### Node.js
- **ES Module loader hooks** (`resolve`, `load`, `globalPreload`, `initialize`)
  - Files: `**/loader.{js,mjs,cjs}`, `**/loaders/*.{js,mjs,cjs}`, `lib/internal/modules/esm/*.js`
  - Usage: `node --experimental-loader=./loader.mjs app.js`

- **Test runner hooks** (`before`, `after`, `beforeEach`, `afterEach`)
  - Files: `**/*.test.{js,mjs,cjs,ts,mts,cts}`
  - Usage: `node --test test-runner.test.js`

### Web APIs
- **Web Workers** (`onmessage`, `onmessageerror`, `onerror`)
  - Files: `**/*.worker.{js,ts}`

- **Service Workers** (`install`, `activate`, `fetch`, `message`, `sync`, `push`)
  - Files: `**/service-worker.{js,ts}`, `**/sw.{js,ts}`

### Build Tools
- **Vite plugins** (`config`, `configResolved`, `buildStart`, `buildEnd`)
  - Files: `**/vite.config.{js,ts}`

- **Webpack plugins** (`apply`)
  - Files: `**/webpack.config.{js,ts}`

### Frameworks
- **Next.js middleware** (`middleware`, `config`)
  - Files: `**/middleware.{js,ts}`

- **Astro** (`getStaticPaths`, `prerender`)
  - Files: `**/*.astro`

## Custom Runtime APIs

Add custom patterns in `.loctree/config.toml`:

```toml
[[runtime_apis]]
framework = "Remix"
exports = ["loader", "action", "meta", "links", "headers"]
file_patterns = ["**/routes/*.{jsx,tsx}", "**/routes/**/*.{jsx,tsx}"]

[[runtime_apis]]
framework = "SvelteKit"
exports = ["load", "actions"]
file_patterns = ["**/+page.{js,ts}", "**/+layout.{js,ts}", "**/+server.{js,ts}"]
```

## CLI Flags

### `--include-runtime`
Include runtime-invoked exports in dead code detection.

Useful for auditing whether your loader hooks/middleware are actually being used.

```bash
loct dead --include-runtime
```

Without this flag (default), runtime APIs are automatically excluded from dead detection.

## Example

### Before P1-03 (False Positives)
```bash
$ loct dead --path lib/internal/modules/esm

Dead exports (7 false positives):
  hooks.js:15 - resolve (never imported)
  hooks.js:30 - load (never imported)
  hooks.js:45 - globalPreload (never imported)
  hooks.js:58 - initialize (never imported)
```

### After P1-03 (Zero False Positives)
```bash
$ loct dead --path lib/internal/modules/esm

No dead exports found.

Runtime-invoked APIs (excluded by default):
  hooks.js:15 - resolve (Node.js ES Module loader hook)
  hooks.js:30 - load (Node.js ES Module loader hook)
  hooks.js:45 - globalPreload (Node.js ES Module loader hook)
  hooks.js:58 - initialize (Node.js ES Module loader hook)

Use --include-runtime to check if these hooks are actually being used.
```

## Impact
- **Before**: 7 false positives on Node.js codebase
- **After**: 0 false positives
- **Accuracy**: 100% reduction in false positive rate for runtime APIs

## Test Fixtures
See `tools/fixtures/nodejs-loader/` for working examples:
- `loader.mjs` - ES Module loader hooks
- `test-runner.test.js` - Test runner hooks
- `.loctree/config.toml` - Custom runtime API configuration

## Implementation
- Registry: `loctree_rs/src/analyzer/dead_parrots/runtime_apis.rs`
- Detection: `loctree_rs/src/analyzer/dead_parrots/mod.rs:650-658`
- Config: `loctree_rs/src/config.rs:23`
- Tests: `loctree_rs/src/analyzer/dead_parrots/mod.rs:1934-2077`

---

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
