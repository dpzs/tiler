// Tests for extension/dbus.js — D-Bus service module
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXTENSION_DIR = join(__dirname, '..');
const DBUS_PATH = join(EXTENSION_DIR, 'dbus.js');

function test(name, fn) {
  try {
    fn();
    return { name, pass: true };
  } catch (e) {
    return { name, pass: false, error: e.message };
  }
}

function getSrc() {
  return readFileSync(DBUS_PATH, 'utf-8');
}

export default function tests() {
  return [
    test('dbus.js exists', () => {
      assert.ok(existsSync(DBUS_PATH), `File not found: ${DBUS_PATH}`);
    }),

    test('dbus.js defines SERVICE_NAME constant', () => {
      const src = getSrc();
      assert.ok(
        src.includes('org.gnome.Shell.Extensions.Tiler'),
        'Must reference the D-Bus service name'
      );
    }),

    test('dbus.js defines OBJECT_PATH constant', () => {
      const src = getSrc();
      assert.ok(
        src.includes('/org/gnome/Shell/Extensions/Tiler'),
        'Must reference the D-Bus object path'
      );
    }),

    test('dbus.js loads the D-Bus interface XML', () => {
      const src = getSrc();
      assert.ok(
        src.includes('dbus-interface.xml') || src.includes('INTERFACE_XML') || src.includes('interfaceXml'),
        'Must reference the D-Bus interface XML file or embed the XML'
      );
    }),

    test('dbus.js implements ListWindows method', () => {
      const src = getSrc();
      assert.ok(src.includes('ListWindows'), 'Must implement ListWindows');
    }),

    test('dbus.js implements MoveResizeWindow method', () => {
      const src = getSrc();
      assert.ok(src.includes('MoveResizeWindow'), 'Must implement MoveResizeWindow');
    }),

    test('dbus.js implements GetMonitors method', () => {
      const src = getSrc();
      assert.ok(src.includes('GetMonitors'), 'Must implement GetMonitors');
    }),

    test('dbus.js implements GetActiveWorkspace method', () => {
      const src = getSrc();
      assert.ok(src.includes('GetActiveWorkspace'), 'Must implement GetActiveWorkspace');
    }),

    test('dbus.js implements GetWindowType method', () => {
      const src = getSrc();
      assert.ok(src.includes('GetWindowType'), 'Must implement GetWindowType');
    }),

    test('dbus.js implements IsFullscreen method', () => {
      const src = getSrc();
      assert.ok(src.includes('IsFullscreen'), 'Must implement IsFullscreen');
    }),

    test('dbus.js has signal emission for WindowOpened', () => {
      const src = getSrc();
      assert.ok(src.includes('WindowOpened'), 'Must emit WindowOpened signal');
    }),

    test('dbus.js has signal emission for WindowClosed', () => {
      const src = getSrc();
      assert.ok(src.includes('WindowClosed'), 'Must emit WindowClosed signal');
    }),

    test('dbus.js has signal emission for WindowFocusChanged', () => {
      const src = getSrc();
      assert.ok(src.includes('WindowFocusChanged'), 'Must emit WindowFocusChanged signal');
    }),

    test('dbus.js has signal emission for WorkspaceChanged', () => {
      const src = getSrc();
      assert.ok(src.includes('WorkspaceChanged'), 'Must emit WorkspaceChanged signal');
    }),

    test('dbus.js has signal emission for WindowFullscreenChanged', () => {
      const src = getSrc();
      assert.ok(src.includes('WindowFullscreenChanged'), 'Must emit WindowFullscreenChanged signal');
    }),

    test('dbus.js has signal emission for WindowGeometryChanged', () => {
      const src = getSrc();
      assert.ok(src.includes('WindowGeometryChanged'), 'Must emit WindowGeometryChanged signal');
    }),

    test('dbus.js has signal emission for MenuKeyPressed', () => {
      const src = getSrc();
      assert.ok(src.includes('MenuKeyPressed'), 'Must emit MenuKeyPressed signal');
    }),

    test('dbus.js exports a TilerDBusService class', () => {
      const src = getSrc();
      assert.ok(
        src.includes('class TilerDBusService') || src.includes('TilerDBusService'),
        'Must define TilerDBusService class'
      );
      assert.ok(src.includes('export'), 'Must export the service class');
    }),

    test('dbus.js has a register/export method for D-Bus', () => {
      const src = getSrc();
      assert.ok(
        src.includes('register') || src.includes('export_') || src.includes('start'),
        'Must have a method to register/start the D-Bus service'
      );
    }),

    test('dbus.js has an unregister/cleanup method', () => {
      const src = getSrc();
      assert.ok(
        src.includes('unregister') || src.includes('unexport') || src.includes('destroy') || src.includes('stop'),
        'Must have a method to unregister/stop the D-Bus service'
      );
    }),

    test('dbus.js uses Gio for D-Bus operations', () => {
      const src = getSrc();
      assert.ok(src.includes('Gio'), 'Must use Gio library for D-Bus');
    }),

    test('dbus.js uses GLib for variant types', () => {
      const src = getSrc();
      assert.ok(
        src.includes('GLib') || src.includes('GLib.Variant'),
        'Must use GLib for D-Bus variant types'
      );
    }),

    test('dbus.js uses JSON.stringify for window list serialization', () => {
      const src = getSrc();
      assert.ok(
        src.includes('JSON.stringify'),
        'Must use JSON.stringify to serialize window/monitor data'
      );
    }),

    test('dbus.js emits signals via D-Bus connection', () => {
      const src = getSrc();
      assert.ok(
        src.includes('emit_signal') || src.includes('emitSignal') || src.includes('emit_signal_'),
        'Must emit D-Bus signals via the connection'
      );
    }),
  ];
}
