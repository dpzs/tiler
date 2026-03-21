#!/usr/bin/env bash
# Test: Rust binary Nix derivation (structural checks)
#
# Verifies the Nix files contain the correct structure for packaging
# the Rust tiler binary as a Nix derivation exposed in the flake.
#
# No `nix` binary required — uses grep and file existence checks.

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

PACKAGE_NIX="$REPO/nix/package.nix"
FLAKE="$REPO/flake.nix"

# -- Package derivation file --

run_test "nix/package.nix exists" \
  test -f "$PACKAGE_NIX"

run_test "package.nix uses craneLib.buildPackage" \
  assert_grep "craneLib.buildPackage" "$PACKAGE_NIX"

run_test "package.nix sets CARGO_BUILD_TARGET" \
  assert_grep "CARGO_BUILD_TARGET" "$PACKAGE_NIX"

run_test "package.nix references musl" \
  assert_grep "musl" "$PACKAGE_NIX"

run_test "package.nix sets strictDeps" \
  assert_grep "strictDeps" "$PACKAGE_NIX"

# -- Nix file syntax: balanced braces --

run_test "package.nix has balanced braces" bash -c '
  opens=$(grep -o "{" "'"$PACKAGE_NIX"'" | wc -l)
  closes=$(grep -o "}" "'"$PACKAGE_NIX"'" | wc -l)
  if [ "$opens" -ne "$closes" ]; then
    echo "unbalanced braces (open=$opens close=$closes)"
    exit 1
  fi
'

# -- flake.nix cross-reference --

run_test "flake.nix imports package.nix" \
  assert_grep "package.nix" "$FLAKE"

# -- Conditional nix eval --

if command -v nix &>/dev/null; then
  run_test "nix eval: package derivation builds" \
    nix eval .#packages.x86_64-linux.tiler.name
else
  printf "SKIP: nix eval (nix not available)\n"
fi

echo ""
echo "Results: $pass passed, $fail failed, $((pass + fail)) total"

if [ "$fail" -gt 0 ]; then
  exit 1
fi
