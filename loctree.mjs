#!/usr/bin/env node
import { createReadStream } from 'node:fs';
import { readdir } from 'node:fs/promises';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const LARGE_FILE_THRESHOLD = 1000;
const COLORS = {
  red: '\u001B[31m',
  reset: '\u001B[0m',
};

function parseArgs(argv) {
  const positional = [];
  const options = {
    exts: null,
    ignorePatterns: [],
    useGitignore: false,
    maxDepth: null,
    color: 'auto',
    output: 'human',
    summary: false,
    summaryLimit: 5,
    showHelp: false,
  };

  const setExtensions = (rawValue) => {
    const cleaned = rawValue
      .split(',')
      .map((segment) => segment.trim().replace(/^\./, '').toLowerCase())
      .filter(Boolean);
    if (cleaned.length) {
      options.exts = new Set(cleaned);
    }
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === '--help' || arg === '-h') {
      options.showHelp = true;
      continue;
    }

    if (arg === '--color' || arg === '-c') {
      options.color = 'always';
      continue;
    }

    if (arg.startsWith('--color=')) {
      const value = arg.slice('--color='.length).trim();
      if (!['auto', 'always', 'never'].includes(value)) {
        throw new Error('--color expects auto|always|never');
      }
      options.color = value;
      continue;
    }

    if (arg === '--gitignore' || arg === '-g') {
      options.useGitignore = true;
      continue;
    }

    if (arg === '-I' || arg === '--ignore') {
      const next = argv[index + 1];
      if (!next || next.startsWith('-')) {
        throw new Error('-I/--ignore requires a path argument');
      }
      options.ignorePatterns.push(next);
      index += 1;
      continue;
    }

    if (arg === '-L' || arg === '--max-depth') {
      const next = argv[index + 1];
      if (!next || next.startsWith('-')) {
        throw new Error('-L/--max-depth requires a non-negative integer');
      }
      const depth = Number.parseInt(next, 10);
      if (Number.isNaN(depth) || depth < 0) {
        throw new Error('Depth must be a non-negative integer');
      }
      options.maxDepth = depth;
      index += 1;
      continue;
    }

    if (arg === '--ext') {
      const next = argv[index + 1];
      if (!next || next.startsWith('-')) {
        throw new Error('--ext flag requires a comma-separated value (e.g. "--ext py,rs")');
      }
      setExtensions(next);
      index += 1;
      continue;
    }

    if (arg.startsWith('--ext=')) {
      const value = arg.slice('--ext='.length).trim();
      if (value) {
        setExtensions(value);
      }
      continue;
    }

    if (arg === '--json') {
      options.output = 'json';
      continue;
    }

    if (arg === '--summary') {
      options.summary = true;
      const possibleLimit = argv[index + 1];
      if (possibleLimit && !possibleLimit.startsWith('-')) {
        const parsed = Number.parseInt(possibleLimit, 10);
        if (!Number.isNaN(parsed) && parsed > 0) {
          options.summaryLimit = parsed;
          index += 1;
        }
      }
      continue;
    }

    if (arg.startsWith('--summary=')) {
      options.summary = true;
      const raw = arg.slice('--summary='.length).trim();
      const parsed = Number.parseInt(raw, 10);
      if (!Number.isNaN(parsed) && parsed > 0) {
        options.summaryLimit = parsed;
      }
      continue;
    }

    if (arg.startsWith('-')) {
      console.warn(`Ignoring unknown flag ${arg}`);
      continue;
    }

    positional.push(arg);
  }

  if (positional.length === 0) {
    positional.push('.');
  }
  return { roots: positional, options };
}

function countFileLines(filePath) {
  return new Promise((resolve, reject) => {
    let count = 0;
    let sawData = false;
    let endedWithNewline = true;

    const stream = createReadStream(filePath);
    stream.on('data', (chunk) => {
      sawData = true;
      for (let i = 0; i < chunk.length; i += 1) {
        if (chunk[i] === 0x0a) {
          count += 1;
        }
      }
      endedWithNewline = chunk[chunk.length - 1] === 0x0a;
    });
    stream.on('end', () => {
      if (!endedWithNewline && sawData) {
        count += 1;
      }
      resolve(count);
    });
    stream.on('error', reject);
  });
}

function normaliseIgnorePatterns(patterns, rootPath) {
  return patterns.map((pattern) =>
    path.isAbsolute(pattern) ? pattern : path.resolve(rootPath, pattern)
  );
}

function buildGitignoreChecker(rootPath) {
  try {
    const gitStatus = spawnSync('git', ['-C', rootPath, 'rev-parse', '--is-inside-work-tree'], {
      stdio: 'ignore',
    });
    if (gitStatus.status !== 0) {
      return null;
    }
  } catch (error) {
    return null;
  }

  return (relativePath) => {
    if (!relativePath) {
      return false;
    }
    try {
      const result = spawnSync('git', ['-C', rootPath, 'check-ignore', '-q', relativePath], {
        stdio: 'ignore',
      });
      return result.status === 0;
    } catch (error) {
      return false;
    }
  };
}

function pathStartsWith(target, candidate) {
  if (target === candidate) {
    return true;
  }
  const normalized = candidate.endsWith(path.sep) ? candidate : `${candidate}${path.sep}`;
  return target.startsWith(normalized);
}

function shouldIgnorePath(fullPath, relativePath, options, gitChecker) {
  if (
    options.ignorePaths.some((ignorePath) => pathStartsWith(fullPath, ignorePath))
  ) {
    return true;
  }
  if (options.useGitignore && gitChecker && gitChecker(relativePath)) {
    return true;
  }
  return false;
}

async function collectLines(root, options) {
  const lines = [];
  const largeEntries = [];
  const stats = {
    directories: 0,
    files: 0,
    filesWithLoc: 0,
    totalLoc: 0,
  };

  const sortKey = (entry) => {
    if (entry.entry.isDirectory()) {
      return `0-${entry.name.toLowerCase()}`;
    }
    return `1-${entry.name.toLowerCase()}`;
  };

  const gitChecker = options.useGitignore ? buildGitignoreChecker(root) : null;

  async function walk(currentPath, prefixFlags, depth) {
    let dirEntries = await readdir(currentPath, { withFileTypes: true });
    dirEntries = dirEntries
      .filter((entry) => entry.name !== '.DS_Store')
      .map((entry) => ({ entry, name: entry.name }))
      .sort((a, b) => sortKey(a).localeCompare(sortKey(b)));

    let anyIncluded = false;

    for (let index = 0; index < dirEntries.length; index += 1) {
      const { entry, name } = dirEntries[index];
      const isLast = index === dirEntries.length - 1;
      const prefix = prefixFlags
        .map((hasNext) => (hasNext ? '│   ' : '    '))
        .join('');
      const branch = isLast ? '└── ' : '├── ';
      const label = `${prefix}${branch}${name}`;
      const fullPath = path.join(currentPath, name);
      const relativePath = path.relative(root, fullPath) || name;

      if (shouldIgnorePath(fullPath, relativePath, options, gitChecker)) {
        continue;
      }

      const isDir = entry.isDirectory();
      let includeCurrent = false;

      if (isDir) {
        if (options.maxDepth == null || depth < options.maxDepth) {
          const childIncluded = await walk(fullPath, [...prefixFlags, !isLast], depth + 1);
          if (childIncluded) {
            stats.directories += 1;
            includeCurrent = true;
          }
        }
      } else {
        const extIndex = name.lastIndexOf('.');
        const ext = extIndex === -1 ? '' : name.slice(extIndex + 1).trim().toLowerCase();
        if (!options.exts || options.exts.has(ext)) {
          includeCurrent = true;
          try {
            const loc = await countFileLines(fullPath);
            stats.files += 1;
            stats.filesWithLoc += 1;
            stats.totalLoc += loc;
            const isLarge = loc >= LARGE_FILE_THRESHOLD;
            if (isLarge) {
              largeEntries.push({ relativePath, loc });
            }
            lines.push({
              label,
              loc,
              relativePath,
              isDirectory: false,
              isLarge,
            });
          } catch (error) {
            console.warn(`Failed to count ${fullPath}: ${error.message ?? error}`);
          }
        }
      }

      if (includeCurrent && isDir) {
        lines.push({
          label,
          loc: null,
          relativePath,
          isDirectory: true,
          isLarge: false,
        });
      }

      anyIncluded = anyIncluded || includeCurrent;
    }

    return anyIncluded;
  }

  await walk(root, [], 0);
  return { lines, largeEntries, stats };
}

function formatUsage() {
  return `loc-tree\n\nUsage: loctree.mjs [root ...] [options]\n\nArguments:\n  root                 One or more folders to inspect (defaults to current dir).\n\nOptions:\n  --ext <list>         Comma-separated extensions to include (e.g. --ext rs,ts,tsx).\n  -I, --ignore <path>  Ignore a folder/file (relative or absolute). Repeatable.\n  --gitignore, -g      Respect current Git ignore rules (requires git).\n  -L, --max-depth <n>  Limit recursion depth (0 = only direct children).\n  --color[=mode]       Colorize large files. mode: auto|always|never (default auto).\n  --json               Emit JSON instead of a tree view.\n  --summary[=N]        Print totals and top large files (N entries, default 5).\n  --help, -h           Show this message.\n`;
}

async function runOne(root, options, isFirst) {
  const rootPath = path.resolve(root);
  if (!fs.existsSync(rootPath) || !fs.statSync(rootPath).isDirectory()) {
    console.error(`${rootPath} is not a directory`);
    process.exit(1);
  }

  options.ignorePaths = normaliseIgnorePatterns(options.ignorePatterns, rootPath);

  const { lines, largeEntries, stats } = await collectLines(rootPath, options);
  const sortedLargeEntries = [...largeEntries].sort((a, b) => b.loc - a.loc);
  const summary = {
    directories: stats.directories,
    files: stats.files,
    filesWithLoc: stats.filesWithLoc,
    totalLoc: stats.totalLoc,
    largeFiles: sortedLargeEntries.slice(0, options.summaryLimit),
  };

  if (options.output === 'json') {
    const entries = lines.map((item) => ({
      path: item.relativePath,
      type: item.isDirectory ? 'dir' : 'file',
      loc: item.loc,
      isLarge: item.isLarge,
    }));

    return {
      root: rootPath,
      options: {
        exts: options.exts ? Array.from(options.exts) : null,
        ignore: options.ignorePatterns,
        maxDepth: options.maxDepth,
        useGitignore: options.useGitignore,
        color: options.color,
        summary: options.summary ? options.summaryLimit : false,
      },
      summary,
      entries,
    };
  }

  if (!isFirst) {
    console.log('');
  }

  if (lines.length === 0) {
    console.log(`${path.basename(rootPath)}/ (empty)`);
    return null;
  }

  const maxLabelLen = lines.reduce((max, item) => Math.max(max, item.label.length), 0);
  const colorEnabled =
    options.color === 'always' || (options.color === 'auto' && process.stdout.isTTY);

  console.log(`${path.basename(rootPath)}/`);
  for (const { label, loc } of lines) {
    if (loc == null) {
      console.log(label);
      continue;
    }
    const baseLine = `${label.padEnd(maxLabelLen, ' ')}  ${loc.toString().padStart(6, ' ')}`;
    if (colorEnabled && loc >= LARGE_FILE_THRESHOLD) {
      console.log(`${COLORS.red}${baseLine}${COLORS.reset}`);
    } else {
      console.log(baseLine);
    }
  }

  if (sortedLargeEntries.length) {
    console.log(`\nLarge files (>= ${LARGE_FILE_THRESHOLD} LOC):`);
    sortedLargeEntries.forEach((entry) => {
      const line = `  ${entry.relativePath} (${entry.loc} LOC)`;
      if (colorEnabled) {
        console.log(`${COLORS.red}${line}${COLORS.reset}`);
      } else {
        console.log(line);
      }
    });
  }

  if (options.summary) {
    console.log(
      `\nSummary: directories: ${summary.directories}, files: ${summary.files}, files with LOC: ${summary.filesWithLoc}, total LOC: ${summary.totalLoc}`
    );
    if (summary.largeFiles.length === 0 && sortedLargeEntries.length === 0) {
      console.log('No files exceed the large-file threshold.');
    }
  }

  return null;
}

async function main() {
  let parsed;
  try {
    parsed = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(error.message ?? error);
    process.exit(1);
  }

  const { roots, options } = parsed;
  if (options.showHelp) {
    console.log(formatUsage());
    process.exit(0);
  }

  const results = [];
  for (let i = 0; i < roots.length; i += 1) {
    const maybeResult = await runOne(roots[i], options, i === 0);
    if (maybeResult) {
      results.push(maybeResult);
    }
  }

  if (options.output === 'json') {
    if (results.length === 1) {
      console.log(JSON.stringify(results[0], null, 2));
    } else {
      console.log(JSON.stringify(results, null, 2));
    }
  }
}

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});
