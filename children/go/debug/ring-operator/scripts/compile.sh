#!/bin/bash

set -e

mkdir -p target

echo "Compiling Go ring-operator component..."

# Generate the bindings
go mod tidy
go generate

# Compile the wasm module
tinygo build -target=wasip2 -o target/main.wasm --wit-package ./wit --wit-world operator main.go
