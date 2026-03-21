// Tests for extension/dbus-interface.xml
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXTENSION_DIR = join(__dirname, '..');
const XML_PATH = join(EXTENSION_DIR, 'dbus-interface.xml');

function test(name, fn) {
  try {
    fn();
    return { name, pass: true };
  } catch (e) {
    return { name, pass: false, error: e.message };
  }
}

function getXml() {
  return readFileSync(XML_PATH, 'utf-8');
}

export default function tests() {
  return [
    test('dbus-interface.xml exists', () => {
      assert.ok(existsSync(XML_PATH), `File not found: ${XML_PATH}`);
    }),

    test('dbus-interface.xml has valid XML structure', () => {
      const xml = getXml();
      assert.ok(xml.includes('<?xml'), 'Missing XML declaration');
      assert.ok(xml.includes('<node'), 'Missing <node> element');
      assert.ok(xml.includes('</node>'), 'Missing </node> closing tag');
    }),

    test('dbus-interface.xml defines interface org.gnome.Shell.Extensions.Tiler', () => {
      const xml = getXml();
      assert.ok(
        xml.includes('name="org.gnome.Shell.Extensions.Tiler"'),
        'Missing interface name'
      );
    }),

    test('dbus-interface.xml defines ListWindows method', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="ListWindows"'), 'Missing ListWindows method');
    }),

    test('dbus-interface.xml defines MoveResizeWindow method', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="MoveResizeWindow"'), 'Missing MoveResizeWindow method');
    }),

    test('dbus-interface.xml defines GetMonitors method', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="GetMonitors"'), 'Missing GetMonitors method');
    }),

    test('dbus-interface.xml defines GetActiveWorkspace method', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="GetActiveWorkspace"'), 'Missing GetActiveWorkspace method');
    }),

    test('dbus-interface.xml defines GetWindowType method', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="GetWindowType"'), 'Missing GetWindowType method');
    }),

    test('dbus-interface.xml defines IsFullscreen method', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="IsFullscreen"'), 'Missing IsFullscreen method');
    }),

    test('dbus-interface.xml defines WindowOpened signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="WindowOpened"'), 'Missing WindowOpened signal');
    }),

    test('dbus-interface.xml defines WindowClosed signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="WindowClosed"'), 'Missing WindowClosed signal');
    }),

    test('dbus-interface.xml defines WindowFocusChanged signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="WindowFocusChanged"'), 'Missing WindowFocusChanged signal');
    }),

    test('dbus-interface.xml defines WorkspaceChanged signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="WorkspaceChanged"'), 'Missing WorkspaceChanged signal');
    }),

    test('dbus-interface.xml defines WindowFullscreenChanged signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="WindowFullscreenChanged"'), 'Missing WindowFullscreenChanged signal');
    }),

    test('dbus-interface.xml defines WindowGeometryChanged signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="WindowGeometryChanged"'), 'Missing WindowGeometryChanged signal');
    }),

    test('dbus-interface.xml defines MenuKeyPressed signal', () => {
      const xml = getXml();
      assert.ok(xml.includes('name="MenuKeyPressed"'), 'Missing MenuKeyPressed signal');
    }),

    test('MoveResizeWindow has window_id argument of type t (uint64)', () => {
      const xml = getXml();
      const methodStart = xml.indexOf('name="MoveResizeWindow"');
      assert.ok(methodStart !== -1, 'MoveResizeWindow not found');
      const methodEnd = xml.indexOf('</method>', methodStart);
      const methodBlock = xml.substring(methodStart, methodEnd);
      assert.ok(
        methodBlock.includes('name="window_id"') && methodBlock.includes('type="t"'),
        'MoveResizeWindow must have window_id arg with type="t" (uint64)'
      );
    }),

    test('MoveResizeWindow has x, y, width, height arguments of type i (int32)', () => {
      const xml = getXml();
      const methodStart = xml.indexOf('name="MoveResizeWindow"');
      const methodEnd = xml.indexOf('</method>', methodStart);
      const methodBlock = xml.substring(methodStart, methodEnd);
      for (const arg of ['x', 'y', 'width', 'height']) {
        assert.ok(
          methodBlock.includes(`name="${arg}"`),
          `MoveResizeWindow missing arg: ${arg}`
        );
      }
      const iArgs = methodBlock.match(/type="i"/g);
      assert.ok(iArgs && iArgs.length >= 4, 'MoveResizeWindow needs at least 4 int32 args');
    }),

    test('ListWindows returns a string (type="s")', () => {
      const xml = getXml();
      const methodStart = xml.indexOf('name="ListWindows"');
      const methodEnd = xml.indexOf('</method>', methodStart);
      const methodBlock = xml.substring(methodStart, methodEnd);
      assert.ok(
        methodBlock.includes('direction="out"') && methodBlock.includes('type="s"'),
        'ListWindows must return string (type="s")'
      );
    }),

    test('GetActiveWorkspace returns uint32 (type="u")', () => {
      const xml = getXml();
      const methodStart = xml.indexOf('name="GetActiveWorkspace"');
      const methodEnd = xml.indexOf('</method>', methodStart);
      const methodBlock = xml.substring(methodStart, methodEnd);
      assert.ok(
        methodBlock.includes('direction="out"') && methodBlock.includes('type="u"'),
        'GetActiveWorkspace must return uint32 (type="u")'
      );
    }),

    test('IsFullscreen returns boolean (type="b")', () => {
      const xml = getXml();
      const methodStart = xml.indexOf('name="IsFullscreen"');
      const methodEnd = xml.indexOf('</method>', methodStart);
      const methodBlock = xml.substring(methodStart, methodEnd);
      assert.ok(
        methodBlock.includes('direction="out"') && methodBlock.includes('type="b"'),
        'IsFullscreen must return boolean (type="b")'
      );
    }),
  ];
}
