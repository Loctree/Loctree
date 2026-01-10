#!/usr/bin/env node

/**
 * Example usage of loctree npm package
 */

const { execLoctreeSync, execLoctree, getBinaryPath } = require('./index.js');

console.log('=== loctree npm package examples ===\n');

// Example 1: Get binary path
console.log('1. Binary location:');
try {
  const binaryPath = getBinaryPath();
  console.log(`   ${binaryPath}\n`);
} catch (err) {
  console.error(`   Error: ${err.message}\n`);
  process.exit(1);
}

// Example 2: Get version
console.log('2. Version check:');
try {
  const version = execLoctreeSync(['--version']);
  console.log(`   ${version.trim()}\n`);
} catch (err) {
  console.error(`   Error: ${err.message}\n`);
}

// Example 3: Show help
console.log('3. Available commands:');
try {
  const help = execLoctreeSync(['--help']);
  console.log(help);
} catch (err) {
  console.error(`   Error: ${err.message}\n`);
}

// Example 4: Analyze current directory (if it has source files)
console.log('4. Analyzing current directory:');
try {
  // This will use inherited stdio, so output goes directly to console
  execLoctree(['.', '--dead', '--confidence', 'high']);
} catch (err) {
  console.error(`   Analysis failed: ${err.message}`);
}
