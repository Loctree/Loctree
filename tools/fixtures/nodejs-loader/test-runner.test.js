// Node.js test runner hooks
// https://nodejs.org/api/test.html
//
// These exports are invoked by Node.js test runner
// Run with: node --test test-runner.test.js
//
// Without runtime API detection, these would be flagged as dead.

import { test } from 'node:test';
import assert from 'node:assert';

/**
 * Before hook - runs once before all tests
 * Invoked by Node.js test runner
 */
export async function before() {
    console.log('Setting up test environment');
    global.testData = { initialized: true };
}

/**
 * After hook - runs once after all tests
 * Invoked by Node.js test runner
 */
export async function after() {
    console.log('Tearing down test environment');
    delete global.testData;
}

/**
 * BeforeEach hook - runs before each test
 * Invoked by Node.js test runner
 */
export async function beforeEach() {
    global.testData.count = (global.testData.count || 0) + 1;
}

/**
 * AfterEach hook - runs after each test
 * Invoked by Node.js test runner
 */
export async function afterEach() {
    console.log('Test completed');
}

// Actual tests (these WOULD be dead without the hooks above)
test('example test 1', () => {
    assert.ok(global.testData.initialized);
});

test('example test 2', () => {
    assert.ok(global.testData.count > 0);
});
