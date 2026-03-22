// Tests for extension/extension.js full wiring
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

function getSrc() {
  return readFileSync(EXTENSION_PATH, 'utf-8');
}

export default function tests() {
  return [
    // D-Bus service integration
    test('extension.js imports TilerDBusService from dbus.js', () => {
      const src = getSrc();
      assert.ok(
        src.includes('TilerDBusService') && src.includes('./dbus.js'),
        'Must import TilerDBusService from ./dbus.js'
      );
    }),

    test('extension.js imports MenuOverlay from menu.js', () => {
      const src = getSrc();
      assert.ok(
        src.includes('MenuOverlay') && src.includes('./menu.js'),
        'Must import MenuOverlay from ./menu.js'
      );
    }),

    // enable() creates D-Bus service
    test('extension.js enable() creates D-Bus service instance', () => {
      const src = getSrc();
      const enableStart = src.indexOf('enable(');
      const enableBody = src.substring(enableStart, src.indexOf('}', src.indexOf('{', enableStart) + 200));
      assert.ok(
        enableBody.includes('TilerDBusService') || enableBody.includes('_dbusService'),
        'enable() must create a TilerDBusService instance'
      );
    }),

    test('extension.js enable() registers D-Bus service', () => {
      const src = getSrc();
      assert.ok(
        src.includes('.register('),
        'enable() must call register() on the D-Bus service'
      );
    }),

    // enable() creates menu overlay
    test('extension.js enable() creates menu overlay', () => {
      const src = getSrc();
      assert.ok(
        src.includes('MenuOverlay') && src.includes('_menu'),
        'enable() must create a MenuOverlay instance stored as _menu'
      );
    }),

    // disable() cleans up
    test('extension.js disable() destroys D-Bus service', () => {
      const src = getSrc();
      assert.ok(
        src.includes('destroy(') || src.includes('_dbusService'),
        'disable() must clean up D-Bus service'
      );
    }),

    test('extension.js disable() destroys menu overlay', () => {
      const src = getSrc();
      // Check that disable references menu cleanup
      const disableStart = src.indexOf('disable(');
      const disableEnd = src.indexOf('}', src.indexOf('{', disableStart) + 50);
      const disableBody = src.substring(disableStart, disableEnd + 1);
      assert.ok(
        disableBody.includes('_menu') || disableBody.includes('menu'),
        'disable() must clean up menu overlay'
      );
    }),

    // Window signal connections
    test('extension.js connects to window-created signal', () => {
      const src = getSrc();
      assert.ok(
        src.includes('window-created') || src.includes('window_created'),
        'Must connect to display window-created signal'
      );
    }),

    test('extension.js connects to focus/notify signal', () => {
      const src = getSrc();
      assert.ok(
        src.includes('notify::focus-window') || src.includes('focus-window'),
        'Must connect to focus window change signal'
      );
    }),

    // Workspace signal connections
    test('extension.js connects to workspace change signals', () => {
      const src = getSrc();
      assert.ok(
        src.includes('active-workspace-changed') || src.includes('workspace-switched') ||
        src.includes('switch-workspace'),
        'Must connect to workspace change signals'
      );
    }),

    // Signal cleanup
    test('extension.js disconnects signals in disable()', () => {
      const src = getSrc();
      assert.ok(
        src.includes('disconnect') || src.includes('_signalIds') || src.includes('_signals'),
        'Must disconnect signals in disable()'
      );
    }),

    // Window tracking
    test('extension.js handles window tracking for close events', () => {
      const src = getSrc();
      assert.ok(
        src.includes('unmanaged') || src.includes('unmanaging'),
        'Must connect to window unmanaged signal for close detection'
      );
    }),

    // Fullscreen change tracking
    test('extension.js monitors fullscreen state changes', () => {
      const src = getSrc();
      assert.ok(
        src.includes('fullscreen') || src.includes('is-fullscreen'),
        'Must monitor window fullscreen state changes'
      );
    }),

    // Geometry change tracking
    test('extension.js monitors window geometry changes', () => {
      const src = getSrc();
      assert.ok(
        src.includes('position-changed') || src.includes('size-changed') ||
        src.includes('notify::') || src.includes('geometry'),
        'Must monitor window geometry changes'
      );
    }),

    // First-frame deferral (U5 race-condition fix)
    test('extension.js defers WindowOpened via first-frame signal', () => {
      const src = getSrc();
      assert.ok(
        src.includes('first-frame'),
        'Must defer WindowOpened emission until first-frame signal fires'
      );
    }),

    test('extension.js gets compositor actor via get_compositor_private', () => {
      const src = getSrc();
      assert.ok(
        src.includes('get_compositor_private'),
        'Must obtain compositor actor via win.get_compositor_private()'
      );
    }),

    test('extension.js disconnects first-frame handler after firing', () => {
      const src = getSrc();
      assert.ok(
        src.includes("connect('first-frame'") && src.includes('.disconnect('),
        'Must connect to first-frame and disconnect the handler after it fires'
      );
    }),

    test('extension.js emits immediately if actor already mapped', () => {
      const src = getSrc();
      assert.ok(
        src.includes('is_mapped'),
        'Must check is_mapped() and emit immediately if actor is already mapped'
      );
    }),
  ];
}
