#!/bin/bash
set -e
cd "`dirname $0`"

# rm -rf target/wit || true
# raen build -p chess --release --standards --channel nightly-2023-03-20
cargo build --release -p chess --target wasm32-unknown-unknown
# cp target/res/*.wasm ./res/
cp target/wasm32-unknown-unknown/release/*.wasm ./res/
# cp -r target/wit/chess/* ./wit/
wasm-opt -O4 res/chess.wasm -o res/chess.wasm --strip-debug --vacuum
