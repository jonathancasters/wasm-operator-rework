#!/bin/bash
set -e

# Configuration
RESULTS_DIR="results/$(date +%Y-%m-%d_%H-%M-%S)"
OPERATOR_COUNTS=(1 5 10 20 50 100)
RUNS_PER_SCENARIO=3
ACTIVE_DURATION=60 # seconds
IDLE_DURATION=30 # seconds

# Check for scenario
if [ -z "$1" ]; then
  echo "Usage: $0 <scenario>"
  echo "Available scenarios: rust-bare-metal, go-bare-metal"
  exit 1
fi
SCENARIO=$1

# Setup
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
RESULTS_DIR="$SCRIPT_DIR/$RESULTS_DIR"
VENV_DIR="$SCRIPT_DIR/.venv"

# Cleanup on exit
trap '"$SCRIPT_DIR/cleanup.sh"' EXIT

# Create Kind cluster
"$SCRIPT_DIR/setup-kind-cluster.sh"

# Compile operator and load image into Kind
if [ "$SCENARIO" == "rust-bare-metal" ]; then
  "$SCRIPT_DIR/operators/rust-operator-bare-metal/compile.sh"
  kind load docker-image ring-operator-bare-metal:latest
elif [ "$SCENARIO" == "go-bare-metal" ]; then
  "$SCRIPT_DIR/operators/go-operator-bare-metal/compile.sh"
  kind load docker-image ring-operator-bare-metal-go:latest
else
  echo "Invalid scenario: $SCENARIO"
  exit 1
fi

# Setup Python virtual environment
setup_python_venv() {
    echo " Setting up Python virtual environment..."
    if [ ! -d "${VENV_DIR}" ]; then
        python3 -m venv "${VENV_DIR}"
    fi
    VENV_PYTHON="${VENV_DIR}/bin/python3"
    "$VENV_PYTHON" -m pip install --index-url https://pypi.org/simple --quiet --upgrade pip > /dev/null
    "$VENV_PYTHON" -m pip install --index-url https://pypi.org/simple --quiet -r "${SCRIPT_DIR}/requirements.txt"
    echo "✅ Python virtual environment is ready."
}

# Setup Python virtual environment
setup_python_venv

# Run benchmark
mkdir -p "$RESULTS_DIR/$SCENARIO"
LATENCY_FILE="$RESULTS_DIR/$SCENARIO/latency.csv"
MEMORY_FILE="$RESULTS_DIR/$SCENARIO/memory.csv"

TEST_DRIVER=""
if [ "$SCENARIO" == "rust-bare-metal" ]; then
  TEST_DRIVER="$SCRIPT_DIR/test_driver_bare_metal.py"
elif [ "$SCENARIO" == "go-bare-metal" ]; then
  TEST_DRIVER="$SCRIPT_DIR/test_driver_bare_metal_go.py"
else
  echo "Invalid scenario: $SCENARIO"
  exit 1
fi

for run in $(seq 1 $RUNS_PER_SCENARIO); do
  for count in "${OPERATOR_COUNTS[@]}"; do
    echo "Running benchmark for $SCENARIO: $count operators, run $run"

    "$VENV_PYTHON" "$TEST_DRIVER" \
      --operator-count "$count" \
      --active-duration "$ACTIVE_DURATION" \
      --idle-duration "$IDLE_DURATION" \
      --latency-file "$LATENCY_FILE" \
      --memory-file "$MEMORY_FILE" \
      --run-number "$run" \
      --script-dir "$SCRIPT_DIR"

    echo "Finished benchmark for $SCENARIO: $count operators, run $run"
  done
done

echo "Benchmark finished. Results are in $RESULTS_DIR"