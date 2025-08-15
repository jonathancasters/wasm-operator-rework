# Ring Benchmark Example

This example demonstrates a cross-language benchmark for the Wasm Operator.
It implements the same reconciliation logic in both Rust and Go to create a "ring" of events between two namespaces.

## Overview

The purpose of this example is to serve as both a demonstration and a benchmark for the Wasm Operator framework. It showcases two different child operators (one written in Go, one in Rust) that perform the exact same logic, allowing for a direct comparison of their performance and behavior within the framework.

This example tests the full lifecycle and capabilities of the parent operator, including:
- Dynamically loading multiple Wasm modules based on a configuration file.
- The ability of child operators to request watches on specific Kubernetes resources.
- The parent operator's dynamic watcher system.
- The correctness of the host function implementation (calling the Kubernetes API from within Wasm).

## How it Works

The core of the benchmark is a simple, elegant feedback loop:

1.  **CRD:** A `Ring` Custom Resource Definition is created to represent the object that will be passed between namespaces.
2.  **Namespaces:** Two namespaces, `ring-a` and `ring-b`, are created to serve as the two halves of the ring.
3.  **Operators:** The parent Wasm operator is deployed and configured to load two child operators:
    -   The **Go operator** requests a watch on `Ring` resources in the `ring-a` namespace.
    -   The **Rust operator** requests a watch on `Ring` resources in the `ring-b` namespace.
4.  **Initiation:** The script creates an initial `Ring` resource named `initial-ring` in the `ring-a` namespace. This `Ring` has a `spec.targetNamespace` field set to `ring-b`.
5.  **The First Pass (Go):**
    -   The parent operator's watcher detects the new `Ring` in `ring-a`.
    -   It invokes the `reconcile` function in the **Go operator**, passing it the `Ring`'s data.
    -   The Go operator's logic creates a *new* `Ring` resource with the same name, but in the `ring-b` namespace. The `targetNamespace` of this new ring is set back to `ring-a`.
6.  **The Second Pass (Rust):**
    -   The parent operator's other watcher detects the new `Ring` in `ring-b`.
    -   It invokes the `reconcile` function in the **Rust operator**.
    -   The Rust operator's logic creates a new `Ring` back in the `ring-a` namespace, completing the circle.

This process would continue indefinitely, passing the `Ring` back and forth. The `run.sh` script verifies that the first pass was successful by checking for the existence of `initial-ring` in the `ring-b` namespace.

## Prerequisites

- Docker
- kind
- kubectl
- Rust
- TinyGo

## Usage

Run the `run.sh` script to set up the benchmark environment, run the test, and clean up.

```bash
./run.sh
```