// Integration tests: verify components work together across module boundaries
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXTENSION_DIR = join(__dirname, '..');

function test(name, fn) {
  try {
    fn();
    return { name, pass: true };
  } catch (e) {
    return { name, pass: false, error: e.message };
  }
}

function readFile(name) {
  return readFileSync(join(EXTENSION_DIR, name), 'utf-8');
}

export default function tests() {
  return [
    // All required files exist
    test('integration: all extension files exist', () => {
      const files = ['metadata.json', 'dbus-interface.xml', 'extension.js', 'dbus.js', 'menu.js'];
      for (const f of files) {
        assert.ok(existsSync(join(EXTENSION_DIR, f)), `Missing: ${f}`);
      }
    }),

    // extension.js imports match actual exports in dbus.js
    test('integration: extension.js imports TilerDBusService which dbus.js exports', () => {
      const ext = readFile('extension.js');
      const dbus = readFile('dbus.js');
      assert.ok(ext.includes("from './dbus.js'"), 'extension.js must import from ./dbus.js');
      assert.ok(ext.includes('TilerDBusService'), 'extension.js must reference TilerDBusService');
      assert.ok(dbus.includes('export class TilerDBusService'), 'dbus.js must export TilerDBusService');
    }),

    // extension.js imports match actual exports in menu.js
    test('integration: extension.js imports MenuOverlay which menu.js exports', () => {
      const ext = readFile('extension.js');
      const menu = readFile('menu.js');
      assert.ok(ext.includes("from './menu.js'"), 'extension.js must import from ./menu.js');
      assert.ok(ext.includes('MenuOverlay'), 'extension.js must reference MenuOverlay');
      assert.ok(menu.includes('export class MenuOverlay'), 'menu.js must export MenuOverlay');
    }),

    // D-Bus interface XML matches dbus.js embedded XML
    test('integration: dbus-interface.xml and dbus.js define same interface name', () => {
      const xml = readFile('dbus-interface.xml');
      const dbus = readFile('dbus.js');
      const iface = 'org.gnome.Shell.Extensions.Tiler';
      assert.ok(xml.includes(iface), 'dbus-interface.xml must define interface');
      assert.ok(dbus.includes(iface), 'dbus.js must reference same interface');
    }),

    // D-Bus methods in XML match method implementations in dbus.js
    test('integration: all D-Bus methods in XML are implemented in dbus.js', () => {
      const dbus = readFile('dbus.js');
      const methods = ['ListWindows', 'MoveResizeWindow', 'GetMonitors', 'GetActiveWorkspace', 'GetWindowType', 'IsFullscreen'];
      for (const m of methods) {
        // Check method is in dbus.js as both XML definition and JS method
        const xmlPattern = `name="${m}"`;
        const jsPattern = `${m}(`;
        assert.ok(dbus.includes(xmlPattern), `dbus.js XML missing method: ${m}`);
        assert.ok(dbus.includes(jsPattern), `dbus.js missing JS implementation: ${m}`);
      }
    }),

    // D-Bus signals in XML match signal emitters in dbus.js
    test('integration: all D-Bus signals in XML have emitter methods in dbus.js', () => {
      const dbus = readFile('dbus.js');
      const signals = [
        'WindowOpened', 'WindowClosed', 'WindowFocusChanged',
        'WorkspaceChanged', 'WindowFullscreenChanged',
        'WindowGeometryChanged', 'MenuKeyPressed',
      ];
      for (const s of signals) {
        // Check signal is in embedded XML
        assert.ok(dbus.includes(`name="${s}"`), `dbus.js XML missing signal: ${s}`);
        // Check emitter method exists
        const emitterName = `emit${s}`;
        assert.ok(dbus.includes(emitterName), `dbus.js missing emitter: ${emitterName}`);
      }
    }),

    // Extension wires signal emissions correctly
    test('integration: extension.js calls D-Bus signal emitters for window events', () => {
      const ext = readFile('extension.js');
      const emitters = [
        'emitWindowOpened', 'emitWindowClosed', 'emitWindowFocusChanged',
        'emitWorkspaceChanged', 'emitWindowFullscreenChanged',
        'emitWindowGeometryChanged', 'emitMenuKeyPressed',
      ];
      for (const e of emitters) {
        assert.ok(ext.includes(e), `extension.js must call ${e}`);
      }
    }),

    // Menu key callback is wired through to D-Bus
    test('integration: menu key callback flows through to D-Bus MenuKeyPressed', () => {
      const ext = readFile('extension.js');
      assert.ok(ext.includes('setKeyCallback'), 'extension.js must call setKeyCallback on menu');
      assert.ok(ext.includes('emitMenuKeyPressed'), 'extension.js key callback must emit MenuKeyPressed');
    }),

    // Object path consistency
    test('integration: object path is consistent between XML and dbus.js', () => {
      const xml = readFile('dbus-interface.xml');
      const dbus = readFile('dbus.js');
      const path = '/org/gnome/Shell/Extensions/Tiler';
      assert.ok(xml.includes(path), 'dbus-interface.xml must define object path');
      assert.ok(dbus.includes(path), 'dbus.js must use same object path');
    }),

    // metadata.json uuid follows GNOME convention
    test('integration: metadata.json uuid matches extension naming convention', () => {
      const meta = JSON.parse(readFile('metadata.json'));
      assert.ok(
        meta.uuid.includes('@'),
        'UUID must follow name@domain convention'
      );
      assert.ok(
        meta.uuid === 'tiler@gnome-extensions',
        'UUID must be tiler@gnome-extensions'
      );
    }),

    // Cleanup completeness
    test('integration: disable() cleans up both D-Bus service and menu', () => {
      const ext = readFile('extension.js');
      const disableStart = ext.indexOf('disable(');
      const disableEnd = ext.indexOf('}', ext.indexOf('{', disableStart) + 50);
      const disableBody = ext.substring(disableStart, disableEnd + 100);
      assert.ok(disableBody.includes('_dbusService'), 'disable() must reference _dbusService');
      assert.ok(disableBody.includes('_menu'), 'disable() must reference _menu');
      assert.ok(disableBody.includes('null'), 'disable() must null out references');
    }),

    // No circular imports
    test('integration: no circular import between extension.js and dbus.js', () => {
      const dbus = readFile('dbus.js');
      assert.ok(
        !dbus.includes('./extension.js') && !dbus.includes('extension.js'),
        'dbus.js must not import extension.js (would create circular dependency)'
      );
    }),

    test('integration: no circular import between extension.js and menu.js', () => {
      const menu = readFile('menu.js');
      assert.ok(
        !menu.includes('./extension.js') && !menu.includes("from 'extension.js"),
        'menu.js must not import extension.js (would create circular dependency)'
      );
    }),

    test('integration: no circular import between dbus.js and menu.js', () => {
      const dbus = readFile('dbus.js');
      const menu = readFile('menu.js');
      assert.ok(!dbus.includes('./menu.js'), 'dbus.js must not import menu.js');
      assert.ok(!menu.includes('./dbus.js'), 'menu.js must not import dbus.js');
    }),
  ];
}
