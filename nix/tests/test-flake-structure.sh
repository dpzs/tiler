#!/usr/bin/env bash
# Test: flake.nix structural checks
#
# Verifies flake.nix declares the expected inputs, outputs, and structure.
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

FLAKE="$REPO/flake.nix"

# -- Flake file exists --

run_test "flake.nix exists" \
  test -f "$FLAKE"

# -- Inputs --

run_test "flake.nix declares inputs.nixpkgs" \
  assert_grep "nixpkgs" "$FLAKE"

run_test "flake.nix declares inputs.crane" \
  assert_grep "crane" "$FLAKE"

# -- Outputs --

run_test "flake.nix outputs include packages" \
  assert_grep "packages" "$FLAKE"

run_test "flake.nix outputs include nixosModules" \
  assert_grep "nixosModules" "$FLAKE"

run_test "flake.nix outputs include devShells" \
  assert_grep "devShells" "$FLAKE"

# -- Nix file syntax: balanced braces --

run_test "flake.nix has balanced braces" bash -c '
  opens=$(grep -o "{" "'"$FLAKE"'" | wc -l)
  closes=$(grep -o "}" "'"$FLAKE"'" | wc -l)
  if [ "$opens" -ne "$closes" ]; then
    echo "unbalanced braces (open=$opens close=$closes)"
    exit 1
  fi
'

echo ""
echo "Results: $pass passed, $fail failed, $((pass + fail)) total"

if [ "$fail" -gt 0 ]; then
  exit 1
fi
