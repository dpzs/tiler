// Tests for extension/menu.js — Menu overlay module
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXTENSION_DIR = join(__dirname, '..');
const MENU_PATH = join(EXTENSION_DIR, 'menu.js');

function test(name, fn) {
  try {
    fn();
    return { name, pass: true };
  } catch (e) {
    return { name, pass: false, error: e.message };
  }
}

function getSrc() {
  return readFileSync(MENU_PATH, 'utf-8');
}

export default function tests() {
  return [
    test('menu.js exists', () => {
      assert.ok(existsSync(MENU_PATH), `File not found: ${MENU_PATH}`);
    }),

    test('menu.js exports a MenuOverlay class', () => {
      const src = getSrc();
      assert.ok(
        src.includes('class MenuOverlay'),
        'Must define MenuOverlay class'
      );
      assert.ok(src.includes('export'), 'Must export the class');
    }),

    test('menu.js has showOverview method', () => {
      const src = getSrc();
      assert.ok(
        src.includes('showOverview') || src.includes('ShowOverview'),
        'Must implement showOverview method'
      );
    }),

    test('menu.js has showZoomed method', () => {
      const src = getSrc();
      assert.ok(
        src.includes('showZoomed') || src.includes('ShowZoomed'),
        'Must implement showZoomed method'
      );
    }),

    test('menu.js has hide method', () => {
      const src = getSrc();
      assert.ok(
        src.includes('hide(') || src.includes('Hide('),
        'Must implement hide method'
      );
    }),

    test('menu.js uses St for UI widgets', () => {
      const src = getSrc();
      assert.ok(src.includes('St'), 'Must use St (Shell Toolkit) for widgets');
    }),

    test('menu.js uses Clutter for positioning', () => {
      const src = getSrc();
      assert.ok(src.includes('Clutter'), 'Must use Clutter for positioning');
    }),

    test('menu.js handles key events', () => {
      const src = getSrc();
      assert.ok(
        src.includes('key_press_event') || src.includes('key-press-event') ||
        src.includes('KeyPressEvent') || src.includes('vfunc_key_press_event'),
        'Must handle key press events'
      );
    }),

    test('menu.js renders monitor boxes in overview', () => {
      const src = getSrc();
      assert.ok(
        src.includes('monitor') && (src.includes('box') || src.includes('Box') || src.includes('widget') || src.includes('Widget')),
        'Must render monitor boxes for overview'
      );
    }),

    test('menu.js renders layout options in zoomed view', () => {
      const src = getSrc();
      assert.ok(
        src.includes('fullscreen') || src.includes('Fullscreen'),
        'Must reference fullscreen layout option'
      );
      assert.ok(
        src.includes('side-by-side') || src.includes('SideBySide') || src.includes('sideBySide'),
        'Must reference side-by-side layout option'
      );
    }),

    test('menu.js creates an overlay actor for the menu', () => {
      const src = getSrc();
      assert.ok(
        src.includes('_overlay') || src.includes('_container') || src.includes('_panel'),
        'Must create an overlay container/actor'
      );
    }),

    test('menu.js supports key event callback for D-Bus forwarding', () => {
      const src = getSrc();
      assert.ok(
        src.includes('onKeyPressed') || src.includes('_onKeyPress') ||
        src.includes('keyCallback') || src.includes('_keyCallback'),
        'Must support a key event callback for D-Bus forwarding'
      );
    }),

    test('menu.js has a destroy/cleanup method', () => {
      const src = getSrc();
      assert.ok(
        src.includes('destroy(') || src.includes('cleanup('),
        'Must have destroy or cleanup method'
      );
    }),

    test('menu.js parses JSON monitor data', () => {
      const src = getSrc();
      assert.ok(
        src.includes('JSON.parse'),
        'Must parse JSON monitor/layout data'
      );
    }),

    test('menu.js tracks menu state (overview vs zoomed)', () => {
      const src = getSrc();
      assert.ok(
        src.includes('_state') || src.includes('_mode') || src.includes('_isZoomed') || src.includes('_view'),
        'Must track whether menu is in overview or zoomed state'
      );
    }),

    // B3: Escape key should not call hide() locally (daemon is single source of truth)
    test('menu.js does not call hide() directly on Escape key', () => {
      const src = getSrc();
      const start = src.indexOf('_onKeyPressEvent');
      const stopIdx = src.indexOf('return Clutter.EVENT_STOP', start);
      const end = src.indexOf('}', stopIdx) + 1;
      const keyHandler = src.substring(start, end);
      assert.ok(
        !keyHandler.includes('this.hide()'),
        '_onKeyPressEvent must not call this.hide() — Escape is handled by the daemon'
      );
    }),

    test('menu.js forwards all keys via callback without special Escape branch', () => {
      const src = getSrc();
      const start = src.indexOf('_onKeyPressEvent');
      const stopIdx = src.indexOf('return Clutter.EVENT_STOP', start);
      const end = src.indexOf('}', stopIdx) + 1;
      const keyHandler = src.substring(start, end);
      assert.ok(
        !keyHandler.includes("=== 'Escape'"),
        '_onKeyPressEvent must not have special Escape key comparison'
      );
    }),
  ];
}
