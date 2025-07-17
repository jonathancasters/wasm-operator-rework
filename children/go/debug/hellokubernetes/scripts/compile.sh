#!/bin/bash

set -e

echo "Building component..."
mkdir -p target
tinygo build -target=wasip2 -o target/hellok8s.wasm --wit-package ./wit --wit-world operator