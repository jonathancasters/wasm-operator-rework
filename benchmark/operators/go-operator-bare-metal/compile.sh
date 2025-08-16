#!/bin/bash
set -e

# Build the operator
docker build -t ring-operator-bare-metal-go:latest .
