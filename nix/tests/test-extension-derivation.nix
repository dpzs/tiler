# Test: GNOME Shell extension Nix derivation
#
# Evaluates the flake and verifies:
#   1. packages.x86_64-linux.gnome-extension exists and is a derivation
#   2. The derivation name contains "tiler-gnome-extension"
#   3. packages.x86_64-linux.default still exists (existing functionality)
#
# Run with:
#   nix eval --file nix/tests/test-extension-derivation.nix

let
  flake = builtins.getFlake (toString ../..);
  pkgs = flake.packages.x86_64-linux;

  # Test 1: gnome-extension output exists and is a derivation
  test-gnome-extension-is-derivation =
    let
      drv = pkgs.gnome-extension;
    in
      assert drv.type == "derivation";
      true;

  # Test 2: derivation name contains "tiler-gnome-extension"
  test-gnome-extension-name =
    let
      name = pkgs.gnome-extension.name;
    in
      assert builtins.match ".*tiler-gnome-extension.*" name != null;
      true;

  # Test 3: default package still exists and is a derivation
  test-default-still-exists =
    let
      drv = pkgs.default;
    in
      assert drv.type == "derivation";
      true;

in
  assert test-gnome-extension-is-derivation;
  assert test-gnome-extension-name;
  assert test-default-still-exists;
  "all extension derivation tests passed"
