import assert from 'node:assert/strict';
import path from 'node:path';
import { describe, it } from 'node:test';
import { parseArgs, normaliseIgnorePatterns } from '../../loctree.mjs';

describe('parseArgs', () => {
  it('should parse extensions correctly', () => {
    const { options } = parseArgs(['--ext', 'js,ts']);
    assert.ok(options.exts.has('js'));
    assert.ok(options.exts.has('ts'));
    assert.equal(options.exts.size, 2);
  });

  it('should handle ignore patterns', () => {
    const { options } = parseArgs(['-I', 'node_modules', '--ignore', 'dist']);
    assert.deepEqual(options.ignorePatterns, ['node_modules', 'dist']);
  });
  
  it('should parse color mode', () => {
    assert.equal(parseArgs(['--color=always']).options.color, 'always');
    assert.equal(parseArgs(['-c']).options.color, 'always');
    assert.equal(parseArgs(['--color=never']).options.color, 'never');
  });

  it('should set json output', () => {
    assert.equal(parseArgs(['--json']).options.output, 'json');
  });

  it('should parse max depth', () => {
    assert.equal(parseArgs(['-L', '2']).options.maxDepth, 2);
  });
});

describe('normaliseIgnorePatterns', () => {
  it('should resolve relative paths', () => {
    const root = path.resolve('/tmp/project');
    const patterns = ['dist', '/abs/path'];
    const normalized = normaliseIgnorePatterns(patterns, root);
    assert.equal(normalized[0], path.resolve(root, 'dist'));
    assert.equal(normalized[1], '/abs/path');
  });
});
