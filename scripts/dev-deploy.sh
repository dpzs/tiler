#!/usr/bin/env bash
# dev-deploy.sh — Build, deploy, and smoke-test the tiler in one shot.
#
# Usage: ./scripts/dev-deploy.sh [--no-restart]
#
# Steps:
#   1. Copy extension files to user GNOME extensions dir
#   2. Build the daemon binary via nix develop + cargo
#   3. Restart the tiler systemd service (using dev binary)
#   4. Run D-Bus smoke tests to verify end-to-end connectivity
#
# Requires: nix, gdbus, systemctl

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
EXT_SRC="$PROJECT_DIR/extension"
EXT_DEST="$HOME/.local/share/gnome-shell/extensions/tiler@gnome-extensions"
TARGET="x86_64-unknown-linux-musl"
BINARY="$PROJECT_DIR/target/$TARGET/debug/tiler"
SOCKET="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/tiler.sock"
DBUS_NAME="org.gnome.Shell.Extensions.Tiler"
DBUS_PATH="/org/gnome/Shell/Extensions/Tiler"
DBUS_IFACE="org.gnome.Shell.Extensions.Tiler"

NO_RESTART=false
for arg in "$@"; do
    case "$arg" in
        --no-restart) NO_RESTART=true ;;
    esac
done

bold()  { printf '\033[1m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
yellow(){ printf '\033[33m%s\033[0m\n' "$*"; }

FAIL=0

# ── Step 1: Deploy extension files ──────────────────────────────────────────
bold "Step 1: Deploying extension files"

mkdir -p "$EXT_DEST"
for f in metadata.json extension.js dbus.js menu.js dbus-interface.xml; do
    if [ -f "$EXT_SRC/$f" ]; then
        # Remove read-only target first (NixOS may install read-only copies)
        [ -f "$EXT_DEST/$f" ] && chmod u+w "$EXT_DEST/$f" 2>/dev/null || true
        cp "$EXT_SRC/$f" "$EXT_DEST/$f"
        echo "  copied $f"
    else
        red "  MISSING: $EXT_SRC/$f"
        FAIL=1
    fi
done

if [ "$FAIL" -eq 0 ]; then
    green "  Extension files deployed to $EXT_DEST"
    yellow "  NOTE: Extension JS changes require GNOME Shell restart (log out/in) to take effect"
else
    red "  Extension deployment had errors"
fi

# ── Step 2: Build daemon ────────────────────────────────────────────────────
bold "Step 2: Building daemon"

cd "$PROJECT_DIR"
if nix develop --command cargo build --target "$TARGET" 2>&1; then
    green "  Build succeeded: $BINARY"
else
    red "  Build FAILED"
    exit 1
fi

# ── Step 3: Restart daemon service ──────────────────────────────────────────
if [ "$NO_RESTART" = false ]; then
    bold "Step 3: Restarting tiler daemon"

    # Create a temporary override to use the dev binary
    OVERRIDE_DIR="$HOME/.config/systemd/user/tiler.service.d"
    mkdir -p "$OVERRIDE_DIR"
    cat > "$OVERRIDE_DIR/dev-override.conf" <<EOF
[Service]
ExecStart=
ExecStart=$BINARY daemon
EOF

    systemctl --user daemon-reload
    systemctl --user restart tiler.service
    sleep 0.5

    if systemctl --user is-active --quiet tiler.service; then
        green "  Daemon restarted (using dev binary)"
    else
        red "  Daemon failed to start"
        systemctl --user status tiler.service --no-pager || true
        exit 1
    fi
else
    bold "Step 3: Skipping daemon restart (--no-restart)"
fi

# ── Step 4: Smoke tests ────────────────────────────────────────────────────
bold "Step 4: Running smoke tests"

smoke_pass=0
smoke_fail=0

smoke() {
    local name="$1"
    shift
    if output=$("$@" 2>&1); then
        green "  PASS: $name"
        smoke_pass=$((smoke_pass + 1))
        return 0
    else
        red "  FAIL: $name"
        echo "        $output"
        smoke_fail=$((smoke_fail + 1))
        return 1
    fi
}

# Test 1: Socket exists
smoke "Unix socket exists" test -S "$SOCKET"

# Test 2: CLI status command
smoke "CLI status responds" "$BINARY" status

# Test 3: D-Bus name is owned (extension is registered)
smoke "D-Bus name owned" gdbus call --session \
    --dest org.freedesktop.DBus \
    --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.NameHasOwner "$DBUS_NAME"

# Test 4: GetMonitors returns valid JSON
smoke "GetMonitors returns JSON" bash -c "
    result=\$(gdbus call --session --dest '$DBUS_NAME' --object-path '$DBUS_PATH' --method '$DBUS_IFACE.GetMonitors' 2>&1) || exit 1
    # Extract the string from the gdbus tuple output: ('json_string',)
    echo \"\$result\" | grep -q 'width' || { echo \"No monitor data: \$result\"; exit 1; }
"

# Test 5: GetActiveWorkspace returns a number
smoke "GetActiveWorkspace responds" gdbus call --session \
    --dest "$DBUS_NAME" --object-path "$DBUS_PATH" \
    --method "$DBUS_IFACE.GetActiveWorkspace"

# Test 6: ListWindows returns valid JSON
smoke "ListWindows returns JSON" bash -c "
    result=\$(gdbus call --session --dest '$DBUS_NAME' --object-path '$DBUS_PATH' --method '$DBUS_IFACE.ListWindows' 2>&1) || exit 1
    echo \"\$result\" | grep -qE 'id|\\[\\]' || { echo \"Bad response: \$result\"; exit 1; }
"

# Test 7: CLI windows command
smoke "CLI windows command" bash -c "
    result=\$('$BINARY' windows 2>&1) || exit 1
    echo \"\$result\" | grep -qE 'id|\\[\\]' || { echo \"Bad response: \$result\"; exit 1; }
"

# Test 8: D-Bus WindowOpened signal has monitor_id parameter (check introspection)
smoke "WindowOpened signal has monitor_id arg" bash -c "
    xml=\$(gdbus introspect --session --dest '$DBUS_NAME' --object-path '$DBUS_PATH' 2>&1) || exit 1
    echo \"\$xml\" | grep -A5 'WindowOpened' | grep -q 'monitor_id' || { echo \"WindowOpened missing monitor_id in D-Bus interface\"; exit 1; }
"

echo ""
bold "Results: $smoke_pass passed, $smoke_fail failed"

if [ "$smoke_fail" -gt 0 ]; then
    red "Some smoke tests failed — see above for details"
    exit 1
else
    green "All smoke tests passed — tiler is operational"
fi
