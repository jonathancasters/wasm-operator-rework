# Scientific Benchmarking Suite

This directory contains a fully automated, scientific benchmarking suite for the wasm-operator-rework project.

## Overview

The benchmark is designed to replicate the methodology described in the foundational master dissertation. It measures the performance (latency and memory usage) of the parent operator under various loads and scenarios.

## How to Run Benchmarks

The main entry point for the benchmark is the `run_all_benchmarks.sh` script. This script will execute all the benchmark scenarios required for a comprehensive analysis.

```bash
./run_all_benchmarks.sh
```

### Benchmark Scenarios

The `run_all_benchmarks.sh` script executes the following three scenarios:

1.  **Mixed Operators (`mixed`)**: This is the default scenario. It deploys an equal mix of Rust and Go operators to simulate a heterogeneous environment.
2.  **Rust-Only Operators (`rust-only`)**: This scenario deploys only Rust-based operators. This is useful for measuring the performance characteristics of the framework when running Rust-based logic.
3.  **Go-Only Operators (`go-only`)**: This scenario deploys only Go-based operators. This is useful for measuring the performance characteristics of the framework when running Go-based logic.

### Configuration

The benchmark parameters (e.g., operator counts, run duration) can be configured at the top of the `run_all_benchmarks.sh` script.

### Individual Scenarios

It is also possible to run a single benchmark scenario by directly using the `benchmark.sh` script and setting the `OPERATOR_TYPE` environment variable.

```bash
export OPERATOR_TYPE=rust-only
./benchmark.sh --output-dir ./results/rust-only
```

## Output

The benchmarks will produce a structured directory layout in the `results` directory (or the directory specified with `--output-dir`). Each run of the `run_all_benchmarks.sh` script will create a new session directory named with a timestamp (e.g., `results/2025-08-14_15-30-00`).

Inside each session directory, each scenario will have its own subdirectory (e.g., `mixed`, `rust-only`, `go-only`).

Inside each scenario directory, you will find the following files:

*   `latency.csv`: Contains raw, per-event latency measurements for all runs of the scenario.
    *   **Columns**: `operator_count`, `run_number`, `latency_ms`
*   `memory.csv`: Contains raw, time-series memory usage data for all runs of the scenario.
    *   **Columns**: `operator_count`, `run_number`, `phase`, `timestamp`, `memory_bytes`

## Visualization

After running the benchmarks, you can generate plots from the collected data using the `visualize.py` script.

```bash
python3 visualize.py ./results
```

This will analyze all the data in the `results` directory and generate the following plots in the current directory:

*   `memory_active_vs_idle.png`
*   `memory_rust_vs_go.png`
*   `latency_rust_vs_go.png`
*   `memory_scalability_all.png`
*   `latency_scalability_all.png`