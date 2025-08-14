# Scientific Benchmarking Suite

This directory contains a fully automated, scientific benchmarking suite for the wasm-operator-rework project.

## Overview

The benchmark is designed to replicate the methodology described in the foundational master dissertation. It measures the performance (latency and memory usage) of the parent operator under various loads.

## Usage

The main entry point for the benchmark is the `benchmark.sh` script.

### Parameters

*   `--operator-counts`: A list of operator counts to test (e.g., "10 20 30 ... 100"). Defaults to "10 20 30 40 50 60 70 80 90 100".
*   `--runs-per-count`: The number of independent test runs for each operator count. Defaults to 5.
*   `--active-duration`: The duration in seconds for the "active" phase of the test. Defaults to 420.
*   `--idle-duration`: The duration in seconds for the "idle" phase of the test. Defaults to 180.
*   `--output-dir`: A directory where all collected data will be stored. Defaults to `benchmark/results`.

### Example

```bash
./benchmark.sh --operator-counts "10 50 100" --runs-per-count 3
```

## Output Files

The benchmark generates the following output files in the specified output directory:

*   `latency_N<operator_count>_run<run_number>.csv`: Contains the end-to-end latencies for each full cycle of the ring for a specific run.
*   `memory_results.csv`: A master CSV file containing the 95th percentile memory usage for both the active and idle phases of each run.
