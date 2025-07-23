#!/bin/bash

set -e

mkdir -p target
echo "Compiling component..."
tinygo build -target=wasip2 -o target/hellok8s.wasm --wit-package ./wit --wit-world operator main.go