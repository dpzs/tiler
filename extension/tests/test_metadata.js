// Tests for extension/metadata.json
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXTENSION_DIR = join(__dirname, '..');
const METADATA_PATH = join(EXTENSION_DIR, 'metadata.json');

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
    test('metadata.json exists', () => {
      assert.ok(existsSync(METADATA_PATH), `File not found: ${METADATA_PATH}`);
    }),

    test('metadata.json is valid JSON', () => {
      const raw = readFileSync(METADATA_PATH, 'utf-8');
      JSON.parse(raw); // throws if invalid
    }),

    test('metadata.json has uuid "tiler@gnome-extensions"', () => {
      const meta = JSON.parse(readFileSync(METADATA_PATH, 'utf-8'));
      assert.strictEqual(meta.uuid, 'tiler@gnome-extensions');
    }),

    test('metadata.json has required fields', () => {
      const meta = JSON.parse(readFileSync(METADATA_PATH, 'utf-8'));
      for (const field of ['uuid', 'name', 'description', 'version', 'shell-version']) {
        assert.ok(field in meta, `Missing required field: ${field}`);
      }
    }),

    test('metadata.json shell-version includes 45, 46, 47', () => {
      const meta = JSON.parse(readFileSync(METADATA_PATH, 'utf-8'));
      assert.ok(Array.isArray(meta['shell-version']), 'shell-version must be an array');
      for (const v of ['45', '46', '47']) {
        assert.ok(meta['shell-version'].includes(v), `shell-version missing "${v}"`);
      }
    }),

    test('metadata.json version is a number', () => {
      const meta = JSON.parse(readFileSync(METADATA_PATH, 'utf-8'));
      assert.strictEqual(typeof meta.version, 'number', 'version must be a number');
    }),
  ];
}
