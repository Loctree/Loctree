# Node.js Runtime API Test Fixture

This fixture demonstrates runtime-invoked APIs that should NOT be flagged as dead code.

## Files

### `loader.mjs`
Node.js ES Module loader hooks. These exports are invoked by the Node.js runtime:
- `resolve()` - Maps import specifiers to URLs
- `load()` - Provides source code for modules
- `globalPreload()` - Runs before any modules load
- `initialize()` - Called when loader is initialized

Run with: `node --experimental-loader=./loader.mjs app.js`

### `test-runner.test.js`
Node.js test runner hooks. These exports are invoked by the test runner:
- `before()` - Setup before all tests
- `after()` - Teardown after all tests
- `beforeEach()` - Runs before each test
- `afterEach()` - Runs after each test

Run with: `node --test test-runner.test.js`

## Expected Behavior

### Without Runtime API Detection
All hook exports would be flagged as dead code with 7 false positives:
```
Dead exports:
  loader.mjs:
    - resolve (line 13)
    - load (line 27)
    - globalPreload (line 44)
    - initialize (line 54)
  test-runner.test.js:
    - before (line 17)
    - after (line 26)
    - beforeEach (line 35)
```

### With Runtime API Detection (P1-03 Fix)
All hook exports should be correctly recognized as runtime-invoked and excluded from dead code detection.

Zero false positives.

## Test Command
```bash
loctree dead --path tools/fixtures/nodejs-loader
```

Expected output: 0 dead exports (all hooks are runtime-invoked)

Created by M&K (c)2025 The LibraxisAI Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
