# Wasm Operator: A Dynamic, Language-Agnostic Kubernetes Operator

## Core Concept

This project implements a Kubernetes operator pattern where the core reconciliation logic is not hardcoded into the operator itself. Instead, the operator acts as a secure host environment that dynamically loads and executes business logic packaged as WebAssembly (Wasm) modules.

This approach decouples the operator's generic Kubernetes-interacting machinery from the specific logic it needs to run. Developers can write their custom logic in various programming languages (like Go, as seen in the `children/` directory), compile it to Wasm, and have the Rust-based parent operator execute it in a sandboxed Wasmtime runtime. 

The communication and capabilities of these Wasm modules are strictly defined by a contract using WebAssembly Interface Type (WIT) files, located in the `wit/` directory. This ensures that the guest modules can only perform actions explicitly exposed by the host operator (e.g., making specific Kubernetes API calls, logging), providing a strong security boundary.

In essence, this project provides a framework for building language-agnostic, secure, and dynamically updatable Kubernetes operators.

## Project Structure

- `parent/`: The main operator code, written in Rust. It uses `wasmtime` to run Wasm modules and interacts with the Kubernetes API. This is the **host**.
- `children/`: Contains the Wasm modules, written in Go. These are the **guests** that contain the business logic.
- `examples/`: Contains example configurations and scripts for running the operator.
- `parent/src/wit/`: Contains the WIT (WebAssembly Interface Type) definitions for the interface between the parent (host) and child (guest) modules.

## Technologies

- **Rust:** The parent operator is written in Rust, using the `wasmtime` library as a Wasm runtime.
- **Go:** The child Wasm modules are written in Go.
- **WebAssembly (Wasm):** The operator is designed to run Wasm modules.
- **Kubernetes:** The operator is designed to run in a Kubernetes environment.
- **Nix:** The project uses Nix for its development environment.

## Development

The development environment is managed by Nix and `devshell`. To get started, run `nix develop`.

---

## Gemini CLI Progress Update

This section details the progress made and the current challenges faced during the development of the Wasm Operator using the Gemini CLI.

### Current Findings:

*   **WIT Interface Refinement:** The WIT interface has been successfully refined and restructured into three separate files:
    *   `parent/wit/world.wit`: Defines the `kube-operator` world, importing necessary types and interfaces.
    *   `parent/wit/types.wit`: Contains common type definitions such as `reconcile-request`, `reconcile-result`, `event-type`, and `log-level`.
    *   `parent/wit/kubernetes.wit`: Defines the `kubernetes` interface, which outlines the host functions available to Wasm modules (e.g., logging, CRUD operations on Kubernetes resources).
    *   Challenges were encountered with `wit-bindgen` syntax for `use` statements and `variant` types, but the current structure is now accepted by the compiler.

*   **Idle Module Unloading (Rust Parent Operator):** Significant progress has been made on implementing the core memory-saving feature:
    *   **`OperatorState` Enum:** Defined to track the state of each operator instance (`Loaded` or `Unloaded`).
        *   `Loaded` state now includes the `bindings::KubeOperator` instance, a `std::sync::Mutex<wasmtime::Store<State>>` to manage the Wasmtime store, a `last_active` timestamp, and `WasmComponentMetadata`.
        *   `Unloaded` state stores the `state_path` (path to serialized memory) and `WasmComponentMetadata`.
    *   **`WasmRuntime` Struct:** Manages all operator instances using a `dashmap::DashMap<OperatorId, OperatorState>`.
    *   **`run_components` Function:** Initializes operators, loads them into the `DashMap` in the `Loaded` state, and spawns a background `idle_check_loop`. It also includes a test call to the `reconcile` function.
    *   **`idle_check_loop` Function:** Periodically iterates through operators, identifies idle ones based on `IDLE_THRESHOLD`, and triggers their unloading via `unload_component`.
    *   **`unload_component` Function:** Partially implemented. It transitions an operator's state to `Unloaded` and prepares for memory serialization. The actual memory access and serialization logic is currently commented out due to ongoing challenges.
    *   **`with_operator` Function:** A helper designed to safely access an operator's `KubeOperator` and `Store`. It handles reloading an operator if it's in the `Unloaded` state. This function has been the primary source of persistent compilation errors related to Rust's ownership, lifetimes, and `Send` trait requirements, particularly concerning the `MutexGuard` and the `KubeOperator`'s lack of `Clone` implementation.

### Current Challenges & Next Steps:

1.  **Resolve Lifetime and `Send` Issues in `with_operator`:** This remains the most critical blocking issue. The current approach of passing `&KubeOperator` and `&mut Store<State>` into an `async move` closure within `tokio::spawn` is causing lifetime and `Send` violations.
    *   **Next Step:** Re-evaluate the design of `with_operator` to find a robust pattern that allows safe access to the `KubeOperator` and `Store` across `await` points without requiring `KubeOperator` to be `Clone` or violating `MutexGuard`'s `Send` constraints. This might involve restructuring how the `KubeOperator` and `Store` are managed or passed.

2.  **Implement Memory Serialization/Deserialization:** Once `with_operator` and `unload_component` can safely access the `Store` and `KubeOperator`, the next step is to complete the memory management:
    *   **Next Step:** Implement the actual serialization of the Wasm instance's linear memory to disk (e.g., using `bincode` or `serde_json` with `std::fs::write`).
    *   **Next Step:** Implement the deserialization of the saved state back into a newly instantiated Wasm module's memory (using `std::fs::read` and `memory.write`). This requires correctly identifying and accessing the Wasm `Memory` object from the `KubeOperator` instance.

3.  **Complete Reloading Logic in `with_operator`:** Fully implement the `Unloaded` branch of `with_operator` to ensure that an operator is correctly reloaded from its saved state before the provided closure is executed.

4.  **Design Cross-Language Benchmarking Example:** Once the core idle unloading/reloading mechanism is fully functional and stable, the final step is to create the `examples/ring-benchmark` directory and implement the ring operator logic in both Rust and Go, as outlined in the initial detailed guide. This will serve as a benchmark and demonstration of the framework's capabilities.
