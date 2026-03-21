// Tests for extension/extension.js skeleton
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXTENSION_DIR = join(__dirname, '..');
const EXTENSION_PATH = join(EXTENSION_DIR, 'extension.js');

function test(name, fn) {
  try {
    fn();
    return { name, pass: true };
  } catch (e) {
    return { name, pass: false, error: e.message };
  }
}

export default function tests() {
  return [
    test('extension.js exists', () => {
      assert.ok(existsSync(EXTENSION_PATH), `File not found: ${EXTENSION_PATH}`);
    }),

    test('extension.js has enable() method', () => {
      const src = readFileSync(EXTENSION_PATH, 'utf-8');
      assert.ok(
        src.includes('enable(') || src.includes('enable ('),
        'extension.js must define an enable() method'
      );
    }),

    test('extension.js has disable() method', () => {
      const src = readFileSync(EXTENSION_PATH, 'utf-8');
      assert.ok(
        src.includes('disable(') || src.includes('disable ('),
        'extension.js must define a disable() method'
      );
    }),

    test('extension.js has a default export', () => {
      const src = readFileSync(EXTENSION_PATH, 'utf-8');
      assert.ok(
        src.includes('export default'),
        'extension.js must have a default export'
      );
    }),

    test('extension.js imports from gnome-shell Extension base', () => {
      const src = readFileSync(EXTENSION_PATH, 'utf-8');
      assert.ok(
        src.includes("resource:///org/gnome/shell/extensions/extension.js"),
        'extension.js must import from GNOME Shell Extension base'
      );
    }),

    test('extension.js extends Extension class', () => {
      const src = readFileSync(EXTENSION_PATH, 'utf-8');
      assert.ok(
        src.includes('extends Extension'),
        'extension.js class must extend Extension'
      );
    }),
  ];
}
