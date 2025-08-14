#!/bin/bash

set -e

echo "Compiling Go ring-operator component..."

# Generate the bindings
go mod tidy
go generate

mkdir -p target

# Compile the wasm module and output to the central build directory
tinygo build -target=wasip2 --wit-package ../../../parent/wit --wit-world child-world -o target/ring-operator-go.wasm main.go
