#!/usr/bin/env bash
# Test: GNOME Shell extension Nix derivation (structural checks)
#
# Verifies the Nix files contain the correct structure for packaging
# the GNOME Shell extension as a Nix derivation exposed in the flake.
#
# No `nix` binary required — uses grep, file existence checks, and cargo test.

set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
pass=0
fail=0

run_test() {
  local desc="$1"; shift
  printf "TEST: %s ... " "$desc"
  if output=$("$@" 2>&1); then
    printf "PASS\n"; pass=$((pass + 1))
  else
    printf "FAIL\n"
    if [ -n "${output:-}" ]; then
      printf "  -> %s\n" "$output"
    fi
    fail=$((fail + 1))
  fi
}

assert_grep() {
  local pattern="$1"
  local file="$2"
  if ! grep -q "$pattern" "$file" 2>/dev/null; then
    echo "pattern '$pattern' not found in $(basename "$file")"
    return 1
  fi
}

EXTENSION_NIX="$REPO/nix/gnome-extension.nix"
FLAKE="$REPO/flake.nix"

# -- Extension derivation file --

run_test "nix/gnome-extension.nix exists" \
  test -f "$EXTENSION_NIX"

run_test "gnome-extension.nix installs to share/gnome-shell/extensions/" \
  assert_grep "share/gnome-shell/extensions" "$EXTENSION_NIX"

run_test "gnome-extension.nix references metadata.json" \
  assert_grep "metadata.json" "$EXTENSION_NIX"

run_test "gnome-extension.nix references extension.js" \
  assert_grep "extension.js" "$EXTENSION_NIX"

run_test "gnome-extension.nix references menu.js" \
  assert_grep "menu.js" "$EXTENSION_NIX"

run_test "gnome-extension.nix references dbus.js" \
  assert_grep "dbus.js" "$EXTENSION_NIX"

run_test "gnome-extension.nix derivation name contains tiler-gnome-extension" \
  assert_grep "tiler-gnome-extension" "$EXTENSION_NIX"

# -- flake.nix package exports --

run_test "flake.nix exports gnome-extension in packages" \
  assert_grep "gnome-extension" "$FLAKE"

run_test "flake.nix still exports default package" \
  assert_grep "default" "$FLAKE"

# -- Nix file syntax: balanced braces --

run_test "gnome-extension.nix has balanced braces" bash -c '
  opens=$(grep -o "{" "'"$EXTENSION_NIX"'" | wc -l)
  closes=$(grep -o "}" "'"$EXTENSION_NIX"'" | wc -l)
  if [ "$opens" -ne "$closes" ]; then
    echo "unbalanced braces (open=$opens close=$closes)"
    exit 1
  fi
'

run_test "flake.nix has balanced braces" bash -c '
  opens=$(grep -o "{" "'"$FLAKE"'" | wc -l)
  closes=$(grep -o "}" "'"$FLAKE"'" | wc -l)
  if [ "$opens" -ne "$closes" ]; then
    echo "unbalanced braces (open=$opens close=$closes)"
    exit 1
  fi
'

# -- Cargo test suite --

run_test "Cargo test suite passes" \
  cargo test --manifest-path "$REPO/Cargo.toml"

echo ""
echo "Results: $pass passed, $fail failed, $((pass + fail)) total"

if [ "$fail" -gt 0 ]; then
  exit 1
fi
