import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, '..', '..');
const fixtureRoot = resolve(repoRoot, 'tools', 'fixtures', 'basic-tree');
const cliDir = resolve(repoRoot, 'loc_tree_rs');

function run(args) {
  return execFileSync('cargo', ['run', '--quiet', '--', ...args], {
    cwd: cliDir,
    encoding: 'utf8',
  });
}

const jsonOut = JSON.parse(run([fixtureRoot, '--json']));
assert.equal(jsonOut.summary.totalLoc, 1014);
assert.equal(jsonOut.summary.files, 6);
assert.equal(jsonOut.summary.directories, 3);
assert.ok(jsonOut.entries.some((e) => e.path === 'big.txt' && e.isLarge));

const summaryOut = run([fixtureRoot, '--summary', '--color=never']);
assert.ok(summaryOut.includes('Summary:'));
assert.ok(summaryOut.includes('directories: 3'));
assert.ok(summaryOut.includes('files: 6'));
assert.ok(summaryOut.includes('total LOC: 1014'));

const cssOut = run([fixtureRoot, '--ext', 'css']);
assert.ok(cssOut.includes('style.css'));
assert.ok(!cssOut.includes('README.md'));

console.log('loc_tree_rs basic tests passed');
