import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

const SERVICE_NAME = 'org.gnome.Shell.Extensions.Tiler';
const OBJECT_PATH = '/org/gnome/Shell/Extensions/Tiler';

const INTERFACE_XML = `
<node name="/org/gnome/Shell/Extensions/Tiler">
  <interface name="org.gnome.Shell.Extensions.Tiler">
    <method name="ListWindows">
      <arg name="windows_json" type="s" direction="out"/>
    </method>
    <method name="MoveResizeWindow">
      <arg name="window_id" type="t" direction="in"/>
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
      <arg name="width" type="i" direction="in"/>
      <arg name="height" type="i" direction="in"/>
    </method>
    <method name="GetMonitors">
      <arg name="monitors_json" type="s" direction="out"/>
    </method>
    <method name="GetActiveWorkspace">
      <arg name="workspace_id" type="u" direction="out"/>
    </method>
    <method name="GetWindowType">
      <arg name="window_id" type="t" direction="in"/>
      <arg name="window_type" type="s" direction="out"/>
    </method>
    <method name="IsFullscreen">
      <arg name="window_id" type="t" direction="in"/>
      <arg name="is_fullscreen" type="b" direction="out"/>
    </method>
    <signal name="WindowOpened">
      <arg name="window_id" type="t"/>
      <arg name="title" type="s"/>
      <arg name="app_class" type="s"/>
    </signal>
    <signal name="WindowClosed">
      <arg name="window_id" type="t"/>
    </signal>
    <signal name="WindowFocusChanged">
      <arg name="window_id" type="t"/>
    </signal>
    <signal name="WorkspaceChanged">
      <arg name="workspace_id" type="u"/>
    </signal>
    <signal name="WindowFullscreenChanged">
      <arg name="window_id" type="t"/>
      <arg name="is_fullscreen" type="b"/>
    </signal>
    <signal name="WindowGeometryChanged">
      <arg name="window_id" type="t"/>
      <arg name="x" type="i"/>
      <arg name="y" type="i"/>
      <arg name="width" type="i"/>
      <arg name="height" type="i"/>
    </signal>
    <signal name="MenuKeyPressed">
      <arg name="key" type="s"/>
      <arg name="modifiers" type="s"/>
    </signal>
  </interface>
</node>`;

export class TilerDBusService {
    constructor() {
        this._dbusImpl = null;
        this._nameOwnerId = 0;
    }

    register() {
        const nodeInfo = Gio.DBusNodeInfo.new_for_xml(INTERFACE_XML);
        this._dbusImpl = Gio.DBusExportedObject.wrapJSObject(
            nodeInfo.interfaces[0],
            this,
        );
        this._dbusImpl.export(Gio.DBus.session, OBJECT_PATH);

        this._nameOwnerId = Gio.bus_own_name(
            Gio.BusType.SESSION,
            SERVICE_NAME,
            Gio.BusNameOwnerFlags.NONE,
            null,
            null,
            null,
        );
    }

    destroy() {
        if (this._dbusImpl) {
            this._dbusImpl.unexport();
            this._dbusImpl = null;
        }
        if (this._nameOwnerId) {
            Gio.bus_unown_name(this._nameOwnerId);
            this._nameOwnerId = 0;
        }
    }

    // --- D-Bus Method Implementations ---

    ListWindows() {
        const windowActors = global.get_window_actors();
        const windows = [];

        for (const actor of windowActors) {
            const win = actor.get_meta_window();
            if (!win)
                continue;

            const rect = win.get_frame_rect();
            const tracker = Shell.WindowTracker.get_default();
            const app = tracker.get_window_app(win);

            windows.push({
                id: win.get_stable_sequence(),
                title: win.get_title() || '',
                app_class: app ? app.get_id() : '',
                monitor_id: win.get_monitor(),
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                is_fullscreen: win.is_fullscreen(),
                workspace_id: win.get_workspace()?.index() ?? 0,
            });
        }

        return JSON.stringify(windows);
    }

    MoveResizeWindow(windowId, x, y, width, height) {
        const win = this._findWindowById(windowId);
        if (!win)
            return;

        win.move_resize_frame(false, x, y, width, height);
    }

    GetMonitors() {
        const display = global.display;
        const monitorCount = display.get_n_monitors();
        const monitors = [];

        for (let i = 0; i < monitorCount; i++) {
            const geom = display.get_monitor_geometry(i);
            monitors.push({
                id: i,
                name: this._getMonitorConnector(display, i),
                position: i,
                width: geom.width,
                height: geom.height,
            });
        }

        return JSON.stringify(monitors);
    }

    _getMonitorConnector(display, index) {
        // GNOME < 48: display.get_monitor_connector() exists
        if (typeof display.get_monitor_connector === 'function')
            return display.get_monitor_connector(index) || `Monitor-${index}`;

        // GNOME 48+: use MonitorManager from backend
        try {
            const monitorManager = global.backend.get_monitor_manager();
            const monitors = monitorManager.get_monitors();
            if (monitors[index])
                return monitors[index].get_connector() || `Monitor-${index}`;
        } catch {
            // fall through
        }

        return `Monitor-${index}`;
    }

    GetActiveWorkspace() {
        const workspaceManager = global.workspace_manager;
        return workspaceManager.get_active_workspace_index();
    }

    GetWindowType(windowId) {
        const win = this._findWindowById(windowId);
        if (!win)
            return 'unknown';

        const typeMap = {
            [Meta.WindowType.NORMAL]: 'toplevel',
            [Meta.WindowType.DIALOG]: 'dialog',
            [Meta.WindowType.MODAL_DIALOG]: 'dialog',
            [Meta.WindowType.POPUP_MENU]: 'popup',
            [Meta.WindowType.DROPDOWN_MENU]: 'popup',
            [Meta.WindowType.TOOLTIP]: 'tooltip',
            [Meta.WindowType.NOTIFICATION]: 'notification',
            [Meta.WindowType.SPLASHSCREEN]: 'splash',
            [Meta.WindowType.UTILITY]: 'utility',
        };

        return typeMap[win.get_window_type()] || 'unknown';
    }

    IsFullscreen(windowId) {
        const win = this._findWindowById(windowId);
        if (!win)
            return false;

        return win.is_fullscreen();
    }

    // --- Signal Emission Helpers ---

    emitWindowOpened(windowId, title, appClass) {
        this._emitSignal('WindowOpened',
            new GLib.Variant('(tss)', [windowId, title, appClass]));
    }

    emitWindowClosed(windowId) {
        this._emitSignal('WindowClosed',
            new GLib.Variant('(t)', [windowId]));
    }

    emitWindowFocusChanged(windowId) {
        this._emitSignal('WindowFocusChanged',
            new GLib.Variant('(t)', [windowId]));
    }

    emitWorkspaceChanged(workspaceId) {
        this._emitSignal('WorkspaceChanged',
            new GLib.Variant('(u)', [workspaceId]));
    }

    emitWindowFullscreenChanged(windowId, isFullscreen) {
        this._emitSignal('WindowFullscreenChanged',
            new GLib.Variant('(tb)', [windowId, isFullscreen]));
    }

    emitWindowGeometryChanged(windowId, x, y, width, height) {
        this._emitSignal('WindowGeometryChanged',
            new GLib.Variant('(tiiii)', [windowId, x, y, width, height]));
    }

    emitMenuKeyPressed(key, modifiers) {
        this._emitSignal('MenuKeyPressed',
            new GLib.Variant('(ss)', [key, modifiers]));
    }

    // --- Private Helpers ---

    _findWindowById(windowId) {
        const actors = global.get_window_actors();
        for (const actor of actors) {
            const win = actor.get_meta_window();
            if (win && win.get_stable_sequence() === windowId)
                return win;
        }
        return null;
    }

    _emitSignal(signalName, args) {
        if (!this._dbusImpl)
            return;

        this._dbusImpl.emit_signal(signalName, args);
    }
}
