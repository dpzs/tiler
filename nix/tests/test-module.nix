# Test: NixOS module evaluation
#
# Evaluates the flake and verifies:
#   1. nixosModules.default exists and is a function
#   2. The module can be imported (basic structure check)
#
# Run with:
#   nix eval --file nix/tests/test-module.nix

let
  flake = builtins.getFlake (toString ../..);

  # Test 1: nixosModules.default exists
  test-module-exists =
    assert builtins.hasAttr "default" flake.nixosModules;
    true;

  # Test 2: nixosModules.default is a function (NixOS module pattern)
  test-module-is-function =
    assert builtins.isFunction flake.nixosModules.default;
    true;

in
  assert test-module-exists;
  assert test-module-is-function;
  "all module tests passed"
