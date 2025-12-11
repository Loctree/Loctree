import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, '..', '..');
const fixtureRoot = resolve(repoRoot, 'tools', 'fixtures', 'basic-tree');
const cliPath = resolve(repoRoot, 'loctree.py');

function run(args) {
  return execFileSync('python3', [cliPath, ...args], {
    cwd: fixtureRoot,
    encoding: 'utf8',
  });
}

const jsonOut = JSON.parse(run(['.', '--json', '--gitignore']));
assert.equal(jsonOut.summary.totalLoc, 1014);
assert.equal(jsonOut.summary.files, 6);
assert.equal(jsonOut.summary.directories, 3);
assert.ok(jsonOut.entries.some((e) => e.path === 'big.txt' && e.isLarge));

const summaryOut = run(['.', '--summary', '--color=never', '--gitignore']);
assert.ok(summaryOut.includes('Summary:'));
assert.ok(summaryOut.includes('directories: 3'));
assert.ok(summaryOut.includes('files: 6'));
assert.ok(summaryOut.includes('total LOC: 1014'));

const cssOut = run(['.', '--ext', 'css', '--gitignore']);
assert.ok(cssOut.includes('style.css'));
assert.ok(!cssOut.includes('README.md'));

const hiddenOut = JSON.parse(run(['.', '--json', '--show-hidden', '--gitignore']));
assert.ok(hiddenOut.entries.some((e) => e.path === '.hidden.txt'));
assert.equal(hiddenOut.summary.totalLoc, 1017);
assert.equal(hiddenOut.summary.files, 8);

// Gitignore should prune target/node_modules
const filteredOut = run(['.', '--ext', 'rs', '--gitignore']);
assert.ok(!filteredOut.includes('target'));
assert.ok(!filteredOut.includes('node_modules'));

console.log('loctree.py basic tests passed');
