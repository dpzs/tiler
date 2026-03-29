#!/usr/bin/env bash
# nested-test.sh — Launch a nested GNOME Shell for interactive tiler testing.
#
# Starts an isolated D-Bus session with a visible GNOME Shell window,
# deploys the extension, registers the keybinding, and starts the daemon.
# Press Super+T in the nested window to test. Ctrl+C or close window to exit.
#
# Usage: nix develop --command bash scripts/nested-test.sh
#   or:  bash scripts/nested-test.sh          (if deps are on PATH)
#
# Options:
#   --skip-build    Skip cargo build (use existing binary)
#   --headless      Use headless mode instead of nested (for CI)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET="x86_64-unknown-linux-musl"
TILER_BIN="$PROJECT_DIR/target/$TARGET/debug/tiler"
EXT_SRC="$PROJECT_DIR/extension"
EXT_DEST="$HOME/.local/share/gnome-shell/extensions/tiler@gnome-extensions"

SKIP_BUILD=false
HEADLESS=false
for arg in "$@"; do
    case "$arg" in
        --skip-build) SKIP_BUILD=true ;;
        --headless)   HEADLESS=true ;;
    esac
done

bold() { printf '\033[1m%s\033[0m\n' "$*"; }

# ── Build ────────────────────────────────────────────────────────────────
if [[ "$SKIP_BUILD" == false ]]; then
    bold "Building tiler..."
    (cd "$PROJECT_DIR" && cargo build --target "$TARGET" 2>&1 | tail -3)
fi

if [[ ! -x "$TILER_BIN" ]]; then
    echo "ERROR: Binary not found: $TILER_BIN"
    echo "Run 'cargo build --target $TARGET' first, or drop --skip-build."
    exit 1
fi

# ── Deploy extension ─────────────────────────────────────────────────────
bold "Deploying extension..."
mkdir -p "$EXT_DEST"
for f in metadata.json extension.js dbus.js menu.js dbus-interface.xml; do
    [ -f "$EXT_DEST/$f" ] && chmod u+w "$EXT_DEST/$f" 2>/dev/null || true
    cp "$EXT_SRC/$f" "$EXT_DEST/$f"
done

# ── Write inner script (runs inside dbus-run-session) ────────────────────
INNER_SCRIPT=$(mktemp)

cat > "$INNER_SCRIPT" << 'INNER_EOF'
#!/usr/bin/env bash
set -euo pipefail

TILER_BIN="$1"
HEADLESS="$2"

DBUS_NAME="org.gnome.Shell.Extensions.Tiler"

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
info() { printf '  %s\n' "$*"; }

cleanup() {
    bold "Shutting down..."
    kill "$DAEMON_PID" 2>/dev/null || true
    kill "$SHELL_PID" 2>/dev/null || true
    wait 2>/dev/null || true
    rm -f /tmp/nested-daemon.log
}
trap cleanup EXIT

# ── Configure dconf ──────────────────────────────────────────────────────
bold "Configuring keybinding & extension..."

# Enable extension
gsettings set org.gnome.shell enabled-extensions "['tiler@gnome-extensions']"

# Register custom keybinding — write ALL keys to user dconf directly
TILER_KB_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/tiler/"
DCONF_LIST="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"

dconf write "$DCONF_LIST" "['$TILER_KB_PATH']"
dconf write "${TILER_KB_PATH}name" "'Tiler Menu'"
dconf write "${TILER_KB_PATH}command" "'$TILER_BIN menu'"
dconf write "${TILER_KB_PATH}binding" "'<Super>t'"

info "Keybinding: <Super>t → $TILER_BIN menu"

# ── Launch GNOME Shell ───────────────────────────────────────────────────
if [[ "$HEADLESS" == "true" ]]; then
    bold "Starting GNOME Shell (headless)..."
    gnome-shell --headless --unsafe-mode \
        --virtual-monitor 1920x1080 \
        >/dev/null 2>&1 &
else
    bold "Starting GNOME Shell (nested)..."
    gnome-shell --nested --wayland \
        >/dev/null 2>&1 &
fi
SHELL_PID=$!

# Wait for extension D-Bus name
info "Waiting for extension to register on D-Bus..."
for i in $(seq 1 100); do
    gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
        -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null | grep -q true && break
    sleep 0.1
done

HAS_NAME=$(gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
    -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null || echo "(error)")

if [[ "$HAS_NAME" != *"true"* ]]; then
    echo "ERROR: Extension D-Bus name did not appear after 10s"
    echo "Check gnome-shell logs for extension errors."
    exit 1
fi
info "Extension D-Bus service: OK"

# ── Start daemon ─────────────────────────────────────────────────────────
bold "Starting tiler daemon..."
"$TILER_BIN" daemon > /tmp/nested-daemon.log 2>&1 &
DAEMON_PID=$!
sleep 1

STATUS=$("$TILER_BIN" status 2>&1 || echo "error")
if [[ "$STATUS" == "Ok" ]]; then
    info "Daemon status: OK"
else
    echo "WARNING: Daemon status: $STATUS"
fi

# ── Ready ────────────────────────────────────────────────────────────────
echo ""
bold "══════════════════════════════════════════════════════"
bold "  Tiler nested session is ready!"
bold "══════════════════════════════════════════════════════"
echo ""
info "GNOME Shell PID: $SHELL_PID"
info "Daemon PID:      $DAEMON_PID"
info "Daemon log:      /tmp/nested-daemon.log"
echo ""
if [[ "$HEADLESS" == "true" ]]; then
    info "Running in headless mode. Use CLI commands to test:"
    info "  $TILER_BIN status"
    info "  $TILER_BIN menu"
    info "  $TILER_BIN windows"
    info ""
    info "Press Ctrl+C to exit."
else
    info "Press Super+T in the nested window to test the keybinding."
    info "Close the window or press Ctrl+C here to exit."
fi
echo ""

# Tail daemon log in background for visibility
tail -f /tmp/nested-daemon.log 2>/dev/null &
TAIL_PID=$!

# Block until GNOME Shell exits
wait "$SHELL_PID" 2>/dev/null || true
kill "$TAIL_PID" 2>/dev/null || true
INNER_EOF

chmod +x "$INNER_SCRIPT"

# ── Launch ───────────────────────────────────────────────────────────────
bold "Launching nested GNOME Shell session..."
echo ""

dbus-run-session -- bash "$INNER_SCRIPT" "$TILER_BIN" "$HEADLESS" 2>&1 \
    | grep -v "^dbus-daemon\|^amdgpu\|^SpiRegistry\|^discover\|^goa-daemon\|Gjs-CRITICAL\|Stack trace\|^#[0-9]\|evolution\|gnome-shell-calendar\|libedbus\|libgoaident\|GVFS-Remote\|fusermount\|xdg-desktop-portal\|GLib-GIO-CRITICAL\|Gdk-Message\|error: fuse\|process:.*WARNING\|^A connection\|RNING"

rm -f "$INNER_SCRIPT"
