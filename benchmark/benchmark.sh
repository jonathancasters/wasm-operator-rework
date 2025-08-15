#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Get the directory of this script so that we can use absolute paths
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# --- Configuration ---
OPERATOR_COUNTS="10"
RUNS_PER_COUNT=1
OPERATOR_TYPE=${OPERATOR_TYPE:-"mixed"} # mixed, go, or rust
ACTIVE_DURATION=60 # 1 minute for testing
IDLE_DURATION=30   # 30 seconds for testing
OUTPUT_DIR="${SCRIPT_DIR}/results"
CLUSTER_NAME="benchmark"
VENV_DIR="${SCRIPT_DIR}/.venv" # Use .venv for convention
K8S_DIR="${SCRIPT_DIR}/k8s"
SKIP_SETUP=false
RUN_NUMBER_OVERRIDE=""

# --- Functions ---

# Function to print usage
usage() {
    echo "Usage: $0 [--operator-counts \"10 20 30\"] [--runs-per-count 5] [--operator-type <mixed|go|rust>] [--active-duration 420] [--idle-duration 120] [--output-dir ./results] [--skip-setup]"
    exit 1
}

# Function to handle cleanup
cleanup() {
    echo "🧹 Cleaning up kind cluster..."
    kind delete cluster --name "${CLUSTER_NAME}" || true
    echo "✅ Cleanup complete."
}

# Function to check for prerequisites
check_prereqs() {
    echo "🔎 Checking for prerequisites..."
    for cmd in kind kubectl docker helm python3; do
        if ! command -v "$cmd" &> /dev/null; then
            echo "❌ Error: $cmd is not installed." >&2
            exit 1
        fi
    done
    echo "✅ All system prerequisites are installed."
}

# Function to set up and manage the Python virtual environment
setup_python_venv() {
    echo "🐍 Setting up Python virtual environment..."
    if [ ! -d "${VENV_DIR}" ]; then
        python3 -m venv "${VENV_DIR}"
    fi
    VENV_PYTHON="${VENV_DIR}/bin/python3"
    "$VENV_PYTHON" -m pip install --index-url https://pypi.org/simple --quiet --upgrade pip > /dev/null
    "$VENV_PYTHON" -m pip install --index-url https://pypi.org/simple --quiet -r "${SCRIPT_DIR}/requirements.txt"
    echo "✅ Python virtual environment is ready."
}

# Proactively clean up resources from previous runs to ensure idempotency
pre_run_cleanup() {
    echo "🧹 Performing pre-run cleanup of Kubernetes resources..."
    kubectl delete -f "${K8S_DIR}/deployment.yaml" --ignore-not-found=true
    kubectl delete configmap parent-config --ignore-not-found=true
    kubectl delete -f "${K8S_DIR}/rbac.yaml" --ignore-not-found=true
    kubectl delete -f "${K8S_DIR}/testresource-crd.yaml" --ignore-not-found=true
    kubectl delete ns -l benchmark-ns=true --ignore-not-found=true
}

# Function to set up the kind cluster
setup_cluster() {
    echo "📦 Setting up kind cluster..."
    if ! kubectl cluster-info --context "kind-${CLUSTER_NAME}" >/dev/null 2>&1;
 then
        echo "Cluster '${CLUSTER_NAME}' not found, creating a new one..."
        kind delete cluster --name "${CLUSTER_NAME}" || true
        kind create cluster --name "${CLUSTER_NAME}"
    else
        echo "✅ kind cluster '${CLUSTER_NAME}' is already running."
    fi
}

# Function to deploy Prometheus and Grafana using Helm
deploy_prometheus() {
    echo "📊 Deploying kube-prometheus-stack..."
    if ! helm status kube-prometheus-stack -n monitoring >/dev/null 2>&1;
 then
        helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
        helm repo update
        helm install kube-prometheus-stack prometheus-community/kube-prometheus-stack \
            --create-namespace \
            --namespace monitoring
    else
        echo "✅ kube-prometheus-stack is already deployed."
    fi

    echo "⏳ Waiting for Prometheus and Grafana pods to be ready..."
    kubectl wait --for=condition=Ready pod -n monitoring -l app.kubernetes.io/name=grafana --timeout=300s
    kubectl wait --for=condition=Ready pod -n monitoring -l app.kubernetes.io/name=prometheus --timeout=300s
    echo "✅ Prometheus and Grafana are ready."
}

# Function to build Wasm modules and the parent operator Docker image
build_and_load_images() {
    echo "🛠️ Building and loading images..."
    mkdir -p "${SCRIPT_DIR}/build"
    echo "    Building Rust operator..."
    (cd "${SCRIPT_DIR}/operators/rust-operator" && ./compile.sh)
    cp "${SCRIPT_DIR}/operators/rust-operator/target/ring-operator-rust.wasm" "${SCRIPT_DIR}/build/rust-operator.wasm"
    echo "    Building Go operator..."
    (cd "${SCRIPT_DIR}/operators/go-operator" && ./compile.sh)
    cp "${SCRIPT_DIR}/operators/go-operator/target/ring-operator-go.wasm" "${SCRIPT_DIR}/build/go-operator.wasm"
    echo "    Building parent operator image..."
    docker build -t wasm-operator-rework:latest -f "${SCRIPT_DIR}/../Dockerfile" "${SCRIPT_DIR}/.."
    echo "    Loading image into kind..."
    kind load docker-image wasm-operator-rework:latest --name "${CLUSTER_NAME}"
    echo "✅ Images built and loaded."
}

# Function to dynamically generate and deploy the parent operator's config
deploy_parent_operator_config() {
    local count=$1
    local op_type=$2
    echo "⚙️ Generating and deploying config for ${count} operators (${op_type})..."

    CONFIG_YAML="apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: parent-config\ndata:\n  configuration.yaml: |"

    for i in $(seq 1 "${count}"); do
        local wasm_module=""
        case "${op_type}" in
            mixed)
                if (( i % 2 == 0 )); then
                    wasm_module="go-operator.wasm"
                else
                    wasm_module="rust-operator.wasm"
                fi
                ;;            go)
                wasm_module="go-operator.wasm"
                ;;            rust)
                wasm_module="rust-operator.wasm"
                ;;        esac

        if [ "$i" -eq "${count}" ]; then
            ACTION_NAMESPACE="ns-final"
        else
            ACTION_NAMESPACE="ns-$((i + 1))"
        fi

        CONFIG_YAML+="\n    ---\n    name: operator-${i}"
        CONFIG_YAML+="\n    wasm: /app/wasm/${wasm_module}"
        CONFIG_YAML+="\n    env:"
        CONFIG_YAML+="\n    - name: WATCH_NAMESPACE"
        CONFIG_YAML+="\n      value: ns-${i}"
        CONFIG_YAML+="\n    - name: ACTION_NAMESPACE"
        CONFIG_YAML+="\n      value: ${ACTION_NAMESPACE}"
    done

    echo -e "${CONFIG_YAML}" | kubectl apply -f -
}

start_port_forward() {
    echo "🔗 Starting port-forward to Prometheus..."
    kubectl port-forward -n monitoring svc/kube-prometheus-stack-prometheus 9090:9090 >/dev/null 2>&1 &
    PF_PID=$!
    # Wait for the port-forward to be ready
    sleep 5
}

stop_port_forward() {
    echo "🔗 Stopping port-forward to Prometheus..."
    kill $PF_PID
}

# Function to run a single test iteration
run_test_iteration() {
    local count=$1
    local run=$2
    local operator_type=$3
    
    local latency_file="${OUTPUT_DIR}/latency.csv"
    local memory_file="${OUTPUT_DIR}/memory.csv"

    mkdir -p "${OUTPUT_DIR}"

    echo "🚀 Deploying for ${count} operators, run #${run}..."

    for i in $(seq 1 "${count}"); do
        kubectl create ns "ns-${i}" || true
        kubectl label ns "ns-${i}" benchmark-ns=true || true
    done
    kubectl create ns "ns-final" || true
    kubectl label ns "ns-final" benchmark-ns=true || true

    kubectl apply -f "${K8S_DIR}/testresource-crd.yaml"
    kubectl apply -f "${K8S_DIR}/rbac.yaml"

    deploy_parent_operator_config "${count}" "${operator_type}"

    kubectl apply -f "${K8S_DIR}/deployment.yaml"

    echo "⏳ Waiting for parent operator to be ready..."
    kubectl wait --for=condition=Available deployment/parent-operator --timeout=120s

    start_port_forward
    trap stop_port_forward RETURN

    # Run the test driver
    echo "🐍 Executing test driver..."
    "${VENV_DIR}/bin/python3" "${SCRIPT_DIR}/test_driver.py" \
        --operator-count "${count}" \
        --run-number "${run}" \
        --latency-file "${latency_file}" \
        --memory-file "${memory_file}" \
        --active-duration "${ACTIVE_DURATION}" \
        --idle-duration "${IDLE_DURATION}"

    stop_port_forward
}







# Function to clean up resources after a test iteration
cleanup_iteration() {
    echo "🧹 Cleaning up iteration resources..."
    kubectl delete -f "${K8S_DIR}/deployment.yaml" --ignore-not-found=true
    kubectl delete configmap parent-config --ignore-not-found=true
    kubectl delete -f "${K8S_DIR}/rbac.yaml" --ignore-not-found=true
    kubectl delete -f "${K8S_DIR}/testresource-crd.yaml" --ignore-not-found=true
    kubectl delete ns -l benchmark-ns=true --ignore-not-found=true
}


# --- Main Execution ---
trap 'cleanup' EXIT

while [[ "$#" -gt 0 ]]; do
    case $1 in
        --operator-counts) OPERATOR_COUNTS="$2"; shift ;;
        --runs-per-count) RUNS_PER_COUNT="$2"; shift ;;
        --operator-type) OPERATOR_TYPE="$2"; shift ;;
        --active-duration) ACTIVE_DURATION="$2"; shift ;;
        --idle-duration) IDLE_DURATION="$2"; shift ;;
        --output-dir) OUTPUT_DIR="$2"; shift ;;
        --skip-setup) SKIP_SETUP=false ;;
        --run-number) RUN_NUMBER_OVERRIDE="$2"; shift ;;
        *) usage ;;
    esac
    shift
done

check_prereqs
setup_python_venv
if [ "$SKIP_SETUP" = "false" ]; then
    setup_cluster
    deploy_prometheus
    build_and_load_images
fi

for count in ${OPERATOR_COUNTS}; do
    if [ -n "$RUN_NUMBER_OVERRIDE" ]; then
        runs_to_execute="$RUN_NUMBER_OVERRIDE"
    else
        runs_to_execute=$(seq 1 "${RUNS_PER_COUNT}")
    fi

    for run in ${runs_to_execute}; do
        echo "--- 🚀 Running test for ${count} operators, run #${run}, type '${OPERATOR_TYPE}' ---"
        run_test_iteration "${count}" "${run}" "${OPERATOR_TYPE}"
        cleanup_iteration
        echo "--- ✅ Iteration for ${count} operators, run #${run} complete ---"
    done
done

echo "--- 🎉 Benchmark suite complete! ---"