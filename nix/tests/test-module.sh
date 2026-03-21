#!/usr/bin/env bash
# Test: NixOS module structural checks
#
# Verifies nix/module.nix contains the correct structure for:
#   - Module options (enable, package, keybinding, settings)
#   - Systemd user service for tiler daemon
#   - TOML config generation
#   - Extension installation
#   - Keybinding via dconf
#   - Flake module export
#
# No `nix` binary required — uses grep and file existence checks.

set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
pass=0
fail=0

run_test() {
  local desc="$1"; shift
  printf "TEST: %s ... " "$desc"
  if "$@" >/dev/null 2>&1; then
    printf "PASS\n"; pass=$((pass + 1))
  else
    printf "FAIL\n"; fail=$((fail + 1))
  fi
}

MODULE="$REPO/nix/module.nix"
FLAKE="$REPO/flake.nix"

# -- Module file exists --

run_test "nix/module.nix exists" test -f "$MODULE"

# -- 1. Module options --

run_test "module has services.tiler option namespace" \
  grep -q "services\.tiler" "$MODULE"

run_test "module has mkEnableOption for enable" \
  grep -q "mkEnableOption" "$MODULE"

run_test "module has keybinding option with types.str" \
  grep -q "types\.str" "$MODULE"

run_test "module has settings option with tomlFormat.type" \
  grep -q "tomlFormat\.type" "$MODULE"

run_test "module has package option with types.package" \
  grep -q "types\.package" "$MODULE"

# -- 2. Systemd user service --

run_test "module defines systemd.user.services.tiler" \
  grep -q "systemd\.user\.services\.tiler" "$MODULE"

run_test "module ExecStart references tiler daemon" \
  grep -q "tiler daemon" "$MODULE"

run_test "module sets Restart to on-failure" \
  grep -q '"on-failure"' "$MODULE"

run_test "module references graphical-session.target" \
  grep -q "graphical-session\.target" "$MODULE"

# -- 3. Config file generation --

run_test "module generates tiler/config.toml via environment.etc" \
  grep -q 'environment\.etc\."tiler/config\.toml"' "$MODULE"

run_test "module uses tomlFormat.generate" \
  grep -q "tomlFormat\.generate" "$MODULE"

# -- 4. Extension installation --

run_test "module has gnomeExtensionPackage option" \
  grep -q "gnomeExtensionPackage" "$MODULE"

run_test "module adds packages to environment.systemPackages" \
  grep -q "environment\.systemPackages" "$MODULE"

# -- 5. Keybinding via dconf --

run_test "module configures dconf keybinding" \
  grep -q "dconf" "$MODULE"

run_test "module references cfg.keybinding" \
  grep -q "cfg\.keybinding" "$MODULE"

# -- 6. Flake module export --

run_test "flake.nix exports nixosModules" \
  grep -q "nixosModules" "$FLAKE"

# -- 7. Cargo tests still pass --

run_test "Cargo test suite passes" \
  cargo test --manifest-path "$REPO/Cargo.toml"

echo ""
echo "Results: $pass passed, $fail failed, $((pass + fail)) total"
[ "$fail" -eq 0 ]
