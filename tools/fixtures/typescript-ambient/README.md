# TypeScript Ambient Declarations Test Fixture

This fixture tests the handling of TypeScript ambient declarations (P2-02).

## Files

### Ambient Declaration Files (should NOT be flagged as dead)

1. **jsx-types.d.ts** - `declare global` with JSX namespace
   - Pattern: Vue.js JSX type definitions
   - Should have `has_ambient_declarations = true`
   - Exports should NOT appear in dead exports list

2. **module-augmentation.d.ts** - `declare module` augmentation
   - Pattern: Vue Router type extensions
   - Should have `has_ambient_declarations = true`
   - Exports should NOT appear in dead exports list

3. **namespace-types.d.ts** - `declare namespace` declarations
   - Pattern: SvelteKit/framework ambient types
   - Should have `has_ambient_declarations = true`
   - Exports should NOT appear in dead exports list

### Regular Files (normal dead code detection)

4. **regular-exports.ts** - Regular TypeScript exports
   - Should have `has_ambient_declarations = false`
   - If unused, SHOULD be flagged as dead code

## Test Command

```bash
cd tools/fixtures/typescript-ambient
loctree -A --dead --output=json > output.json
```

## Expected Results

- `jsx-types.d.ts`: has_ambient_declarations = true, no dead exports
- `module-augmentation.d.ts`: has_ambient_declarations = true, no dead exports
- `namespace-types.d.ts`: has_ambient_declarations = true, no dead exports
- `regular-exports.ts`: has_ambient_declarations = false, all 3 exports flagged as dead

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
