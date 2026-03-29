#!/usr/bin/env bash
# diagnose-keybinding.sh — Dump all state relevant to Super+T keybinding.
# Run on your NixOS host (not in a devcontainer).
#
# Usage: bash scripts/diagnose-keybinding.sh

set -euo pipefail

bold()  { printf '\033[1m%s\033[0m\n' "$*"; }
green() { printf '\033[32m  OK:   %s\033[0m\n' "$*"; }
red()   { printf '\033[31m  FAIL: %s\033[0m\n' "$*"; }
warn()  { printf '\033[33m  WARN: %s\033[0m\n' "$*"; }
info()  { printf '  INFO: %s\n' "$*"; }

FAIL_COUNT=0

# ── Custom Keybindings List ────────────────────────────────────────────────
bold "── Custom Keybindings List ──"
DCONF_LIST_KEY="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"
LIST=$(dconf read "$DCONF_LIST_KEY" 2>/dev/null || echo "")
info "dconf key: $DCONF_LIST_KEY"
info "value: ${LIST:-<unset>}"

TILER_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/tiler/"
if echo "$LIST" | grep -q "$TILER_PATH"; then
    green "Tiler path present in custom-keybindings list"
else
    red "Tiler path MISSING from custom-keybindings list"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# ── Tiler Keybinding Properties ───────────────────────────────────────────
echo ""
bold "── Tiler Keybinding Properties ──"
DCONF_TILER="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/tiler"

KB_NAME=$(dconf read "$DCONF_TILER/name" 2>/dev/null || echo "")
KB_CMD=$(dconf read "$DCONF_TILER/command" 2>/dev/null || echo "")
KB_BINDING=$(dconf read "$DCONF_TILER/binding" 2>/dev/null || echo "")

info "name:    ${KB_NAME:-<unset>}"
info "command: ${KB_CMD:-<unset>}"
info "binding: ${KB_BINDING:-<unset>}"

# Check binding value
if [[ "$KB_BINDING" == *"<Super>t"* ]]; then
    green "Binding is <Super>t"
else
    red "Binding is NOT <Super>t (got: ${KB_BINDING:-<unset>})"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Check command path exists — the primary suspect after flake updates
if [[ -n "$KB_CMD" ]]; then
    # Strip surrounding quotes from dconf output
    CMD_PATH=$(echo "$KB_CMD" | sed "s/^'//;s/'$//" | awk '{print $1}')
    if [[ -x "$CMD_PATH" ]]; then
        green "Command binary exists: $CMD_PATH"
    else
        red "Command binary DOES NOT EXIST: $CMD_PATH"
        red "This is likely the cause — the nix store path changed after flake update"
        FAIL_COUNT=$((FAIL_COUNT + 1))

        # Try to find the current tiler binary
        CURRENT=$(which tiler 2>/dev/null || echo "")
        if [[ -n "$CURRENT" ]]; then
            info "Current tiler binary: $CURRENT"
            info "FIX: dconf write $DCONF_TILER/command \"'$CURRENT menu'\""
        fi
    fi
else
    red "Command is unset — keybinding has no action"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# ── Binding Conflicts ─────────────────────────────────────────────────────
echo ""
bold "── Binding Conflicts (<Super>t) ──"
CONFLICTS=$(gsettings list-recursively 2>/dev/null | grep -i "'<Super>t'" || true)
if [[ -n "$CONFLICTS" ]]; then
    warn "Other bindings using <Super>t:"
    echo "$CONFLICTS" | while read -r line; do info "  $line"; done
else
    green "No conflicting <Super>t bindings found"
fi

# Also check all custom keybinding slots for duplicates
echo ""
bold "── All Custom Keybinding Slots ──"
if [[ -n "$LIST" ]] && [[ "$LIST" != "@as []" ]]; then
    # Parse the dconf array
    PATHS=$(echo "$LIST" | tr -d "[]'" | tr ',' '\n' | sed 's/^ *//')
    for p in $PATHS; do
        p_dconf=$(echo "$p" | sed 's|/$||')
        slot_name=$(dconf read "${p_dconf}/name" 2>/dev/null || echo "<unset>")
        slot_cmd=$(dconf read "${p_dconf}/command" 2>/dev/null || echo "<unset>")
        slot_bind=$(dconf read "${p_dconf}/binding" 2>/dev/null || echo "<unset>")
        info "Slot: $p"
        info "  name=$slot_name  binding=$slot_bind"
        info "  command=$slot_cmd"
    done
else
    info "No custom keybinding slots configured"
fi

# ── Extension Status ──────────────────────────────────────────────────────
echo ""
bold "── Extension Status ──"
ENABLED=$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo "")
info "enabled-extensions: ${ENABLED:-<unset>}"

if echo "$ENABLED" | grep -q "tiler@gnome-extensions"; then
    green "Tiler extension is enabled"
else
    red "Tiler extension is NOT in enabled-extensions"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# gnome-extensions CLI info if available
if command -v gnome-extensions &>/dev/null; then
    gnome-extensions show tiler@gnome-extensions 2>/dev/null || warn "gnome-extensions show failed"
fi

# ── D-Bus Service ─────────────────────────────────────────────────────────
echo ""
bold "── D-Bus Service ──"
DBUS_NAME="org.gnome.Shell.Extensions.Tiler"
HAS_NAME=$(gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus \
    -m org.freedesktop.DBus.NameHasOwner "$DBUS_NAME" 2>/dev/null || echo "(error)")
info "D-Bus name $DBUS_NAME: $HAS_NAME"

if [[ "$HAS_NAME" == *"true"* ]]; then
    green "Extension D-Bus service is registered"
else
    red "Extension D-Bus service is NOT registered"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# ── Daemon & Services ─────────────────────────────────────────────────────
echo ""
bold "── Systemd Services ──"
systemctl --user status tiler.service --no-pager 2>&1 | head -5 || true
echo ""
systemctl --user status tiler-keybinding.service --no-pager 2>&1 | head -5 || true

echo ""
SOCK="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/tiler.sock"
if [[ -S "$SOCK" ]]; then
    green "Daemon socket exists: $SOCK"
else
    red "Daemon socket MISSING: $SOCK"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# ── Summary ───────────────────────────────────────────────────────────────
echo ""
bold "══════════════════════════════════"
if [[ "$FAIL_COUNT" -eq 0 ]]; then
    printf '\033[32m%s\033[0m\n' "All checks passed"
else
    printf '\033[31m%s check(s) failed\033[0m\n' "$FAIL_COUNT"
    echo ""
    bold "Most likely cause:"
    info "After flake update, the nix store path for the tiler binary changed."
    info "The dconf custom-keybinding 'command' still points to the old path."
    info ""
    info "Quick fix — run on your NixOS host:"
    info "  dconf write $DCONF_TILER/command \"'\$(which tiler) menu'\""
    info ""
    info "Permanent fix — the tiler-keybinding.service should also write"
    info "the command/binding/name keys, not just the keybinding list."
fi
