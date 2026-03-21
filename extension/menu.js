import St from 'gi://St';
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const MENU_STATE = {
    HIDDEN: 0,
    OVERVIEW: 1,
    ZOOMED: 2,
};

const LAYOUT_OPTIONS = [
    { id: 1, name: 'Fullscreen', label: '1: Fullscreen', icon: '[ ]' },
    { id: 2, name: 'SideBySide', label: '2: Side-by-side', icon: '[|]' },
    { id: 3, name: 'TopBottom', label: '3: Top-bottom', icon: '[-]' },
    { id: 4, name: 'Quadrants', label: '4: Quadrants', icon: '[+]' },
];

export class MenuOverlay {
    constructor() {
        this._state = MENU_STATE.HIDDEN;
        this._overlay = null;
        this._monitors = [];
        this._zoomedMonitorId = null;
        this._keyCallback = null;
    }

    setKeyCallback(callback) {
        this._keyCallback = callback;
    }

    showOverview(monitorsJson) {
        this._monitors = JSON.parse(monitorsJson);
        this._state = MENU_STATE.OVERVIEW;
        this._buildOverlay();
        this._renderOverview();
    }

    showZoomed(monitorId, layoutsJson) {
        this._zoomedMonitorId = monitorId;
        this._state = MENU_STATE.ZOOMED;
        const layouts = JSON.parse(layoutsJson);
        this._buildOverlay();
        this._renderZoomed(monitorId, layouts);
    }

    hide() {
        this._state = MENU_STATE.HIDDEN;
        this._zoomedMonitorId = null;
        this._destroyOverlay();
    }

    destroy() {
        this.hide();
        this._keyCallback = null;
    }

    _buildOverlay() {
        this._destroyOverlay();

        this._overlay = new St.Widget({
            reactive: true,
            x_expand: true,
            y_expand: true,
            style_class: 'tiler-menu-overlay',
        });

        // Semi-transparent background
        this._overlay.set_style(
            'background-color: rgba(0, 0, 0, 0.75); ' +
            'border-radius: 12px; ' +
            'padding: 24px;'
        );

        this._overlay.connect('key-press-event', (actor, event) => {
            return this._onKeyPressEvent(actor, event);
        });

        Main.layoutManager.addTopChrome(this._overlay);

        // Center the overlay on the primary monitor
        const primary = Main.layoutManager.primaryMonitor;
        if (primary) {
            const overlayWidth = Math.min(primary.width * 0.6, 800);
            const overlayHeight = Math.min(primary.height * 0.5, 500);
            this._overlay.set_size(overlayWidth, overlayHeight);
            this._overlay.set_position(
                primary.x + (primary.width - overlayWidth) / 2,
                primary.y + (primary.height - overlayHeight) / 2,
            );
        }

        this._overlay.grab_key_focus();
    }

    _destroyOverlay() {
        if (this._overlay) {
            Main.layoutManager.removeChrome(this._overlay);
            this._overlay.destroy();
            this._overlay = null;
        }
    }

    _renderOverview() {
        if (!this._overlay)
            return;

        const container = new St.BoxLayout({
            vertical: true,
            x_expand: true,
            y_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });

        // Title
        const title = new St.Label({
            text: 'Tiler — Select Monitor',
            style: 'font-size: 18px; font-weight: bold; color: white; margin-bottom: 16px;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        container.add_child(title);

        // Monitor boxes in a horizontal row
        const monitorRow = new St.BoxLayout({
            vertical: false,
            x_align: Clutter.ActorAlign.CENTER,
            style: 'spacing: 12px;',
        });

        for (const monitor of this._monitors) {
            const monitorBox = this._createMonitorWidget(monitor);
            monitorRow.add_child(monitorBox);
        }

        container.add_child(monitorRow);

        // Instructions
        const hint = new St.Label({
            text: 'Press number to select | Shift+number to move window | Esc to close',
            style: 'font-size: 12px; color: #aaa; margin-top: 16px;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        container.add_child(hint);

        this._overlay.add_child(container);
    }

    _createMonitorWidget(monitor) {
        const box = new St.BoxLayout({
            vertical: true,
            style:
                'background-color: rgba(255,255,255,0.1); ' +
                'border: 2px solid rgba(255,255,255,0.3); ' +
                'border-radius: 8px; ' +
                'padding: 12px; ' +
                'min-width: 120px;',
            x_align: Clutter.ActorAlign.CENTER,
        });

        const label = new St.Label({
            text: `${monitor.id + 1}`,
            style: 'font-size: 24px; font-weight: bold; color: white;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(label);

        const nameLabel = new St.Label({
            text: monitor.name || `Monitor ${monitor.id + 1}`,
            style: 'font-size: 11px; color: #ccc;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(nameLabel);

        const sizeLabel = new St.Label({
            text: `${monitor.width}x${monitor.height}`,
            style: 'font-size: 10px; color: #999;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(sizeLabel);

        return box;
    }

    _renderZoomed(monitorId, layouts) {
        if (!this._overlay)
            return;

        const container = new St.BoxLayout({
            vertical: true,
            x_expand: true,
            y_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });

        // Title
        const title = new St.Label({
            text: `Monitor ${monitorId + 1} — Select Layout`,
            style: 'font-size: 18px; font-weight: bold; color: white; margin-bottom: 16px;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        container.add_child(title);

        // Layout option grid (2x2)
        const grid = new St.BoxLayout({
            vertical: true,
            x_align: Clutter.ActorAlign.CENTER,
            style: 'spacing: 8px;',
        });

        for (let row = 0; row < 2; row++) {
            const rowBox = new St.BoxLayout({
                vertical: false,
                style: 'spacing: 8px;',
            });

            for (let col = 0; col < 2; col++) {
                const idx = row * 2 + col;
                if (idx < LAYOUT_OPTIONS.length) {
                    const opt = LAYOUT_OPTIONS[idx];
                    const widget = this._createLayoutOptionWidget(opt);
                    rowBox.add_child(widget);
                }
            }

            grid.add_child(rowBox);
        }

        container.add_child(grid);

        // Enforcement toggles
        const enforcementHint = new St.Label({
            text: '9: Enforce on | 0: Enforce off | Esc: Close',
            style: 'font-size: 12px; color: #aaa; margin-top: 16px;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        container.add_child(enforcementHint);

        this._overlay.add_child(container);
    }

    _createLayoutOptionWidget(option) {
        const box = new St.BoxLayout({
            vertical: true,
            style:
                'background-color: rgba(255,255,255,0.1); ' +
                'border: 1px solid rgba(255,255,255,0.2); ' +
                'border-radius: 6px; ' +
                'padding: 8px; ' +
                'min-width: 100px;',
            x_align: Clutter.ActorAlign.CENTER,
        });

        const icon = new St.Label({
            text: option.icon,
            style: 'font-size: 20px; font-family: monospace; color: white;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(icon);

        const label = new St.Label({
            text: option.label,
            style: 'font-size: 11px; color: #ccc;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(label);

        return box;
    }

    _onKeyPressEvent(actor, event) {
        const keyval = event.get_key_symbol();
        const state = event.get_state();

        const keyName = Clutter.keyval_name(keyval);
        const modifiers = [];
        if (state & Clutter.ModifierType.SHIFT_MASK)
            modifiers.push('shift');
        if (state & Clutter.ModifierType.CONTROL_MASK)
            modifiers.push('ctrl');
        if (state & Clutter.ModifierType.MOD1_MASK)
            modifiers.push('alt');
        if (state & Clutter.ModifierType.SUPER_MASK)
            modifiers.push('super');

        const modString = modifiers.join('+');

        // Forward key event to D-Bus callback
        if (this._keyCallback)
            this._keyCallback(keyName, modString);

        // Handle Escape to close
        if (keyName === 'Escape') {
            this.hide();
            return Clutter.EVENT_STOP;
        }

        return Clutter.EVENT_STOP;
    }
}
