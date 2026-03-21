import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

export default class TilerExtension extends Extension {
    enable() {
        // TODO: Register D-Bus service, connect window/workspace signals
    }

    disable() {
        // TODO: Unregister D-Bus service, clean up signal handlers
    }
}
