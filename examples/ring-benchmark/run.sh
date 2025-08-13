#!/usr/bin/env bash

set -euo pipefail

# Get the directory of this script so that we can use absolute paths
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

CLUSTER_NAME="ring-benchmark-cluster"
CONFIG_PATH="$SCRIPT_DIR/configuration.yaml"
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
kind create cluster --name "$CLUSTER_NAME" --config "$SCRIPT_DIR/kind-config.yaml"

echo "🔍 Waiting for kube-system pods to be ready..."
kubectl wait --for=condition=Ready pod --all -n kube-system --timeout=60s || true

# 1. Setup
echo "Creating CRD..."
kubectl apply -f "$SCRIPT_DIR/crd/rings.example.com.crd.yaml"

echo "Applying RBAC rules..."
kubectl apply -f "$SCRIPT_DIR/rbac.yaml"

echo "Creating namespaces..."
kubectl create namespace ring-a || true
kubectl create namespace ring-b || true

# 2. Build and deploy operators
echo "--- Building Rust operator component ---"
(cd "$SCRIPT_DIR/rust-operator" && ./compile.sh)

echo "--- Building Go operator component ---"
(cd "$SCRIPT_DIR/go-operator" && ./compile.sh)

# This assumes your parent operator's Dockerfile is in the repo root
mkdir -p "$SCRIPT_DIR/../../build"
cp "$SCRIPT_DIR/rust-operator/target/main.wasm" "$SCRIPT_DIR/../../build/ring-operator-rust.wasm"
cp "$SCRIPT_DIR/go-operator/target/main.wasm" "$SCRIPT_DIR/../../build/ring-operator-go.wasm"

docker build -t wasm-operator-rework:latest "$SCRIPT_DIR/../../"

echo "--- Loading image into kind ---"
kind load docker-image wasm-operator-rework:latest --name "$CLUSTER_NAME"

kubectl create configmap wasm-operator-config --from-file="$SCRIPT_DIR/configuration.yaml"

# 3. Execution
echo "Starting parent operator..."
kubectl apply -f "$SCRIPT_DIR/deployment.yaml"

echo "Waiting for operator to be ready..."
kubectl rollout status deployment/wasm-operator --timeout=60s

# 4. Create initial resource
echo "Creating initial Ring..."
cat <<EOF | kubectl apply -f -
apiVersion: "example.com/v1"
kind: "Ring"
metadata:
  name: "initial-ring"
  namespace: "ring-a"
spec:
  targetNamespace: "ring-b"
EOF

# 5. Verification
echo "Waiting for reconciliation..."
sleep 10 # Give the operators time to reconcile

echo "Verifying that the ring was passed..."
if kubectl get ring initial-ring -n ring-b; then
    echo "SUCCESS: Ring was passed to the next namespace!"
else
    echo "FAILURE: Ring was not passed to the next namespace."
fi