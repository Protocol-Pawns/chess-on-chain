#!/bin/bash
set -e
cd "`dirname $0`"

REGEN_DATA=false
for arg in "$@"; do
    case "$arg" in
        --regen-data) REGEN_DATA=true ;;
    esac
done

mkdir -p res

# Optionally regenerate static data (opening book, Zobrist keys).
# This downloads Stockfish and runs a lengthy analysis (~20-30 min).
# Skip by default; the committed static_book.rs / zobrist_keys.rs are used as-is.
if [ "$REGEN_DATA" = true ]; then
    echo "=== Regenerating static data (this takes a while) ==="
    bash scripts/setup.sh
    PYTHONPATH=scripts/.pydeps/chess-1.11.2 python3 scripts/generate_zobrist.py
    PYTHONPATH=scripts/.pydeps/chess-1.11.2 python3 scripts/generate_static_data.py
else
    echo "=== Using committed static data (pass --regen-data to regenerate) ==="
fi

NEAR_TC="--override-toolchain 1.86.0"

build_and_optimize() {
    local manifest="$1"
    local outdir="$2"
    local extra="${3:-}"

    cargo near build non-reproducible-wasm $NEAR_TC --manifest-path "$manifest" $extra --out-dir "$outdir"

    # Re-optimize with wasm-opt -O4 (speed) on top of cargo-near's built-in -O (size).
    # --strip-producers --vacuum are required for nearcore wasm validation correctness.
    local wasm_file
    wasm_file=$(find "$outdir" -name '*.wasm' | head -1)
    if [ -n "$wasm_file" ] && command -v wasm-opt &>/dev/null; then
        echo "  -> Optimizing $(basename "$wasm_file") with wasm-opt -O4..."
        wasm-opt -O4 --strip-debug --strip-producers --vacuum "$wasm_file" -o "$wasm_file"
    fi
}

echo "=== Building chess contract ==="
build_and_optimize ./crates/chess/Cargo.toml ./res

echo "=== Building test-token contract ==="
build_and_optimize ./crates/test-token/Cargo.toml ./res

echo "=== Building chess contract (integration-test) ==="
tmpdir=$(mktemp -d)
build_and_optimize ./crates/chess/Cargo.toml "$tmpdir" "--features integration-test"
mv "$tmpdir/chess.wasm" res/chess_testing.wasm
rm -rf "$tmpdir"

echo "=== Done ==="
ls -lh res/*.wasm
