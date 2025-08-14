#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Get the directory of this script so that we can use absolute paths
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# --- Configuration ---
OPERATOR_COUNTS="10"
RUNS_PER_COUNT=1
OPERATOR_TYPE="mixed" # mixed, go, or rust
ACTIVE_DURATION=60 # 1 minute for testing
IDLE_DURATION=30   # 30 seconds for testing
OUTPUT_DIR="${SCRIPT_DIR}/results"
CLUSTER_NAME="benchmark"
VENV_DIR="${SCRIPT_DIR}/.venv" # Use .venv for convention
K8S_DIR="${SCRIPT_DIR}/k8s"

# --- Functions ---

# Function to print usage
usage() {
    echo "Usage: $0 [--operator-counts \"10 20 30\"] [--runs-per-count 5] [--operator-type <mixed|go|rust>] [--active-duration 420] [--idle-duration 120] [--output-dir ./results]"
    exit 1
}

# Function to handle cleanup
cleanup() {
    echo "--- Cleaning up kind cluster ---"
    kind delete cluster --name "${CLUSTER_NAME}" || true
    echo "--- Cleanup complete ---"
}

# Function to check for prerequisites
check_prereqs() {
    echo "--- Checking for prerequisites ---"
    for cmd in kind kubectl docker helm python3; do
        if ! command -v "$cmd" &> /dev/null; then
            echo "Error: $cmd is not installed." >&2
            exit 1
        fi
    done
    echo "All system prerequisites are installed."
}

# Function to set up and manage the Python virtual environment
setup_python_venv() {
    echo "--- Setting up Python virtual environment in ${VENV_DIR} ---"
    if [ ! -d "${VENV_DIR}" ]; then
        echo "Creating virtual environment..."
        python3 -m venv "${VENV_DIR}"
    fi
    VENV_PYTHON="${VENV_DIR}/bin/python3"
    echo "Upgrading pip..."
    "$VENV_PYTHON" -m pip install --upgrade pip > /dev/null
    echo "Installing dependencies from requirements.txt..."
    "$VENV_PYTHON" -m pip install --index-url https://pypi.org/simple -r "${SCRIPT_DIR}/requirements.txt"
    echo "--- Python virtual environment is ready ---"
}

# Proactively clean up resources from previous runs to ensure idempotency
pre_run_cleanup() {
    echo "--- Performing pre-run cleanup of Kubernetes resources ---"
    kubectl delete -f "${K8S_DIR}/deployment.yaml" --ignore-not-found=true
    kubectl delete configmap parent-config --ignore-not-found=true
    kubectl delete -f "${K8S_DIR}/rbac.yaml" --ignore-not-found=true
    kubectl delete -f "${K8S_DIR}/testresource-crd.yaml" --ignore-not-found=true
    kubectl delete ns -l benchmark-ns=true --ignore-not-found=true
    echo "--- Pre-run cleanup complete ---"
}

# Function to set up the kind cluster
setup_cluster() {
    echo "--- Setting up kind cluster ---"
    if ! kubectl cluster-info --context "kind-${CLUSTER_NAME}" >/dev/null 2>&1;
 then
        echo "Cluster '${CLUSTER_NAME}' not found or not responsive. Deleting remnants and creating a new one..."
        kind delete cluster --name "${CLUSTER_NAME}" || true
        kind create cluster --name "${CLUSTER_NAME}"
    else
        echo "kind cluster '${CLUSTER_NAME}' is already running and responsive."
    fi
}

# Function to deploy Prometheus and Grafana using Helm
deploy_prometheus() {
    echo "--- Deploying kube-prometheus-stack ---"
    if ! helm status kube-prometheus-stack -n monitoring >/dev/null 2>&1;
 then
        helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
        helm repo update
        helm install kube-prometheus-stack prometheus-community/kube-prometheus-stack \
            --create-namespace \
            --namespace monitoring
    else
        echo "kube-prometheus-stack is already deployed."
    fi

    echo "Waiting for Prometheus and Grafana pods to be ready..."
    kubectl wait --for=condition=Ready pod -n monitoring -l app.kubernetes.io/name=grafana --timeout=300s
    kubectl wait --for=condition=Ready pod -n monitoring -l app.kubernetes.io/name=prometheus --timeout=300s
    echo "Prometheus and Grafana are ready."
}

# Function to build Wasm modules and the parent operator Docker image
build_and_load_images() {
    echo "--- Building and loading images ---"
    echo "Creating central build artifact directory..."
    mkdir -p "${SCRIPT_DIR}/build"
    echo "Building Rust operator..."
    (cd "${SCRIPT_DIR}/operators/rust-operator" && ./compile.sh)
    cp "${SCRIPT_DIR}/operators/rust-operator/target/ring-operator-rust.wasm" "${SCRIPT_DIR}/build/rust-operator.wasm"
    echo "Building Go operator..."
    (cd "${SCRIPT_DIR}/operators/go-operator" && ./compile.sh)
    cp "${SCRIPT_DIR}/operators/go-operator/target/ring-operator-go.wasm" "${SCRIPT_DIR}/build/go-operator.wasm"
    echo "Building parent operator image..."
    docker build -t wasm-operator-rework:latest -f "${SCRIPT_DIR}/../Dockerfile" "${SCRIPT_DIR}/.."
    echo "Loading image into kind..."
    kind load docker-image wasm-operator-rework:latest --name "${CLUSTER_NAME}"
}

# Function to dynamically generate and deploy the parent operator's config
deploy_parent_operator_config() {
    local count=$1
    local op_type=$2
    echo "--- Generating and deploying parent operator config for ${count} operators of type '${op_type}' ---"

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
                ;;
            go)
                wasm_module="go-operator.wasm"
                ;;
            rust)
                wasm_module="rust-operator.wasm"
                ;;
        esac

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

# Function to run a single test iteration
run_test_iteration() {
    local count=$1
    local run=$2
    echo "--- Deploying for ${count} operators, run #${run} ---"

    for i in $(seq 1 "${count}"); do
        kubectl create ns "ns-${i}" || true
        kubectl label ns "ns-${i}" benchmark-ns=true || true
    done
    kubectl create ns "ns-final" || true
    kubectl label ns "ns-final" benchmark-ns=true || true

    echo "Applying Kubernetes manifests from ${K8S_DIR}..."
    kubectl apply -f "${K8S_DIR}/testresource-crd.yaml"
    kubectl apply -f "${K8S_DIR}/rbac.yaml"

    deploy_parent_operator_config "${count}" "${OPERATOR_TYPE}"

    kubectl apply -f "${K8S_DIR}/deployment.yaml"

    echo "Waiting for parent operator to be ready..."
    kubectl wait --for=condition=Available deployment/parent-operator --timeout=120s

    echo "--- Executing test driver for ${count} operators, run #${run} ---"
    "${VENV_DIR}/bin/python3" "${SCRIPT_DIR}/test_driver.py" --operator-count "${count}" --run-number "${run}" --output-dir "${OUTPUT_DIR}" --active-duration "${ACTIVE_DURATION}" --idle-duration "${IDLE_DURATION}"
}

# Function to collect metrics from Prometheus
collect_metrics() {
    local count=$1
    local run=$2
    echo "--- Collecting metrics for ${count} operators, run #${run} ---"
    mkdir -p "${OUTPUT_DIR}"
    echo "Port-forwarding to Prometheus..."
    kubectl port-forward -n monitoring svc/kube-prometheus-stack-prometheus 9090:9090 &
    PROMETHEUS_PID=$!
    sleep 5
    PROMQL_QUERY='container_memory_working_set_bytes{container="parent-operator"}'
    echo "Querying Prometheus..."
    curl -s -g "http://localhost:9090/api/v1/query?query=${PROMQL_QUERY}" > "${OUTPUT_DIR}/memory_N${count}_run${run}.json"
    echo "Stopping port-forward..."
    kill "${PROMETHEUS_PID}"
}

# Function to clean up resources after a test iteration
cleanup_iteration() {
    echo "--- Cleaning up iteration resources ---"
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
        *) usage ;;
    esac
    shift
done

check_prereqs
setup_python_venv
setup_cluster
pre_run_cleanup
deploy_prometheus
build_and_load_images

for count in ${OPERATOR_COUNTS}; do
    for run in $(seq 1 "${RUNS_PER_COUNT}"); do
        echo "--- Running test for ${count} operators, run #${run} ---"
        run_test_iteration "${count}" "${run}"
        collect_metrics "${count}" "${run}"
        cleanup_iteration
        echo "--- Iteration for ${count} operators, run #${run} complete ---"
    done
done

echo "--- Benchmark suite complete ---"