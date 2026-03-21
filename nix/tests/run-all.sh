#!/usr/bin/env bash
# Run all nix test scripts and aggregate results
#
# Runs every test-*.sh in this directory, collects pass/fail counts,
# and reports an aggregate summary.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

total_pass=0
total_fail=0
total_scripts=0
failed_scripts=()

for script in "$SCRIPT_DIR"/test-*.sh; do
  [ -f "$script" ] || continue
  name="$(basename "$script")"
  printf "\n=== %s ===\n" "$name"
  total_scripts=$((total_scripts + 1))

  if output=$(bash "$script" 2>&1); then
    echo "$output"
    # Parse pass/fail from "Results: N passed, M failed" line
    p=$(echo "$output" | grep -oP '\d+(?= passed)' || echo 0)
    f=$(echo "$output" | grep -oP '\d+(?= failed)' || echo 0)
    total_pass=$((total_pass + p))
    total_fail=$((total_fail + f))
  else
    echo "$output"
    p=$(echo "$output" | grep -oP '\d+(?= passed)' || echo 0)
    f=$(echo "$output" | grep -oP '\d+(?= failed)' || echo 0)
    total_pass=$((total_pass + p))
    total_fail=$((total_fail + f))
    failed_scripts+=("$name")
  fi
done

echo ""
echo "========================================"
echo "AGGREGATE: $total_pass passed, $total_fail failed across $total_scripts scripts"
if [ ${#failed_scripts[@]} -gt 0 ]; then
  echo "FAILED SCRIPTS: ${failed_scripts[*]}"
  exit 1
else
  echo "ALL SCRIPTS PASSED"
fi
