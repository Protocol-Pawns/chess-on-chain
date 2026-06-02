#!/bin/bash
set -e
cd "`dirname $0`"

mkdir -p res

NEAR_TC="--override-toolchain 1.86.0"

cargo near build non-reproducible-wasm $NEAR_TC --manifest-path ./crates/chess/Cargo.toml --out-dir ./res
cargo near build non-reproducible-wasm $NEAR_TC --manifest-path ./crates/test-token/Cargo.toml --out-dir ./res
cargo near build non-reproducible-wasm $NEAR_TC --manifest-path ./crates/nada-bot-stub/Cargo.toml --out-dir ./res

tmpdir=$(mktemp -d)
cargo near build non-reproducible-wasm $NEAR_TC --manifest-path ./crates/chess/Cargo.toml --features integration-test --out-dir "$tmpdir"
mv "$tmpdir/chess.wasm" res/chess_testing.wasm
rm -rf "$tmpdir"
