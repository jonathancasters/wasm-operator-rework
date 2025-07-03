#!/usr/bin/env bash

set -euo pipefail

CLUSTER_NAME="wasm-test-cluster"
CONFIG_PATH="./test-data/empty-config.yaml"
MANIFEST_PATH="../../../parent/Cargo.toml"

cleanup() {
    echo "🧹 Cleaning up..."
    if kind get clusters | grep -q "$CLUSTER_NAME"; then
        kind delete cluster --name "$CLUSTER_NAME"
    fi
    rm -f "$CONFIG_PATH"
    echo "✅ Cleanup complete."
}
trap cleanup EXIT

echo "🔧 Checking for kind..."
if ! command -v kind &>/dev/null; then
  echo "❌ 'kind' not found. Please install it first: https://kind.sigs.k8s.io/"
  exit 1
fi

echo "🔄 Deleting any existing cluster named $CLUSTER_NAME..."
kind delete cluster --name "$CLUSTER_NAME" || true

echo "🚀 Creating kind cluster named $CLUSTER_NAME..."
kind create cluster --name "$CLUSTER_NAME"

echo "🔍 Waiting for kube-system pods to be ready..."
kubectl wait --for=condition=Ready pod --all -n kube-system --timeout=60s || true

echo "🚀 Applying pod configuration..."
kubectl apply -f ./configuration.yaml

echo "⏳ Waiting for test pods to be ready..."
kubectl wait --for=condition=Ready pod/test-pod-1 pod/test-pod-2 --timeout=60s || true

echo "📦 Creating dummy config..."
mkdir -p ./test-data
: > "$CONFIG_PATH"

echo "📡 Running Rust application via cargo..."
KUBECONFIG="$(kind get kubeconfig-path --name=$CLUSTER_NAME 2>/dev/null)" \
cargo run --manifest-path "$MANIFEST_PATH" -- --debug "$CONFIG_PATH"

echo "✅ Test complete."
