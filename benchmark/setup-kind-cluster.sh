#!/bin/bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Create Kind cluster
kind create cluster --config "$SCRIPT_DIR/kind-config.yaml"

# Install Prometheus
kubectl apply -f "$SCRIPT_DIR/prometheus/prometheus-deployment.yaml"