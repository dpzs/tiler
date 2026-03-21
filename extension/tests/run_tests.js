#!/usr/bin/env node
// Simple test runner for GNOME Shell extension tests
// Usage: node extension/tests/run_tests.js

import { readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const testFiles = readdirSync(__dirname)
  .filter(f => f.startsWith('test_') && f.endsWith('.js'))
  .sort();

let passed = 0;
let failed = 0;
const failures = [];

for (const file of testFiles) {
  const mod = await import(join(__dirname, file));
  const tests = mod.default || mod.tests;
  if (typeof tests !== 'function') {
    console.error(`  SKIP ${file} — no default export or tests function`);
    continue;
  }

  const results = await tests();
  for (const result of results) {
    if (result.pass) {
      console.log(`  PASS  ${result.name}`);
      passed++;
    } else {
      console.log(`  FAIL  ${result.name}`);
      console.log(`        ${result.error}`);
      failed++;
      failures.push(result);
    }
  }
}

console.log(`\n${passed} passed, ${failed} failed, ${passed + failed} total`);

if (failed > 0) {
  process.exit(1);
}
