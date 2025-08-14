#!/bin/bash
set -e

# Define benchmark parameters
OPERATOR_COUNTS="10 20 30 40 50 60 70 80 90 100"
RUNS_PER_COUNT=3
ACTIVE_DURATION=60 # 7 minutes
IDLE_DURATION=30 # 300 # 5 minutes
BASE_OUTPUT_DIR="./results"

SESSION_DIR="${BASE_OUTPUT_DIR}/$(date +%Y-%m-%d_%H-%M-%S)"

echo "--- 🚀 Starting Benchmark Suite --- "
echo "--- Results will be stored in ${SESSION_DIR} ---"

# --- Scenario 1: Mixed Operators (Default) ---
echo "
---
--- 🚀 Starting Mixed Operator Benchmark Scenario
---
"
export OPERATOR_TYPE="mixed"
./benchmark.sh \
  --operator-counts "${OPERATOR_COUNTS}" \
  --runs-per-count ${RUNS_PER_COUNT} \
  --active-duration ${ACTIVE_DURATION} \
  --idle-duration ${IDLE_DURATION} \
  --output-dir "${SESSION_DIR}/mixed"

# --- Scenario 2: Rust-Only Operators ---
echo "
---
--- 🚀 Starting Rust-Only Benchmark Scenario
---"
export OPERATOR_TYPE="rust"
./benchmark.sh \
  --operator-counts "${OPERATOR_COUNTS}" \
  --runs-per-count ${RUNS_PER_COUNT} \
  --active-duration ${ACTIVE_DURATION} \
  --idle-duration ${IDLE_DURATION} \
  --output-dir "${SESSION_DIR}/rust-only"

# --- Scenario 3: Go-Only Operators ---
echo "
---
--- 🚀 Starting Go-Only Benchmark Scenario
---"
export OPERATOR_TYPE="go"
./benchmark.sh \
  --operator-counts "${OPERATOR_COUNTS}" \
  --runs-per-count ${RUNS_PER_COUNT} \
  --active-duration ${ACTIVE_DURATION} \
  --idle-duration ${IDLE_DURATION} \
  --output-dir "${SESSION_DIR}/go-only"

echo "
---
--- 🎉 All benchmark scenarios complete!
---"
echo "Results are stored in the ${SESSION_DIR} directory."

