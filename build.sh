#!/bin/bash
set -e
cd "`dirname $0`"
rm -rf target/wit || true
raen build -p chess --release --standards
cp target/res/*.wasm ./res/
cp -r target/wit/chess/* ./wit/
wasm-opt -O4 res/chess.wasm -o res/chess.wasm --strip-debug --vacuum
