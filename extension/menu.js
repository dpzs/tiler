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
    { id: 1, name: 'Fullscreen', label: '1: Fullscreen', icon: '⬜' },
    { id: 2, name: 'SideBySide', label: '2: Side-by-side', icon: '◧' },
    { id: 3, name: 'TopBottom', label: '3: Top-bottom', icon: '⬒' },
    { id: 4, name: 'Quadrants', label: '4: Quadrants', icon: '◫' },
];

// Map shifted digit symbols back to their base digit (US QWERTY).
// When Shift is held, Clutter returns the shifted keysym (e.g. "exclam"
// for Shift+1) instead of the digit.  The daemon expects plain "1"-"9".
const SHIFTED_DIGIT_MAP = {
    'exclam': '1',
    'at': '2',
    'numbersign': '3',
    'dollar': '4',
    'percent': '5',
    'asciicircum': '6',
    'ampersand': '7',
    'asterisk': '8',
    'parenleft': '9',
    'parenright': '0',
};

// Max dimension (px) for the monitor orientation preview box.
const PREVIEW_MAX = 72;

export class MenuOverlay {
    constructor() {
        this._state = MENU_STATE.HIDDEN;
        this._overlay = null;
        this._panel = null;
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

    // -- Private: overlay scaffold ----------------------------------------

    _buildOverlay() {
        this._destroyOverlay();

        // Full-screen dim backdrop
        this._overlay = new St.Widget({
            reactive: true,
            layout_manager: new Clutter.BinLayout(),
            style: 'background-color: rgba(0, 0, 0, 0.55);',
        });

        // Auto-sizing content panel, centered within the backdrop
        this._panel = new St.BoxLayout({
            vertical: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: true,
            y_expand: true,
            style:
                'background-color: rgba(36, 36, 36, 0.96); ' +
                'border-radius: 16px; ' +
                'padding: 32px 40px; ' +
                'border: 1px solid rgba(255,255,255,0.08);',
        });
        this._overlay.add_child(this._panel);

        this._overlay.connect('key-press-event', (_actor, event) => {
            return this._onKeyPressEvent(_actor, event);
        });

        Main.layoutManager.addTopChrome(this._overlay);

        const monitors = Main.layoutManager.monitors;
        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        for (const m of monitors) {
            minX = Math.min(minX, m.x);
            minY = Math.min(minY, m.y);
            maxX = Math.max(maxX, m.x + m.width);
            maxY = Math.max(maxY, m.y + m.height);
        }
        this._overlay.set_size(maxX - minX, maxY - minY);
        this._overlay.set_position(minX, minY);

        this._overlay.grab_key_focus();
    }

    _destroyOverlay() {
        if (this._overlay) {
            Main.layoutManager.removeChrome(this._overlay);
            this._overlay.destroy();
            this._overlay = null;
            this._panel = null;
        }
    }

    // -- Private: Overview mode -------------------------------------------

    _renderOverview() {
        if (!this._panel)
            return;

        // Title
        this._panel.add_child(new St.Label({
            text: 'Tiler — Select Monitor',
            style:
                'font-size: 16px; font-weight: bold; color: white; ' +
                'margin-bottom: 20px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        // Monitor row
        const monitorRow = new St.BoxLayout({
            vertical: false,
            x_align: Clutter.ActorAlign.CENTER,
            style: 'spacing: 16px;',
        });

        for (const monitor of this._monitors)
            monitorRow.add_child(this._createMonitorWidget(monitor));

        this._panel.add_child(monitorRow);

        // Hint
        this._panel.add_child(new St.Label({
            text: 'Press number to select  ·  Shift+number to move window  ·  Esc to close',
            style:
                'font-size: 11px; color: rgba(255,255,255,0.45); ' +
                'margin-top: 20px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));
    }

    _createMonitorWidget(monitor) {
        const box = new St.BoxLayout({
            vertical: true,
            x_align: Clutter.ActorAlign.CENTER,
            style:
                'background-color: rgba(255,255,255,0.06); ' +
                'border: 1px solid rgba(255,255,255,0.12); ' +
                'border-radius: 10px; ' +
                'padding: 16px 20px;',
        });

        // Aspect-ratio-correct orientation preview
        const w = monitor.width || 1;
        const h = monitor.height || 1;
        const scale = PREVIEW_MAX / Math.max(w, h);
        const previewW = Math.round(w * scale);
        const previewH = Math.round(h * scale);

        const preview = new St.Widget({
            style:
                `width: ${previewW}px; height: ${previewH}px; ` +
                'background-color: rgba(255,255,255,0.10); ' +
                'border: 1px solid rgba(255,255,255,0.25); ' +
                'border-radius: 4px;',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(preview);

        // Key number
        box.add_child(new St.Label({
            text: `${monitor.id + 1}`,
            style:
                'font-size: 22px; font-weight: bold; color: white; ' +
                'margin-top: 10px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        // Connector name
        box.add_child(new St.Label({
            text: monitor.name || `Monitor ${monitor.id + 1}`,
            style: 'font-size: 11px; color: rgba(255,255,255,0.55);',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        // Resolution
        box.add_child(new St.Label({
            text: `${monitor.width}\u00D7${monitor.height}`,
            style:
                'font-size: 10px; color: rgba(255,255,255,0.35); ' +
                'margin-top: 2px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        return box;
    }

    // -- Private: Zoomed (layout picker) mode -----------------------------

    _renderZoomed(monitorId, layouts) {
        if (!this._panel)
            return;

        // Title
        this._panel.add_child(new St.Label({
            text: `Monitor ${monitorId + 1} — Select Layout`,
            style:
                'font-size: 16px; font-weight: bold; color: white; ' +
                'margin-bottom: 20px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        // 2x2 layout grid
        const grid = new St.BoxLayout({
            vertical: true,
            x_align: Clutter.ActorAlign.CENTER,
            style: 'spacing: 10px;',
        });

        for (let row = 0; row < 2; row++) {
            const rowBox = new St.BoxLayout({
                vertical: false,
                style: 'spacing: 10px;',
            });

            for (let col = 0; col < 2; col++) {
                const idx = row * 2 + col;
                if (idx < LAYOUT_OPTIONS.length)
                    rowBox.add_child(this._createLayoutOptionWidget(LAYOUT_OPTIONS[idx]));
            }

            grid.add_child(rowBox);
        }

        this._panel.add_child(grid);

        // Enforcement hint
        this._panel.add_child(new St.Label({
            text: '9: Enforce on  ·  0: Enforce off  ·  Esc: Close',
            style:
                'font-size: 11px; color: rgba(255,255,255,0.45); ' +
                'margin-top: 20px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));
    }

    _createLayoutOptionWidget(option) {
        const box = new St.BoxLayout({
            vertical: true,
            x_align: Clutter.ActorAlign.CENTER,
            style:
                'background-color: rgba(255,255,255,0.06); ' +
                'border: 1px solid rgba(255,255,255,0.10); ' +
                'border-radius: 8px; ' +
                'padding: 12px 16px; ' +
                'min-width: 110px;',
        });

        box.add_child(new St.Label({
            text: option.icon,
            style: 'font-size: 24px; color: white;',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        box.add_child(new St.Label({
            text: option.label,
            style:
                'font-size: 11px; color: rgba(255,255,255,0.7); ' +
                'margin-top: 6px;',
            x_align: Clutter.ActorAlign.CENTER,
        }));

        return box;
    }

    // -- Private: key handling --------------------------------------------

    _onKeyPressEvent(actor, event) {
        const keyval = event.get_key_symbol();
        const state = event.get_state();

        let keyName = Clutter.keyval_name(keyval);

        const modifiers = [];
        if (state & Clutter.ModifierType.SHIFT_MASK)
            modifiers.push('shift');
        if (state & Clutter.ModifierType.CONTROL_MASK)
            modifiers.push('ctrl');
        if (state & Clutter.ModifierType.MOD1_MASK)
            modifiers.push('alt');
        if (state & Clutter.ModifierType.SUPER_MASK)
            modifiers.push('super');

        // Resolve shifted digit symbols to their base digit so the daemon
        // receives "1"-"9" regardless of keyboard layout shift behaviour.
        if (SHIFTED_DIGIT_MAP[keyName])
            keyName = SHIFTED_DIGIT_MAP[keyName];

        const modString = modifiers.join('+');

        if (this._keyCallback)
            this._keyCallback(keyName, modString);

        return Clutter.EVENT_STOP;
    }
}
