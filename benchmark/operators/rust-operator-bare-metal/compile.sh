#!/bin/bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Build the Rust binary
TAG=$(git rev-parse --short HEAD)

# Build the operator
docker run --rm -it -v "$SCRIPT_DIR":/build -w /build rust:1.88.0 cargo build --release

# Build the Docker image
docker build -t ring-operator-bare-metal:latest "$SCRIPT_DIR"
docker tag ring-operator-bare-metal:latest ring-operator-bare-metal:$TAG
