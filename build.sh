#!/bin/bash
set -e
cd "`dirname $0`"

# rm -rf target/wit || true
# raen build -p chess --release --standards --channel nightly-2023-03-20
cargo build --release -p chess --target wasm32-unknown-unknown
cargo build --release -p test-token --target wasm32-unknown-unknown
# cp target/res/*.wasm ./res/
cp target/wasm32-unknown-unknown/release/*.wasm ./res/
# cp -r target/wit/chess/* ./wit/

cargo build --release -p chess --target wasm32-unknown-unknown --features=integration-test
cp target/wasm32-unknown-unknown/release/chess.wasm ./res/chess_testing.wasm

wasm-opt -O4 res/chess.wasm -o res/chess.wasm --strip-debug --vacuum
wasm-opt -O4 res/chess_testing.wasm -o res/chess_testing.wasm --strip-debug --vacuum
wasm-opt -O4 res/test_token.wasm -o res/test_token.wasm --strip-debug --vacuum
