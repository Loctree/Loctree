import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, '..', '..');
const fixtureRoot = resolve(repoRoot, 'tools', 'fixtures', 'basic-tree');
const cliDir = resolve(repoRoot, 'loctree_rs');

function run(args) {
  return execFileSync('cargo', ['run', '--quiet', '--', ...args], {
    cwd: cliDir,
    encoding: 'utf8',
  });
}

const jsonOut = JSON.parse(run([fixtureRoot, '--json', '--gitignore']));
assert.equal(jsonOut.summary.totalLoc, 1014);
assert.equal(jsonOut.summary.files, 6);
assert.equal(jsonOut.summary.directories, 3);
assert.ok(jsonOut.entries.some((e) => e.path === 'big.txt' && e.isLarge));

const summaryOut = run([fixtureRoot, '--summary', '--color=never', '--gitignore']);
assert.ok(summaryOut.includes('Summary:'));
assert.ok(summaryOut.includes('directories: 3'));
assert.ok(summaryOut.includes('files: 6'));
assert.ok(summaryOut.includes('total LOC: 1014'));

const cssOut = run([fixtureRoot, '--ext', 'css', '--gitignore']);
assert.ok(cssOut.includes('style.css'));
assert.ok(!cssOut.includes('README.md'));

const hiddenOut = JSON.parse(run([fixtureRoot, '--json', '--show-hidden', '--gitignore']));
assert.ok(hiddenOut.entries.some((e) => e.path === '.hidden.txt'));
assert.equal(hiddenOut.summary.totalLoc, 1017);
assert.equal(hiddenOut.summary.files, 8);

// Gitignore should prune target/node_modules
const filteredOut = run([fixtureRoot, '--ext', 'rs', '--gitignore']);
assert.ok(!filteredOut.includes('target'));
assert.ok(!filteredOut.includes('node_modules'));

const importGraphRoot = resolve(repoRoot, 'tools', 'fixtures', 'import-graph');
const analysis = JSON.parse(run([importGraphRoot, '-A', '--json', '--ext', 'ts', '--color=never']));
assert.equal(analysis.filesAnalyzed, 4);
const dupShared = analysis.duplicateExports.find((d) => d.name === 'shared');
assert.ok(dupShared);
assert.ok(dupShared.files.length >= 2);
assert.ok(analysis.reexportCascades.some((c) => c.from.endsWith('a.ts') && c.to.endsWith('c.ts')));
assert.ok(analysis.dynamicImports.some((d) => d.file.endsWith('d.ts')));
assert.ok(Array.isArray(analysis.duplicateExportsRanked));
const rankedShared = analysis.duplicateExportsRanked.find((d) => d.name === 'shared');
assert.ok(rankedShared && rankedShared.canonical);

const jsonlOut = run([importGraphRoot, '-A', '--jsonl', '--ext', 'ts']);
const lines = jsonlOut.trim().split('\n');
assert.equal(lines.length, 1);
const parsedJsonl = JSON.parse(lines[0]);
assert.equal(parsedJsonl.root.endsWith('import-graph'), true);

// Tauri command coverage
const tauriRoot = resolve(repoRoot, 'tools', 'fixtures', 'tauri-coverage');
const tauriCoverage = JSON.parse(
  run([tauriRoot, '-A', '--json', '--ext', 'ts,rs', '--color=never'])
);
const missingNames = tauriCoverage.commands.missingHandlers.map((c) => c.name);
assert.ok(missingNames.includes('frontend_missing'));
const unusedNames = tauriCoverage.commands.unusedHandlers.map((c) => c.name);
assert.ok(unusedNames.includes('backend_only'));
const backendNames = tauriCoverage.commands.backend.map((c) => c.name);
assert.ok(backendNames.includes('explicit_rename')); // rename="..."
assert.ok(backendNames.includes('snakeCaseFunc')); // rename_all=camelCase
assert.ok(backendNames.includes('RenameAllPascal')); // rename_all=PascalCase

console.log('loctree_rs basic tests passed');
