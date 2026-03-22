import GLib from 'gi://GLib';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import Shell from 'gi://Shell';
import { TilerDBusService } from './dbus.js';
import { MenuOverlay } from './menu.js';

// Max ms to wait for first-frame before emitting WindowOpened anyway.
// Handles headless mode (no GPU paint) and slow-to-render windows.
const FIRST_FRAME_TIMEOUT_MS = 1000;

export default class TilerExtension extends Extension {
    enable() {
        this._dbusService = new TilerDBusService();
        this._dbusService.register();

        this._menu = new MenuOverlay();
        this._dbusService.setMenuOverlay(this._menu);
        this._menu.setKeyCallback((key, modifiers) => {
            this._dbusService.emitMenuKeyPressed(key, modifiers);
        });

        this._signalIds = [];
        this._windowSignals = new Map();
        this._pendingFirstFrame = new Map();

        this._connectDisplaySignals();
        this._connectWorkspaceSignals();
    }

    disable() {
        this._disconnectAllSignals();

        if (this._menu) {
            this._menu.destroy();
            this._menu = null;
        }

        if (this._dbusService) {
            this._dbusService.destroy();
            this._dbusService = null;
        }
    }

    _connectDisplaySignals() {
        const display = global.display;

        // Window created
        const windowCreatedId = display.connect('window-created', (d, win) => {
            this._onWindowCreated(win);
        });
        this._signalIds.push({ obj: display, id: windowCreatedId });

        // Focus window changed
        const focusId = display.connect('notify::focus-window', () => {
            const win = display.focus_window;
            if (win) {
                this._dbusService.emitWindowFocusChanged(
                    win.get_stable_sequence(),
                );
            }
        });
        this._signalIds.push({ obj: display, id: focusId });
    }

    _connectWorkspaceSignals() {
        const workspaceManager = global.workspace_manager;

        const wsChangedId = workspaceManager.connect('active-workspace-changed', () => {
            const idx = workspaceManager.get_active_workspace_index();
            this._dbusService.emitWorkspaceChanged(idx);
        });
        this._signalIds.push({ obj: workspaceManager, id: wsChangedId });
    }

    _onWindowCreated(win) {
        try {
            const windowId = win.get_stable_sequence();
            const title = win.get_title() || '';
            const tracker = Shell.WindowTracker.get_default();
            const app = tracker.get_window_app(win);
            const appClass = app ? app.get_id() : '';

            const actor = win.get_compositor_private();
            if (!actor) return;

            const emitAndConnect = () => {
                const pending = this._pendingFirstFrame.get(windowId);
                if (pending?.timeoutId)
                    GLib.source_remove(pending.timeoutId);
                this._pendingFirstFrame.delete(windowId);
                this._dbusService.emitWindowOpened(windowId, title, appClass, win.get_monitor());

                const perWindowSignals = [];

                const unmanagedId = win.connect('unmanaged', () => {
                    this._dbusService.emitWindowClosed(windowId);
                    this._disconnectWindowSignals(windowId);
                });
                perWindowSignals.push(unmanagedId);

                const fullscreenId = win.connect('notify::fullscreen', () => {
                    this._dbusService.emitWindowFullscreenChanged(
                        windowId, win.is_fullscreen(),
                    );
                });
                perWindowSignals.push(fullscreenId);

                const positionId = win.connect('position-changed', () => {
                    const rect = win.get_frame_rect();
                    if (!rect) return;
                    this._dbusService.emitWindowGeometryChanged(
                        windowId, rect.x, rect.y, rect.width, rect.height,
                    );
                });
                perWindowSignals.push(positionId);

                const sizeId = win.connect('size-changed', () => {
                    const rect = win.get_frame_rect();
                    if (!rect) return;
                    this._dbusService.emitWindowGeometryChanged(
                        windowId, rect.x, rect.y, rect.width, rect.height,
                    );
                });
                perWindowSignals.push(sizeId);

                this._windowSignals.set(windowId, { win, signals: perWindowSignals });
            };

            if (actor.is_mapped()) {
                emitAndConnect();
            } else {
                const firstFrameId = actor.connect('first-frame', () => {
                    actor.disconnect(firstFrameId);
                    emitAndConnect();
                });
                // Fallback: if first-frame never fires (headless mode, GPU stall),
                // emit after timeout so the daemon still tracks the window.
                const timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, FIRST_FRAME_TIMEOUT_MS, () => {
                    if (this._pendingFirstFrame.has(windowId)) {
                        try { actor.disconnect(firstFrameId); } catch (_) {}
                        emitAndConnect();
                    }
                    return GLib.SOURCE_REMOVE;
                });
                this._pendingFirstFrame.set(windowId, { actor, id: firstFrameId, timeoutId });
            }
        } catch (e) {
            log(`[tiler] _onWindowCreated failed: ${e.message}`);
        }
    }

    _disconnectWindowSignals(windowId) {
        const entry = this._windowSignals.get(windowId);
        if (!entry)
            return;

        for (const sigId of entry.signals) {
            try {
                entry.win.disconnect(sigId);
            } catch (e) {
                // Window may already be destroyed
            }
        }
        this._windowSignals.delete(windowId);
    }

    _disconnectAllSignals() {
        // Disconnect display/workspace signals
        if (this._signalIds) {
            for (const { obj, id } of this._signalIds) {
                try {
                    obj.disconnect(id);
                } catch (e) {
                    // Object may be destroyed
                }
            }
            this._signalIds = [];
        }

        // Disconnect pending first-frame signals and their timeout fallbacks
        if (this._pendingFirstFrame) {
            for (const [, { actor, id, timeoutId }] of this._pendingFirstFrame) {
                try {
                    if (timeoutId) GLib.source_remove(timeoutId);
                    actor.disconnect(id);
                } catch (e) {
                    // Actor may already be destroyed
                }
            }
            this._pendingFirstFrame.clear();
        }

        // Disconnect all per-window signals
        if (this._windowSignals) {
            for (const [windowId] of this._windowSignals) {
                this._disconnectWindowSignals(windowId);
            }
            this._windowSignals.clear();
        }
    }
}
