#!/usr/bin/env bash

set -euo pipefail

CLUSTER_NAME="wasm-test-cluster"
CONFIG_PATH="./configuration.yaml"
MANIFEST_PATH="../../../parent/Cargo.toml"
PARENT_PID=""

cleanup() {
    echo "🧹 Cleaning up..."
    if [ -n "$PARENT_PID" ]; then
        kill "$PARENT_PID" || true
    fi
    kubectl delete namespace ring-a --ignore-not-found=true || true
    kubectl delete namespace ring-b --ignore-not-found=true || true
    if kind get clusters | grep -q "$CLUSTER_NAME"; then
        kind delete cluster --name "$CLUSTER_NAME"
    fi
    echo "✅ Cleanup complete."
}
trap cleanup EXIT

echo "🔧 Checking for Docker..."
if ! docker info >/dev/null 2>&1; then
  echo "❌ Docker daemon is not running. Please start it first."
  exit 1
fi

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

# 1. Setup
echo "Creating CRD..."
kubectl apply -f ./crd.yaml

echo "Creating namespaces..."
kubectl create namespace ring-a || true
kubectl create namespace ring-b || true

# 2. Execution
echo "Starting parent operator..."
KUBECONFIG="$(kind get kubeconfig-path --name="$CLUSTER_NAME" 2>/dev/null)" \
cargo run --manifest-path "$MANIFEST_PATH" "$CONFIG_PATH" &
PARENT_PID=$!

sleep 5 # Give the operator time to start

# 3. Create initial resource
echo "Creating initial TestResource..."
cat <<EOF | kubectl apply -f -
apiVersion: "amurant.io/v1"
kind: "TestResource"
metadata:
  name: "initial-resource"
  namespace: "ring-a"
spec:
  nonce: 1
EOF

# 4. Verification
echo "Waiting for reconciliation..."
sleep 10 # Give the operators time to reconcile

echo "Verifying nonces..."
NONCE_A=$(kubectl get testresource initial-resource -n ring-a -o jsonpath='{.spec.nonce}')
NONCE_B=$(kubectl get testresource initial-resource -n ring-b -o jsonpath='{.spec.nonce}')

echo "Nonce in ring-a: $NONCE_A"
echo "Nonce in ring-b: $NONCE_B"

if [ "$NONCE_A" -eq 1 ] && [ "$NONCE_B" -eq 1 ]; then
    echo "SUCCESS: Nonces are correct!"
else
    echo "FAILURE: Nonces are incorrect."
fi
