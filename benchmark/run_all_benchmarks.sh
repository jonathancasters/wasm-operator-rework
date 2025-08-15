#!/bin/bash
set -e

#!/bin/bash
set -e

# --- Configuration ---
OPERATOR_COUNTS="10 20 30 40 50 60 70 80 90 100"
RUNS_PER_COUNT=3
ACTIVE_DURATION=90
IDLE_DURATION=45
BASE_OUTPUT_DIR="./results"
RESUME_FROM=""

# --- Argument Parsing ---
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --operator-counts)
            OPERATOR_COUNTS="$2"
            shift
            ;;
        --runs-per-count)
            RUNS_PER_COUNT="$2"
            shift
            ;;
        --active-duration)
            ACTIVE_DURATION="$2"
            shift
            ;;
        --idle-duration)
            IDLE_DURATION="$2"
            shift
            ;;
        --resume-from)
            RESUME_FROM="$2"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
    shift
done

# --- Session Initialization ---
if [ -n "$RESUME_FROM" ]; then
    SESSION_DIR="$RESUME_FROM"
    echo "--- 🚀 Resuming Benchmark Suite from ${SESSION_DIR} ---"
else
    SESSION_DIR="${BASE_OUTPUT_DIR}/$(date +%Y-%m-%d_%H-%M-%S)"
    echo "--- 🚀 Starting New Benchmark Suite --- "
    echo "--- Results will be stored in ${SESSION_DIR} --- "
    mkdir -p "$SESSION_DIR"
fi

STATE_FILE="${SESSION_DIR}/.benchmark_state.json"

if [ ! -f "$STATE_FILE" ]; then
    echo '{"mixed": {}, "rust": {}, "go": {}}' > "$STATE_FILE"
fi

# --- Functions ---
get_completed_runs() {
    local scenario=$1
    local op_count=$2
    jq -r --arg scenario "$scenario" --arg op_count "$op_count" '.[$scenario][$op_count] // 0' "$STATE_FILE"
}

update_state() {
    local scenario=$1
    local op_count=$2
    local run=$3
    jq --arg scenario "$scenario" --arg op_count "$op_count" --argjson run "$run" '.[$scenario][$op_count] = $run' "$STATE_FILE" > "${STATE_FILE}.tmp" && mv "${STATE_FILE}.tmp" "$STATE_FILE"
}

# --- Scenario Execution ---
SCENARIOS=("mixed" "rust" "go")

for scenario in "${SCENARIOS[@]}"; do
    echo "
---
--- 🚀 Starting ${scenario} Benchmark Scenario
---"
    export OPERATOR_TYPE="$scenario"
    
    for count in ${OPERATOR_COUNTS}; do
        completed_runs=$(get_completed_runs "$scenario" "$count")

        if [ "$completed_runs" -gt 0 ]; then
            echo "ℹ️  Resuming ${scenario} for ${count} operators. ${completed_runs} of ${RUNS_PER_COUNT} runs already complete."
        fi

        if [ "$completed_runs" -ge "$RUNS_PER_COUNT" ]; then
            echo "✅ Skipping ${count} operators, all runs completed."
            continue
        fi

        for run in $(seq $((completed_runs + 1)) "${RUNS_PER_COUNT}"); do
            ./benchmark.sh \
              --operator-counts "${count}" \
              --run-number "${run}" \
              --active-duration ${ACTIVE_DURATION} \
              --idle-duration ${IDLE_DURATION} \
              --output-dir "${SESSION_DIR}/${scenario}"
            
            update_state "$scenario" "$count" "$run"
        done
    done
done

echo "
---
--- 🎉 All benchmark scenarios complete!
---"
echo "Results are stored in the ${SESSION_DIR} directory."

