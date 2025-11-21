#!/usr/bin/env node
import { createReadStream } from 'node:fs';
import { readdir } from 'node:fs/promises';
import path from 'node:path';

function parseArgs(argv) {
  const positional = [];
  const options = { exts: null };

  for (const arg of argv) {
    if (arg.startsWith('--ext=')) {
      const value = arg.slice('--ext='.length).trim();
      if (value) {
        options.exts = new Set(
          value
            .split(',')
            .map((segment) => segment.trim().replace(/^\./, ''))
            .filter(Boolean)
        );
      }
      continue;
    }
    if (arg.startsWith('--')) {
      console.warn(`Ignoring unknown flag ${arg}`);
      continue;
    }
    positional.push(arg);
  }

  if (positional.length === 0) {
    throw new Error('Usage: loc-tree.mjs <root> [--ext=rs,ts,tsx]');
  }

  return { root: positional[0], options };
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

async function collectLines(root, exts) {
  const lines = [];

  const sortKey = (entry) => {
    if (entry.entry.isDirectory()) {
      return `0-${entry.name.toLowerCase()}`;
    }
    return `1-${entry.name.toLowerCase()}`;
  };

  async function walk(currentPath, prefixFlags) {
    let dirEntries = await readdir(currentPath, { withFileTypes: true });
    dirEntries = dirEntries
      .filter((entry) => entry.name !== '.DS_Store')
      .map((entry) => ({ entry, name: entry.name }))
      .sort((a, b) => sortKey(a).localeCompare(sortKey(b)));

    for (let index = 0; index < dirEntries.length; index += 1) {
      const { entry, name } = dirEntries[index];
      const isLast = index === dirEntries.length - 1;
      const prefix = prefixFlags
        .map((hasNext) => (hasNext ? '│   ' : '    '))
        .join('');
      const branch = isLast ? '└── ' : '├── ';
      const label = `${prefix}${branch}${name}`;
      const fullPath = path.join(currentPath, name);
      const record = { label, loc: null };

      if (entry.isFile()) {
        const extIndex = name.lastIndexOf('.');
        const ext =
          extIndex === -1 ? '' : name.slice(extIndex + 1).trim().toLowerCase();
        if (!exts || exts.has(ext)) {
          try {
            record.loc = await countFileLines(fullPath);
          } catch (error) {
            console.warn(`Failed to count ${fullPath}: ${error.message ?? error}`);
          }
        }
      }

      lines.push(record);

      if (entry.isDirectory()) {
        await walk(fullPath, [...prefixFlags, !isLast]);
      }
    }
  }

  await walk(root, []);
  return lines;
}

async function main() {
  const { root, options } = parseArgs(process.argv.slice(2));
  const rootPath = path.resolve(root);
  const lines = await collectLines(rootPath, options.exts);

  if (lines.length === 0) {
    console.log(`${path.basename(rootPath)}/ (empty)`);
    return;
  }

  const maxLabelLen = lines.reduce(
    (max, item) => Math.max(max, item.label.length),
    0
  );

  console.log(`${path.basename(rootPath)}/`);
  for (const { label, loc } of lines) {
    if (loc == null) {
      console.log(label);
      continue;
    }
    const padding = ' '.repeat(maxLabelLen - label.length + 2);
    const locStr = loc.toString().padStart(6, ' ');
    console.log(`${label}${padding}${locStr}`);
  }
}

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});
