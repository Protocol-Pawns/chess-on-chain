#!/bin/bash
# Runs the AI gas test N times and extracts Medium/Hard gas numbers.
# Usage: ./measure_gas.sh <runs>
# The test will fail on VeryHard (>1000 TGas) but Medium/Hard numbers are still printed.

RUNS="${1:-3}"

for i in $(seq 1 "$RUNS"); do
    echo "=== Run $i/$RUNS ==="
    cargo test -p chess-test --test chess test_ai_gas_budgets -- --nocapture 2>&1 \
        | rg "FULL-SEARCH"
    echo ""
done
