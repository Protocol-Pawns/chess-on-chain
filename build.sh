#!/bin/bash
set -e
cd "`dirname $0`"

cargo build --release -p chess --target wasm32-unknown-unknown
cargo build --release -p test-token --target wasm32-unknown-unknown
cargo build --release -p iah-registry-stub --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

cargo near abi --manifest-path ./crates/chess/Cargo.toml
cp target/near/chess/chess_abi.json ./res/

cargo build --release -p chess --target wasm32-unknown-unknown --features=integration-test
cp target/wasm32-unknown-unknown/release/chess.wasm ./res/chess_testing.wasm

wasm-opt -O4 res/chess.wasm -o res/chess.wasm --strip-debug --vacuum
wasm-opt -O4 res/chess_testing.wasm -o res/chess_testing.wasm --strip-debug --vacuum
wasm-opt -O4 res/test_token.wasm -o res/test_token.wasm --strip-debug --vacuum
wasm-opt -O4 res/iah_registry_stub.wasm -o res/iah_registry_stub.wasm --strip-debug --vacuum
