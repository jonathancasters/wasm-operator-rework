#!/bin/bash

set -e

echo "Compiling Rust ring-operator component..."

cargo build --release --target wasm32-wasip2

mkdir -p target

# Copy the final artifact to the central build directory
cp target/wasm32-wasip2/release/ring_operator_rust.wasm target/ring-operator-rust.wasm
