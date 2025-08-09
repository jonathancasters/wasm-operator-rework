#!/bin/bash

set -e

mkdir -p target

echo "Compiling Rust ring-operator component..."

cargo build --release --target wasm32-wasip2

cp target/wasm32-wasip2/release/ring_operator.wasm target/main.wasm
